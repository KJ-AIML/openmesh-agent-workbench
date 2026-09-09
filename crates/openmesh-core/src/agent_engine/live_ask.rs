//! Shared live Agent Engine ask path for LAN peer ask + Continuity Proxy.
//!
//! Distinct from LocalScaffold online-proxy paste: requires a configured API key
//! and returns structured errors when the peer cannot answer.

use super::engine_loop::run_agent_turn;
use super::policy::{authorize_agent_turn, AgentOrigin, AgentRequestContext};
use super::provider::{resolve_provider_kind, OpenAiCompatibleProvider, ProviderConfig};
use super::registry::ToolExecutor;
use super::secrets::{AgentSecretStore, CascadingSecretStore};
use super::types::{AgentDefinition, AgentEngineError, AgentSession, EngineTurnResult};
use super::workspace_tools::WorkspaceToolExecutor;
use crate::llm_runtime::LlmRuntime;
use crate::storage::{default_settings, read_global, Settings};
use thiserror::Error;

pub const LIVE_ASK_SYSTEM_PROMPT: &str = r#"You are answering a read-only OpenMesh live ask against this local workspace.
Use tools for factual project state. Do not invent Continuity/mesh/team facts.
Do not write or modify project files. Keep answers concise and useful.
Never request or echo API keys or secrets."#;

#[derive(Debug, Clone)]
pub struct LiveAskRequest {
    pub question: String,
    /// Optional freshness / evidence context prepended to the user message.
    pub context_prefix: Option<String>,
    pub provider_name: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
    /// Extra system prompt lines (after the live-ask base prompt).
    pub system_extra: Option<String>,
    /// Must be a remote query origin (`LanPeer` or `ContinuityQuery`).
    pub origin: AgentOrigin,
}

#[derive(Debug, Error)]
pub enum LiveAskError {
    #[error("API key not configured on this peer. Save a key in Settings (or set OPENMESH_AGENT_API_KEY) before answering live asks.")]
    MissingApiKey,
    #[error("empty question")]
    EmptyQuestion,
    #[error("live ask origin is not a remote query origin")]
    InvalidOrigin,
    #[error("policy: {0}")]
    Policy(String),
    #[error("provider: {0}")]
    Provider(String),
    #[error("engine: {0}")]
    Engine(String),
}

impl LiveAskError {
    pub fn code(&self) -> &'static str {
        match self {
            LiveAskError::MissingApiKey => "missing_api_key",
            LiveAskError::EmptyQuestion => "empty_question",
            LiveAskError::InvalidOrigin => "invalid_origin",
            LiveAskError::Policy(_) => "policy_error",
            LiveAskError::Provider(_) => "provider_error",
            LiveAskError::Engine(_) => "engine_error",
        }
    }

    pub fn to_json_body(&self) -> String {
        serde_json::json!({
            "error": self.to_string(),
            "code": self.code(),
        })
        .to_string()
    }
}

/// Resolve API key + provider settings for a live ask.
pub fn resolve_live_ask_config(
    request: &LiveAskRequest,
) -> Result<(String, AgentDefinition), LiveAskError> {
    if request.question.trim().is_empty() {
        return Err(LiveAskError::EmptyQuestion);
    }
    let store = CascadingSecretStore::default();
    let api_key = store
        .get_api_key()
        .map_err(|e| LiveAskError::Provider(e.to_string()))?
        .filter(|k| !k.trim().is_empty())
        .ok_or(LiveAskError::MissingApiKey)?;

    let settings = read_global::<Settings>("settings.json").unwrap_or_else(default_settings);
    let model = request
        .model
        .clone()
        .or_else(|| settings.provider.default_model.clone())
        .or_else(|| settings.models.coding_model.clone())
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".into());
    let provider_name = request
        .provider_name
        .clone()
        .or_else(|| settings.provider.name.clone());
    let base_url = request
        .base_url
        .clone()
        .or_else(|| settings.provider.api_base_url.clone());

    let (provider, resolved_base) =
        resolve_provider_kind(provider_name.as_deref(), base_url.as_deref());

    let mut system = LIVE_ASK_SYSTEM_PROMPT.to_string();
    if let Some(extra) = request.system_extra.as_deref() {
        if !extra.trim().is_empty() {
            system.push_str("\n\n");
            system.push_str(extra.trim());
        }
    }

    let def = AgentDefinition {
        name: "openmesh-live-ask".into(),
        system_prompt: system,
        provider,
        model,
        base_url: resolved_base,
        // Empty allowlist = all built-in read-mostly tools (see filter_tools).
        tool_allowlist: vec![],
        max_tool_iterations: 3,
    };
    Ok((api_key, def))
}

fn compose_user_text(request: &LiveAskRequest) -> String {
    match request.context_prefix.as_deref() {
        Some(prefix) if !prefix.trim().is_empty() => {
            format!("{}\n\nQuestion: {}", prefix.trim(), request.question.trim())
        }
        _ => request.question.trim().to_string(),
    }
}

/// Run a live ask with an injected provider (unit tests / ScriptedProvider).
pub fn run_live_ask_with_provider(
    project_path: &str,
    def: &AgentDefinition,
    request: &LiveAskRequest,
    provider: &dyn LlmRuntime,
    executor: &dyn ToolExecutor,
) -> Result<EngineTurnResult, LiveAskError> {
    if request.question.trim().is_empty() {
        return Err(LiveAskError::EmptyQuestion);
    }
    if !request.origin.is_live_ask() {
        return Err(LiveAskError::InvalidOrigin);
    }
    let auth = authorize_agent_turn(&AgentRequestContext::live_ask(request.origin, project_path))
        .map_err(|e| LiveAskError::Policy(e.to_string()))?;
    let mut session = AgentSession::default();
    let user_text = compose_user_text(request);
    run_agent_turn(&auth, def, &mut session, &user_text, provider, executor)
        .map_err(|e| LiveAskError::Engine(e.to_string()))
}

/// Production live ask: CascadingSecretStore + OpenAI-compatible provider + workspace tools.
pub fn run_live_ask(
    project_path: &str,
    request: &LiveAskRequest,
) -> Result<EngineTurnResult, LiveAskError> {
    let (api_key, def) = resolve_live_ask_config(request)?;
    let agent_cfg = ProviderConfig::from_definition(&def, &api_key)
        .map_err(|e| LiveAskError::Provider(e.to_string()))?;
    let spec = crate::llm_runtime::resolve_blocking_spec(&agent_cfg);
    let client = OpenAiCompatibleProvider::from_runtime_spec(spec)
        .map_err(|e| LiveAskError::Provider(e.to_string()))?;
    let executor = WorkspaceToolExecutor::new(project_path.to_string(), request.origin);
    run_live_ask_with_provider(project_path, &def, request, &client, &executor)
}

/// Map AgentEngineError missing-key into LiveAskError when constructing providers manually.
pub fn map_engine_missing_key(err: AgentEngineError) -> LiveAskError {
    match err {
        AgentEngineError::MissingApiKey => LiveAskError::MissingApiKey,
        other => LiveAskError::Engine(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::provider::{AssistantTurn, ScriptedProvider};
    use crate::agent_engine::registry::StubToolExecutor;
    use crate::storage::init_project;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> String {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("openmesh-live-ask-{}-{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn live_ask_with_scripted_provider_returns_answer() {
        let project = temp_project();
        let provider = ScriptedProvider::new(vec![AssistantTurn {
            content: "Ship LAN ask via Agent Engine.".into(),
            tool_calls: vec![],
            usage: None,
        }]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let mut def = AgentDefinition::default_workspace_agent("test-model");
        def.system_prompt = LIVE_ASK_SYSTEM_PROMPT.into();
        def.tool_allowlist = vec!["__none__".into()];
        let req = LiveAskRequest {
            question: "What should we ship?".into(),
            context_prefix: Some("Evidence freshness: fresh enough for tier LowImpact.".into()),
            provider_name: None,
            model: Some("test-model".into()),
            base_url: None,
            system_extra: None,
            origin: AgentOrigin::LanPeer,
        };
        let result =
            run_live_ask_with_provider(&project, &def, &req, &provider, &executor).unwrap();
        assert!(result.assistant_text.contains("Agent Engine"));
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn empty_question_fails_closed() {
        let provider = ScriptedProvider::new(vec![]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let def = AgentDefinition::default_workspace_agent("m");
        let req = LiveAskRequest {
            question: "   ".into(),
            context_prefix: None,
            provider_name: None,
            model: None,
            base_url: None,
            system_extra: None,
            origin: AgentOrigin::LanPeer,
        };
        let err =
            run_live_ask_with_provider("/tmp/ws", &def, &req, &provider, &executor).unwrap_err();
        assert_eq!(err.code(), "empty_question");
    }

    #[test]
    fn local_chat_origin_cannot_use_live_ask_path() {
        let provider = ScriptedProvider::new(vec![AssistantTurn {
            content: "should not run".into(),
            tool_calls: vec![],
            usage: None,
        }]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let def = AgentDefinition::default_workspace_agent("m");
        let req = LiveAskRequest {
            question: "hello".into(),
            context_prefix: None,
            provider_name: None,
            model: None,
            base_url: None,
            system_extra: None,
            origin: AgentOrigin::LocalChat,
        };
        let err =
            run_live_ask_with_provider("/tmp/ws", &def, &req, &provider, &executor).unwrap_err();
        assert_eq!(err.code(), "invalid_origin");
    }

    #[test]
    fn missing_api_key_error_has_stable_code() {
        let err = LiveAskError::MissingApiKey;
        assert_eq!(err.code(), "missing_api_key");
        assert!(err.to_json_body().contains("missing_api_key"));
        assert!(err.to_string().contains("API key"));
    }

    #[test]
    fn live_ask_prompt_is_read_only() {
        assert!(LIVE_ASK_SYSTEM_PROMPT.contains("read-only"));
        assert!(LIVE_ASK_SYSTEM_PROMPT.contains("Do not write"));
    }
}
