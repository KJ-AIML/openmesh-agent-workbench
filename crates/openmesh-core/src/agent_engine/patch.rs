//! Human-approved workspace patches (Phase 2).
//!
//! LLM may `propose_patch`. Apply/reject/rollback are host/IPC only.

use super::path_safety::{
    deny_sensitive_path, normalize_rel, resolve_write_target, sha256_hex, workspace_root,
};
use crate::storage::{atomic_write, get_project_dir, now_iso, safe_child_path};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_PATCH_BYTES: usize = 256 * 1024;
const EMPTY_HASH: &str =
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PatchStatus {
    Proposed,
    Applied,
    Rejected,
    Stale,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchFileChange {
    pub path: String,
    pub base_sha256: String,
    pub new_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchRecord {
    pub id: String,
    pub status: PatchStatus,
    pub summary: String,
    pub files: Vec<PatchFileChange>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolled_back_at: Option<String>,
    #[serde(default)]
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunRecord {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub detail: serde_json::Value,
    pub created_at: String,
}

fn agent_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("agent")
}

fn patches_dir(project_path: &str) -> PathBuf {
    agent_dir(project_path).join("patches")
}

fn backups_dir(project_path: &str, patch_id: &str) -> PathBuf {
    agent_dir(project_path).join("backups").join(patch_id)
}

fn runs_dir(project_path: &str) -> PathBuf {
    agent_dir(project_path).join("runs")
}

fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| format!("mkdir {}: {e}", path.display()))
}

fn new_id(prefix: &str) -> String {
    format!(
        "{prefix}-{}",
        uuid_like()
    )
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

pub fn append_run(
    project_path: &str,
    kind: &str,
    status: &str,
    detail: serde_json::Value,
) -> Result<AgentRunRecord, String> {
    let dir = runs_dir(project_path);
    ensure_dir(&dir)?;
    let record = AgentRunRecord {
        id: new_id("run"),
        kind: kind.into(),
        status: status.into(),
        detail,
        created_at: now_iso(),
    };
    let path = dir.join(format!("{}.json", record.id));
    let json = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    atomic_write(&path, &json)?;
    Ok(record)
}

pub fn list_recent_runs(project_path: &str, limit: usize) -> Result<Vec<AgentRunRecord>, String> {
    let dir = runs_dir(project_path);
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut ids: Vec<_> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_suffix(".json").map(|s| s.to_string())
        })
        .collect();
    ids.sort();
    ids.reverse();
    ids.truncate(limit.max(1));
    let mut out = Vec::new();
    for id in ids {
        if let Ok(r) = read_run(project_path, &id) {
            out.push(r);
        }
    }
    Ok(out)
}

fn read_run(project_path: &str, id: &str) -> Result<AgentRunRecord, String> {
    let path = runs_dir(project_path).join(format!("{id}.json"));
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn patch_path(project_path: &str, id: &str) -> Result<PathBuf, String> {
    if id.is_empty()
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("invalid patch id".into());
    }
    Ok(patches_dir(project_path).join(format!("{id}.json")))
}

pub fn read_patch(project_path: &str, id: &str) -> Result<PatchRecord, String> {
    let path = patch_path(project_path, id)?;
    let text = fs::read_to_string(&path).map_err(|_| format!("patch not found: {id}"))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn write_patch(project_path: &str, patch: &PatchRecord) -> Result<(), String> {
    ensure_dir(&patches_dir(project_path))?;
    let path = patch_path(project_path, &patch.id)?;
    let json = serde_json::to_string_pretty(patch).map_err(|e| e.to_string())?;
    atomic_write(&path, &json)
}

fn current_base_hash(project_path: &str, rel: &str) -> Result<(String, Option<String>), String> {
    let root = workspace_root(project_path)?;
    let joined = safe_child_path(&root, rel)?;
    deny_sensitive_path(&joined)?;
    if !joined.exists() {
        return Ok((EMPTY_HASH.to_string(), None));
    }
    let bytes = fs::read(&joined).map_err(|e| e.to_string())?;
    Ok((sha256_hex(&bytes), Some(String::from_utf8_lossy(&bytes).into_owned())))
}

/// Propose a patch from tool arguments JSON.
/// Expected: `{ "summary": "...", "files": [ { "path", "newContent" } ] }`
pub fn propose_patch_from_args(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: serde_json::Value =
        serde_json::from_str(arguments_json).map_err(|e| format!("invalid JSON: {e}"))?;
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("Proposed workspace change")
        .trim()
        .to_string();
    let files = args
        .get("files")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "propose_patch requires files[]".to_string())?;
    if files.is_empty() {
        return Err("propose_patch requires at least one file".into());
    }
    if files.len() > 20 {
        return Err("too many files in one patch (max 20)".into());
    }

    let mut changes = Vec::new();
    for f in files {
        let path = f
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "each file requires path".to_string())?;
        let new_content = f
            .get("newContent")
            .or_else(|| f.get("new_content"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "each file requires newContent".to_string())?;
        if new_content.len() > MAX_PATCH_BYTES {
            return Err(format!("newContent too large for {path}"));
        }
        let rel = normalize_rel(path)?;
        let _ = resolve_write_target(project_path, &rel)?;
        let (base_sha256, _) = current_base_hash(project_path, &rel)?;
        changes.push(PatchFileChange {
            path: rel,
            base_sha256,
            new_content: new_content.to_string(),
        });
    }

    let run = append_run(
        project_path,
        "propose_patch",
        "proposed",
        json!({ "summary": summary, "fileCount": changes.len() }),
    )?;

    let patch = PatchRecord {
        id: new_id("patch"),
        status: PatchStatus::Proposed,
        summary,
        files: changes,
        created_at: now_iso(),
        applied_at: None,
        rejected_at: None,
        rolled_back_at: None,
        run_id: run.id.clone(),
    };
    write_patch(project_path, &patch)?;

    Ok(serde_json::to_string_pretty(&json!({
        "patchId": patch.id,
        "status": "proposed",
        "summary": patch.summary,
        "files": patch.files.iter().map(|f| json!({
            "path": f.path,
            "baseSha256": f.base_sha256,
            "bytes": f.new_content.len(),
        })).collect::<Vec<_>>(),
        "runId": run.id,
        "message": "Patch proposed. Human must approve via Agent Chat or agent_patch_apply IPC before files change.",
    }))
    .unwrap_or_else(|_| "{}".into()))
}

pub fn apply_patch(project_path: &str, patch_id: &str) -> Result<PatchRecord, String> {
    let mut patch = read_patch(project_path, patch_id)?;
    if patch.status != PatchStatus::Proposed {
        return Err(format!(
            "patch {} is {:?}; only proposed patches can be applied",
            patch_id, patch.status
        ));
    }

    // Precondition: hashes still match.
    for f in &patch.files {
        let (current, _) = current_base_hash(project_path, &f.path)?;
        if current != f.base_sha256 {
            patch.status = PatchStatus::Stale;
            write_patch(project_path, &patch)?;
            let _ = append_run(
                project_path,
                "apply_patch",
                "stale",
                json!({ "patchId": patch_id, "path": f.path }),
            );
            return Err(format!(
                "stale base hash for {}: file changed since propose",
                f.path
            ));
        }
    }

    let backup = backups_dir(project_path, patch_id);
    ensure_dir(&backup)?;

    for f in &patch.files {
        let (target, _) = resolve_write_target(project_path, &f.path)?;
        // Backup existing content (or marker for create).
        let backup_file = backup.join(f.path.replace('/', "__"));
        if let Some(parent) = backup_file.parent() {
            ensure_dir(parent)?;
        }
        if target.exists() {
            let existing = fs::read_to_string(&target).map_err(|e| e.to_string())?;
            atomic_write(&backup_file, &existing)?;
        } else {
            atomic_write(&backup_file, "")?;
            // sidecar marker
            atomic_write(
                &backup_file.with_extension("created"),
                "1",
            )?;
        }
        if let Some(parent) = target.parent() {
            ensure_dir(parent)?;
        }
        atomic_write(&target, &f.new_content)?;
    }

    patch.status = PatchStatus::Applied;
    patch.applied_at = Some(now_iso());
    write_patch(project_path, &patch)?;
    let _ = append_run(
        project_path,
        "apply_patch",
        "applied",
        json!({ "patchId": patch_id, "files": patch.files.iter().map(|f| &f.path).collect::<Vec<_>>() }),
    );
    Ok(patch)
}

pub fn reject_patch(project_path: &str, patch_id: &str) -> Result<PatchRecord, String> {
    let mut patch = read_patch(project_path, patch_id)?;
    if patch.status != PatchStatus::Proposed && patch.status != PatchStatus::Stale {
        return Err(format!(
            "patch {} cannot be rejected from status {:?}",
            patch_id, patch.status
        ));
    }
    patch.status = PatchStatus::Rejected;
    patch.rejected_at = Some(now_iso());
    write_patch(project_path, &patch)?;
    let _ = append_run(
        project_path,
        "reject_patch",
        "rejected",
        json!({ "patchId": patch_id }),
    );
    Ok(patch)
}

pub fn rollback_patch(project_path: &str, patch_id: &str) -> Result<PatchRecord, String> {
    let mut patch = read_patch(project_path, patch_id)?;
    if patch.status != PatchStatus::Applied {
        return Err(format!(
            "patch {} is {:?}; only applied patches can roll back",
            patch_id, patch.status
        ));
    }
    let backup = backups_dir(project_path, patch_id);
    if !backup.is_dir() {
        return Err("no backup found for rollback".into());
    }

    for f in &patch.files {
        let (target, _) = resolve_write_target(project_path, &f.path)?;
        let backup_file = backup.join(f.path.replace('/', "__"));
        let created_marker = backup_file.with_extension("created");
        if created_marker.exists() {
            if target.exists() {
                fs::remove_file(&target).map_err(|e| e.to_string())?;
            }
        } else if backup_file.exists() {
            let content = fs::read_to_string(&backup_file).map_err(|e| e.to_string())?;
            if let Some(parent) = target.parent() {
                ensure_dir(parent)?;
            }
            atomic_write(&target, &content)?;
        }
    }

    patch.status = PatchStatus::RolledBack;
    patch.rolled_back_at = Some(now_iso());
    write_patch(project_path, &patch)?;
    let _ = append_run(
        project_path,
        "rollback_patch",
        "rolled_back",
        json!({ "patchId": patch_id }),
    );
    Ok(patch)
}

pub fn format_patch_summary(patch: &PatchRecord) -> String {
    let files = patch
        .files
        .iter()
        .map(|f| format!("- {} ({} bytes)", f.path, f.new_content.len()))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Patch {} [{}]\n{}\nFiles:\n{}\n\n--- preview (first file) ---\n{}",
        patch.id,
        format!("{:?}", patch.status).to_lowercase(),
        patch.summary,
        files,
        patch
            .files
            .first()
            .map(|f| {
                let preview: String = f.new_content.chars().take(2000).collect();
                format!("// {}\n{preview}", f.path)
            })
            .unwrap_or_default()
    )
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
            "openmesh-patch-{}-{}",
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
    fn propose_apply_rollback_happy_path() {
        let project = temp_project();
        fs::create_dir_all(Path::new(&project).join("src")).unwrap();
        fs::write(Path::new(&project).join("src/a.txt"), "old\n").unwrap();

        let out = propose_patch_from_args(
            &project,
            r#"{"summary":"update a","files":[{"path":"src/a.txt","newContent":"new\n"}]}"#,
        )
        .unwrap();
        assert!(out.contains("patchId"), "{out}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let id = v["patchId"].as_str().unwrap().to_string();

        let applied = apply_patch(&project, &id).unwrap();
        assert_eq!(applied.status, PatchStatus::Applied);
        assert_eq!(
            fs::read_to_string(Path::new(&project).join("src/a.txt")).unwrap(),
            "new\n"
        );

        let rolled = rollback_patch(&project, &id).unwrap();
        assert_eq!(rolled.status, PatchStatus::RolledBack);
        assert_eq!(
            fs::read_to_string(Path::new(&project).join("src/a.txt")).unwrap(),
            "old\n"
        );
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn stale_hash_rejects_apply() {
        let project = temp_project();
        fs::write(Path::new(&project).join("f.txt"), "v1\n").unwrap();
        let out = propose_patch_from_args(
            &project,
            r#"{"summary":"x","files":[{"path":"f.txt","newContent":"v2\n"}]}"#,
        )
        .unwrap();
        let id = serde_json::from_str::<serde_json::Value>(&out).unwrap()["patchId"]
            .as_str()
            .unwrap()
            .to_string();
        fs::write(Path::new(&project).join("f.txt"), "changed\n").unwrap();
        let err = apply_patch(&project, &id).unwrap_err();
        assert!(err.contains("stale"), "{err}");
        let patch = read_patch(&project, &id).unwrap();
        assert_eq!(patch.status, PatchStatus::Stale);
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn denies_sensitive_and_escape() {
        let project = temp_project();
        let err = propose_patch_from_args(
            &project,
            r#"{"summary":"x","files":[{"path":".env","newContent":"x=1\n"}]}"#,
        )
        .unwrap_err();
        assert!(err.contains("sensitive") || err.contains("Invalid"), "{err}");

        let err2 = propose_patch_from_args(
            &project,
            r#"{"summary":"x","files":[{"path":"../outside.txt","newContent":"nope\n"}]}"#,
        )
        .unwrap_err();
        assert!(
            err2.contains("Invalid") || err2.contains("escapes") || err2.contains("path"),
            "{err2}"
        );
        let _ = fs::remove_dir_all(&project);
    }
}
