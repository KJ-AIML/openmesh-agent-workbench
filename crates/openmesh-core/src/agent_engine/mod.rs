//! OpenMesh Agent Engine (0.1.23) — live LLM + tool loop.
//!
//! Parallel to Work Proxy draft (`proxy_runtime_axga`), which remains tool-free.
//! Inspired by coding-agent architecture concepts; OpenMesh-branded reimplementation.

pub mod continue_ops;
pub mod engine_loop;
pub mod extensions;
pub mod live_ask;
pub mod patch;
pub mod path_safety;
pub mod provider;
pub mod recipes;
pub mod registry;
pub mod secrets;
pub mod types;
pub mod workspace_tools;

pub use engine_loop::run_agent_turn;
pub use extensions::{
    build_skills_prompt_section, enrich_system_prompt, install_from_path, load_inventory,
    local_catalog, parse_skill_markdown, CatalogEntry, ExtensionSource, ExtensionsInventory,
    ExtensionsSettings, HookDefinition, HookEvent, PluginRecord, SkillPack,
};
pub use live_ask::{
    run_live_ask, run_live_ask_with_provider, LiveAskError, LiveAskRequest, LIVE_ASK_SYSTEM_PROMPT,
};
pub use patch::{
    apply_patch, format_patch_summary, list_recent_runs, propose_patch_from_args, read_patch,
    reject_patch, rollback_patch, AgentRunRecord, PatchRecord, PatchStatus,
};
pub use recipes::{
    cancel_recipe_run, ensure_default_recipes, get_recipe, list_recipes, run_recipe, LogCallback,
    Recipe, RecipeRunResult,
};
pub use continue_ops::{
    approve_handoff, create_handoff_draft, link_session, list_session_links, mesh_query,
    pending_questions_json, update_task, write_delegate_brief, SessionLink,
};
pub use workspace_tools::WorkspaceToolExecutor;
pub use provider::{
    build_request_body, parse_chat_completion, probe_provider, resolve_provider_kind,
    AssistantTurn, ChatProvider, OpenAiCompatibleProvider, ProviderConfig, ProviderProbeResult,
    ScriptedProvider,
};
pub use registry::{
    builtin_tool_specs, default_tool_names, filter_tools, StubToolExecutor, ToolExecutor,
};
pub use secrets::{
    AgentSecretStore, CascadingSecretStore, EnvSecretStore, FileSecretStore, MemorySecretStore,
};
pub use types::{
    AgentDefinition, AgentEngineError, AgentProviderKind, AgentSession, ChatMessage, ChatRole,
    EngineTurnResult, ToolCallRequest, ToolSpec, ToolStep, AGENT_ENGINE_PROTOCOL,
    DEFAULT_MAX_TOOL_ITERATIONS, DEFAULT_SYSTEM_PROMPT,
};

#[cfg(test)]
mod provider_parse_tests {
    use super::*;

    #[test]
    fn parses_tool_calls_payload() {
        let body = r#"{
          "choices": [{
            "message": {
              "content": null,
              "tool_calls": [{
                "id": "c1",
                "type": "function",
                "function": { "name": "list_docs", "arguments": "{}" }
              }]
            }
          }]
        }"#;
        let turn = parse_chat_completion(body).unwrap();
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].name, "list_docs");
    }

    #[test]
    fn request_body_includes_tools() {
        let tools = builtin_tool_specs();
        let msgs = vec![ChatMessage {
            role: ChatRole::User,
            content: "hi".into(),
            tool_call_id: None,
            name: None,
            tool_calls: vec![],
        }];
        let body = build_request_body("gpt-4o-mini", &msgs, &tools);
        assert!(body["tools"].as_array().unwrap().len() >= 3);
        assert_eq!(body["tool_choice"], "auto");
    }

    #[test]
    fn resolve_provider_kind_defaults_and_overrides() {
        let (kind, base) = resolve_provider_kind(Some("openai"), None);
        assert_eq!(kind, AgentProviderKind::OpenAi);
        assert!(base.is_none());

        let (kind, base) = resolve_provider_kind(Some("deepseek"), None);
        assert_eq!(kind, AgentProviderKind::DeepSeek);
        assert!(base.is_none());

        let (kind, base) = resolve_provider_kind(Some("xai"), None);
        assert_eq!(kind, AgentProviderKind::OpenAiCompatible);
        assert_eq!(base.as_deref(), Some("https://api.x.ai/v1"));

        let (kind, base) =
            resolve_provider_kind(Some("openai"), Some("https://example.com/v1"));
        assert_eq!(kind, AgentProviderKind::OpenAiCompatible);
        assert_eq!(base.as_deref(), Some("https://example.com/v1"));
    }

    #[test]
    fn probe_rejects_dashscope_coding_plan_without_network() {
        let result = probe_provider(
            "sk-test",
            "qwen-plus",
            Some("openai-compatible"),
            Some("https://coding-intl.dashscope.aliyuncs.com/v1"),
        )
        .expect("probe returns structured result");
        assert!(!result.ok);
        assert!(result.error.as_deref().unwrap_or("").contains("Coding Plan"));
        assert_eq!(result.latency_ms, 0);
        assert!(result.reply_preview.is_none());
    }

    #[test]
    fn provider_probe_result_serializes_camel_case() {
        let result = ProviderProbeResult {
            ok: true,
            model: "gpt-4o-mini".into(),
            base_url: "https://api.openai.com/v1".into(),
            latency_ms: 42,
            reply_preview: Some("ok".into()),
            error: None,
        };
        let v = serde_json::to_value(&result).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["baseUrl"], "https://api.openai.com/v1");
        assert_eq!(v["latencyMs"], 42);
        assert_eq!(v["replyPreview"], "ok");
        assert!(v.get("error").is_none());
    }
}
