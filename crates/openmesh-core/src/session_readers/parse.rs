use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use super::redact::redact_secrets;

const PREVIEW_CHARS: usize = 280;
const MAX_JSONL_LINES: usize = 80;

#[derive(Debug, Default, Clone)]
pub struct SessionHints {
    pub title: Option<String>,
    pub project_hint: Option<String>,
    pub preview: Option<String>,
    pub created_at: Option<String>,
    pub session_id: Option<String>,
}

pub fn one_line(text: &str, limit: usize) -> String {
    let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= limit {
        collapsed
    } else {
        collapsed.chars().take(limit).collect::<String>() + "..."
    }
}

pub(crate) fn strip_wrappers(text: &str) -> String {
    // Cursor wraps user text in <user_query>...</user_query>
    if let Some(start) = text.find("<user_query>") {
        let after = &text[start + "<user_query>".len()..];
        if let Some(end) = after.find("</user_query>") {
            return after[..end].trim().to_string();
        }
    }
    // Drop common injected meta wrappers for title selection / import.
    // Grok uses hyphenated <system-reminder>; Cursor often uses underscores.
    let trimmed = text.trim_start();
    for prefix in [
        "<recommended_plugins>",
        "<environment_context",
        "<user_instructions",
        "<system_reminder",
        "<system-reminder",
        "<user_info>",
        "<timestamp>",
        "<manually_attached_skills",
    ] {
        if trimmed.starts_with(prefix) {
            return String::new();
        }
    }
    text.trim().to_string()
}

pub(crate) fn content_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if let Some(s) = item.as_str() {
                    parts.push(s.to_string());
                    continue;
                }
                if let Some(obj) = item.as_object() {
                    let ty = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if matches!(ty, "text" | "input_text" | "output_text") {
                        if let Some(t) = obj.get("text").and_then(|v| v.as_str()) {
                            parts.push(t.to_string());
                        }
                    }
                }
            }
            parts.join("\n")
        }
        Value::Object(obj) => obj
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        _ => String::new(),
    }
}

fn push_user_title(hints: &mut SessionHints, raw: &str) {
    let cleaned = strip_wrappers(raw);
    if cleaned.is_empty() {
        return;
    }
    if hints.title.is_none() {
        hints.title = Some(one_line(&cleaned, 120));
    }
    if hints.preview.is_none() {
        hints.preview = Some(redact_secrets(&one_line(&cleaned, PREVIEW_CHARS)));
    }
}

/// Parse Codex rollout JSONL head for session_meta + first user message.
pub fn parse_codex_rollout(path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let Ok(file) = File::open(path) else {
        return hints;
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if hints.created_at.is_none() {
            if let Some(ts) = record.get("timestamp").and_then(|v| v.as_str()) {
                hints.created_at = Some(ts.to_string());
            }
        }
        let payload = record.get("payload");
        match ty {
            "session_meta" => {
                if let Some(p) = payload.and_then(|v| v.as_object()) {
                    if let Some(cwd) = p.get("cwd").and_then(|v| v.as_str()) {
                        hints.project_hint = Some(cwd.to_string());
                    }
                    if let Some(id) = p
                        .get("id")
                        .or_else(|| p.get("session_id"))
                        .and_then(|v| v.as_str())
                    {
                        hints.session_id = Some(id.to_string());
                    }
                }
            }
            "response_item" => {
                if let Some(p) = payload.and_then(|v| v.as_object()) {
                    if p.get("type").and_then(|v| v.as_str()) == Some("message")
                        && p.get("role").and_then(|v| v.as_str()) == Some("user")
                    {
                        let text = content_text(p.get("content").unwrap_or(&Value::Null));
                        push_user_title(&mut hints, &text);
                    }
                }
            }
            "event_msg" => {
                if let Some(p) = payload.and_then(|v| v.as_object()) {
                    if p.get("type").and_then(|v| v.as_str()) == Some("user_message") {
                        if let Some(text) = p.get("message").and_then(|v| v.as_str()) {
                            push_user_title(&mut hints, text);
                        }
                    }
                }
            }
            _ => {}
        }
        if hints.title.is_some() && hints.project_hint.is_some() {
            break;
        }
    }
    hints
}

/// Parse Claude Code JSONL (flat events with type user/assistant/summary/...).
pub fn parse_claude_jsonl(path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let Ok(file) = File::open(path) else {
        return hints;
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if hints.created_at.is_none() {
            if let Some(ts) = record.get("timestamp").and_then(|v| v.as_str()) {
                hints.created_at = Some(ts.to_string());
            }
        }
        if hints.project_hint.is_none() {
            if let Some(cwd) = record.get("cwd").and_then(|v| v.as_str()) {
                hints.project_hint = Some(cwd.to_string());
            }
        }
        match ty {
            "custom-title" => {
                if let Some(t) = record.get("customTitle").and_then(|v| v.as_str()) {
                    hints.title = Some(one_line(t, 120));
                }
            }
            "ai-title" => {
                if hints.title.is_none() {
                    if let Some(t) = record.get("aiTitle").and_then(|v| v.as_str()) {
                        hints.title = Some(one_line(t, 120));
                    }
                }
            }
            "summary" => {
                if hints.title.is_none() {
                    if let Some(t) = record.get("summary").and_then(|v| v.as_str()) {
                        hints.title = Some(one_line(t, 120));
                    }
                }
            }
            "user" => {
                if let Some(msg) = record.get("message") {
                    let text = content_text(msg.get("content").unwrap_or(msg));
                    push_user_title(&mut hints, &text);
                }
            }
            _ => {}
        }
    }
    if hints.session_id.is_none() {
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            hints.session_id = Some(stem.to_string());
        }
    }
    hints
}

/// Parse Cursor agent-transcript JSONL (`{role, message}`).
pub fn parse_cursor_transcript(path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let Ok(file) = File::open(path) else {
        return hints;
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let role = record.get("role").and_then(|v| v.as_str()).unwrap_or("");
        if role != "user" {
            continue;
        }
        let message = record.get("message").unwrap_or(&Value::Null);
        let text = content_text(message.get("content").unwrap_or(message));
        push_user_title(&mut hints, &text);
        if hints.title.is_some() {
            break;
        }
    }
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        hints.session_id = Some(stem.to_string());
    }
    // Project hint from path: .../projects/<encoded>/agent-transcripts/...
    if let Some(projects_idx) = path.components().position(|c| {
        c.as_os_str()
            .to_str()
            .map(|s| s == "projects")
            .unwrap_or(false)
    }) {
        let comps: Vec<_> = path.components().collect();
        if let Some(encoded) = comps.get(projects_idx + 1) {
            if let Some(name) = encoded.as_os_str().to_str() {
                if name != "agent-transcripts" {
                    hints.project_hint = Some(decode_cursor_project_slug(name));
                }
            }
        }
    }
    hints
}

/// Best-effort display hint only — Cursor drops `_` and maps `/` to `-`, so
/// round-trips are lossy (hyphens in original path segments become `/`).
fn decode_cursor_project_slug(slug: &str) -> String {
    if slug.starts_with("Users-") || slug.starts_with("home-") {
        format!("/{}", slug.replace('-', "/"))
    } else {
        slug.replace('-', "/")
    }
}

/// Parse Gemini chat JSON (single object with messages array or similar).
pub fn parse_gemini_chat(path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return hints;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        // Fall back to first bytes as preview
        hints.preview = Some(redact_secrets(&one_line(&raw, PREVIEW_CHARS)));
        return hints;
    };
    if let Some(id) = value
        .get("sessionId")
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
    {
        hints.session_id = Some(id.to_string());
    }
    if let Some(ts) = value
        .get("startTime")
        .or_else(|| value.get("timestamp"))
        .and_then(|v| v.as_str())
    {
        hints.created_at = Some(ts.to_string());
    }
    let messages = value
        .get("messages")
        .or_else(|| value.get("history"))
        .and_then(|v| v.as_array());
    if let Some(messages) = messages {
        for msg in messages {
            let role = msg
                .get("role")
                .or_else(|| msg.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if role != "user" && role != "user_message" {
                continue;
            }
            let text = content_text(msg.get("content").or_else(|| msg.get("text")).unwrap_or(msg));
            push_user_title(&mut hints, &text);
            if hints.title.is_some() {
                break;
            }
        }
    }
    if hints.title.is_none() {
        if let Some(t) = value.get("title").and_then(|v| v.as_str()) {
            hints.title = Some(one_line(t, 120));
        }
    }
    hints
}

/// Parse Grok session summary.json (+ optional chat_history.jsonl sibling).
pub fn parse_grok_session(summary_path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let mut agent_name: Option<String> = None;
    if let Ok(raw) = std::fs::read_to_string(summary_path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            let info = value.get("info").cloned().unwrap_or(Value::Null);
            if let Some(id) = info.get("id").and_then(|v| v.as_str()) {
                hints.session_id = Some(id.to_string());
            }
            if let Some(cwd) = info.get("cwd").and_then(|v| v.as_str()) {
                hints.project_hint = Some(cwd.to_string());
            }
            if let Some(summary) = value.get("session_summary").and_then(|v| v.as_str()) {
                if !summary.trim().is_empty() {
                    hints.title = Some(one_line(summary, 120));
                    hints.preview = Some(redact_secrets(&one_line(summary, PREVIEW_CHARS)));
                }
            }
            if let Some(ts) = value.get("created_at").and_then(|v| v.as_str()) {
                hints.created_at = Some(ts.to_string());
            }
            if let Some(agent) = value.get("agent_name").and_then(|v| v.as_str()) {
                agent_name = Some(one_line(agent, 120));
            }
        }
    }

    let history = summary_path
        .parent()
        .map(|p| p.join("chat_history.jsonl"));
    if let Some(history) = history {
        if history.is_file() {
            if let Ok(file) = File::open(&history) {
                for (idx, line) in BufReader::new(file).lines().enumerate() {
                    if idx >= MAX_JSONL_LINES {
                        break;
                    }
                    let Ok(line) = line else { continue };
                    let Ok(record) = serde_json::from_str::<Value>(&line) else {
                        continue;
                    };
                    let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    // Skip synthetic context rows Grok stores as type=user.
                    if record.get("synthetic_reason").is_some() {
                        continue;
                    }
                    if ty == "user" || ty == "human" {
                        // Grok user turns use content arrays, not bare strings.
                        let text = content_text(
                            record
                                .get("content")
                                .or_else(|| record.get("text"))
                                .unwrap_or(&Value::Null),
                        );
                        push_user_title(&mut hints, &text);
                        if hints.title.is_some() {
                            break;
                        }
                    }
                }
            }
        }
    }
    // Prefer summary / first user message; fall back to agent_name.
    if hints.title.is_none() {
        hints.title = agent_name;
    }
    hints
}

/// Best-effort OpenCode session JSON object.
pub fn parse_opencode_session_json(path: &Path) -> SessionHints {
    let mut hints = SessionHints::default();
    let Ok(raw) = std::fs::read_to_string(path) else {
        return hints;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        hints.preview = Some(redact_secrets(&one_line(&raw, PREVIEW_CHARS)));
        return hints;
    };
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        hints.session_id = Some(id.to_string());
    }
    if let Some(title) = value
        .get("title")
        .or_else(|| value.get("name"))
        .and_then(|v| v.as_str())
    {
        hints.title = Some(one_line(title, 120));
    }
    if let Some(cwd) = value
        .get("directory")
        .or_else(|| value.get("cwd"))
        .or_else(|| value.get("path"))
        .and_then(|v| v.as_str())
    {
        hints.project_hint = Some(cwd.to_string());
    }
    if let Some(ts) = value
        .get("time")
        .and_then(|t| t.get("created"))
        .and_then(|v| v.as_i64())
        .or_else(|| value.get("timeCreated").and_then(|v| v.as_i64()))
    {
        if let Some(dt) = chrono::DateTime::from_timestamp_millis(ts)
            .or_else(|| chrono::DateTime::from_timestamp(ts, 0))
        {
            hints.created_at = Some(dt.to_rfc3339());
        }
    }
    if hints.preview.is_none() {
        if let Some(title) = &hints.title {
            hints.preview = Some(redact_secrets(title));
        }
    }
    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn parses_codex_meta_and_user() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rollout-2026-08-03T00-00-00-019fc37f-b1eb-7303-b5a3-70a5d303d7de.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-08-03T00:00:00Z","type":"session_meta","payload":{{"id":"019fc37f-b1eb-7303-b5a3-70a5d303d7de","cwd":"/tmp/demo","source":"cli"}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"timestamp":"2026-08-03T00:00:01Z","type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"Fix the login bug"}}]}}}}"#
        )
        .unwrap();
        let hints = parse_codex_rollout(&path);
        assert_eq!(hints.project_hint.as_deref(), Some("/tmp/demo"));
        assert_eq!(hints.title.as_deref(), Some("Fix the login bug"));
        assert_eq!(
            hints.session_id.as_deref(),
            Some("019fc37f-b1eb-7303-b5a3-70a5d303d7de")
        );
    }

    #[test]
    fn strips_cursor_user_query_wrapper() {
        let dir = tempdir().unwrap();
        let path = dir
            .path()
            .join("projects")
            .join("Users-me-demo")
            .join("agent-transcripts")
            .join("abc")
            .join("abc.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":"<timestamp>x</timestamp>\n<user_query>\nResearch session stores\n</user_query>"}}]}}}}"#
        )
        .unwrap();
        let hints = parse_cursor_transcript(&path);
        assert_eq!(hints.title.as_deref(), Some("Research session stores"));
        assert!(hints
            .project_hint
            .as_deref()
            .unwrap_or("")
            .contains("Users/me/demo"));
    }

    #[test]
    fn parse_grok_title_from_array_user_query() {
        let dir = tempdir().unwrap();
        let sess = dir.path().join("g1");
        std::fs::create_dir_all(&sess).unwrap();
        std::fs::write(
            sess.join("summary.json"),
            r#"{"info":{"id":"g1","cwd":"/tmp/demo"},"session_summary":"","agent_name":"grok-build"}"#,
        )
        .unwrap();
        let mut f = std::fs::File::create(sess.join("chat_history.jsonl")).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","content":[{{"type":"text","text":"<user_info>\nx\n</user_info>"}}]}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","content":[{{"type":"text","text":"<user_query>\nWire resume roles\n</user_query>"}}]}}"#,
        )
        .unwrap();
        let hints = parse_grok_session(&sess.join("summary.json"));
        assert_eq!(hints.title.as_deref(), Some("Wire resume roles"));
        assert_eq!(hints.project_hint.as_deref(), Some("/tmp/demo"));
        assert_eq!(hints.session_id.as_deref(), Some("g1"));
    }
}
