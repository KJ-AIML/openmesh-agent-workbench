//! Resolve Chat/Engine specs against the same provider records the HTTP proxy uses.

use super::spec::ProviderRuntimeSpec;
use super::LlmRuntimeError;
use crate::agent_engine::provider::ProviderConfig;
use crate::proxy_server::{
    ProxyProviderProtocol, ProxyServerConfig, DEFAULT_PROXY_HOST, DEFAULT_PROXY_PORT,
};

/// True when `url` targets the built-in OpenMesh HTTP listener.
pub fn looks_like_builtin_proxy_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url.trim()) else {
        return false;
    };
    let host = parsed.host_str().unwrap_or("");
    let loopback = host == "127.0.0.1" || host.eq_ignore_ascii_case("localhost") || host == "::1";
    if !loopback {
        return false;
    }
    let port = parsed.port().unwrap_or(DEFAULT_PROXY_PORT);
    port == DEFAULT_PROXY_PORT
        && (parsed.path().is_empty() || parsed.path() == "/" || parsed.path() == "/v1")
        && DEFAULT_PROXY_HOST == "127.0.0.1"
}

/// First enabled OpenAI-compatible upstream that already has a bearer key.
pub fn spec_from_proxy_config(
    config: &ProxyServerConfig,
    model: &str,
    upstream_id: Option<&str>,
) -> Result<ProviderRuntimeSpec, LlmRuntimeError> {
    let mut candidates = config.upstreams.iter().filter(|up| {
        up.enabled
            && up.protocol == ProxyProviderProtocol::OpenAiCompatible
            && up
                .api_key
                .as_deref()
                .map(str::trim)
                .is_some_and(|k| !k.is_empty())
    });
    let upstream = if let Some(id) = upstream_id {
        candidates
            .find(|up| up.id == id)
            .ok_or(LlmRuntimeError::InvalidModel)?
    } else {
        candidates
            .next()
            .ok_or(LlmRuntimeError::MissingCredentials)?
    };
    ProviderRuntimeSpec::from_proxy_upstream(upstream, model)
}

/// If Chat was aimed at the local HTTP proxy, use the proxy's upstream in-process.
/// Otherwise keep the Agent Engine direct spec. Listener need not be running.
pub fn prefer_in_process_spec(
    agent: &ProviderConfig,
    proxy: Option<&ProxyServerConfig>,
) -> ProviderRuntimeSpec {
    if looks_like_builtin_proxy_url(&agent.base_url) {
        if let Some(config) = proxy {
            if let Ok(spec) = spec_from_proxy_config(config, &agent.model, None) {
                return spec;
            }
        }
    }
    agent.to_runtime_spec("agent")
}

pub fn load_proxy_config_if_present() -> Option<ProxyServerConfig> {
    crate::proxy_server::read_proxy_config(&crate::proxy_server::default_proxy_config_path()).ok()
}

/// Settings/direct spec, or in-process proxy upstream when Chat points at loopback.
pub fn resolve_blocking_spec(agent: &ProviderConfig) -> ProviderRuntimeSpec {
    let proxy = load_proxy_config_if_present();
    prefer_in_process_spec(agent, proxy.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy_server::{ProxyModelConfig, ProxyUpstreamConfig};

    fn upstream(id: &str, key: &str) -> ProxyUpstreamConfig {
        ProxyUpstreamConfig {
            id: id.into(),
            base_url: "https://api.x.ai/v1".into(),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: Some(key.into()),
            enabled: true,
            priority: 0,
            account_id: None,
            oauth_provider: None,
            models: vec![ProxyModelConfig {
                id: "grok-3".into(),
                owned_by: "xai".into(),
                capabilities: vec!["chat".into()],
            }],
        }
    }

    #[test]
    fn loopback_proxy_url_is_detected() {
        assert!(looks_like_builtin_proxy_url("http://127.0.0.1:8317"));
        assert!(looks_like_builtin_proxy_url("http://localhost:8317/v1"));
        assert!(!looks_like_builtin_proxy_url("https://api.openai.com/v1"));
        assert!(!looks_like_builtin_proxy_url("http://127.0.0.1:9999"));
    }

    #[test]
    fn chat_aimed_at_local_proxy_uses_upstream_without_listener() {
        let agent = ProviderConfig {
            api_key: "unused-local".into(),
            model: "grok-3".into(),
            base_url: "http://127.0.0.1:8317/v1".into(),
        };
        let mut proxy = ProxyServerConfig::default();
        proxy.allow_unauthenticated = true;
        proxy.upstreams = vec![upstream("xai", "sk-real")];
        let spec = prefer_in_process_spec(&agent, Some(&proxy));
        assert_eq!(spec.base_url, "https://api.x.ai/v1");
        assert_eq!(spec.api_key, "sk-real");
        assert_eq!(spec.model, "grok-3");
    }

    #[test]
    fn direct_provider_url_is_unchanged() {
        let agent = ProviderConfig {
            api_key: "sk-openai".into(),
            model: "gpt-4o-mini".into(),
            base_url: "https://api.openai.com/v1".into(),
        };
        let spec = prefer_in_process_spec(&agent, None);
        assert_eq!(spec.base_url, "https://api.openai.com/v1");
        assert_eq!(spec.api_key, "sk-openai");
    }
}
