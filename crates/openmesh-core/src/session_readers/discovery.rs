use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::parse::{
    parse_claude_jsonl, parse_codex_rollout, parse_cursor_transcript, parse_gemini_chat,
    parse_grok_session, parse_opencode_session_json, SessionHints,
};
use super::redact::redact_secrets;

const MAX_WALK_FILES: usize = 20_000;
const UUID_LEN_MIN: usize = 32;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedForeignSession {
    pub id: String,
    pub tool_name: String,
    pub title: String,
    pub session_path: String,
    pub file_name: String,
    pub created_at: String,
    pub last_active_at: String,
    pub file_size_bytes: u64,
    pub summary_preview: Option<String>,
    pub project_hint: Option<String>,
}

/// Normalize tool identifiers from UI / settings to canonical short names.
pub fn normalize_tool(tool: &str) -> Option<&'static str> {
    match tool {
        "codex" => Some("codex"),
        "claude" | "claude-code" => Some("claude"),
        "opencode" => Some("opencode"),
        "cursor" => Some("cursor"),
        "gemini" | "gemini-cli" => Some("gemini"),
        "grok" => Some("grok"),
        _ => None,
    }
}

/// Sensible default directory for each provider (may not exist yet).
pub fn default_session_dir(tool: &str) -> Option<PathBuf> {
    candidate_session_dirs(tool)
        .into_iter()
        .next()
        .or_else(|| {
            let home = dirs::home_dir()?;
            match normalize_tool(tool)? {
                "codex" => Some(home.join(".codex").join("sessions")),
                "claude" => Some(home.join(".claude").join("projects")),
                "opencode" => Some(home.join(".local").join("share").join("opencode")),
                "cursor" => Some(home.join(".cursor").join("projects")),
                "gemini" => Some(home.join(".gemini").join("tmp")),
                "grok" => Some(home.join(".grok").join("sessions")),
                _ => None,
            }
        })
}

/// OS/env-aware candidate roots for a provider (first existing wins at detect time).
pub fn candidate_session_dirs(tool: &str) -> Vec<PathBuf> {
    let Some(tool) = normalize_tool(tool) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut push = |p: PathBuf| {
        if !out.iter().any(|e| e == &p) {
            out.push(p);
        }
    };

    match tool {
        "codex" => {
            if let Ok(home) = std::env::var("CODEX_HOME") {
                let base = expand_user_path(&home);
                push(base.join("sessions"));
                push(base);
            }
            if let Some(home) = dirs::home_dir() {
                push(home.join(".codex").join("sessions"));
                push(home.join(".codex"));
            }
        }
        "claude" => {
            if let Ok(cfg) = std::env::var("CLAUDE_CONFIG_DIR") {
                push(expand_user_path(&cfg).join("projects"));
            }
            if let Some(home) = dirs::home_dir() {
                push(home.join(".claude").join("projects"));
            }
        }
        "opencode" => {
            if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
                push(expand_user_path(&xdg).join("opencode"));
            }
            if let Some(home) = dirs::home_dir() {
                push(home.join(".local").join("share").join("opencode"));
                push(home.join(".opencode"));
            }
            // Windows native data dir when present
            if let Some(data) = dirs::data_local_dir() {
                push(data.join("opencode"));
            }
        }
        "cursor" => {
            if let Some(home) = dirs::home_dir() {
                push(home.join(".cursor").join("projects"));
                push(home.join(".cursor").join("chats"));
                push(home.join(".cursor"));
            }
        }
        "gemini" => {
            if let Some(home) = dirs::home_dir() {
                push(home.join(".gemini").join("tmp"));
                push(home.join(".gemini"));
            }
        }
        "grok" => {
            if let Ok(home) = std::env::var("GROK_HOME") {
                let base = expand_user_path(&home);
                push(base.join("sessions"));
                push(base);
            }
            if let Some(home) = dirs::home_dir() {
                push(home.join(".grok").join("sessions"));
                push(home.join(".grok"));
            }
        }
        _ => {}
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProviderRoot {
    pub tool: String,
    pub path: String,
}

/// Optional per-tool path overrides from Settings (empty/None = auto-detect).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionScanOverrides {
    pub codex_dir: Option<String>,
    pub claude_code_dir: Option<String>,
    pub opencode_dir: Option<String>,
    pub cursor_dir: Option<String>,
    pub gemini_dir: Option<String>,
    pub grok_dir: Option<String>,
}

fn override_for_tool(overrides: &SessionScanOverrides, tool: &str) -> Option<String> {
    match tool {
        "codex" => overrides.codex_dir.clone(),
        "claude" => overrides.claude_code_dir.clone(),
        "opencode" => overrides.opencode_dir.clone(),
        "cursor" => overrides.cursor_dir.clone(),
        "gemini" => overrides.gemini_dir.clone(),
        "grok" => overrides.grok_dir.clone(),
        _ => None,
    }
}

fn dir_exists(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.is_dir() && !m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Auto-detect provider roots that exist on this machine (any OS).
///
/// Settings overrides win when the path exists; otherwise the first existing
/// candidate from env/home/XDG/local-data is used. Missing providers are skipped.
pub fn detect_provider_roots(overrides: Option<&SessionScanOverrides>) -> Vec<DetectedProviderRoot> {
    let empty = SessionScanOverrides::default();
    let overrides = overrides.unwrap_or(&empty);
    let tools = ["codex", "claude", "opencode", "cursor", "gemini", "grok"];
    let mut detected = Vec::new();

    for tool in tools {
        if let Some(raw) = override_for_tool(overrides, tool) {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                let path = expand_user_path(trimmed);
                if dir_exists(&path) {
                    detected.push(DetectedProviderRoot {
                        tool: tool.to_string(),
                        path: path.to_string_lossy().to_string(),
                    });
                    continue;
                }
            }
        }

        for candidate in candidate_session_dirs(tool) {
            if dir_exists(&candidate) {
                detected.push(DetectedProviderRoot {
                    tool: tool.to_string(),
                    path: candidate.to_string_lossy().to_string(),
                });
                break;
            }
        }
    }
    detected
}

/// Scan every provider that exists on this device, scoped to `workspace_cwd`.
pub fn scan_workspace_sessions(
    workspace_cwd: &str,
    limit: Option<u32>,
    overrides: Option<&SessionScanOverrides>,
) -> Result<Vec<ScannedForeignSession>, String> {
    let workspace = workspace_cwd.trim();
    if workspace.is_empty() {
        return Err("workspace_cwd is required".to_string());
    }
    let max = limit.unwrap_or(100) as usize;
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::<String>::new();

    for root in detect_provider_roots(overrides) {
        match scan_agent_sessions(&root.tool, &root.path, Some(max as u32), Some(workspace)) {
            Ok(sessions) => {
                for session in sessions {
                    if seen.insert(session.id.clone()) {
                        all.push(session);
                    }
                }
            }
            Err(_) => {
                // Provider root vanished or unreadable — skip, keep others.
                continue;
            }
        }
    }

    all.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
    if all.len() > max {
        all.truncate(max);
    }
    Ok(all)
}

fn expand_user_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

fn system_time_to_rfc3339(time: SystemTime) -> String {
    let secs = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    DateTime::<Utc>::from_timestamp(secs, 0)
        .map(|dt| dt.to_rfc3339())
        .unwrap_or_else(|| "unknown".to_string())
}

fn file_times(path: &Path) -> (String, String, u64) {
    let meta = fs::symlink_metadata(path).ok();
    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
    let modified = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .map(system_time_to_rfc3339)
        .unwrap_or_else(|| "unknown".to_string());
    let created = meta
        .as_ref()
        .and_then(|m| m.created().ok())
        .map(system_time_to_rfc3339)
        .unwrap_or_else(|| modified.clone());
    (created, modified, size)
}

fn looks_like_uuid(stem: &str) -> bool {
    let compact: String = stem.chars().filter(|c| *c != '-').collect();
    compact.len() >= UUID_LEN_MIN && compact.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_codex_rollout_name(name: &str) -> bool {
    let base = name.strip_suffix(".zst").unwrap_or(name);
    base.starts_with("rollout-") && base.ends_with(".jsonl")
}

fn walk_files(root: &Path, max_files: usize) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if out.len() >= max_files {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if out.len() >= max_files {
                break;
            }
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                out.push(path);
            }
        }
    }
    out
}

fn build_session(
    tool: &str,
    path: &Path,
    hints: SessionHints,
    fallback_title: &str,
) -> ScannedForeignSession {
    let (created_meta, modified, size) = file_times(path);
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    let session_id = hints
        .session_id
        .clone()
        .unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or(&file_name).to_string());
    let title = hints
        .title
        .clone()
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| fallback_title.to_string());
    let created_at = hints.created_at.clone().unwrap_or(created_meta);
    let preview = hints.summary_preview_or(&title);

    ScannedForeignSession {
        id: format!("{tool}_{session_id}"),
        tool_name: tool.to_string(),
        title: title.chars().take(120).collect(),
        session_path: path.to_string_lossy().to_string(),
        file_name,
        created_at,
        last_active_at: modified,
        file_size_bytes: size,
        summary_preview: preview,
        project_hint: hints.project_hint.clone(),
    }
}

trait SessionHintsExt {
    fn summary_preview_or(&self, title: &str) -> Option<String>;
}

impl SessionHintsExt for SessionHints {
    fn summary_preview_or(&self, title: &str) -> Option<String> {
        self.preview
            .clone()
            .or_else(|| Some(redact_secrets(title)))
    }
}

fn scan_codex(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut sessions = Vec::new();

    // Prefer Codex state SQLite when scanning ~/.codex or its sessions subtree.
    if let Some(home) = codex_home_from_scan_root(root) {
        if let Some(db) = newest_codex_state_db(&home) {
            if let Some(from_db) = scan_codex_sqlite(&home, &db, limit) {
                if !from_db.is_empty() {
                    return from_db;
                }
            }
        }
    }

    let sessions_root = if root.join("sessions").is_dir() {
        root.join("sessions")
    } else {
        root.to_path_buf()
    };

    let mut files: Vec<PathBuf> = walk_files(&sessions_root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(is_codex_rollout_name)
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|p| {
        std::cmp::Reverse(
            fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        )
    });

    for path in files.into_iter().take(limit) {
        let hints = parse_codex_rollout(&path);
        let fallback = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("codex session")
            .replace(['_', '-'], " ");
        sessions.push(build_session("codex", &path, hints, &fallback));
    }
    sessions
}

fn codex_home_from_scan_root(root: &Path) -> Option<PathBuf> {
    if root.file_name().and_then(|n| n.to_str()) == Some("sessions") {
        return root.parent().map(|p| p.to_path_buf());
    }
    if root.join("sessions").is_dir() || root.join("config.toml").is_file() {
        return Some(root.to_path_buf());
    }
    // Common: user pointed at ~/.codex/sessions/YYYY
    let mut cur = root;
    for _ in 0..4 {
        if cur.file_name().and_then(|n| n.to_str()) == Some("sessions") {
            return cur.parent().map(|p| p.to_path_buf());
        }
        cur = cur.parent()?;
    }
    None
}

fn newest_codex_state_db(home: &Path) -> Option<PathBuf> {
    let mut best: Option<(u32, PathBuf)> = None;
    let Ok(entries) = fs::read_dir(home) else {
        return None;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(num) = name
            .strip_prefix("state_")
            .and_then(|rest| rest.strip_suffix(".sqlite"))
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let path = entry.path();
        if path.is_symlink() || !path.is_file() {
            continue;
        }
        if best.as_ref().map(|(n, _)| num > *n).unwrap_or(true) {
            best = Some((num, path));
        }
    }
    best.map(|(_, p)| p)
}

fn scan_codex_sqlite(home: &Path, db_path: &Path, limit: usize) -> Option<Vec<ScannedForeignSession>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;

    let updated_col = if table_has_column(&conn, "threads", "updated_at_ms") {
        "updated_at_ms"
    } else if table_has_column(&conn, "threads", "updated_at") {
        "updated_at"
    } else {
        return None;
    };
    let title_expr = if table_has_column(&conn, "threads", "title") {
        "title"
    } else {
        "''"
    };
    let first_expr = if table_has_column(&conn, "threads", "first_user_message") {
        "first_user_message"
    } else {
        "''"
    };
    let sql = format!(
        "SELECT id, rollout_path, cwd, {title_expr}, {first_expr}, {updated_col}, created_at \
         FROM threads WHERE archived = 0 AND source IN ('cli', 'vscode') \
         ORDER BY {updated_col} DESC, id ASC LIMIT ?1"
    );
    let mut stmt = conn.prepare(&sql).ok()?;
    let rows = stmt
        .query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .ok()?;

    let mut sessions = Vec::new();
    for row in rows.flatten() {
        let (id, rollout_path, cwd, title, first_user, updated, created) = row;
        let mut path = PathBuf::from(&rollout_path);
        if !path.is_absolute() {
            path = home.join(&rollout_path);
        }
        if !path.is_file() {
            let zst = PathBuf::from(format!("{}.zst", path.display()));
            if zst.is_file() {
                path = zst;
            } else {
                continue;
            }
        }
        let mut hints = SessionHints {
            session_id: Some(id.clone()),
            project_hint: Some(cwd),
            title: None,
            preview: None,
            created_at: millis_or_secs_to_rfc3339(created),
        };
        let title_value = if !title.trim().is_empty() {
            title
        } else {
            first_user
        };
        if !title_value.trim().is_empty() {
            hints.title = Some(super::parse::one_line(&title_value, 120));
            hints.preview = Some(redact_secrets(&super::parse::one_line(&title_value, 280)));
        } else {
            // Fill from rollout head when index title is empty
            let parsed = parse_codex_rollout(&path);
            if hints.title.is_none() {
                hints.title = parsed.title;
            }
            if hints.preview.is_none() {
                hints.preview = parsed.preview;
            }
        }
        let fallback = id.clone();
        let mut session = build_session("codex", &path, hints, &fallback);
        if let Some(ts) = millis_or_secs_to_rfc3339(updated) {
            session.last_active_at = ts;
        }
        sessions.push(session);
    }
    Some(sessions)
}

fn table_has_column(conn: &rusqlite::Connection, table: &str, column: &str) -> bool {
    let Ok(mut stmt) = conn.prepare(&format!("PRAGMA table_info(\"{table}\")")) else {
        return false;
    };
    let Ok(rows) = stmt.query_map([], |row| row.get::<_, String>(1)) else {
        return false;
    };
    let found = rows.flatten().any(|name| name == column);
    found
}

fn millis_or_secs_to_rfc3339(value: i64) -> Option<String> {
    let ms = if value.abs() < 1_000_000_000_000 {
        value * 1000
    } else {
        value
    };
    DateTime::<Utc>::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339())
}

fn scan_claude(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut files: Vec<PathBuf> = walk_files(root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("jsonl")
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .map(looks_like_uuid)
                    .unwrap_or(false)
                // Prefer top-level session files; skip nested tool dumps if any
                && !p
                    .components()
                    .any(|c| c.as_os_str() == "tool-results" || c.as_os_str() == "subagents")
        })
        .collect();
    sort_by_mtime_desc(&mut files);
    files
        .into_iter()
        .take(limit)
        .map(|path| {
            let hints = parse_claude_jsonl(&path);
            let fallback = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("claude session")
                .to_string();
            build_session("claude", &path, hints, &fallback)
        })
        .collect()
}

fn scan_cursor(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut files: Vec<PathBuf> = walk_files(root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| {
            let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                return false;
            };
            if !name.ends_with(".jsonl") {
                return false;
            }
            // Prefer agent-transcripts/<uuid>/<uuid>.jsonl
            let parent = p.parent();
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let in_transcripts = p
                .components()
                .any(|c| c.as_os_str() == "agent-transcripts");
            let not_subagent = !p.components().any(|c| c.as_os_str() == "subagents");
            in_transcripts
                && not_subagent
                && looks_like_uuid(stem)
                && parent
                    .and_then(|par| par.file_name())
                    .and_then(|n| n.to_str())
                    == Some(stem)
        })
        .collect();

    // Also accept ~/.cursor/chats/<hash>/<uuid>/store.db as session markers
    if files.is_empty() {
        files = walk_files(root, MAX_WALK_FILES)
            .into_iter()
            .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("store.db"))
            .collect();
    }

    sort_by_mtime_desc(&mut files);
    files
        .into_iter()
        .take(limit)
        .map(|path| {
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let hints = parse_cursor_transcript(&path);
                let fallback = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cursor session")
                    .to_string();
                build_session("cursor", &path, hints, &fallback)
            } else {
                // store.db — metadata-only listing
                let session_id = path
                    .parent()
                    .and_then(|p| p.file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("cursor")
                    .to_string();
                let hints = SessionHints {
                    session_id: Some(session_id.clone()),
                    title: Some(format!("Cursor CLI {session_id}")),
                    ..Default::default()
                };
                build_session("cursor", &path, hints, &session_id)
            }
        })
        .collect()
}

fn scan_opencode(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut sessions = Vec::new();

    let db = root.join("opencode.db");
    if db.is_file() {
        if let Some(from_db) = scan_opencode_sqlite(&db, limit) {
            sessions.extend(from_db);
        }
    }

    if sessions.len() >= limit {
        return sessions;
    }

    let mut files: Vec<PathBuf> = walk_files(root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("json")
                && p.components().any(|c| {
                    let s = c.as_os_str();
                    s == "session" || s == "sessions"
                })
        })
        .collect();
    sort_by_mtime_desc(&mut files);

    let remaining = limit.saturating_sub(sessions.len());
    for path in files.into_iter().take(remaining) {
        let hints = parse_opencode_session_json(&path);
        let fallback = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("opencode session")
            .to_string();
        sessions.push(build_session("opencode", &path, hints, &fallback));
    }
    sessions
}

fn scan_opencode_sqlite(db_path: &Path, limit: usize) -> Option<Vec<ScannedForeignSession>> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .ok()?;

    // Newer OpenCode stores JSON payloads in data columns; try a few shapes.
    let candidates = [
        "SELECT id, title, directory, time_updated FROM session ORDER BY time_updated DESC LIMIT ?1",
        "SELECT id, title, directory, time_updated FROM sessions ORDER BY time_updated DESC LIMIT ?1",
    ];
    for sql in candidates {
        let Ok(mut stmt) = conn.prepare(sql) else {
            continue;
        };
        let Ok(rows) = stmt.query_map(rusqlite::params![limit as i64], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        }) else {
            continue;
        };
        let mut sessions = Vec::new();
        for row in rows.flatten() {
            let (id, title, directory, updated) = row;
            let path = db_path.to_path_buf();
            let title_line = title
                .as_ref()
                .filter(|t| !t.trim().is_empty())
                .map(|t| super::parse::one_line(t, 120));
            let preview = title_line
                .as_ref()
                .map(|t| redact_secrets(&super::parse::one_line(t, 280)));
            let hints = SessionHints {
                session_id: Some(id.clone()),
                title: title_line,
                project_hint: directory,
                created_at: updated.and_then(millis_or_secs_to_rfc3339),
                preview,
            };
            let mut session = build_session("opencode", &path, hints, &id);
            if let Some(ts) = updated.and_then(millis_or_secs_to_rfc3339) {
                session.last_active_at = ts;
            }
            sessions.push(session);
        }
        if !sessions.is_empty() {
            return Some(sessions);
        }
    }
    None
}

fn scan_gemini(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut files: Vec<PathBuf> = walk_files(root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()) == Some("json")
                && p.components().any(|c| c.as_os_str() == "chats")
        })
        .collect();
    sort_by_mtime_desc(&mut files);
    files
        .into_iter()
        .take(limit)
        .map(|path| {
            let hints = parse_gemini_chat(&path);
            let fallback = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("gemini session")
                .to_string();
            build_session("gemini", &path, hints, &fallback)
        })
        .collect()
}

fn scan_grok(root: &Path, limit: usize) -> Vec<ScannedForeignSession> {
    let mut files: Vec<PathBuf> = walk_files(root, MAX_WALK_FILES)
        .into_iter()
        .filter(|p| p.file_name().and_then(|n| n.to_str()) == Some("summary.json"))
        .collect();
    sort_by_mtime_desc(&mut files);
    files
        .into_iter()
        .take(limit)
        .map(|path| {
            let hints = parse_grok_session(&path);
            let fallback = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("grok session")
                .to_string();
            build_session("grok", &path, hints, &fallback)
        })
        .collect()
}

fn sort_by_mtime_desc(files: &mut [PathBuf]) {
    files.sort_by_key(|p| {
        std::cmp::Reverse(
            fs::metadata(p)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
        )
    });
}

/// Normalize a filesystem path for workspace comparisons.
pub fn normalize_workspace_path(raw: &str) -> String {
    let expanded = expand_user_path(raw);
    let lossy = expanded.to_string_lossy();
    let trimmed = lossy.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    // Collapse duplicate slashes without resolving symlinks (sessions store logical cwds).
    let mut out = String::with_capacity(trimmed.len());
    let mut prev_slash = false;
    for ch in trimmed.chars() {
        if ch == '/' {
            if !prev_slash {
                out.push(ch);
            }
            prev_slash = true;
        } else {
            out.push(ch);
            prev_slash = false;
        }
    }
    out
}

fn slugify_path(cwd: &str) -> String {
    cwd.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Cursor project folder name under `~/.cursor/projects/<slug>/`.
///
/// Empirically (macOS): strip leading separators, map `/` and `\` to `-`,
/// **drop** `_` (not replace), keep existing `-`, map other non-alnum to `-`,
/// then collapse runs of `-`.
///
/// Example: `/Users/kjct0s_/Developer/experiments/openmesh-ws` →
/// `Users-kjct0s-Developer-experiments-openmesh-ws`.
pub fn cursor_project_slug(cwd: &str) -> String {
    let trimmed = cwd.trim().trim_start_matches(['/', '\\']);
    let mut raw = String::with_capacity(trimmed.len());
    for c in trimmed.chars() {
        if c == '/' || c == '\\' {
            raw.push('-');
        } else if c == '_' {
            // Cursor drops underscores rather than turning them into hyphens.
        } else if c.is_ascii_alphanumeric() || c == '-' {
            raw.push(c);
        } else {
            raw.push('-');
        }
    }
    let mut out = String::with_capacity(raw.len());
    let mut prev_dash = false;
    for c in raw.chars() {
        if c == '-' {
            if !prev_dash {
                out.push('-');
            }
            prev_dash = true;
        } else {
            out.push(c);
            prev_dash = false;
        }
    }
    out.trim_matches('-').to_string()
}

/// True when Cursor project folders refer to the same workspace or an ancestor/descendant.
fn cursor_slugs_related(workspace_slug: &str, project_slug: &str) -> bool {
    if workspace_slug.is_empty() || project_slug.is_empty() {
        return false;
    }
    workspace_slug == project_slug
        || workspace_slug.starts_with(&format!("{project_slug}-"))
        || project_slug.starts_with(&format!("{workspace_slug}-"))
}

/// Extract `<slug>` from `.../projects/<slug>/agent-transcripts/...`
/// (or `.../<slug>/agent-transcripts/...` when the scan root is already `projects`).
fn cursor_project_slug_from_session_path(path: &str) -> Option<String> {
    let comps: Vec<&str> = Path::new(path)
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if let Some(idx) = comps.iter().position(|c| *c == "projects") {
        if let Some(name) = comps.get(idx + 1).copied() {
            if !name.is_empty() && name != "agent-transcripts" && name != "chats" {
                return Some(name.to_string());
            }
        }
    }
    if let Some(idx) = comps.iter().position(|c| *c == "agent-transcripts") {
        if idx > 0 {
            let name = comps[idx - 1];
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn url_encode_path(cwd: &str) -> String {
    cwd.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}

/// True when a session belongs to the open OpenMesh workspace path.
///
/// Matches exact cwd, nested work under that root, or provider path encodings
/// (Claude slug, Cursor project folder, Grok percent-encoded cwd).
pub fn session_matches_workspace(session: &ScannedForeignSession, workspace_cwd: &str) -> bool {
    let workspace = normalize_workspace_path(workspace_cwd);
    if workspace.is_empty() {
        return true;
    }

    let ws_cursor_slug = cursor_project_slug(&workspace);

    if let Some(hint) = session.project_hint.as_deref() {
        let hint_n = normalize_workspace_path(hint);
        if !hint_n.is_empty()
            && (hint_n == workspace
                || hint_n.starts_with(&(workspace.clone() + "/"))
                || workspace.starts_with(&(hint_n.clone() + "/")))
        {
            return true;
        }
        // Cursor decode is lossy (`_` dropped, `-` ↔ `/`); compare re-encoded slugs.
        let hint_slug = cursor_project_slug(hint);
        if cursor_slugs_related(&ws_cursor_slug, &hint_slug) {
            return true;
        }
    }

    let path = &session.session_path;
    let slug = slugify_path(&workspace);
    let slug_no_lead = slug.trim_start_matches('-');
    let encoded = url_encode_path(&workspace);
    if (!slug_no_lead.is_empty() && path.contains(slug_no_lead)) || path.contains(&encoded) {
        return true;
    }

    // Cursor: match project folder slug, including parent/child workspace folders.
    if let Some(project_slug) = cursor_project_slug_from_session_path(path) {
        if cursor_slugs_related(&ws_cursor_slug, &project_slug) {
            return true;
        }
    }
    if !ws_cursor_slug.is_empty() && path.contains(&ws_cursor_slug) {
        return true;
    }

    false
}

/// Scan a configured provider directory and return recent foreign sessions.
///
/// When `workspace_cwd` is set, only sessions tied to that project path are returned.
pub fn scan_agent_sessions(
    tool: &str,
    directory_path: &str,
    limit: Option<u32>,
    workspace_cwd: Option<&str>,
) -> Result<Vec<ScannedForeignSession>, String> {
    let canonical = normalize_tool(tool).ok_or_else(|| {
        format!(
            "Tool '{tool}' is not in the allowlist. Allowed: codex, claude, opencode, cursor, gemini, grok"
        )
    })?;
    let dir = expand_user_path(directory_path);
    let meta = fs::metadata(&dir).map_err(|e| format!("Directory does not exist: {e}"))?;
    if !meta.is_dir() {
        return Err("Path is not a directory".to_string());
    }

    let max = limit.unwrap_or(100) as usize;
    // Walk wider before cwd filter so workspace-scoped views still find hits.
    let walk_limit = if workspace_cwd.map(|s| !s.trim().is_empty()).unwrap_or(false) {
        max.saturating_mul(8).max(200).min(2_000)
    } else {
        max
    };

    let mut sessions = match canonical {
        "codex" => scan_codex(&dir, walk_limit),
        "claude" => scan_claude(&dir, walk_limit),
        "cursor" => scan_cursor(&dir, walk_limit),
        "opencode" => scan_opencode(&dir, walk_limit),
        "gemini" => scan_gemini(&dir, walk_limit),
        "grok" => scan_grok(&dir, walk_limit),
        _ => Vec::new(),
    };

    if let Some(cwd) = workspace_cwd.map(str::trim).filter(|s| !s.is_empty()) {
        sessions.retain(|s| session_matches_workspace(s, cwd));
    }

    // Stable secondary sort
    sessions.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
    if sessions.len() > max {
        sessions.truncate(max);
    }
    Ok(sessions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn discovers_nested_codex_rollout() {
        let dir = tempdir().unwrap();
        let nested = dir.path().join("2026/08/03");
        fs::create_dir_all(&nested).unwrap();
        let path = nested.join(
            "rollout-2026-08-03T00-00-00-019fc37f-b1eb-7303-b5a3-70a5d303d7de.jsonl",
        );
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-08-03T00:00:00Z","type":"session_meta","payload":{{"id":"019fc37f-b1eb-7303-b5a3-70a5d303d7de","cwd":"/tmp/demo","source":"cli"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-08-03T00:00:01Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"Ship the scanner"}}]}}}}"#
        )
        .unwrap();

        let sessions =
            scan_agent_sessions("codex", dir.path().to_str().unwrap(), Some(10), None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "Ship the scanner");
        assert_eq!(sessions[0].project_hint.as_deref(), Some("/tmp/demo"));
        assert_eq!(sessions[0].tool_name, "codex");

        let filtered = scan_agent_sessions(
            "codex",
            dir.path().to_str().unwrap(),
            Some(10),
            Some("/tmp/demo"),
        )
        .unwrap();
        assert_eq!(filtered.len(), 1);
        let other = scan_agent_sessions(
            "codex",
            dir.path().to_str().unwrap(),
            Some(10),
            Some("/tmp/other"),
        )
        .unwrap();
        assert!(other.is_empty());
    }

    #[test]
    fn discovers_nested_claude_jsonl() {
        let dir = tempdir().unwrap();
        let project = dir.path().join("-Users-me-demo");
        fs::create_dir_all(&project).unwrap();
        let path = project.join("019fc37f-b1eb-7303-b5a3-70a5d303d7de.jsonl");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","cwd":"/Users/me/demo","timestamp":"2026-08-03T00:00:00Z","message":{{"role":"user","content":[{{"type":"text","text":"Continue the PR"}}]}}}}"#
        )
        .unwrap();

        let sessions = scan_agent_sessions(
            "claude-code",
            dir.path().to_str().unwrap(),
            Some(10),
            None,
        )
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "Continue the PR");
        assert_eq!(sessions[0].project_hint.as_deref(), Some("/Users/me/demo"));
    }

    #[test]
    fn discovers_grok_summary() {
        let dir = tempdir().unwrap();
        let session = dir
            .path()
            .join("%2Ftmp%2Fdemo")
            .join("019fc277-d3ee-72b1-b3c0-36f8ba9f2f75");
        fs::create_dir_all(&session).unwrap();
        fs::write(
            session.join("summary.json"),
            r#"{"info":{"id":"019fc277-d3ee-72b1-b3c0-36f8ba9f2f75","cwd":"/tmp/demo"},"session_summary":"","created_at":"2026-08-02T12:34:20Z","agent_name":"grok-build-plan"}"#,
        )
        .unwrap();
        fs::write(
            session.join("chat_history.jsonl"),
            r#"{"type":"user","content":"Resume my Codex work"}
"#,
        )
        .unwrap();

        let sessions =
            scan_agent_sessions("grok", dir.path().to_str().unwrap(), Some(10), None).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "Resume my Codex work");
        assert_eq!(sessions[0].project_hint.as_deref(), Some("/tmp/demo"));
    }

    #[test]
    fn normalize_accepts_aliases() {
        assert_eq!(normalize_tool("claude-code"), Some("claude"));
        assert_eq!(normalize_tool("gemini-cli"), Some("gemini"));
        assert_eq!(normalize_tool("nope"), None);
    }

    #[test]
    fn cursor_project_slug_drops_underscores_and_maps_separators() {
        assert_eq!(
            cursor_project_slug("/Users/kjct0s_/Developer/experiments/openmesh-ws"),
            "Users-kjct0s-Developer-experiments-openmesh-ws"
        );
        assert_eq!(
            cursor_project_slug(
                "/Users/kjct0s_/Developer/Axtra-Intellion-WS/repos/axtra-intellion-saas-platform"
            ),
            "Users-kjct0s-Developer-Axtra-Intellion-WS-repos-axtra-intellion-saas-platform"
        );
        // Windows separators become hyphens; underscores are dropped.
        assert_eq!(
            cursor_project_slug(r"C:\Users\me\demo_app"),
            "C-Users-me-demoapp"
        );
        // Must not produce the old buggy double-dash form for `_` before `/`.
        assert!(!cursor_project_slug("/Users/kjct0s_/Developer").contains("--"));
    }

    #[test]
    fn discovers_cursor_agent_transcript_for_workspace() {
        let dir = tempdir().unwrap();
        let workspace = "/Users/kjct0s_/Developer/experiments/openmesh-ws";
        let slug = cursor_project_slug(workspace);
        let session_id = "6e7ebc72-51c1-4b6a-83db-6fdabadef21f";
        // Mirror ~/.cursor/projects/<slug>/agent-transcripts/<id>/<id>.jsonl
        let transcript_dir = dir
            .path()
            .join("projects")
            .join(&slug)
            .join("agent-transcripts")
            .join(session_id);
        fs::create_dir_all(&transcript_dir).unwrap();
        let path = transcript_dir.join(format!("{session_id}.jsonl"));
        fs::write(
            &path,
            r#"{"role":"user","message":{"content":[{"type":"text","text":"<timestamp>x</timestamp>\n<user_query>\nAdd Cursor session scanning\n</user_query>"}]}}
"#,
        )
        .unwrap();
        // Subagent dump must be ignored
        let sub = transcript_dir.join("subagents");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(format!("{session_id}.jsonl")), "{\"role\":\"user\"}\n").unwrap();

        let projects_root = dir.path().join("projects");
        let sessions = scan_agent_sessions(
            "cursor",
            projects_root.to_str().unwrap(),
            Some(10),
            Some(workspace),
        )
        .unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].tool_name, "cursor");
        assert_eq!(sessions[0].title, "Add Cursor session scanning");
        assert!(sessions[0].session_path.ends_with(".jsonl"));

        // Nested OpenMesh project under the same Cursor workspace still matches.
        let nested = format!("{workspace}/repos/openmesh-agent-workbench");
        let nested_hits = scan_agent_sessions(
            "cursor",
            projects_root.to_str().unwrap(),
            Some(10),
            Some(&nested),
        )
        .unwrap();
        assert_eq!(nested_hits.len(), 1);

        let other = scan_agent_sessions(
            "cursor",
            projects_root.to_str().unwrap(),
            Some(10),
            Some("/Users/kjct0s_/Developer/experiments/other-ws"),
        )
        .unwrap();
        assert!(other.is_empty());
    }

    #[test]
    fn cursor_empty_projects_dir_returns_empty_ok() {
        let dir = tempdir().unwrap();
        let sessions = scan_agent_sessions(
            "cursor",
            dir.path().to_str().unwrap(),
            Some(10),
            Some("/Users/kjct0s_/Developer/experiments/openmesh-ws"),
        )
        .unwrap();
        assert!(sessions.is_empty());

        let missing = dir.path().join("does-not-exist");
        assert!(scan_agent_sessions("cursor", missing.to_str().unwrap(), Some(5), None).is_err());
    }

    #[test]
    fn session_matches_workspace_uses_cursor_slug_relatedness() {
        let session = ScannedForeignSession {
            id: "cursor_abc".into(),
            tool_name: "cursor".into(),
            title: "t".into(),
            session_path: "/Users/me/.cursor/projects/Users-kjct0s-Developer-experiments-openmesh-ws/agent-transcripts/abc/abc.jsonl".into(),
            file_name: "abc.jsonl".into(),
            created_at: "2026-08-01T00:00:00Z".into(),
            last_active_at: "2026-08-01T00:00:00Z".into(),
            file_size_bytes: 1,
            summary_preview: None,
            project_hint: Some("/Users/kjct0s/Developer/experiments/openmesh/ws".into()),
        };
        assert!(session_matches_workspace(
            &session,
            "/Users/kjct0s_/Developer/experiments/openmesh-ws"
        ));
        assert!(session_matches_workspace(
            &session,
            "/Users/kjct0s_/Developer/experiments/openmesh-ws/repos/openmesh-agent-workbench"
        ));
        assert!(!session_matches_workspace(
            &session,
            "/Users/kjct0s_/Developer/experiments/heli-ws"
        ));
    }
}
