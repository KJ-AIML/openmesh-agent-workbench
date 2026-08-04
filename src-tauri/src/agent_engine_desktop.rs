//! Dev Track 0.1.23 — OpenMesh Agent Engine Desktop IPC.

use openmesh_core::agent_engine::{
    apply_patch, cancel_recipe_run, enrich_system_prompt, format_patch_summary, get_recipe,
    list_recipes, list_recent_runs, load_inventory, probe_provider, read_patch, reject_patch,
    resolve_provider_kind, rollback_patch, run_agent_turn, run_recipe, write_delegate_brief,
    AgentDefinition, AgentSecretStore, AgentSession, CascadingSecretStore, ChatMessage, ChatRole,
    EngineTurnResult, LogCallback, OpenAiCompatibleProvider, PatchRecord, ProviderConfig,
    ProviderProbeResult, Recipe, RecipeRunResult, ToolExecutor, WorkspaceToolExecutor,
};
use openmesh_core::storage::{default_settings, read_global, Settings};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tauri::Emitter;

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
            "propose_patch",
            "list_recipes",
            "pending_questions",
            "create_handoff_draft",
            "update_task",
            "link_session",
            "mesh_query",
            "continuity_summary",
            "list_mesh_peers",
            // approve_handoff stays human-slash via dedicated IPC, not free slash tool exec
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

#[tauri::command]
pub async fn agent_patch_get(project_path: String, patch_id: String) -> Result<PatchRecord, String> {
    tauri::async_runtime::spawn_blocking(move || read_patch(&project_path, &patch_id))
        .await
        .map_err(|e| format!("patch get failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_patch_apply(project_path: String, patch_id: String) -> Result<PatchRecord, String> {
    tauri::async_runtime::spawn_blocking(move || apply_patch(&project_path, &patch_id))
        .await
        .map_err(|e| format!("patch apply failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_patch_reject(
    project_path: String,
    patch_id: String,
) -> Result<PatchRecord, String> {
    tauri::async_runtime::spawn_blocking(move || reject_patch(&project_path, &patch_id))
        .await
        .map_err(|e| format!("patch reject failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_patch_rollback(
    project_path: String,
    patch_id: String,
) -> Result<PatchRecord, String> {
    tauri::async_runtime::spawn_blocking(move || rollback_patch(&project_path, &patch_id))
        .await
        .map_err(|e| format!("patch rollback failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_patch_summary(
    project_path: String,
    patch_id: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let patch = read_patch(&project_path, &patch_id)?;
        Ok(format_patch_summary(&patch))
    })
    .await
    .map_err(|e| format!("patch summary failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_recipe_list(project_path: String) -> Result<Vec<Recipe>, String> {
    tauri::async_runtime::spawn_blocking(move || list_recipes(&project_path))
        .await
        .map_err(|e| format!("recipe list failed to join: {e}"))?
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRecipeRunRequest {
    pub recipe_id: String,
    #[serde(default)]
    pub run_key: Option<String>,
}

#[tauri::command]
pub async fn agent_recipe_run(
    app: tauri::AppHandle,
    project_path: String,
    request: AgentRecipeRunRequest,
) -> Result<RecipeRunResult, String> {
    let run_key = request
        .run_key
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("{}:{}", project_path, request.recipe_id));
    let recipe_id = request.recipe_id;
    tauri::async_runtime::spawn_blocking(move || {
        let app_log = app.clone();
        let key_for_events = run_key.clone();
        let on_log: LogCallback = Arc::new(move |line| {
            let _ = app_log.emit(
                "agent-run-log",
                json!({ "runKey": key_for_events, "line": line }),
            );
        });
        let result = run_recipe(&project_path, &recipe_id, &run_key, Some(on_log))?;
        let _ = app.emit(
            "agent-run-done",
            json!({
                "runKey": run_key,
                "ok": result.ok,
                "recipeId": result.recipe_id,
                "exitCode": result.exit_code,
            }),
        );
        Ok(result)
    })
    .await
    .map_err(|e| format!("recipe run failed to join: {e}"))?
}

#[tauri::command]
pub fn agent_recipe_cancel(run_key: String) -> Result<bool, String> {
    Ok(cancel_recipe_run(&run_key))
}

#[tauri::command]
pub async fn agent_recipe_get(project_path: String, recipe_id: String) -> Result<Recipe, String> {
    tauri::async_runtime::spawn_blocking(move || get_recipe(&project_path, &recipe_id))
        .await
        .map_err(|e| format!("recipe get failed to join: {e}"))?
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDelegateBriefRequest {
    pub tool: String,
    pub summary: String,
}

#[tauri::command]
pub async fn agent_delegate_brief(
    project_path: String,
    request: AgentDelegateBriefRequest,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = write_delegate_brief(&project_path, &request.tool, &request.summary)?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("delegate brief failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_runs_recent(
    project_path: String,
    limit: Option<usize>,
) -> Result<Vec<openmesh_core::agent_engine::AgentRunRecord>, String> {
    tauri::async_runtime::spawn_blocking(move || list_recent_runs(&project_path, limit.unwrap_or(10)))
        .await
        .map_err(|e| format!("runs list failed to join: {e}"))?
}

#[tauri::command]
pub async fn agent_handoff_approve(
    project_path: String,
    handoff_id: String,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        openmesh_core::agent_engine::approve_handoff(
            &project_path,
            &json!({ "handoffId": handoff_id }).to_string(),
        )
    })
    .await
    .map_err(|e| format!("handoff approve failed to join: {e}"))?
}
