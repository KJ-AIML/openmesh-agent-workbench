//! Shared read-mostly workspace tool executor for Desktop, CLI, and live ask.
//!
//! Docs/notes tools list under `<project>/.openmesh/{docs,notes}` — the same
//! paths the Tauri storage APIs use. Never join untrusted relative segments
//! without `storage::safe_child_path`.

use super::registry::ToolExecutor;
use crate::context_service;
use crate::continuity::{
    current_state_projection_path, load_continuity_input_snapshot, read_current_state_projection,
    rebuild_current_state_projection,
};
use crate::mesh::peers::list_peers;
use crate::pilot::build_pilot_pack;
use crate::rc::build_rc_pack;
use crate::return_digest::build_pending_questions_view;
use crate::storage::{get_project_dir, read_project, Project};
use serde_json::json;
use std::fs;
use std::process::Command;

/// Read-mostly workspace tools (no file writes).
pub struct WorkspaceToolExecutor {
    pub project_path: String,
}

impl ToolExecutor for WorkspaceToolExecutor {
    fn execute(&self, tool_name: &str, arguments_json: &str) -> Result<String, String> {
        match tool_name {
            "project_info" => {
                let project: Option<Project> = read_project(&self.project_path, "project.json");
                Ok(serde_json::to_string_pretty(&json!({
                    "path": self.project_path,
                    "project": project,
                }))
                .unwrap_or_else(|_| "{}".into()))
            }
            "list_docs" => list_openmesh_dir_names(&self.project_path, "docs"),
            "list_notes" => list_openmesh_dir_names(&self.project_path, "notes"),
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
            "git_status" => git_status_text(&self.project_path),
            "search_context" => {
                let args: serde_json::Value =
                    serde_json::from_str(arguments_json).unwrap_or(json!({}));
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let hits = context_service::search_project_context(
                    &self.project_path,
                    query,
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

fn list_openmesh_dir_names(project_path: &str, folder: &str) -> Result<String, String> {
    let dir = get_project_dir(project_path).join(folder);
    if !dir.is_dir() {
        return Ok(format!("(no .openmesh/{folder}/ directory)"));
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        names.push(name);
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
    Ok(serde_json::to_string_pretty(&json!({
        "pending": pending,
        "peerCount": peers.len(),
        "peers": peers,
    }))
    .unwrap_or_else(|_| "{}".into()))
}

fn git_status_text(project_path: &str) -> Result<String, String> {
    let output = Command::new("git")
        .args(["-C", project_path, "status", "--porcelain=v1", "-b"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_project;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-workspace-tools-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn list_docs_uses_openmesh_docs() {
        let project = temp_project();
        let docs = get_project_dir(&project).join("docs");
        fs::write(docs.join("hello.md"), "# hi").unwrap();
        // Decoy at repo root — must not be listed.
        fs::create_dir_all(std::path::Path::new(&project).join("docs")).unwrap();
        fs::write(
            std::path::Path::new(&project).join("docs").join("decoy.md"),
            "nope",
        )
        .unwrap();

        let exec = WorkspaceToolExecutor {
            project_path: project.clone(),
        };
        let out = exec.execute("list_docs", "{}").unwrap();
        assert!(out.contains("hello.md"), "out={out}");
        assert!(!out.contains("decoy.md"), "out={out}");
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn list_notes_uses_openmesh_notes() {
        let project = temp_project();
        let notes = get_project_dir(&project).join("notes");
        fs::write(notes.join("scratch.md"), "note").unwrap();

        let exec = WorkspaceToolExecutor {
            project_path: project.clone(),
        };
        let out = exec.execute("list_notes", "{}").unwrap();
        assert!(out.contains("scratch.md"), "out={out}");
        let _ = fs::remove_dir_all(&project);
    }
}
