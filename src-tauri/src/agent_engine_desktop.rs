//! Dev Track 0.1.23 — OpenMesh Agent Engine Desktop IPC.

use openmesh_core::agent_engine::{
    probe_provider, resolve_provider_kind, run_agent_turn, AgentDefinition, AgentSecretStore,
    AgentSession, CascadingSecretStore, ChatMessage, ChatRole, EngineTurnResult,
    OpenAiCompatibleProvider, ProviderConfig, ProviderProbeResult, ToolExecutor,
};
use openmesh_core::context_service;
use openmesh_core::mesh::peers::list_peers;
use openmesh_core::pilot::build_pilot_pack;
use openmesh_core::rc::build_rc_pack;
use openmesh_core::storage::{read_project, Project};
use openmesh_core::continuity::{
    current_state_projection_path, load_continuity_input_snapshot, read_current_state_projection,
    rebuild_current_state_projection,
};
use openmesh_core::return_digest::build_pending_questions_view;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    /// Optional unsaved key from the Settings input (not persisted by this call).
    #[serde(default)]
    pub api_key: Option<String>,
}

#[tauri::command]
pub fn agent_provider_test(request: AgentProviderTestRequest) -> Result<ProviderProbeResult, String> {
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

#[tauri::command]
pub fn agent_engine_turn(
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

    let cfg = ProviderConfig::from_definition(&def, &api_key).map_err(|e| e.to_string())?;
    let provider_client = OpenAiCompatibleProvider::new(cfg).map_err(|e| e.to_string())?;
    let executor = OpenMeshToolExecutor {
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

struct OpenMeshToolExecutor {
    project_path: String,
}

impl ToolExecutor for OpenMeshToolExecutor {
    fn execute(&self, tool_name: &str, arguments_json: &str) -> Result<String, String> {
        match tool_name {
            "project_info" => {
                let project: Option<Project> = read_project(&self.project_path, "project.json");
                Ok(serde_json::to_string_pretty(&serde_json::json!({
                    "path": self.project_path,
                    "project": project,
                }))
                .unwrap_or_else(|_| "{}".into()))
            }
            "list_docs" => list_dir_names(&self.project_path, "docs"),
            "list_notes" => list_dir_names(&self.project_path, "notes"),
            "continuity_summary" => continuity_summary_json(&self.project_path),
            "list_mesh_peers" => {
                let peers = list_peers(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&peers).unwrap_or_else(|_| "[]".into()))
            }
            "pilot_status" => {
                let pack = build_pilot_pack(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&pack).unwrap_or_else(|_| "{}".into()))
            }
            "rc_status" => {
                let pack = build_rc_pack(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&pack).unwrap_or_else(|_| "{}".into()))
            }
            "git_status" => git_status_json(&self.project_path),
            "search_context" => {
                let args: serde_json::Value =
                    serde_json::from_str(arguments_json).unwrap_or(serde_json::json!({}));
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let hits = context_service::search_project_context(
                    &self.project_path,
                    &query,
                    None,
                    Some(12),
                )
                .map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&hits).unwrap_or_else(|_| "[]".into()))
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

fn list_dir_names(project_path: &str, folder: &str) -> Result<String, String> {
    let dir = PathBuf::from(project_path).join(folder);
    if !dir.is_dir() {
        return Ok(format!("(no {folder}/ directory)"));
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        names.push(entry.file_name().to_string_lossy().to_string());
    }
    names.sort();
    Ok(serde_json::to_string_pretty(&names).unwrap_or_else(|_| "[]".into()))
}

fn continuity_summary_json(project_path: &str) -> Result<String, String> {
    let pending = (|| {
        let snapshot = load_continuity_input_snapshot(project_path).ok()?;
        let current = if current_state_projection_path(project_path).exists() {
            read_current_state_projection(project_path).ok()
        } else {
            rebuild_current_state_projection(project_path).ok()
        }?;
        build_pending_questions_view(project_path, &snapshot, &current).ok()
    })();
    let peers = list_peers(project_path).unwrap_or_default();
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "pending": pending,
        "peerCount": peers.len(),
        "peers": peers,
    }))
    .unwrap_or_else(|_| "{}".into()))
}

fn git_status_json(project_path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", project_path, "status", "--porcelain=v1", "-b"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
