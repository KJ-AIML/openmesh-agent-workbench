//! OpenMesh LLM runtime port (A5.1).
//!
//! Agent policy/tool authority stays in `agent_engine::policy`. This module is
//! the provider-transport contract: messages in, completion (optional tools and
//! usage) out. It is not an authority layer.

use crate::agent_engine::types::{AgentEngineError, ChatMessage, ToolCallRequest, ToolSpec};
use serde::{Deserialize, Serialize};

/// Token usage reported by a provider. All fields optional — never fabricated.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

/// One model round-trip. Tool calls are data for the Agent Engine loop.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmCompletion {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<LlmUsage>,
}

impl LlmCompletion {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: vec![],
            usage: None,
        }
    }
}

/// Normalized provider-runtime failure. Not an agent-policy error.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmRuntimeError {
    #[error("API key not configured")]
    MissingCredentials,
    #[error("authentication failed")]
    Authentication,
    #[error("provider unavailable")]
    ProviderUnavailable,
    #[error("rate limited")]
    RateLimited,
    #[error("invalid model")]
    InvalidModel,
    #[error("context limit")]
    ContextLimit,
    #[error("cancelled")]
    Cancelled,
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("{0}")]
    Upstream(String),
}

impl LlmRuntimeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::MissingCredentials => "missing_credentials",
            Self::Authentication => "authentication",
            Self::ProviderUnavailable => "provider_unavailable",
            Self::RateLimited => "rate_limited",
            Self::InvalidModel => "invalid_model",
            Self::ContextLimit => "context_limit",
            Self::Cancelled => "cancelled",
            Self::InvalidResponse(_) => "invalid_response",
            Self::Upstream(_) => "upstream",
        }
    }

    /// Map HTTP status + body (already redacted/truncated by the caller).
    pub fn from_http_status(status: u16, body: &str) -> Self {
        let lower = body.to_ascii_lowercase();
        match status {
            401 | 403 => Self::Authentication,
            429 => Self::RateLimited,
            404 if lower.contains("model") => Self::InvalidModel,
            _ if lower.contains("context_length")
                || lower.contains("maximum context")
                || lower.contains("context window") =>
            {
                Self::ContextLimit
            }
            408 | 502 | 503 | 504 => Self::ProviderUnavailable,
            _ => Self::Upstream(format!("HTTP {status}: {body}")),
        }
    }
}

impl From<LlmRuntimeError> for AgentEngineError {
    fn from(err: LlmRuntimeError) -> Self {
        match err {
            LlmRuntimeError::MissingCredentials => AgentEngineError::MissingApiKey,
            LlmRuntimeError::InvalidResponse(s) => AgentEngineError::InvalidResponse(s),
            other => AgentEngineError::Provider(format!("{}: {}", other.code(), other)),
        }
    }
}

/// Blocking completion port used by Agent Engine.
///
/// Streaming is not part of this contract today (Chat is non-stream).
pub trait LlmRuntime: Send + Sync {
    fn complete(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolSpec],
    ) -> Result<LlmCompletion, LlmRuntimeError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_status_classes_are_distinct() {
        assert_eq!(
            LlmRuntimeError::from_http_status(401, "nope").code(),
            "authentication"
        );
        assert_eq!(
            LlmRuntimeError::from_http_status(429, "slow").code(),
            "rate_limited"
        );
        assert_eq!(
            LlmRuntimeError::from_http_status(404, "model not found").code(),
            "invalid_model"
        );
        assert_eq!(
            LlmRuntimeError::from_http_status(400, "context_length_exceeded").code(),
            "context_limit"
        );
        assert_eq!(
            LlmRuntimeError::from_http_status(503, "down").code(),
            "provider_unavailable"
        );
        assert_eq!(
            LlmRuntimeError::from_http_status(500, "boom").code(),
            "upstream"
        );
    }

    #[test]
    fn missing_credentials_maps_to_engine_missing_key() {
        let err = AgentEngineError::from(LlmRuntimeError::MissingCredentials);
        assert!(matches!(err, AgentEngineError::MissingApiKey));
    }

    #[test]
    fn engine_loop_source_does_not_name_http_transport() {
        let src = include_str!("../agent_engine/engine_loop.rs");
        assert!(
            !src.contains("OpenAiCompatibleProvider"),
            "engine loop must not construct the HTTP provider"
        );
        assert!(
            !src.contains("reqwest"),
            "engine loop must not call reqwest"
        );
        assert!(
            src.contains("LlmRuntime"),
            "engine loop must depend on the LLM runtime port"
        );
    }
}
