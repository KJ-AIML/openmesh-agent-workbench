//! Shared workspace tool executor for Desktop, CLI, and live ask.
//!
//! Docs/notes tools list under `<project>/.openmesh/{docs,notes}`.
//! Source tools are confined to the canonical workspace root.
//! `propose_patch` stages changes for human approval (does not apply).

use super::continue_ops;
use super::path_safety::{
    deny_sensitive_path, normalize_rel, resolve_dir_in_workspace, resolve_file_in_workspace,
    workspace_root,
};
use super::patch::propose_patch_from_args;
use super::recipes;
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
use crate::storage::{get_project_dir, read_project, safe_child_path, Project};
use serde_json::json;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Command;

const MAX_READ_BYTES: usize = 64 * 1024;
const MAX_GREP_MATCHES: usize = 40;
const MAX_LIST_ENTRIES: usize = 200;
const MAX_DIFF_CHARS: usize = 24_000;

/// Workspace tools (reads + propose_patch + Continuity helpers; no silent apply).
pub struct WorkspaceToolExecutor {
    pub project_path: String,
}

impl ToolExecutor for WorkspaceToolExecutor {
    fn execute(&self, tool_name: &str, arguments_json: &str) -> Result<String, String> {
        let args: serde_json::Value =
            serde_json::from_str(arguments_json).unwrap_or(json!({}));
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
            "list_dir" => {
                let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                list_workspace_dir(&self.project_path, path)
            }
            "read_file" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "read_file requires path".to_string())?;
                read_workspace_file(&self.project_path, path)
            }
            "grep" => {
                let pattern = args
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "grep requires pattern".to_string())?;
                let glob = args.get("glob").and_then(|v| v.as_str());
                grep_workspace(&self.project_path, pattern, glob)
            }
            "git_diff" => {
                let path = args.get("path").and_then(|v| v.as_str());
                let staged = args
                    .get("staged")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                git_diff_text(&self.project_path, path, staged)
            }
            "propose_patch" => propose_patch_from_args(&self.project_path, arguments_json),
            "list_recipes" => {
                let list = recipes::list_recipes(&self.project_path)?;
                Ok(serde_json::to_string_pretty(&list).unwrap_or_else(|_| "[]".into()))
            }
            "pending_questions" => continue_ops::pending_questions_json(&self.project_path),
            "create_handoff_draft" => {
                continue_ops::create_handoff_draft(&self.project_path, arguments_json)
            }
            "approve_handoff" => continue_ops::approve_handoff(&self.project_path, arguments_json),
            "update_task" => continue_ops::update_task(&self.project_path, arguments_json),
            "link_session" => continue_ops::link_session(&self.project_path, arguments_json),
            "mesh_query" => continue_ops::mesh_query(&self.project_path, arguments_json),
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
            "ui_navigate" => {
                let route = args
                    .get("route")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "ui_navigate requires route".to_string())?;
                crate::app_actions::ui_navigate_json(route)
            }
            "app_get_context" => {
                let routes: Vec<&str> = crate::app_actions::ALLOWED_UI_ROUTES.to_vec();
                Ok(serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "projectPath": self.project_path,
                    "allowedRoutes": routes,
                    "note": "Live route/mode come from the Desktop UI; use ui_navigate with an allowed route.",
                }))
                .unwrap_or_else(|_| "{}".into()))
            }
            "app_propose_action" => {
                let action_val = args
                    .get("action")
                    .cloned()
                    .ok_or_else(|| "app_propose_action requires action object".to_string())?;
                let action: crate::app_actions::AppAction = serde_json::from_value(action_val)
                    .map_err(|e| format!("invalid AppAction: {e}"))?;
                let needs = !matches!(
                    action.confirmation_policy(),
                    crate::app_actions::ConfirmationPolicy::None
                );
                Ok(serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "kind": "app_propose_action",
                    "needsConfirmation": needs,
                    "risk": format!("{:?}", action.risk_class()).to_ascii_lowercase(),
                    "label": action.label(),
                    "appAction": action,
                }))
                .unwrap_or_else(|_| "{}".into()))
            }
            "canvas_upsert_auto_ui" => {
                crate::canvas::upsert_auto_ui_from_tool(&self.project_path, arguments_json)
            }
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

fn read_workspace_file(project_path: &str, relative: &str) -> Result<String, String> {
    let path = resolve_file_in_workspace(project_path, relative)?;
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_READ_BYTES as u64 {
        return Err(format!(
            "file too large ({} bytes; max {MAX_READ_BYTES})",
            meta.len()
        ));
    }
    let bytes = fs::read(&path).map_err(|e| e.to_string())?;
    if bytes.contains(&0) {
        return Err("refusing to read binary file".into());
    }
    let text = String::from_utf8(bytes).map_err(|_| "file is not valid UTF-8".to_string())?;
    Ok(format!("// path: {}\n{}", relative.trim(), text))
}

fn list_workspace_dir(project_path: &str, relative: &str) -> Result<String, String> {
    let dir = resolve_dir_in_workspace(project_path, relative)?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" {
            continue;
        }
        let suffix = if entry.path().is_dir() { "/" } else { "" };
        entries.push(format!("{name}{suffix}"));
        if entries.len() >= MAX_LIST_ENTRIES {
            entries.push("…(truncated)".into());
            break;
        }
    }
    entries.sort();
    let label = {
        let t = relative.trim();
        if t.is_empty() || t == "." {
            "."
        } else {
            t
        }
    };
    Ok(format!(
        "dir: {label}\n{}",
        entries
            .iter()
            .map(|e| format!("- {e}"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}

fn grep_workspace(
    project_path: &str,
    pattern: &str,
    glob: Option<&str>,
) -> Result<String, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("pattern is required".into());
    }
    if pattern.len() > 200 {
        return Err("pattern too long (max 200)".into());
    }
    // Prefer ripgrep; fall back to a bounded walk if unavailable.
    match grep_with_rg(project_path, pattern, glob) {
        Ok(out) => Ok(out),
        Err(rg_err) => {
            if rg_err.contains("No such file")
                || rg_err.contains("not found")
                || rg_err.contains("os error 2")
            {
                grep_fallback_walk(project_path, pattern, glob)
            } else {
                Err(rg_err)
            }
        }
    }
}

fn grep_with_rg(
    project_path: &str,
    pattern: &str,
    glob: Option<&str>,
) -> Result<String, String> {
    let root = workspace_root(project_path)?;
    let mut cmd = Command::new("rg");
    cmd.current_dir(&root);
    cmd.args([
        "-n",
        "--no-heading",
        "--color",
        "never",
        "--hidden",
        "--glob",
        "!.git/**",
        "--glob",
        "!node_modules/**",
        "--glob",
        "!target/**",
        "--glob",
        "!dist/**",
        "--glob",
        "!.openmesh/index/**",
        "--max-filesize",
        "256K",
        "-m",
        &MAX_GREP_MATCHES.to_string(),
    ]);
    if let Some(g) = glob.map(str::trim).filter(|s| !s.is_empty()) {
        if g.chars().any(|c| c == ';' || c == '|' || c == '&' || c == '`' || c == '$') {
            return Err("invalid glob".into());
        }
        cmd.arg("--glob").arg(g);
    }
    cmd.arg("--").arg(pattern).arg(".");
    let output = cmd.output().map_err(|e| e.to_string())?;
    // rg exit 1 = no matches
    if !output.status.success() && output.status.code() != Some(1) {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("rg failed with status {}", output.status)
        } else {
            err
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    if stdout.trim().is_empty() {
        return Ok(format!("No matches for “{pattern}”."));
    }
    Ok(stdout)
}

fn grep_fallback_walk(
    project_path: &str,
    pattern: &str,
    glob: Option<&str>,
) -> Result<String, String> {
    let root = workspace_root(project_path)?;
    let needle = pattern.to_lowercase();
    let mut hits = Vec::new();
    walk_grep(&root, &root, &needle, glob, &mut hits)?;
    if hits.is_empty() {
        return Ok(format!("No matches for “{pattern}” (fallback search)."));
    }
    Ok(hits.join("\n"))
}

fn walk_grep(
    root: &Path,
    dir: &Path,
    needle_lower: &str,
    glob: Option<&str>,
    hits: &mut Vec<String>,
) -> Result<(), String> {
    if hits.len() >= MAX_GREP_MATCHES {
        return Ok(());
    }
    let entries = fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries {
        if hits.len() >= MAX_GREP_MATCHES {
            break;
        }
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git"
            || name == "node_modules"
            || name == "target"
            || name == "dist"
            || name == ".DS_Store"
        {
            continue;
        }
        if path.is_dir() {
            walk_grep(root, &path, needle_lower, glob, hits)?;
            continue;
        }
        if !path_matches_simple_glob(&name, glob) {
            continue;
        }
        if deny_sensitive_path(&path).is_err() {
            continue;
        }
        let Ok(file) = fs::File::open(&path) else {
            continue;
        };
        let reader = BufReader::new(file);
        for (idx, line) in reader.lines().enumerate() {
            if hits.len() >= MAX_GREP_MATCHES {
                break;
            }
            let Ok(line) = line else { continue };
            if line.to_lowercase().contains(needle_lower) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let clipped: String = line.chars().take(240).collect();
                hits.push(format!("{rel}:{}:{clipped}", idx + 1));
            }
        }
    }
    Ok(())
}

fn path_matches_simple_glob(name: &str, glob: Option<&str>) -> bool {
    let Some(g) = glob.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    if let Some(ext) = g.strip_prefix("*.") {
        return name.ends_with(ext);
    }
    name == g
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

fn git_diff_text(
    project_path: &str,
    path: Option<&str>,
    staged: bool,
) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(project_path).arg("diff").arg("--no-color");
    if staged {
        cmd.arg("--cached");
    }
    let rel_owned;
    if let Some(p) = path.map(str::trim).filter(|s| !s.is_empty()) {
        // Reject traversal (`..`) even when the pathspec is missing on disk.
        let rel = normalize_rel(p)?;
        if rel.contains('\0') {
            return Err("invalid path".into());
        }
        let root = workspace_root(project_path)?;
        let _ = safe_child_path(&root, &rel)?;
        rel_owned = rel;
        cmd.arg("--").arg(&rel_owned);
    }

    let output = cmd.output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).to_string());
    }
    let mut text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        return Ok(if staged {
            "No staged changes.".into()
        } else {
            "No unstaged changes.".into()
        });
    }
    if text.chars().count() > MAX_DIFF_CHARS {
        text = text.chars().take(MAX_DIFF_CHARS).collect::<String>() + "\n…(truncated)";
    }
    Ok(text)
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

    #[test]
    fn read_file_confined_and_reads_utf8() {
        let project = temp_project();
        fs::create_dir_all(Path::new(&project).join("src")).unwrap();
        fs::write(Path::new(&project).join("src/main.rs"), "fn main() {}\n").unwrap();

        let exec = WorkspaceToolExecutor {
            project_path: project.clone(),
        };
        let out = exec
            .execute("read_file", r#"{"path":"src/main.rs"}"#)
            .unwrap();
        assert!(out.contains("fn main()"), "out={out}");

        let err = exec
            .execute("read_file", r#"{"path":"../outside.txt"}"#)
            .unwrap_err();
        assert!(err.contains("Invalid path") || err.contains("escapes") || err.contains("not found"), "err={err}");

        fs::write(Path::new(&project).join(".env"), "SECRET=1\n").unwrap();
        let denied = exec
            .execute("read_file", r#"{"path":".env"}"#)
            .unwrap_err();
        assert!(denied.contains("sensitive"), "denied={denied}");
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn list_dir_and_grep_find_source() {
        let project = temp_project();
        fs::create_dir_all(Path::new(&project).join("src")).unwrap();
        fs::write(
            Path::new(&project).join("src/lib.rs"),
            "pub fn openmesh_marker() {}\n",
        )
        .unwrap();

        let exec = WorkspaceToolExecutor {
            project_path: project.clone(),
        };
        let listing = exec.execute("list_dir", r#"{"path":"src"}"#).unwrap();
        assert!(listing.contains("lib.rs"), "listing={listing}");

        let grep = exec
            .execute("grep", r#"{"pattern":"openmesh_marker"}"#)
            .unwrap();
        assert!(grep.contains("openmesh_marker"), "grep={grep}");
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn ui_navigate_allowlists_routes() {
        let project = temp_project();
        let exec = WorkspaceToolExecutor {
            project_path: project.clone(),
        };
        let out = exec
            .execute("ui_navigate", r#"{"route":"docs"}"#)
            .unwrap();
        assert!(out.contains("\"/docs\""), "out={out}");
        assert!(out.contains("ui_navigate"), "out={out}");
        let err = exec
            .execute("ui_navigate", r#"{"route":"/evil"}"#)
            .unwrap_err();
        assert!(err.contains("unsupported"), "err={err}");
        let _ = fs::remove_dir_all(&project);
    }
}
