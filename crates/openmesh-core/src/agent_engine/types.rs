//! OpenMesh Agent Engine — domain types (0.1.23).

use serde::{Deserialize, Serialize};

pub const AGENT_ENGINE_PROTOCOL: &str = "openmesh-agent/0.1";
/// Provider round-trips that may include tool calls (keep low — each hop is slow).
pub const DEFAULT_MAX_TOOL_ITERATIONS: u32 = 4;
/// Hard cap on tool executions in a single model message (prevents tool storms).
pub const DEFAULT_MAX_TOOLS_PER_ITERATION: usize = 3;
pub const DEFAULT_TOOL_RESULT_MAX_CHARS: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentProviderKind {
    OpenAi,
    DeepSeek,
    /// OpenAI-compatible endpoint (e.g. xAI) via custom base URL.
    OpenAiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDefinition {
    pub name: String,
    pub system_prompt: String,
    pub provider: AgentProviderKind,
    pub model: String,
    /// Required for OpenAiCompatible; optional override for others.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(default)]
    pub tool_allowlist: Vec<String>,
    #[serde(default = "default_max_iters")]
    pub max_tool_iterations: u32,
}

fn default_max_iters() -> u32 {
    DEFAULT_MAX_TOOL_ITERATIONS
}

impl AgentDefinition {
    pub fn default_workspace_agent(model: impl Into<String>) -> Self {
        Self {
            name: "openmesh-workspace".into(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.into(),
            provider: AgentProviderKind::OpenAi,
            model: model.into(),
            base_url: None,
            tool_allowlist: vec![], // empty = Ask (read-only) tools
            max_tool_iterations: DEFAULT_MAX_TOOL_ITERATIONS,
        }
    }
}

pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are the OpenMesh workspace agent.
Help with the active local project. Be decisive and concise.

Tool budget (critical — each tool call is slow):
- Prefer answering from context. Use at most 1–3 tool calls total unless the user
  explicitly asks for a deep investigation.
- Call at most 2 tools in parallel. Never spray list_dir/grep/search_context repeatedly.
- If a term is unknown after one quick search (or none), ask a clarifying question
  instead of exhaustive searching.
- Do not invent Continuity/mesh/team facts.

UI navigation:
- Use ui_navigate to open in-app pages (work/chat/docs/sprint/notes/settings/…).
- Do not invent mouse clicks or OS automation.

Writes:
- Ask mode cannot write files or notes. Tell the user to switch to Plan/Act, or use
  Notes/Docs UI / slash tools when appropriate.
- When propose_patch is available, propose exact newContent — patches apply only after
  human approval. Never claim a file was written without approval.

Never request or echo API keys or secrets."#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCallRequest>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    /// JSON object string from the model.
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStep {
    pub tool_name: String,
    pub tool_call_id: String,
    pub ok: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineTurnResult {
    pub assistant_text: String,
    pub tool_steps: Vec<ToolStep>,
    pub iterations: u32,
    pub model: String,
    pub provider: String,
    pub refused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, thiserror::Error)]
pub enum AgentEngineError {
    #[error("API key not configured")]
    MissingApiKey,
    #[error("provider error: {0}")]
    Provider(String),
    #[error("tool error: {0}")]
    Tool(String),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("max tool iterations exceeded ({0})")]
    MaxIterations(u32),
    #[error("io: {0}")]
    Io(String),
}
