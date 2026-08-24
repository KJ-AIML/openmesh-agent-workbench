//! OpenMesh-owned OAuth lifecycle commands.
//!
//! The IPC names remain compatible with the former OAuth screen, but the
//! implementation now owns the pending authorization session, loopback
//! callback listener, provider exchange, and OS credential persistence. The
//! provider adapters themselves live in `openmesh-core` so the CLI can reuse
//! the same wire contracts later.

use crate::proxy_runtime_desktop::{BuiltInProxyManager, BuiltInProxyStatus};
use chrono::Utc;
use openmesh_core::oauth::{
    adapter_for, generate_state, parse_callback_url, OAuthCallbackHandle, OAuthCallbackServer,
    OAuthError, OAuthProviderAdapter, OAuthProviderId, OAuthTokenStore, PkceCodes,
    OPENMESH_OAUTH_KEYRING_SERVICE,
};
use openmesh_core::proxy_server::{ProxyOAuthCredential, ProxyOAuthCredentialResolver};
use openmesh_core::storage::{default_settings, read_global, write_global, Settings};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfigStatus {
    pub management_port: u16,
    pub endpoint: String,
    pub data_plane_endpoint: String,
    pub sidecar_enabled: bool,
    pub secret_configured: bool,
    pub sidecar_client_key_configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderSummary {
    pub provider: String,
    pub total: u32,
    pub enabled: u32,
    pub disabled: u32,
    pub unavailable: u32,
    pub runtime_only: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConnectionStatus {
    pub management_port: u16,
    pub endpoint: String,
    pub sidecar_enabled: bool,
    pub secret_configured: bool,
    pub sidecar_client_key_configured: bool,
    pub data_plane_status: OAuthDataPlaneState,
    pub data_plane_endpoint: String,
    pub data_plane_error: Option<String>,
    pub status: OAuthConnectionState,
    pub error: Option<String>,
    pub providers: Vec<OAuthProviderSummary>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum OAuthConnectionState {
    NotConfigured,
    Unreachable,
    Unauthorized,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub enum OAuthDataPlaneState {
    NotConfigured,
    Unreachable,
    Unauthorized,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OAuthRuntimeCapabilityState {
    Available,
    ReadOnly,
    Deferred,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthRuntimeCapabilities {
    pub oauth_login: OAuthRuntimeCapabilityState,
    pub account_summary: OAuthRuntimeCapabilityState,
    pub model_catalog: OAuthRuntimeCapabilityState,
    pub chat_data_plane: OAuthRuntimeCapabilityState,
    pub provider_configuration: OAuthRuntimeCapabilityState,
    pub auth_file_writes: OAuthRuntimeCapabilityState,
    pub quotas: OAuthRuntimeCapabilityState,
    pub usage_analytics: OAuthRuntimeCapabilityState,
    pub core_lifecycle: OAuthRuntimeCapabilityState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthRuntimeConfig {
    pub port: Option<u16>,
    pub routing_strategy: Option<String>,
    pub api_key_count: u32,
    pub proxy_configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthRuntimeStatus {
    pub mode: String,
    pub ownership: String,
    pub management_port: u16,
    pub management_endpoint: String,
    pub management_status: OAuthConnectionState,
    pub management_secret_configured: bool,
    pub management_error: Option<String>,
    pub data_plane_endpoint: String,
    pub data_plane_status: OAuthDataPlaneState,
    pub sidecar_client_key_configured: bool,
    pub data_plane_error: Option<String>,
    pub configuration_status: String,
    pub configuration_error: Option<String>,
    pub configuration: Option<OAuthRuntimeConfig>,
    pub providers: Vec<OAuthProviderSummary>,
    pub capabilities: OAuthRuntimeCapabilities,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResult {
    pub url: String,
    pub state: Option<String>,
    pub flow: OAuthStartFlow,
    pub user_code: Option<String>,
    pub verification_uri: Option<String>,
    pub verification_uri_complete: Option<String>,
    pub poll_interval_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthStartFlow {
    Browser,
    Device,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStatusResult {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthModelDefinition {
    pub id: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone)]
enum OAuthFlowStatus {
    Pending,
    Ready { account_id: String },
    Error(String),
    Cancelled,
}

struct OAuthSession {
    provider: OAuthProviderId,
    state: String,
    callback: Option<OAuthCallbackHandle>,
    status: Mutex<OAuthFlowStatus>,
}

/// Desktop-owned OAuth coordinator. The map intentionally retains completed
/// sessions until the app exits so a polling UI can read the terminal result;
/// token material is never kept in this coordinator after the keyring save.
pub struct OAuthDesktopManager {
    sessions: Mutex<BTreeMap<String, Arc<OAuthSession>>>,
    token_store: openmesh_core::oauth::KeyringTokenStore,
    client: reqwest::Client,
}

#[derive(Clone)]
pub(crate) struct DesktopOAuthCredentialResolver {
    token_store: openmesh_core::oauth::KeyringTokenStore,
    client: reqwest::Client,
}

#[async_trait::async_trait]
impl ProxyOAuthCredentialResolver for DesktopOAuthCredentialResolver {
    async fn resolve(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<ProxyOAuthCredential>, String> {
        let mut token = self
            .token_store
            .load(provider, account_id)
            .map_err(|error| error.to_string())?;
        let Some(current) = token.take() else {
            return Ok(None);
        };
        let adapter = adapter_for(provider, self.client.clone()).map_err(display_oauth_error)?;
        let current = if current.needs_refresh(Utc::now(), adapter.refresh_lead()) {
            let refreshed = adapter
                .refresh(&current)
                .await
                .map_err(display_oauth_error)?;
            self.token_store
                .save(&refreshed)
                .map_err(|_| "OAuth token could not be stored securely".to_string())?;
            refreshed
        } else {
            current
        };
        Ok(Some(ProxyOAuthCredential {
            access_token: current.access_token,
            token_type: current.token_type,
            metadata: current
                .metadata
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_owned()))
                })
                .collect(),
        }))
    }
}

impl Default for OAuthDesktopManager {
    fn default() -> Self {
        Self {
            sessions: Mutex::new(BTreeMap::new()),
            token_store: openmesh_core::oauth::KeyringTokenStore::new(
                OPENMESH_OAUTH_KEYRING_SERVICE,
            )
            .expect("static OAuth keyring service name is valid"),
            client: reqwest::Client::builder()
                .user_agent("OpenMesh/0.1 built-in OAuth")
                .build()
                .expect("default OAuth HTTP client should build"),
        }
    }
}

impl OAuthDesktopManager {
    pub(crate) fn credential_resolver(&self) -> Arc<dyn ProxyOAuthCredentialResolver> {
        Arc::new(DesktopOAuthCredentialResolver {
            token_store: self.token_store.clone(),
            client: self.client.clone(),
        })
    }

    async fn start(&self, provider: OAuthProviderId) -> Result<OAuthStartResult, String> {
        let adapter = adapter_for(provider, self.client.clone()).map_err(display_oauth_error)?;
        if matches!(provider, OAuthProviderId::Grok | OAuthProviderId::Kimi) {
            return self.start_device(provider, adapter).await;
        }
        let pkce = PkceCodes::generate().map_err(|error| error.to_string())?;
        let state = generate_state().map_err(|error| error.to_string())?;
        let redirect_uri = adapter.default_redirect_uri().to_owned();
        let redirect = reqwest::Url::parse(&redirect_uri)
            .map_err(|_| "OAuth adapter returned an invalid redirect URI".to_string())?;
        let callback_path = redirect.path();
        let callback_port = redirect
            .port_or_known_default()
            .ok_or_else(|| "OAuth redirect URI does not specify a callback port".to_string())?;
        let mut callback_server = OAuthCallbackServer::bind(callback_port, callback_path)
            .await
            .map_err(|error| error.to_string())?;
        let authorization = match adapter.authorization_url(&redirect_uri, &state, &pkce) {
            Ok(authorization) => authorization,
            Err(error) => {
                let _ = callback_server.stop().await;
                return Err(display_oauth_error(error));
            }
        };
        let session = Arc::new(OAuthSession {
            provider,
            state: state.clone(),
            callback: Some(callback_server.handle()),
            status: Mutex::new(OAuthFlowStatus::Pending),
        });
        self.sessions
            .lock()
            .map_err(|_| "OAuth session state is unavailable".to_string())?
            .insert(state.clone(), session.clone());
        let token_store = self.token_store.clone();
        let pkce_verifier = pkce.verifier.clone();
        tauri::async_runtime::spawn(async move {
            let callback = callback_server.wait_for_callback().await;
            let callback = match callback {
                Ok(callback) => callback,
                Err(error) => {
                    set_session_error(&session, error.to_string());
                    return;
                }
            };
            let callback_state = callback.state.as_deref().or_else(|| {
                callback
                    .code
                    .as_deref()
                    .and_then(|code| code.split_once('#').map(|(_, state)| state))
            });
            if callback_state != Some(session.state.as_str()) {
                set_session_error(&session, OAuthError::StateMismatch.to_string());
                return;
            }
            let token = match adapter
                .exchange_code(&callback, &redirect_uri, &pkce_verifier)
                .await
            {
                Ok(token) => token,
                Err(error) => {
                    set_session_error(&session, display_oauth_error(error));
                    return;
                }
            };
            let account_id = token.account_id.clone();
            if let Err(error) = token_store.save(&token) {
                set_session_error(
                    &session,
                    format!("OAuth token could not be stored securely: {error}"),
                );
                return;
            }
            if let Ok(mut status) = session.status.lock() {
                if matches!(*status, OAuthFlowStatus::Pending) {
                    *status = OAuthFlowStatus::Ready { account_id };
                }
            }
        });
        Ok(OAuthStartResult {
            url: authorization.authorization_url.to_string(),
            state: Some(state),
            flow: OAuthStartFlow::Browser,
            user_code: None,
            verification_uri: None,
            verification_uri_complete: None,
            poll_interval_seconds: None,
        })
    }

    async fn start_device(
        &self,
        provider: OAuthProviderId,
        adapter: Arc<dyn OAuthProviderAdapter>,
    ) -> Result<OAuthStartResult, String> {
        let device = adapter
            .start_device_flow()
            .await
            .map_err(display_oauth_error)?;
        if device.provider != provider {
            return Err("OAuth device flow returned the wrong provider".to_string());
        }
        let state = generate_state().map_err(|error| error.to_string())?;
        let verification_url = device
            .verification_uri_complete
            .clone()
            .unwrap_or_else(|| device.verification_uri.clone());
        let session = Arc::new(OAuthSession {
            provider,
            state: state.clone(),
            callback: None,
            status: Mutex::new(OAuthFlowStatus::Pending),
        });
        self.sessions
            .lock()
            .map_err(|_| "OAuth session state is unavailable".to_string())?
            .insert(state.clone(), session.clone());
        let token_store = self.token_store.clone();
        let device_for_task = device.clone();
        let poll_interval = std::time::Duration::from_secs(device.interval_seconds.max(1));
        tauri::async_runtime::spawn(async move {
            loop {
                if Utc::now() >= device_for_task.expires_at {
                    set_session_error(&session, "OAuth device authorization timed out".to_string());
                    return;
                }
                tokio::time::sleep(poll_interval).await;
                if Utc::now() >= device_for_task.expires_at {
                    set_session_error(&session, "OAuth device authorization timed out".to_string());
                    return;
                }
                let token = match adapter.poll_device_flow(&device_for_task).await {
                    Ok(Some(token)) => token,
                    Ok(None) => continue,
                    Err(error) => {
                        set_session_error(&session, display_oauth_error(error));
                        return;
                    }
                };
                let account_id = token.account_id.clone();
                if let Err(error) = token_store.save(&token) {
                    set_session_error(
                        &session,
                        format!("OAuth token could not be stored securely: {error}"),
                    );
                    return;
                }
                if let Ok(mut status) = session.status.lock() {
                    if matches!(*status, OAuthFlowStatus::Pending) {
                        *status = OAuthFlowStatus::Ready { account_id };
                    }
                }
                return;
            }
        });
        Ok(OAuthStartResult {
            url: verification_url,
            state: Some(state),
            flow: OAuthStartFlow::Device,
            user_code: Some(device.user_code.clone()),
            verification_uri: Some(device.verification_uri.clone()),
            verification_uri_complete: device.verification_uri_complete.clone(),
            poll_interval_seconds: Some(device.interval_seconds),
        })
    }

    fn session(&self, state: &str) -> Result<Arc<OAuthSession>, String> {
        self.sessions
            .lock()
            .map_err(|_| "OAuth session state is unavailable".to_string())?
            .get(state)
            .cloned()
            .ok_or_else(|| "OAuth state is unknown or expired".to_string())
    }

    fn status(&self, state: &str) -> Result<OAuthStatusResult, String> {
        let session = self.session(state)?;
        let status = session
            .status
            .lock()
            .map_err(|_| "OAuth session state is unavailable".to_string())?;
        Ok(match &*status {
            OAuthFlowStatus::Pending => OAuthStatusResult {
                status: "pending".to_string(),
                error: None,
            },
            OAuthFlowStatus::Ready { account_id } => {
                let _ = account_id;
                OAuthStatusResult {
                    status: "ok".to_string(),
                    error: None,
                }
            }
            OAuthFlowStatus::Error(error) => OAuthStatusResult {
                status: "error".to_string(),
                error: Some(error.clone()),
            },
            OAuthFlowStatus::Cancelled => OAuthStatusResult {
                status: "error".to_string(),
                error: Some("OAuth authorization was cancelled".to_string()),
            },
        })
    }

    async fn submit_callback(
        &self,
        provider: OAuthProviderId,
        redirect_url: &str,
    ) -> Result<(), String> {
        let callback = parse_callback_url(redirect_url).map_err(display_oauth_error)?;
        let state = callback
            .state
            .clone()
            .or_else(|| {
                callback
                    .code
                    .as_deref()
                    .and_then(|code| code.split_once('#').map(|(_, state)| state.to_owned()))
            })
            .ok_or_else(|| "OAuth callback did not include state".to_string())?;
        let session = self.session(&state)?;
        if session.provider != provider {
            return Err("OAuth callback provider does not match the pending session".to_string());
        }
        session
            .callback
            .as_ref()
            .ok_or_else(|| "OAuth device flow does not accept a callback URL".to_string())?
            .submit(callback)
            .await
            .map_err(|error| error.to_string())
    }

    async fn cancel(&self, state: &str) -> Result<(), String> {
        let session = self.session(state)?;
        if let Ok(mut status) = session.status.lock() {
            *status = OAuthFlowStatus::Cancelled;
        }
        if let Some(callback) = session.callback.as_ref() {
            callback.stop().await;
        }
        Ok(())
    }
}

fn set_session_error(session: &OAuthSession, error: String) {
    if let Ok(mut status) = session.status.lock() {
        if matches!(*status, OAuthFlowStatus::Pending) {
            *status = OAuthFlowStatus::Error(error);
        }
    }
}

fn display_oauth_error(error: OAuthError) -> String {
    match error {
        OAuthError::Http { .. } => "OAuth provider request failed".to_string(),
        OAuthError::Provider { status, .. } => {
            format!("OAuth provider rejected the request (HTTP {status})")
        }
        OAuthError::Storage(_) => "OAuth token could not be stored securely".to_string(),
        other => other.to_string(),
    }
}

fn settings() -> Settings {
    read_global::<Settings>("settings.json").unwrap_or_else(default_settings)
}

fn port() -> u16 {
    settings().oauth.management_port.max(1)
}

fn endpoints(port: u16) -> (String, String) {
    (
        format!("http://127.0.0.1:{port}/v0/management"),
        format!("http://127.0.0.1:{port}/v1"),
    )
}

fn provider_summaries(manager: &BuiltInProxyManager) -> Vec<OAuthProviderSummary> {
    let Some(config) = manager.config_snapshot() else {
        return Vec::new();
    };
    let mut result = Vec::new();
    for (provider, protocol) in [
        (
            "openai",
            openmesh_core::proxy_server::ProxyProviderProtocol::OpenAiCompatible,
        ),
        (
            "claude",
            openmesh_core::proxy_server::ProxyProviderProtocol::Anthropic,
        ),
        (
            "gemini",
            openmesh_core::proxy_server::ProxyProviderProtocol::Gemini,
        ),
    ] {
        let upstreams = config
            .upstreams
            .iter()
            .filter(|upstream| upstream.protocol == protocol)
            .collect::<Vec<_>>();
        if upstreams.is_empty() {
            continue;
        }
        let total = upstreams.len() as u32;
        let enabled = upstreams.iter().filter(|upstream| upstream.enabled).count() as u32;
        result.push(OAuthProviderSummary {
            provider: provider.to_string(),
            total,
            enabled,
            disabled: total.saturating_sub(enabled),
            unavailable: 0,
            runtime_only: 0,
        });
    }
    result
}

fn routing_strategy_name(strategy: &openmesh_core::proxy_server::ProxyRoutingStrategy) -> String {
    match strategy {
        openmesh_core::proxy_server::ProxyRoutingStrategy::FirstCompatible => {
            "first-compatible".to_string()
        }
        openmesh_core::proxy_server::ProxyRoutingStrategy::RoundRobin => "round-robin".to_string(),
        openmesh_core::proxy_server::ProxyRoutingStrategy::FillFirst => "fill-first".to_string(),
    }
}

fn config_status(manager: &BuiltInProxyManager) -> OAuthConfigStatus {
    let status = manager.snapshot();
    let management_port = status.port.unwrap_or_else(port);
    let (endpoint, data_plane_endpoint) = endpoints(management_port);
    OAuthConfigStatus {
        management_port,
        endpoint,
        data_plane_endpoint,
        sidecar_enabled: status.running,
        secret_configured: status.api_key_configured,
        sidecar_client_key_configured: status.api_key_configured,
    }
}

fn connection_status(manager: &BuiltInProxyManager) -> OAuthConnectionStatus {
    let status: BuiltInProxyStatus = manager.snapshot();
    let management_port = status.port.unwrap_or_else(port);
    let (endpoint, data_plane_endpoint) = endpoints(management_port);
    let ready = status.running;
    OAuthConnectionStatus {
        management_port,
        endpoint,
        sidecar_enabled: ready,
        secret_configured: status.api_key_configured,
        sidecar_client_key_configured: status.api_key_configured,
        data_plane_status: if ready {
            OAuthDataPlaneState::Ready
        } else {
            OAuthDataPlaneState::NotConfigured
        },
        data_plane_endpoint,
        data_plane_error: status.error.clone(),
        status: if ready {
            OAuthConnectionState::Ready
        } else {
            OAuthConnectionState::NotConfigured
        },
        error: status.error,
        providers: provider_summaries(manager),
    }
}

#[tauri::command]
pub fn oauth_config_status(state: tauri::State<'_, Arc<BuiltInProxyManager>>) -> OAuthConfigStatus {
    config_status(state.inner().as_ref())
}

#[tauri::command]
pub fn oauth_connection_status(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<OAuthConnectionStatus, String> {
    Ok(connection_status(state.inner().as_ref()))
}

#[tauri::command]
pub fn oauth_runtime_status(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<OAuthRuntimeStatus, String> {
    let connection = connection_status(state.inner().as_ref());
    let config = state.config_snapshot();
    Ok(OAuthRuntimeStatus {
        mode: "managed".to_string(),
        ownership: "built-in".to_string(),
        management_port: connection.management_port,
        management_endpoint: connection.endpoint.clone(),
        management_status: connection.status.clone(),
        management_secret_configured: connection.secret_configured,
        management_error: connection.error.clone(),
        data_plane_endpoint: connection.data_plane_endpoint.clone(),
        data_plane_status: connection.data_plane_status.clone(),
        sidecar_client_key_configured: connection.sidecar_client_key_configured,
        data_plane_error: connection.data_plane_error.clone(),
        configuration_status: if config.is_some() {
            "available"
        } else {
            "not-configured"
        }
        .to_string(),
        configuration_error: None,
        configuration: config.map(|config| OAuthRuntimeConfig {
            port: Some(config.port),
            routing_strategy: Some(routing_strategy_name(&config.routing_strategy)),
            api_key_count: config.api_keys.len() as u32,
            proxy_configured: !config.upstreams.is_empty(),
        }),
        providers: connection.providers,
        capabilities: OAuthRuntimeCapabilities {
            oauth_login: OAuthRuntimeCapabilityState::Available,
            account_summary: OAuthRuntimeCapabilityState::ReadOnly,
            model_catalog: OAuthRuntimeCapabilityState::Available,
            chat_data_plane: OAuthRuntimeCapabilityState::Available,
            provider_configuration: OAuthRuntimeCapabilityState::Available,
            auth_file_writes: OAuthRuntimeCapabilityState::Deferred,
            quotas: OAuthRuntimeCapabilityState::Deferred,
            usage_analytics: OAuthRuntimeCapabilityState::Available,
            core_lifecycle: OAuthRuntimeCapabilityState::Available,
        },
    })
}

#[tauri::command]
pub fn oauth_set_management_port(port: u16) -> Result<OAuthConfigStatus, String> {
    if port == 0 {
        return Err("OpenMesh proxy port must be between 1 and 65535".to_string());
    }
    let mut settings = settings();
    settings.oauth.management_port = port;
    settings.oauth.sidecar_enabled = false;
    write_global("settings.json", &settings)?;
    Ok(OAuthConfigStatus {
        management_port: port,
        endpoint: endpoints(port).0,
        data_plane_endpoint: endpoints(port).1,
        sidecar_enabled: false,
        secret_configured: false,
        sidecar_client_key_configured: false,
    })
}

#[tauri::command]
pub fn oauth_set_management_secret(_secret: String) -> Result<OAuthConfigStatus, String> {
    Err("OpenMesh no longer uses a separate management secret; the built-in proxy generates its local client key.".to_string())
}

#[tauri::command]
pub fn oauth_set_sidecar_client_key(_api_key: String) -> Result<OAuthConfigStatus, String> {
    Err("OpenMesh manages the built-in proxy client key automatically.".to_string())
}

#[tauri::command]
pub fn oauth_clear_sidecar_client_key(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<OAuthConfigStatus, String> {
    Ok(config_status(state.inner().as_ref()))
}

#[tauri::command]
pub fn oauth_set_sidecar_enabled(
    enabled: bool,
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<OAuthConfigStatus, String> {
    if enabled {
        return Err("Use the OpenMesh built-in proxy lifecycle controls instead.".to_string());
    }
    let mut settings = settings();
    settings.oauth.sidecar_enabled = false;
    write_global("settings.json", &settings)?;
    Ok(config_status(state.inner().as_ref()))
}

#[tauri::command]
pub fn oauth_clear_management_secret(
    state: tauri::State<'_, Arc<BuiltInProxyManager>>,
) -> Result<OAuthConfigStatus, String> {
    Ok(config_status(state.inner().as_ref()))
}

#[tauri::command]
pub async fn oauth_start(
    provider: String,
    state: tauri::State<'_, Arc<OAuthDesktopManager>>,
) -> Result<OAuthStartResult, String> {
    let provider = provider
        .parse::<OAuthProviderId>()
        .map_err(display_oauth_error)?;
    state.start(provider).await
}

#[tauri::command]
pub async fn oauth_status(
    state: String,
    manager: tauri::State<'_, Arc<OAuthDesktopManager>>,
) -> Result<OAuthStatusResult, String> {
    manager.status(&state)
}

#[tauri::command]
pub async fn oauth_submit_callback(
    provider: String,
    redirect_url: String,
    manager: tauri::State<'_, Arc<OAuthDesktopManager>>,
) -> Result<(), String> {
    let provider = provider
        .parse::<OAuthProviderId>()
        .map_err(display_oauth_error)?;
    manager.submit_callback(provider, &redirect_url).await
}

#[tauri::command]
pub async fn oauth_cancel(
    state: String,
    manager: tauri::State<'_, Arc<OAuthDesktopManager>>,
) -> Result<(), String> {
    manager.cancel(&state).await
}

#[tauri::command]
pub fn oauth_open_url(url: String) -> Result<(), String> {
    let parsed = reqwest::Url::parse(&url).map_err(|_| "OAuth URL is invalid".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.username() != ""
        || parsed.password().is_some()
    {
        return Err("OAuth URL must be an HTTP(S) URL without embedded credentials".to_string());
    }
    open::that(parsed.as_str()).map_err(|error| format!("Could not open OAuth URL: {error}"))
}

#[tauri::command]
pub fn oauth_model_definitions(_provider: String) -> Result<Vec<OAuthModelDefinition>, String> {
    Ok(Vec::new())
}
