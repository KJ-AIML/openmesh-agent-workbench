//! Continue-mode helpers: tasks, handoffs, session links, gated mesh query.

use super::patch::{append_run, list_recent_runs};
use crate::continuity::{
    current_state_projection_path, load_continuity_input_snapshot, read_current_state_projection,
    rebuild_current_state_projection,
};
use crate::handoff::{
    approve_handoff_note, build_handoff_note, build_handoff_recipient, resolve_handoff_window,
    write_handoff_note, BuildHandoffRequest,
};
use crate::mesh::query::{query_remote_peer_proxy, MeshRemoteQueryRequest};
use crate::return_digest::build_pending_questions_view;
use crate::storage::{atomic_write, get_project_dir, now_iso, read_project, write_project, Project, Task};
use crate::trust_admin::{evaluate_remote_query, read_trust_policy, QueryPermission};
use crate::authority_policy::FreshnessTier;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLink {
    pub chat_session_id: String,
    pub foreign_tool: String,
    pub foreign_session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub foreign_session_path: Option<String>,
    pub created_at: String,
}

fn session_links_path(project_path: &str) -> PathBuf {
    get_project_dir(project_path)
        .join("agent")
        .join("session-links.json")
}

pub fn list_session_links(project_path: &str) -> Result<Vec<SessionLink>, String> {
    let path = session_links_path(project_path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn link_session(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).unwrap_or(json!({}));
    let chat_session_id = args
        .get("chatSessionId")
        .or_else(|| args.get("chat_session_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "link_session requires chatSessionId".to_string())?
        .trim()
        .to_string();
    let foreign_tool = args
        .get("foreignTool")
        .or_else(|| args.get("foreign_tool"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .trim()
        .to_string();
    let foreign_session_id = args
        .get("foreignSessionId")
        .or_else(|| args.get("foreign_session_id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "link_session requires foreignSessionId".to_string())?
        .trim()
        .to_string();
    let foreign_session_path = args
        .get("foreignSessionPath")
        .or_else(|| args.get("foreign_session_path"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let mut links = list_session_links(project_path)?;
    links.retain(|l| {
        !(l.chat_session_id == chat_session_id && l.foreign_session_id == foreign_session_id)
    });
    let link = SessionLink {
        chat_session_id,
        foreign_tool,
        foreign_session_id,
        foreign_session_path,
        created_at: now_iso(),
    };
    links.push(link.clone());
    let dir = get_project_dir(project_path).join("agent");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&links).map_err(|e| e.to_string())?;
    atomic_write(&session_links_path(project_path), &json)?;
    let _ = append_run(
        project_path,
        "link_session",
        "ok",
        json!({ "chatSessionId": link.chat_session_id, "foreignSessionId": link.foreign_session_id }),
    );
    Ok(serde_json::to_string_pretty(&link).unwrap_or_else(|_| "{}".into()))
}

pub fn update_task(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).map_err(|e| format!("invalid JSON: {e}"))?;
    let task_id = args
        .get("taskId")
        .or_else(|| args.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "update_task requires taskId".to_string())?;
    let mut tasks: Vec<Task> = read_project(project_path, "tasks.json").unwrap_or_default();
    let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) else {
        return Err(format!("task not found: {task_id}"));
    };
    if let Some(status) = args.get("status").and_then(|v| v.as_str()) {
        task.status = status.to_string();
    }
    if let Some(title) = args.get("title").and_then(|v| v.as_str()) {
        task.title = title.to_string();
    }
    if let Some(notes) = args.get("notes").and_then(|v| v.as_str()) {
        task.notes = Some(notes.to_string());
    }
    if let Some(next) = args.get("nextAction").or_else(|| args.get("next_action")).and_then(|v| v.as_str()) {
        task.next_action = Some(next.to_string());
    }
    task.updated_at = now_iso();
    let updated = task.clone();
    write_project(project_path, "tasks.json", &tasks)?;
    let _ = append_run(
        project_path,
        "update_task",
        "ok",
        json!({ "taskId": updated.id, "status": updated.status }),
    );
    Ok(serde_json::to_string_pretty(&updated).unwrap_or_else(|_| "{}".into()))
}

pub fn pending_questions_json(project_path: &str) -> Result<String, String> {
    let snapshot = load_continuity_input_snapshot(project_path).map_err(|e| e.to_string())?;
    let current = if current_state_projection_path(project_path).exists() {
        read_current_state_projection(project_path).map_err(|e| e.to_string())?
    } else {
        rebuild_current_state_projection(project_path).map_err(|e| e.to_string())?
    };
    let view = build_pending_questions_view(project_path, &snapshot, &current)
        .map_err(|e| e.to_string())?;
    Ok(serde_json::to_string_pretty(&view).unwrap_or_else(|_| "{}".into()))
}

pub fn create_handoff_draft(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).unwrap_or(json!({}));
    let recipient_label = args
        .get("recipient")
        .and_then(|v| v.as_str())
        .unwrap_or("teammate")
        .trim();
    let role = args.get("role").and_then(|v| v.as_str());

    let project: Project = read_project(project_path, "project.json")
        .ok_or_else(|| "project.json missing — init project first".to_string())?;
    let recipient = build_handoff_recipient(recipient_label, role).map_err(|e| e.to_string())?;
    let now = now_iso();
    let window = resolve_handoff_window(None, None, &now).map_err(|e| e.to_string())?;
    let snapshot = load_continuity_input_snapshot(project_path).map_err(|e| e.to_string())?;
    let current = if current_state_projection_path(project_path).exists() {
        read_current_state_projection(project_path).map_err(|e| e.to_string())?
    } else {
        rebuild_current_state_projection(project_path).map_err(|e| e.to_string())?
    };
    let request = BuildHandoffRequest {
        workspace_id: project.id,
        recipient,
        window,
        now_rfc3339: now,
    };
    let note = build_handoff_note(&snapshot, &current, &request).map_err(|e| e.to_string())?;
    write_handoff_note(project_path, &note).map_err(|e| e.to_string())?;

    // Attach recent agent runs as a brief note under agent/briefs.
    let runs = list_recent_runs(project_path, 5).unwrap_or_default();
    let brief_dir = get_project_dir(project_path).join("agent").join("briefs");
    fs::create_dir_all(&brief_dir).map_err(|e| e.to_string())?;
    let brief_path = brief_dir.join(format!("{}.md", note.handoff_id));
    let mut brief = format!(
        "# Handoff brief {}\n\nRecipient: {}\n\n## Recent agent runs\n",
        note.handoff_id, recipient_label
    );
    for r in runs {
        brief.push_str(&format!("- {} [{}] {}\n", r.id, r.kind, r.status));
    }
    atomic_write(&brief_path, &brief)?;

    let _ = append_run(
        project_path,
        "create_handoff_draft",
        "ok",
        json!({ "handoffId": note.handoff_id }),
    );
    Ok(serde_json::to_string_pretty(&json!({
        "handoffId": note.handoff_id,
        "status": note.status,
        "briefPath": brief_path.to_string_lossy(),
    }))
    .unwrap_or_else(|_| "{}".into()))
}

pub fn approve_handoff(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).unwrap_or(json!({}));
    let id = args
        .get("handoffId")
        .or_else(|| args.get("id"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "approve_handoff requires handoffId".to_string())?;
    let note = approve_handoff_note(project_path, id, &now_iso()).map_err(|e| e.to_string())?;
    let _ = append_run(
        project_path,
        "approve_handoff",
        "ok",
        json!({ "handoffId": note.handoff_id }),
    );
    Ok(serde_json::to_string_pretty(&json!({
        "handoffId": note.handoff_id,
        "status": note.status,
        "approvedAt": note.approved_at,
    }))
    .unwrap_or_else(|_| "{}".into()))
}

pub fn mesh_query(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).unwrap_or(json!({}));
    let peer = args
        .get("peer")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "mesh_query requires peer".to_string())?;
    let question = args
        .get("question")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "mesh_query requires question".to_string())?;

    let policy = read_trust_policy(project_path).map_err(|e| e.to_string())?;
    let decision = evaluate_remote_query(&policy, None, Some(peer));
    if decision.permission != QueryPermission::Allowed {
        return Err(format!("mesh query denied: {}", decision.reason));
    }

    let request = MeshRemoteQueryRequest {
        peer: peer.to_string(),
        question: question.to_string(),
        query_id: format!("agent-{}", chrono::Utc::now().timestamp_millis()),
        now: chrono::Utc::now(),
        freshness_tier: FreshnessTier::Standard,
        include_relay_received: true,
    };
    let answer =
        query_remote_peer_proxy(project_path, &request, false).map_err(|e| e.to_string())?;
    let _ = append_run(
        project_path,
        "mesh_query",
        if answer.refused { "refused" } else { "ok" },
        json!({ "peer": peer }),
    );
    Ok(serde_json::to_string_pretty(&answer).unwrap_or_else(|_| "{}".into()))
}

/// Write a short delegate brief under `.openmesh/agent/briefs/`.
pub fn write_delegate_brief(
    project_path: &str,
    tool: &str,
    summary: &str,
) -> Result<PathBuf, String> {
    let dir = get_project_dir(project_path).join("agent").join("briefs");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let id = format!(
        "brief-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let path = dir.join(format!("{id}.md"));
    let body = format!(
        "# OpenMesh delegate brief\n\nTool: {tool}\nCreated: {}\n\n{summary}\n",
        now_iso()
    );
    atomic_write(&path, &body)?;
    let _ = append_run(
        project_path,
        "delegate_brief",
        "ok",
        json!({ "tool": tool, "path": path.to_string_lossy() }),
    );
    Ok(path)
}
