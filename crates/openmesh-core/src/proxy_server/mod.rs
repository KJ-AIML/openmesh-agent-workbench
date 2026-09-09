//! OpenAI-compatible HTTP adapter over OpenMesh provider runtime.
//!
//! Agent Engine talks to `llm_runtime` in-process and does **not** require this
//! listener. Starting or stopping the bind (default `127.0.0.1`) must not
//! delete proxy YAML / upstream credentials used by `ProviderRuntimeSpec`.
//! This module is independent of Tauri so desktop and `openmesh-cli proxy serve`
//! can host the same adapter.

mod config;
mod rate_limiter;
mod routes;
pub mod storage;
mod usage;

pub use config::{
    ProxyConfigError, ProxyModelConfig, ProxyProviderProtocol, ProxyRoutingStrategy,
    ProxyServerConfig, ProxyUpstreamConfig, DEFAULT_MAX_RETRIES, DEFAULT_PROXY_HOST,
    DEFAULT_PROXY_PORT, DEFAULT_REQUEST_TIMEOUT_SECS,
};
pub use storage::{
    default_proxy_config_path, read_proxy_config, write_proxy_config, ProxyConfigStorageError,
    PROXY_CONFIG_DIR, PROXY_CONFIG_FILE,
};
pub use usage::{ProxyUsageLog, ProxyUsageSink, ProxyUsageSnapshot};

use async_trait::async_trait;
use axum::Router;
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpListener;

#[derive(Clone, Debug)]
pub struct ProxyOAuthCredential {
    pub access_token: String,
    pub token_type: String,
    /// Non-secret provider identity values needed on the upstream wire, such
    /// as Kimi's device ID. Secret token material is never copied here.
    pub metadata: BTreeMap<String, String>,
}

impl ProxyOAuthCredential {
    pub fn authorization_header(&self) -> Option<String> {
        let access_token = self.access_token.trim();
        if access_token.is_empty() {
            return None;
        }
        let token_type = if self.token_type.trim().is_empty() {
            "Bearer"
        } else {
            self.token_type.trim()
        };
        Some(format!("{token_type} {access_token}"))
    }
}

#[async_trait]
pub trait ProxyOAuthCredentialResolver: Send + Sync {
    async fn resolve(
        &self,
        provider: crate::oauth::OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<ProxyOAuthCredential>, String>;
}

#[derive(Clone)]
pub struct ProxyServer {
    state: Arc<ProxyServerState>,
}

struct ProxyServerState {
    config: Arc<RwLock<ProxyServerConfig>>,
    config_path: Option<PathBuf>,
    client: reqwest::Client,
    request_prefix: String,
    request_counter: AtomicU64,
    routing_counter: AtomicU64,
    rate_limits: rate_limiter::ProxyRateLimitTracker,
    usage: usage::ProxyUsageTracker,
    oauth_resolver: RwLock<Option<Arc<dyn ProxyOAuthCredentialResolver>>>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyServerError {
    #[error("proxy configuration is invalid: {0}")]
    Configuration(#[from] ProxyConfigError),
    #[error("proxy HTTP client could not be created")]
    Client(#[source] reqwest::Error),
    #[error("proxy configuration could not be persisted")]
    Storage(#[source] ProxyConfigStorageError),
    #[error("proxy listener could not be created")]
    Listener(#[source] std::io::Error),
    #[error("proxy server failed")]
    Serve(#[source] std::io::Error),
}

impl ProxyServer {
    pub fn new(config: ProxyServerConfig) -> Result<Self, ProxyServerError> {
        Self::new_with_config_path_and_usage_sink(config, None, None)
    }

    pub fn new_with_config_path(
        config: ProxyServerConfig,
        config_path: Option<PathBuf>,
    ) -> Result<Self, ProxyServerError> {
        Self::new_with_config_path_and_usage_sink(config, config_path, None)
    }

    pub fn new_with_config_path_and_usage_sink(
        config: ProxyServerConfig,
        config_path: Option<PathBuf>,
        usage_sink: Option<Arc<dyn ProxyUsageSink>>,
    ) -> Result<Self, ProxyServerError> {
        config.validate()?;
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .user_agent("OpenMesh-BuiltInProxy/0.1")
            .build()
            .map_err(ProxyServerError::Client)?;
        Ok(Self {
            state: Arc::new(ProxyServerState {
                config: Arc::new(RwLock::new(config)),
                config_path,
                client,
                request_prefix: format!(
                    "openmesh-{}-{}-",
                    std::process::id(),
                    chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                request_counter: AtomicU64::new(1),
                routing_counter: AtomicU64::new(0),
                rate_limits: rate_limiter::ProxyRateLimitTracker::default(),
                usage: usage_sink
                    .map(usage::ProxyUsageTracker::with_sink)
                    .unwrap_or_default(),
                oauth_resolver: RwLock::new(None),
            }),
        })
    }

    pub fn config(&self) -> ProxyServerConfig {
        self.state.config()
    }

    pub fn persist_config(&self) -> Result<(), ProxyServerError> {
        self.state
            .persist_config()
            .map_err(ProxyServerError::Storage)
    }

    pub fn replace_config(&self, config: ProxyServerConfig) -> Result<(), ProxyServerError> {
        config.validate()?;
        self.state
            .persist_config_value(&config)
            .map_err(ProxyServerError::Storage)?;
        self.state.replace_config(config)?;
        Ok(())
    }

    pub fn set_oauth_credential_resolver(&self, resolver: Arc<dyn ProxyOAuthCredentialResolver>) {
        *self
            .state
            .oauth_resolver
            .write()
            .expect("proxy OAuth resolver lock") = Some(resolver);
    }

    pub fn router(&self) -> Router {
        routes::router(self.state.clone())
    }

    /// Bind the configured host and port. Port `0` is rejected by config
    /// validation for normal operation; tests can use `bind_on` with an
    /// explicitly selected ephemeral listener.
    pub async fn bind(&self) -> Result<TcpListener, ProxyServerError> {
        let config = self.config();
        let address = format!("{}:{}", config.bind_host, config.port);
        TcpListener::bind(&address)
            .await
            .map_err(ProxyServerError::Listener)
    }

    pub async fn serve<F>(self, listener: TcpListener, shutdown: F) -> Result<(), ProxyServerError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        axum::serve(listener, self.router())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(ProxyServerError::Serve)
    }
}

impl ProxyServerState {
    pub(crate) fn config(&self) -> ProxyServerConfig {
        self.config.read().expect("proxy config lock").clone()
    }

    pub(crate) fn replace_config(&self, config: ProxyServerConfig) -> Result<(), ProxyConfigError> {
        config.validate()?;
        *self.config.write().expect("proxy config lock") = config;
        Ok(())
    }

    pub(crate) fn persist_config(&self) -> Result<(), ProxyConfigStorageError> {
        let config = self.config();
        self.persist_config_value(&config)
    }

    pub(crate) fn persist_config_value(
        &self,
        config: &ProxyServerConfig,
    ) -> Result<(), ProxyConfigStorageError> {
        if let Some(path) = self.config_path.as_deref() {
            write_proxy_config(path, config)?;
        }
        Ok(())
    }

    pub(crate) fn client(&self) -> &reqwest::Client {
        &self.client
    }

    pub(crate) fn next_request_id(&self) -> String {
        let sequence = self.request_counter.fetch_add(1, Ordering::Relaxed);
        format!("{}{sequence}", self.request_prefix)
    }

    pub(crate) fn next_route_index(&self, count: usize) -> usize {
        if count == 0 {
            return 0;
        }
        (self.routing_counter.fetch_add(1, Ordering::Relaxed) as usize) % count
    }

    pub(crate) fn mark_rate_limited(&self, upstream_id: &str) {
        self.rate_limits.mark_limited(upstream_id);
    }

    pub(crate) fn is_rate_limited(&self, upstream_id: &str) -> bool {
        self.rate_limits.is_limited(upstream_id)
    }

    pub(crate) fn oauth_resolver(&self) -> Option<Arc<dyn ProxyOAuthCredentialResolver>> {
        self.oauth_resolver
            .read()
            .expect("proxy OAuth resolver lock")
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn test_config() -> ProxyServerConfig {
        ProxyServerConfig {
            bind_host: DEFAULT_PROXY_HOST.to_string(),
            port: 8317,
            api_keys: vec!["openmesh-test-key".to_string()],
            allow_unauthenticated: false,
            request_timeout_secs: 30,
            routing_strategy: Default::default(),
            max_retries: DEFAULT_MAX_RETRIES,
            upstreams: vec![ProxyUpstreamConfig {
                id: "test".to_string(),
                base_url: "http://127.0.0.1:9000/v1".to_string(),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("upstream-key".to_string()),
                enabled: true,
                priority: 0,
                account_id: None,
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "test-model".to_string(),
                    owned_by: "test".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            }],
            model_aliases: BTreeMap::new(),
            model_fallbacks: BTreeMap::new(),
        }
    }

    #[test]
    fn server_rejects_missing_auth_configuration() {
        let mut config = test_config();
        config.api_keys.clear();
        let error = match ProxyServer::new(config) {
            Ok(_) => panic!("missing auth must fail closed"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ProxyServerError::Configuration(ProxyConfigError::AuthenticationNotConfigured)
        ));
    }

    #[test]
    fn server_debug_does_not_include_secrets() {
        let config = test_config();
        let debug = format!("{config:?}");
        assert!(!debug.contains("openmesh-test-key"));
        assert!(!debug.contains("upstream-key"));
        assert!(debug.contains("configured"));
    }

    #[test]
    fn server_persists_management_config_without_changing_runtime_shape() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("proxy.yaml");
        let server = ProxyServer::new_with_config_path(test_config(), Some(path.clone()))
            .expect("server config");
        server.persist_config().expect("persist config");
        let loaded = read_proxy_config(&path).expect("read persisted config");
        assert_eq!(loaded.upstreams[0].id, "test");
        assert_eq!(loaded.api_keys, vec!["openmesh-test-key"]);
        let mut updated = server.config();
        updated.routing_strategy = ProxyRoutingStrategy::FillFirst;
        server.replace_config(updated).expect("replace config");
        let replaced = read_proxy_config(&path).expect("read replaced config");
        assert_eq!(replaced.routing_strategy, ProxyRoutingStrategy::FillFirst);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}
