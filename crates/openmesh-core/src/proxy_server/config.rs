use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const DEFAULT_PROXY_HOST: &str = "127.0.0.1";
pub const DEFAULT_PROXY_PORT: u16 = 8317;
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 120;
pub const DEFAULT_MAX_RETRIES: u32 = 2;
pub const MAX_PROXY_BODY_BYTES: usize = 8 * 1024 * 1024;

/// Configuration for the OpenMesh-owned HTTP proxy.
///
/// Secrets intentionally remain in this type because the first runtime slice
/// needs to be usable without a separate secret-management process. Callers
/// must avoid logging the value directly; its `Debug` implementation is
/// redacted and the server never echoes configured keys.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyServerConfig {
    #[serde(default = "default_bind_host")]
    pub bind_host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub api_keys: Vec<String>,
    #[serde(default)]
    pub allow_unauthenticated: bool,
    #[serde(default = "default_request_timeout_secs")]
    pub request_timeout_secs: u64,
    #[serde(default)]
    pub routing_strategy: ProxyRoutingStrategy,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    #[serde(default)]
    pub upstreams: Vec<ProxyUpstreamConfig>,
    #[serde(default)]
    pub model_aliases: BTreeMap<String, String>,
    #[serde(default)]
    pub model_fallbacks: BTreeMap<String, Vec<String>>,
}

/// Wire protocol spoken by an upstream provider executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyProviderProtocol {
    OpenAiCompatible,
    Anthropic,
    Gemini,
}

impl Default for ProxyProviderProtocol {
    fn default() -> Self {
        Self::OpenAiCompatible
    }
}

/// Selection policy for compatible upstream accounts.
///
/// All strategies use the lowest configured priority as the account
/// preference; round-robin rotates the equal-model candidates after that
/// ordering. Quota-aware selection is layered on top once provider quota
/// probes are available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyRoutingStrategy {
    FirstCompatible,
    RoundRobin,
    FillFirst,
}

impl Default for ProxyRoutingStrategy {
    fn default() -> Self {
        Self::FirstCompatible
    }
}

/// An upstream provider executor.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyUpstreamConfig {
    pub id: String,
    pub base_url: String,
    #[serde(default)]
    pub protocol: ProxyProviderProtocol,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub account_id: Option<String>,
    /// Reference an OAuth token in the host credential store. The access token
    /// is intentionally not serialized into this configuration.
    #[serde(default)]
    pub oauth_provider: Option<String>,
    #[serde(default)]
    pub models: Vec<ProxyModelConfig>,
}

/// Model metadata exposed through `GET /v1/models`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyModelConfig {
    pub id: String,
    #[serde(default = "default_owned_by")]
    pub owned_by: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl Default for ProxyServerConfig {
    fn default() -> Self {
        Self {
            bind_host: default_bind_host(),
            port: default_port(),
            api_keys: Vec::new(),
            allow_unauthenticated: false,
            request_timeout_secs: default_request_timeout_secs(),
            routing_strategy: ProxyRoutingStrategy::default(),
            max_retries: default_max_retries(),
            upstreams: Vec::new(),
            model_aliases: BTreeMap::new(),
            model_fallbacks: BTreeMap::new(),
        }
    }
}

impl fmt::Debug for ProxyServerConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyServerConfig")
            .field("bind_host", &self.bind_host)
            .field("port", &self.port)
            .field("api_keys", &redacted_count(self.api_keys.len()))
            .field("allow_unauthenticated", &self.allow_unauthenticated)
            .field("request_timeout_secs", &self.request_timeout_secs)
            .field("routing_strategy", &self.routing_strategy)
            .field("max_retries", &self.max_retries)
            .field("upstreams", &self.upstreams)
            .field("model_aliases", &self.model_aliases)
            .field("model_fallbacks", &self.model_fallbacks)
            .finish()
    }
}

impl fmt::Debug for ProxyUpstreamConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyUpstreamConfig")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .field("protocol", &self.protocol)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("enabled", &self.enabled)
            .field("priority", &self.priority)
            .field("account_id", &self.account_id)
            .field("oauth_provider", &self.oauth_provider)
            .field("models", &self.models)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProxyConfigError {
    #[error("proxy bind host must not be empty")]
    EmptyBindHost,
    #[error("proxy port must be between 1 and 65535")]
    InvalidPort,
    #[error("proxy request timeout must be greater than zero")]
    InvalidRequestTimeout,
    #[error("proxy retry count must not exceed 8")]
    InvalidMaxRetries,
    #[error("proxy API key must not be empty")]
    EmptyApiKey,
    #[error("proxy requires at least one API key unless allowUnauthenticated is enabled")]
    AuthenticationNotConfigured,
    #[error("proxy upstream list must not be empty")]
    NoUpstreams,
    #[error("proxy upstream id must not be empty")]
    EmptyUpstreamId,
    #[error("proxy upstream id is duplicated: {0}")]
    DuplicateUpstreamId(String),
    #[error("proxy upstream {id} has an invalid base URL")]
    InvalidUpstreamUrl { id: String },
    #[error("proxy upstream {id} API key must not be empty")]
    EmptyUpstreamApiKey { id: String },
    #[error("proxy upstream {id} account id must not be empty")]
    EmptyAccountId { id: String },
    #[error("proxy upstream {id} OAuth provider is invalid")]
    InvalidOAuthProvider { id: String },
    #[error("proxy upstream {id} OAuth account requires an account id")]
    MissingOAuthAccountId { id: String },
    #[error("proxy upstream {upstream} has an empty model id")]
    EmptyModelId { upstream: String },
    #[error("proxy upstream {upstream} has duplicate model id: {model}")]
    DuplicateModelId { upstream: String, model: String },
    #[error("proxy model alias must not be empty")]
    EmptyModelAlias,
    #[error("proxy model alias target must not be empty: {0}")]
    EmptyModelAliasTarget(String),
}

impl ProxyServerConfig {
    pub fn validate(&self) -> Result<(), ProxyConfigError> {
        if self.bind_host.trim().is_empty() {
            return Err(ProxyConfigError::EmptyBindHost);
        }
        if self.port == 0 {
            return Err(ProxyConfigError::InvalidPort);
        }
        if self.request_timeout_secs == 0 {
            return Err(ProxyConfigError::InvalidRequestTimeout);
        }
        if self.max_retries > 8 {
            return Err(ProxyConfigError::InvalidMaxRetries);
        }
        if !self.allow_unauthenticated && self.api_keys.is_empty() {
            return Err(ProxyConfigError::AuthenticationNotConfigured);
        }
        if self.api_keys.iter().any(|key| key.trim().is_empty()) {
            return Err(ProxyConfigError::EmptyApiKey);
        }
        if self.upstreams.is_empty() {
            return Err(ProxyConfigError::NoUpstreams);
        }

        let mut upstream_ids = BTreeSet::new();
        for upstream in &self.upstreams {
            if upstream.id.trim().is_empty() {
                return Err(ProxyConfigError::EmptyUpstreamId);
            }
            if !upstream_ids.insert(upstream.id.clone()) {
                return Err(ProxyConfigError::DuplicateUpstreamId(upstream.id.clone()));
            }

            let parsed = reqwest::Url::parse(upstream.base_url.trim()).map_err(|_| {
                ProxyConfigError::InvalidUpstreamUrl {
                    id: upstream.id.clone(),
                }
            })?;
            if !matches!(parsed.scheme(), "http" | "https")
                || parsed.host_str().is_none()
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || parsed.query().is_some()
                || parsed.fragment().is_some()
            {
                return Err(ProxyConfigError::InvalidUpstreamUrl {
                    id: upstream.id.clone(),
                });
            }
            if upstream
                .api_key
                .as_ref()
                .is_some_and(|key| key.trim().is_empty())
            {
                return Err(ProxyConfigError::EmptyUpstreamApiKey {
                    id: upstream.id.clone(),
                });
            }
            if upstream
                .account_id
                .as_ref()
                .is_some_and(|account_id| account_id.trim().is_empty())
            {
                return Err(ProxyConfigError::EmptyAccountId {
                    id: upstream.id.clone(),
                });
            }
            if let Some(provider) = upstream.oauth_provider.as_deref() {
                if provider
                    .trim()
                    .parse::<crate::oauth::OAuthProviderId>()
                    .is_err()
                {
                    return Err(ProxyConfigError::InvalidOAuthProvider {
                        id: upstream.id.clone(),
                    });
                }
                if upstream
                    .account_id
                    .as_deref()
                    .is_none_or(|account_id| account_id.trim().is_empty())
                {
                    return Err(ProxyConfigError::MissingOAuthAccountId {
                        id: upstream.id.clone(),
                    });
                }
            }

            let mut model_ids = BTreeSet::new();
            for model in &upstream.models {
                if model.id.trim().is_empty() {
                    return Err(ProxyConfigError::EmptyModelId {
                        upstream: upstream.id.clone(),
                    });
                }
                if !model_ids.insert(model.id.clone()) {
                    return Err(ProxyConfigError::DuplicateModelId {
                        upstream: upstream.id.clone(),
                        model: model.id.clone(),
                    });
                }
            }
        }

        for (alias, target) in &self.model_aliases {
            if alias.trim().is_empty() {
                return Err(ProxyConfigError::EmptyModelAlias);
            }
            if target.trim().is_empty() {
                return Err(ProxyConfigError::EmptyModelAliasTarget(alias.clone()));
            }
        }
        for (model, fallbacks) in &self.model_fallbacks {
            if model.trim().is_empty() {
                return Err(ProxyConfigError::EmptyModelAlias);
            }
            let mut seen = BTreeSet::new();
            for fallback in fallbacks {
                if fallback.trim().is_empty() {
                    return Err(ProxyConfigError::EmptyModelAliasTarget(model.clone()));
                }
                if !seen.insert(fallback) {
                    return Err(ProxyConfigError::DuplicateModelId {
                        upstream: format!("fallback:{model}"),
                        model: fallback.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn normalized_upstream_url(upstream: &ProxyUpstreamConfig) -> String {
        upstream.base_url.trim().trim_end_matches('/').to_string()
    }
}

fn default_bind_host() -> String {
    DEFAULT_PROXY_HOST.to_string()
}

fn default_port() -> u16 {
    DEFAULT_PROXY_PORT
}

fn default_request_timeout_secs() -> u64 {
    DEFAULT_REQUEST_TIMEOUT_SECS
}

fn default_max_retries() -> u32 {
    DEFAULT_MAX_RETRIES
}

fn default_enabled() -> bool {
    true
}

fn default_owned_by() -> String {
    "openmesh".to_string()
}

fn redacted_count(count: usize) -> String {
    if count == 0 {
        "none".to_string()
    } else {
        format!("{count} configured")
    }
}
