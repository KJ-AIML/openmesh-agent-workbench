//! Shared provider identity for Agent Engine and the HTTP proxy adapter.

use super::LlmRuntimeError;
use crate::agent_engine::provider::ProviderConfig;
use crate::proxy_server::{ProxyProviderProtocol, ProxyUpstreamConfig};

/// Canonical in-process description of one provider/model/credential triple.
/// Secrets stay in memory; this type is never written to project JSON.
#[derive(Debug, Clone)]
pub struct ProviderRuntimeSpec {
    pub id: String,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
    pub protocol: ProxyProviderProtocol,
}

impl ProviderRuntimeSpec {
    pub fn from_agent_config(id: impl Into<String>, config: &ProviderConfig) -> Self {
        Self {
            id: id.into(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            model: config.model.clone(),
            api_key: config.api_key.clone(),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
        }
    }

    /// Build from a proxy upstream that already has a bearer key (not OAuth).
    pub fn from_proxy_upstream(
        upstream: &ProxyUpstreamConfig,
        model: &str,
    ) -> Result<Self, LlmRuntimeError> {
        let key = upstream
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .ok_or(LlmRuntimeError::MissingCredentials)?;
        let model = if model.trim().is_empty() {
            upstream
                .models
                .first()
                .map(|m| m.id.as_str())
                .filter(|id| !id.is_empty())
                .ok_or(LlmRuntimeError::InvalidModel)?
        } else {
            model.trim()
        };
        Ok(Self {
            id: upstream.id.clone(),
            base_url: crate::proxy_server::ProxyServerConfig::normalized_upstream_url(upstream),
            model: model.to_string(),
            api_key: key.to_string(),
            protocol: upstream.protocol,
        })
    }

    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url.trim_end_matches('/'))
    }
}

impl ProviderConfig {
    pub fn to_runtime_spec(&self, id: impl Into<String>) -> ProviderRuntimeSpec {
        ProviderRuntimeSpec::from_agent_config(id, self)
    }
}
