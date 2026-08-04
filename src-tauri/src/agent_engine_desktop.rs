//! Dev Track 0.1.23 — OpenMesh Agent Engine Desktop IPC.

use openmesh_core::agent_engine::{
    enrich_system_prompt, load_inventory, probe_provider, resolve_provider_kind, run_agent_turn,
    AgentDefinition, AgentSecretStore, AgentSession, CascadingSecretStore, ChatMessage, ChatRole,
    EngineTurnResult, OpenAiCompatibleProvider, ProviderConfig, ProviderProbeResult, ToolExecutor,
    WorkspaceToolExecutor,
};
use openmesh_core::storage::{default_settings, read_global, Settings};
use serde::{Deserialize, Serialize};

fn secrets() -> CascadingSecretStore {
    CascadingSecretStore::default()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSecretStatus {
    pub configured: bool,
    pub store: String,
}

#[tauri::command]
pub fn agent_secret_status() -> Result<AgentSecretStatus, String> {
    let store = secrets();
    Ok(AgentSecretStatus {
        configured: store.is_configured().map_err(|e| e.to_string())?,
        store: store.file.path().display().to_string(),
    })
}

#[tauri::command]
pub fn agent_secret_set(api_key: String) -> Result<AgentSecretStatus, String> {
    let store = secrets();
    store.set_api_key(&api_key).map_err(|e| e.to_string())?;
    agent_secret_status()
}

#[tauri::command]
pub fn agent_secret_clear() -> Result<AgentSecretStatus, String> {
    let store = secrets();
    store.clear_api_key().map_err(|e| e.to_string())?;
    agent_secret_status()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderTestRequest {
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    /// Unsaved key from the Settings input — not persisted by this call.
    #[serde(default)]
    pub api_key: Option<String>,
}

fn agent_provider_test_blocking(
    request: AgentProviderTestRequest,
) -> Result<ProviderProbeResult, String> {
    let store = secrets();
    let api_key = request
        .api_key
        .filter(|k| !k.trim().is_empty())
        .or_else(|| store.get_api_key().ok().flatten())
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            "No API key. Enter a key above (or Save Key first), then Test connection.".to_string()
        })?;

    let model = request
        .model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".into());

    probe_provider(
        &api_key,
        &model,
        request.provider_name.as_deref(),
        request.base_url.as_deref(),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_provider_test(
    request: AgentProviderTestRequest,
) -> Result<ProviderProbeResult, String> {
    tauri::async_runtime::spawn_blocking(move || agent_provider_test_blocking(request))
        .await
        .map_err(|e| format!("provider test failed to join: {e}"))?
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEngineTurnRequest {
    pub question: String,
    #[serde(default)]
    pub messages: Vec<AgentUiMessage>,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentUiMessage {
    pub role: String,
    pub content: String,
}

/// Blocking LLM + tool loop. Must never run on the UI/IPC thread — Tauri's
/// default sync command path executes inline inside `webview.on_message`,
/// which freezes the macOS window (rainbow beachball) for the whole turn.
fn agent_engine_turn_blocking(
    project_path: String,
    request: AgentEngineTurnRequest,
) -> Result<EngineTurnResult, String> {
    let store = secrets();
    let api_key = store
        .get_api_key()
        .map_err(|e| e.to_string())?
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| "API key not configured. Save a key in Settings.".to_string())?;

    let model = request
        .model
        .filter(|m| !m.trim().is_empty())
        .unwrap_or_else(|| "gpt-4o-mini".into());

    let (provider, base_url) = resolve_provider_kind(
        request.provider_name.as_deref(),
        request.base_url.as_deref(),
    );

    let mut def = AgentDefinition::default_workspace_agent(&model);
    def.provider = provider;
    def.base_url = base_url;

    let mut session = AgentSession {
        messages: request
            .messages
            .into_iter()
            .filter_map(|m| {
                let role = match m.role.to_lowercase().as_str() {
                    "system" => ChatRole::System,
                    "assistant" => ChatRole::Assistant,
                    "user" => ChatRole::User,
                    _ => return None,
                };
                Some(ChatMessage {
                    role,
                    content: m.content,
                    tool_call_id: None,
                    name: None,
                    tool_calls: vec![],
                })
            })
            .collect(),
    };

    // Inject enabled skills + declarative hooks into the system prompt.
    let ext_settings = read_global::<Settings>("settings.json")
        .unwrap_or_else(default_settings)
        .extensions;
    let inventory = load_inventory(Some(&project_path), &ext_settings);
    let is_new_chat = !session
        .messages
        .iter()
        .any(|m| matches!(m.role, ChatRole::User | ChatRole::Assistant));
    def.system_prompt = enrich_system_prompt(&def.system_prompt, &inventory, is_new_chat);

    let cfg = ProviderConfig::from_definition(&def, &api_key).map_err(|e| e.to_string())?;
    let provider_client = OpenAiCompatibleProvider::new(cfg).map_err(|e| e.to_string())?;
    let executor = WorkspaceToolExecutor {
        project_path: project_path.clone(),
    };

    run_agent_turn(
        &def,
        &mut session,
        &request.question,
        &provider_client,
        &executor,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn agent_engine_turn(
    project_path: String,
    request: AgentEngineTurnRequest,
) -> Result<EngineTurnResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        agent_engine_turn_blocking(project_path, request)
    })
    .await
    .map_err(|e| format!("agent engine turn failed to join: {e}"))?
}

/// Direct read-tool invoke for Agent Chat slash/keyword fast paths.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentWorkspaceToolRequest {
    pub tool_name: String,
    #[serde(default)]
    pub arguments_json: String,
}

#[tauri::command]
pub async fn agent_workspace_tool(
    project_path: String,
    request: AgentWorkspaceToolRequest,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let allowed = [
            "list_dir",
            "read_file",
            "grep",
            "git_diff",
            "git_status",
            "list_docs",
            "list_notes",
            "project_info",
            "search_context",
        ];
        if !allowed.iter().any(|n| *n == request.tool_name) {
            return Err(format!(
                "tool not allowed via slash IPC: {}",
                request.tool_name
            ));
        }
        let args = if request.arguments_json.trim().is_empty() {
            "{}".to_string()
        } else {
            request.arguments_json
        };
        let executor = WorkspaceToolExecutor { project_path };
        executor.execute(&request.tool_name, &args)
    })
    .await
    .map_err(|e| format!("workspace tool failed to join: {e}"))?
}
