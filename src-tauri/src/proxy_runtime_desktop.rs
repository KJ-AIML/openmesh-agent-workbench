//! Tauri lifecycle commands for the OpenMesh-owned proxy runtime.
//!
//! The HTTP implementation lives in `openmesh-core`; this module only owns
//! desktop lifecycle state and intentionally exposes no provider-specific
//! transport logic.

use openmesh_core::agent_engine::{AgentSecretStore, CascadingSecretStore};
use openmesh_core::proxy_server::{
    default_proxy_config_path, read_proxy_config, write_proxy_config, ProxyModelConfig,
    ProxyOAuthCredentialResolver, ProxyProviderProtocol, ProxyRoutingStrategy, ProxyServer,
    ProxyServerConfig, ProxyUpstreamConfig, ProxyUsageLog, ProxyUsageSink,
};
use openmesh_core::storage::{default_settings, read_global, Settings};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

struct DesktopProxyUsageSink;

impl ProxyUsageSink for DesktopProxyUsageSink {
    fn record(&self, log: &ProxyUsageLog) {
        if crate::usage_tracking::init_usage_db().is_err() {
            return;
        }
        let _ = crate::usage_tracking::record_proxy_request(
            crate::usage_tracking::ProxyRequestRecord {
                id: log.request_id.clone(),
                timestamp: log.observed_at.clone(),
                provider: log
                    .upstream_id
                    .clone()
                    .unwrap_or_else(|| "openmesh".to_string()),
                model: log.model.clone().unwrap_or_else(|| "unknown".to_string()),
                input_tokens: log.input_tokens.unwrap_or_default(),
                output_tokens: log.output_tokens.unwrap_or_default(),
                latency_ms: log.latency_ms,
                status_code: Some(log.status),
                account_label: log.account_id.clone(),
            },
        );
    }
}

#[derive(Default)]
pub struct BuiltInProxyManager {
    inner: Mutex<InnerState>,
}

struct InnerState {
    config: Option<ProxyServerConfig>,
    server: Option<ProxyServer>,
    endpoint: Option<String>,
    task: Option<
        tauri::async_runtime::JoinHandle<Result<(), openmesh_core::proxy_server::ProxyServerError>>,
    >,
    stop: Option<oneshot::Sender<()>>,
    running: Arc<AtomicBool>,
    starting: bool,
    error: Option<String>,
}

impl Default for InnerState {
    fn default() -> Self {
        Self {
            config: None,
            server: None,
            endpoint: None,
            task: None,
            stop: None,
            running: Arc::new(AtomicBool::new(false)),
            starting: false,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInProxyStatus {
    pub ownership: &'static str,
    pub mode: &'static str,
    pub running: bool,
    pub bind_host: Option<String>,
    pub port: Option<u16>,
    pub endpoint: Option<String>,
    pub api_key_configured: bool,
    pub upstream_count: usize,
    pub model_count: usize,
    pub error: Option<String>,
}

impl BuiltInProxyManager {
    pub(crate) fn snapshot(&self) -> BuiltInProxyStatus {
        let mut inner = self.inner.lock().expect("built-in proxy manager lock");
        let running = inner.running.load(Ordering::Acquire);
        if !running {
            inner.task.take();
            inner.stop.take();
            inner.endpoint = None;
        }
        BuiltInProxyStatus {
            ownership: "built-in",
            mode: "managed",
            running,
            bind_host: inner.config.as_ref().map(|config| config.bind_host.clone()),
            port: inner.config.as_ref().map(|config| config.port),
            endpoint: inner.endpoint.clone(),
            api_key_configured: inner
                .config
                .as_ref()
                .is_some_and(|config| !config.api_keys.is_empty()),
            upstream_count: inner
                .config
                .as_ref()
                .map(|config| config.upstreams.len())
                .unwrap_or_default(),
            model_count: inner
                .config
                .as_ref()
                .map(|config| {
                    config
                        .upstreams
                        .iter()
                        .map(|upstream| upstream.models.len())
                        .sum()
                })
                .unwrap_or_default(),
            error: inner.error.clone(),
        }
    }

    async fn start(
        &self,
        config: ProxyServerConfig,
        oauth_resolver: Option<Arc<dyn ProxyOAuthCredentialResolver>>,
    ) -> Result<BuiltInProxyStatus, String> {
        {
            let mut inner = self.inner.lock().expect("built-in proxy manager lock");
            if inner.starting || inner.running.load(Ordering::Acquire) {
                return Err("OpenMesh built-in proxy is already running".to_string());
            }
            inner.starting = true;
        }

        let server = match ProxyServer::new_with_config_path_and_usage_sink(
            config.clone(),
            Some(default_proxy_config_path()),
            Some(Arc::new(DesktopProxyUsageSink)),
        ) {
            Ok(server) => server,
            Err(error) => {
                self.clear_starting();
                return Err(error.to_string());
            }
        };
        if let Some(resolver) = oauth_resolver {
            server.set_oauth_credential_resolver(resolver);
        }
        let listener = match server.bind().await {
            Ok(listener) => listener,
            Err(error) => {
                self.clear_starting();
                return Err(error.to_string());
            }
        };
        let address = listener.local_addr().map_err(|error| {
            self.clear_starting();
            format!("could not read proxy listener address: {error}")
        })?;
        if let Err(error) = server.persist_config() {
            self.clear_starting();
            return Err(error.to_string());
        }
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let running = {
            let inner = self.inner.lock().expect("built-in proxy manager lock");
            inner.running.clone()
        };
        running.store(true, Ordering::Release);
        let runtime_server = server.clone();
        let task = tauri::async_runtime::spawn(async move {
            let result = runtime_server
                .serve(listener, async {
                    let _ = stop_rx.await;
                })
                .await;
            running.store(false, Ordering::Release);
            result
        });

        let mut inner = self.inner.lock().expect("built-in proxy manager lock");
        inner.config = Some(config);
        inner.server = Some(server);
        inner.endpoint = Some(format!("http://{address}"));
        inner.task = Some(task);
        inner.stop = Some(stop_tx);
        inner.starting = false;
        inner.error = None;
        drop(inner);
        Ok(self.snapshot())
    }

    async fn stop(&self) -> Result<BuiltInProxyStatus, String> {
        let (stop, task) = {
            let mut inner = self.inner.lock().expect("built-in proxy manager lock");
            (inner.stop.take(), inner.task.take())
        };
        if let Some(stop) = stop {
            let _ = stop.send(());
        }
        if let Some(task) = task {
            task.await
                .map_err(|error| format!("proxy task failed to join: {error}"))?
                .map_err(|error| error.to_string())?;
        }
        let mut inner = self.inner.lock().expect("built-in proxy manager lock");
        inner.server = None;
        inner.running.store(false, Ordering::Release);
        inner.starting = false;
        inner.endpoint = None;
        inner.error = None;
        Ok(BuiltInProxyStatus {
            ownership: "built-in",
            mode: "managed",
            running: false,
            bind_host: inner.config.as_ref().map(|config| config.bind_host.clone()),
            port: inner.config.as_ref().map(|config| config.port),
            endpoint: None,
            api_key_configured: inner
                .config
                .as_ref()
                .is_some_and(|config| !config.api_keys.is_empty()),
            upstream_count: inner
                .config
                .as_ref()
                .map(|config| config.upstreams.len())
                .unwrap_or_default(),
            model_count: inner
                .config
                .as_ref()
                .map(|config| {
                    config
                        .upstreams
                        .iter()
                        .map(|upstream| upstream.models.len())
                        .sum()
                })
                .unwrap_or_default(),
            error: None,
        })
    }

    fn clear_starting(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.starting = false;
        }
    }

    pub(crate) fn config_snapshot(&self) -> Option<ProxyServerConfig> {
        self.inner
            .lock()
            .ok()
            .and_then(|inner| inner.config.clone())
    }

    pub(crate) fn replace_config(&self, config: ProxyServerConfig) -> Result<(), String> {
        config.validate().map_err(|error| error.to_string())?;
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "OpenMesh built-in proxy state is unavailable".to_string())?;
        if let (Some(current), Some(server)) = (inner.config.as_ref(), inner.server.as_ref()) {
            if current.bind_host != config.bind_host
                || current.port != config.port
                || current.request_timeout_secs != config.request_timeout_secs
            {
                return Err(
                    "bindHost, port, or requestTimeoutSecs changes require restarting the OpenMesh proxy"
                        .to_string(),
                );
            }
            server
                .replace_config(config.clone())
                .map_err(|error| error.to_string())?;
        } else {
            write_proxy_config(&default_proxy_config_path(), &config)
                .map_err(|error| error.to_string())?;
        }
        inner.config = Some(config);
        Ok(())
    }
}

impl Drop for BuiltInProxyManager {
    fn drop(&mut self) {
        if let Ok(inner) = self.inner.get_mut() {
            if let Some(stop) = inner.stop.take() {
                let _ = stop.send(());
            }
        }
    }
}

#[tauri::command]
pub fn proxy_runtime_status(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> BuiltInProxyStatus {
    state.snapshot()
}

#[tauri::command]
pub async fn proxy_runtime_start(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
    oauth: tauri::State<'_, Arc<crate::oauth_desktop::OAuthDesktopManager>>,
    config: ProxyServerConfig,
) -> Result<BuiltInProxyStatus, String> {
    state.start(config, Some(oauth.credential_resolver())).await
}

#[tauri::command]
pub async fn proxy_runtime_stop(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<BuiltInProxyStatus, String> {
    state.stop().await
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInProxyManagementUpstream {
    pub id: String,
    pub base_url: String,
    pub protocol: ProxyProviderProtocol,
    pub api_key_configured: bool,
    pub enabled: bool,
    pub priority: i32,
    pub account_id: Option<String>,
    pub oauth_provider: Option<String>,
    pub models: Vec<ProxyModelConfig>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltInProxyManagementConfig {
    pub bind_host: String,
    pub port: u16,
    pub allow_unauthenticated: bool,
    pub request_timeout_secs: u64,
    pub routing_strategy: ProxyRoutingStrategy,
    pub max_retries: u32,
    pub api_key_count: usize,
    pub upstream_count: usize,
    pub model_alias_count: usize,
    pub upstreams: Vec<BuiltInProxyManagementUpstream>,
    pub model_aliases: BTreeMap<String, String>,
    pub model_fallbacks: BTreeMap<String, Vec<String>>,
}

fn management_config_view(config: &ProxyServerConfig) -> BuiltInProxyManagementConfig {
    BuiltInProxyManagementConfig {
        bind_host: config.bind_host.clone(),
        port: config.port,
        allow_unauthenticated: config.allow_unauthenticated,
        request_timeout_secs: config.request_timeout_secs,
        routing_strategy: config.routing_strategy,
        max_retries: config.max_retries,
        api_key_count: config.api_keys.len(),
        upstream_count: config.upstreams.len(),
        model_alias_count: config.model_aliases.len(),
        upstreams: config
            .upstreams
            .iter()
            .map(|upstream| BuiltInProxyManagementUpstream {
                id: upstream.id.clone(),
                base_url: upstream.base_url.clone(),
                protocol: upstream.protocol,
                api_key_configured: upstream
                    .api_key
                    .as_ref()
                    .is_some_and(|key| !key.trim().is_empty()),
                enabled: upstream.enabled,
                priority: upstream.priority,
                account_id: upstream.account_id.clone(),
                oauth_provider: upstream.oauth_provider.clone(),
                models: upstream.models.clone(),
            })
            .collect(),
        model_aliases: config.model_aliases.clone(),
        model_fallbacks: config.model_fallbacks.clone(),
    }
}

#[tauri::command]
pub fn proxy_management_config(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<BuiltInProxyManagementConfig, String> {
    state
        .config_snapshot()
        .as_ref()
        .map(management_config_view)
        .ok_or_else(|| "Start the OpenMesh built-in proxy before editing providers.".to_string())
}

fn merge_management_patch(
    current: &ProxyServerConfig,
    patch: Value,
) -> Result<ProxyServerConfig, String> {
    let mut patch = patch
        .as_object()
        .cloned()
        .ok_or_else(|| "Proxy configuration update must be a JSON object.".to_string())?;
    for key in ["apiKeyCount", "upstreamCount", "modelAliasCount"] {
        patch.remove(key);
    }
    if let Some(upstreams) = patch.get_mut("upstreams").and_then(Value::as_array_mut) {
        for upstream in upstreams {
            if let Some(object) = upstream.as_object_mut() {
                object.remove("apiKeyConfigured");
            }
        }
    }
    let mut merged = serde_json::to_value(current)
        .map_err(|_| "Proxy configuration could not be serialized.".to_string())?;
    let object = merged
        .as_object_mut()
        .ok_or_else(|| "Proxy configuration could not be serialized.".to_string())?;
    for (key, value) in patch {
        object.insert(key, value);
    }
    if let Some(upstreams) = object.get_mut("upstreams").and_then(Value::as_array_mut) {
        for upstream in upstreams {
            let Some(upstream_object) = upstream.as_object_mut() else {
                continue;
            };
            if upstream_object.contains_key("apiKey") {
                continue;
            }
            let Some(id) = upstream_object.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(existing) = current.upstreams.iter().find(|item| item.id == id) {
                if let Some(api_key) = existing.api_key.as_ref() {
                    upstream_object.insert("apiKey".to_string(), Value::String(api_key.clone()));
                }
            }
        }
    }
    serde_json::from_value(merged).map_err(|_| "Proxy configuration update is invalid.".to_string())
}

#[tauri::command]
pub fn proxy_management_update(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
    patch: Value,
) -> Result<BuiltInProxyManagementConfig, String> {
    let current = state
        .config_snapshot()
        .ok_or_else(|| "Start the OpenMesh built-in proxy before editing providers.".to_string())?;
    let config = merge_management_patch(&current, patch)?;
    state.replace_config(config)?;
    proxy_management_config(state)
}

#[tauri::command]
pub async fn proxy_runtime_start_default(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
    oauth: tauri::State<'_, Arc<crate::oauth_desktop::OAuthDesktopManager>>,
) -> Result<BuiltInProxyStatus, String> {
    let settings = read_global::<Settings>("settings.json").unwrap_or_else(default_settings);
    let persisted_path = default_proxy_config_path();
    if persisted_path.exists() {
        let persisted = read_proxy_config(&persisted_path).map_err(|error| error.to_string())?;
        if persisted.bind_host != "127.0.0.1" && persisted.bind_host != "localhost" {
            return Err("Desktop proxy configuration must bind to loopback; use `openmesh-cli proxy serve` for a remote listener.".to_string());
        }
        return state
            .start(persisted, Some(oauth.credential_resolver()))
            .await;
    }
    let upstream_key = CascadingSecretStore::default()
        .get_api_key()
        .map_err(|error| error.to_string())?
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "Save an API key in Settings → Provider before starting the built-in proxy.".to_string()
        })?;
    let model = settings
        .provider
        .default_model
        .clone()
        .or(settings.models.coding_model.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".to_string());
    let base_url = settings
        .provider
        .api_base_url
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_base_url(settings.provider.name.as_deref()));
    let client_key = read_or_create_client_key()?;
    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: settings.oauth.management_port.max(1),
        api_keys: vec![client_key],
        allow_unauthenticated: false,
        request_timeout_secs: 120,
        routing_strategy: Default::default(),
        max_retries: openmesh_core::proxy_server::DEFAULT_MAX_RETRIES,
        upstreams: vec![ProxyUpstreamConfig {
            id: "provider-settings".to_string(),
            base_url,
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: Some(upstream_key),
            enabled: true,
            priority: 0,
            account_id: None,
            oauth_provider: None,
            models: vec![ProxyModelConfig {
                id: model,
                owned_by: "openmesh-provider".to_string(),
                capabilities: vec![
                    "chat".to_string(),
                    "responses".to_string(),
                    "embeddings".to_string(),
                ],
            }],
        }],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    state.start(config, Some(oauth.credential_resolver())).await
}

fn default_base_url(provider: Option<&str>) -> String {
    let normalized = provider.unwrap_or_default().to_ascii_lowercase();
    if normalized.contains("deepseek") {
        "https://api.deepseek.com/v1".to_string()
    } else if normalized.contains("xai") || normalized.contains("grok") {
        "https://api.x.ai/v1".to_string()
    } else {
        "https://api.openai.com/v1".to_string()
    }
}

fn client_key_path() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("openmesh")
        .join("builtin-proxy-client-key")
}

fn read_or_create_client_key() -> Result<String, String> {
    let path = client_key_path();
    if let Ok(value) = fs::read_to_string(&path) {
        let value = value.trim().to_string();
        if !value.is_empty() {
            return Ok(value);
        }
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create the proxy key directory: {error}"))?;
    }
    let value = format!("om_{}", uuid::Uuid::new_v4().simple());
    fs::write(&path, &value)
        .map_err(|error| format!("Could not save the proxy client key: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Could not protect the proxy client key: {error}"))?;
    }
    Ok(value)
}
