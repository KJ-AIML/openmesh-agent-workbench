//! Read-only foreign transcript extraction for OpenMesh Chat resume/import.
//!
//! Never writes provider session files. Caps size so huge JSONL histories
//! become a truncated copy with an explicit note.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::discovery::normalize_tool;
use super::parse::{content_text, one_line, strip_wrappers};
use super::redact::redact_secrets;

/// Soft caps for a single imported copy into OpenMesh Chat.
const MAX_MESSAGES: usize = 200;
const MAX_CHARS_PER_MESSAGE: usize = 8_000;
const MAX_TOTAL_CHARS: usize = 200_000;
const MAX_JSONL_LINES: usize = 50_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignTranscriptMessage {
    pub role: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForeignTranscript {
    pub tool: String,
    pub path: String,
    pub title: Option<String>,
    pub messages: Vec<ForeignTranscriptMessage>,
    /// True when older turns were dropped or per-message text was clipped.
    pub truncated: bool,
    pub truncation_note: Option<String>,
    /// Sources with no readable message body (e.g. OpenCode metadata-only).
    pub preview_only: bool,
}

fn push_message(
    out: &mut Vec<ForeignTranscriptMessage>,
    role: &str,
    raw: &str,
    total_chars: &mut usize,
    truncated: &mut bool,
) {
    let cleaned = if role == "user" {
        let stripped = strip_wrappers(raw);
        if stripped.is_empty() {
            return;
        }
        stripped
    } else {
        raw.trim().to_string()
    };
    if cleaned.is_empty() {
        return;
    }
    if out.len() >= MAX_MESSAGES {
        *truncated = true;
        return;
    }
    let mut text = redact_secrets(&cleaned);
    if text.chars().count() > MAX_CHARS_PER_MESSAGE {
        text = text
            .chars()
            .take(MAX_CHARS_PER_MESSAGE)
            .collect::<String>()
            + "\n…[truncated]";
        *truncated = true;
    }
    if *total_chars + text.len() > MAX_TOTAL_CHARS {
        *truncated = true;
        return;
    }
    *total_chars += text.len();
    out.push(ForeignTranscriptMessage {
        role: role.to_string(),
        text,
    });
}

fn finalize(
    tool: &str,
    path: &Path,
    title: Option<String>,
    mut messages: Vec<ForeignTranscriptMessage>,
    truncated: bool,
    preview_only: bool,
) -> ForeignTranscript {
    let mut note = None;
    let mut was_truncated = truncated;
    if messages.len() > MAX_MESSAGES {
        let dropped = messages.len() - MAX_MESSAGES;
        messages = messages.split_off(messages.len() - MAX_MESSAGES);
        was_truncated = true;
        note = Some(format!(
            "Kept the newest {MAX_MESSAGES} messages; dropped {dropped} older turns."
        ));
    } else if truncated {
        note = Some(
            "Import was capped for size (message count, per-message length, or total characters). Older or oversized turns may be missing."
                .to_string(),
        );
    }
    ForeignTranscript {
        tool: tool.to_string(),
        path: path.display().to_string(),
        title,
        messages,
        truncated: was_truncated,
        truncation_note: note,
        preview_only,
    }
}

fn extract_codex(path: &Path) -> ForeignTranscript {
    let mut messages = Vec::new();
    let mut title = None;
    let mut total = 0usize;
    let mut truncated = false;
    let Ok(file) = File::open(path) else {
        return finalize("codex", path, None, messages, false, true);
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            truncated = true;
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let payload = record.get("payload");
        match ty {
            "response_item" => {
                if let Some(p) = payload.and_then(|v| v.as_object()) {
                    if p.get("type").and_then(|v| v.as_str()) == Some("message") {
                        let role = p.get("role").and_then(|v| v.as_str()).unwrap_or("");
                        if role == "user" || role == "assistant" {
                            let text = content_text(p.get("content").unwrap_or(&Value::Null));
                            if title.is_none() && role == "user" {
                                let cleaned = strip_wrappers(&text);
                                if !cleaned.is_empty() {
                                    title = Some(one_line(&cleaned, 120));
                                }
                            }
                            push_message(&mut messages, role, &text, &mut total, &mut truncated);
                        }
                    }
                }
            }
            "event_msg" => {
                if let Some(p) = payload.and_then(|v| v.as_object()) {
                    let et = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if et == "user_message" {
                        if let Some(text) = p.get("message").and_then(|v| v.as_str()) {
                            if title.is_none() {
                                let cleaned = strip_wrappers(text);
                                if !cleaned.is_empty() {
                                    title = Some(one_line(&cleaned, 120));
                                }
                            }
                            // Prefer response_item messages; skip duplicate event_msg if we already have content.
                            if messages.is_empty() {
                                push_message(
                                    &mut messages,
                                    "user",
                                    text,
                                    &mut total,
                                    &mut truncated,
                                );
                            }
                        }
                    } else if et == "agent_message" {
                        if let Some(text) = p.get("message").and_then(|v| v.as_str()) {
                            if messages
                                .last()
                                .map(|m| m.role != "assistant")
                                .unwrap_or(true)
                            {
                                push_message(
                                    &mut messages,
                                    "assistant",
                                    text,
                                    &mut total,
                                    &mut truncated,
                                );
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    let preview_only = messages.is_empty();
    finalize("codex", path, title, messages, truncated, preview_only)
}

fn extract_claude(path: &Path) -> ForeignTranscript {
    let mut messages = Vec::new();
    let mut title = None;
    let mut total = 0usize;
    let mut truncated = false;
    let Ok(file) = File::open(path) else {
        return finalize("claude", path, None, messages, false, true);
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            truncated = true;
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match ty {
            "custom-title" => {
                if let Some(t) = record.get("customTitle").and_then(|v| v.as_str()) {
                    title = Some(one_line(t, 120));
                }
            }
            "ai-title" => {
                if title.is_none() {
                    if let Some(t) = record.get("aiTitle").and_then(|v| v.as_str()) {
                        title = Some(one_line(t, 120));
                    }
                }
            }
            "summary" => {
                if title.is_none() {
                    if let Some(t) = record.get("summary").and_then(|v| v.as_str()) {
                        title = Some(one_line(t, 120));
                    }
                }
            }
            "user" | "assistant" => {
                let role = if ty == "user" { "user" } else { "assistant" };
                if let Some(msg) = record.get("message") {
                    let text = content_text(msg.get("content").unwrap_or(msg));
                    if title.is_none() && role == "user" {
                        let cleaned = strip_wrappers(&text);
                        if !cleaned.is_empty() {
                            title = Some(one_line(&cleaned, 120));
                        }
                    }
                    push_message(&mut messages, role, &text, &mut total, &mut truncated);
                }
            }
            _ => {}
        }
    }
    let preview_only = messages.is_empty();
    finalize("claude", path, title, messages, truncated, preview_only)
}

fn extract_cursor(path: &Path) -> ForeignTranscript {
    let mut messages = Vec::new();
    let mut title = None;
    let mut total = 0usize;
    let mut truncated = false;
    let Ok(file) = File::open(path) else {
        return finalize("cursor", path, None, messages, false, true);
    };
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        if idx >= MAX_JSONL_LINES {
            truncated = true;
            break;
        }
        let Ok(line) = line else { continue };
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let role = record.get("role").and_then(|v| v.as_str()).unwrap_or("");
        if role != "user" && role != "assistant" {
            continue;
        }
        let message = record.get("message").unwrap_or(&Value::Null);
        let text = content_text(message.get("content").unwrap_or(message));
        if title.is_none() && role == "user" {
            let cleaned = strip_wrappers(&text);
            if !cleaned.is_empty() {
                title = Some(one_line(&cleaned, 120));
            }
        }
        push_message(&mut messages, role, &text, &mut total, &mut truncated);
    }
    let preview_only = messages.is_empty();
    finalize("cursor", path, title, messages, truncated, preview_only)
}

fn extract_gemini(path: &Path) -> ForeignTranscript {
    let mut messages = Vec::new();
    let mut title = None;
    let mut total = 0usize;
    let mut truncated = false;
    let Ok(raw) = std::fs::read_to_string(path) else {
        return finalize("gemini", path, None, messages, false, true);
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return finalize("gemini", path, None, messages, false, true);
    };
    if let Some(t) = value.get("title").and_then(|v| v.as_str()) {
        title = Some(one_line(t, 120));
    }
    let list = value
        .get("messages")
        .or_else(|| value.get("history"))
        .and_then(|v| v.as_array());
    if let Some(list) = list {
        for msg in list {
            let role_raw = msg
                .get("role")
                .or_else(|| msg.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let role = match role_raw {
                "user" | "user_message" | "human" => "user",
                "model" | "assistant" | "gemini" => "assistant",
                _ => continue,
            };
            let text = content_text(
                msg.get("content")
                    .or_else(|| msg.get("text"))
                    .unwrap_or(msg),
            );
            if title.is_none() && role == "user" {
                let cleaned = strip_wrappers(&text);
                if !cleaned.is_empty() {
                    title = Some(one_line(&cleaned, 120));
                }
            }
            push_message(&mut messages, role, &text, &mut total, &mut truncated);
        }
    }
    let preview_only = messages.is_empty();
    finalize("gemini", path, title, messages, truncated, preview_only)
}

fn extract_grok(path: &Path) -> ForeignTranscript {
    let mut messages = Vec::new();
    let mut title = None;
    let mut total = 0usize;
    let mut truncated = false;

    // Accept summary.json or chat_history.jsonl path.
    let summary_path = if path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "summary.json")
        .unwrap_or(false)
    {
        path.to_path_buf()
    } else if path
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s == "chat_history.jsonl")
        .unwrap_or(false)
    {
        path.parent()
            .map(|p| p.join("summary.json"))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    if let Ok(raw) = std::fs::read_to_string(&summary_path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            if let Some(summary) = value.get("session_summary").and_then(|v| v.as_str()) {
                if !summary.trim().is_empty() {
                    title = Some(one_line(summary, 120));
                }
            }
            if title.is_none() {
                if let Some(t) = value.get("generated_title").and_then(|v| v.as_str()) {
                    if !t.trim().is_empty() {
                        title = Some(one_line(t, 120));
                    }
                }
            }
        }
    }

    let history = summary_path
        .parent()
        .map(|p| p.join("chat_history.jsonl"))
        .filter(|p| p.is_file());
    if let Some(history) = history {
        if let Ok(file) = File::open(&history) {
            for (idx, line) in BufReader::new(file).lines().enumerate() {
                if idx >= MAX_JSONL_LINES {
                    truncated = true;
                    break;
                }
                let Ok(line) = line else { continue };
                let Ok(record) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                let ty = record.get("type").and_then(|v| v.as_str()).unwrap_or("");
                // Grok stores injected context as type=user with synthetic_reason;
                // those are not human turns. Skip reasoning/tool noise too.
                if record.get("synthetic_reason").is_some() {
                    continue;
                }
                let role = match ty {
                    "user" | "human" => "user",
                    "assistant" | "model" | "grok" => "assistant",
                    // system / reasoning / tool_result are not chat turns for OpenMesh import
                    // (resumeIntoChat already seeds a provenance system note).
                    _ => continue,
                };
                // User turns use content: [{type:text,text:...}]; assistants are often bare strings.
                let text = content_text(
                    record
                        .get("content")
                        .or_else(|| record.get("text"))
                        .unwrap_or(&Value::Null),
                );
                if title.is_none() && role == "user" {
                    let cleaned = strip_wrappers(&text);
                    if !cleaned.is_empty() {
                        title = Some(one_line(&cleaned, 120));
                    }
                }
                push_message(&mut messages, role, &text, &mut total, &mut truncated);
            }
        }
    }

    let preview_only = messages.is_empty();
    finalize("grok", &summary_path, title, messages, truncated, preview_only)
}

fn extract_opencode(path: &Path) -> ForeignTranscript {
    // OpenCode session JSON is usually metadata; full turns live elsewhere.
    let mut title = None;
    if let Ok(raw) = std::fs::read_to_string(path) {
        if let Ok(value) = serde_json::from_str::<Value>(&raw) {
            if let Some(t) = value
                .get("title")
                .or_else(|| value.get("name"))
                .and_then(|v| v.as_str())
            {
                title = Some(one_line(t, 120));
            }
            // Best-effort: some exports embed messages.
            if let Some(list) = value
                .get("messages")
                .or_else(|| value.get("history"))
                .and_then(|v| v.as_array())
            {
                let mut messages = Vec::new();
                let mut total = 0usize;
                let mut truncated = false;
                for msg in list {
                    let role_raw = msg
                        .get("role")
                        .or_else(|| msg.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let role = match role_raw {
                        "user" | "human" => "user",
                        "assistant" | "model" => "assistant",
                        _ => continue,
                    };
                    let text = content_text(
                        msg.get("content")
                            .or_else(|| msg.get("text"))
                            .unwrap_or(msg),
                    );
                    push_message(&mut messages, role, &text, &mut total, &mut truncated);
                }
                if !messages.is_empty() {
                    return finalize("opencode", path, title, messages, truncated, false);
                }
            }
        }
    }
    finalize("opencode", path, title, Vec::new(), false, true)
}

/// Read a foreign provider session file as an inert message list for OpenMesh Chat.
///
/// This is read-only: the path is opened for reading only; nothing is written back.
pub fn read_foreign_transcript(tool: &str, session_path: &str) -> Result<ForeignTranscript, String> {
    let canonical = normalize_tool(tool).ok_or_else(|| {
        format!(
            "Tool '{tool}' is not supported for transcript import. Allowed: codex, claude, cursor, gemini, grok, opencode"
        )
    })?;
    let path = PathBuf::from(session_path);
    if !path.exists() {
        return Err(format!("Session path does not exist: {session_path}"));
    }
    if !path.is_file() {
        return Err(format!("Session path is not a file: {session_path}"));
    }

    let transcript = match canonical {
        "codex" => extract_codex(&path),
        "claude" => extract_claude(&path),
        "cursor" => extract_cursor(&path),
        "gemini" => extract_gemini(&path),
        "grok" => extract_grok(&path),
        "opencode" => extract_opencode(&path),
        _ => {
            return Err(format!(
                "Tool '{canonical}' has no transcript extractor yet"
            ))
        }
    };
    Ok(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn extracts_cursor_user_and_assistant() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("abc.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"role":"user","message":{{"content":[{{"type":"text","text":"<user_query>\nFix login\n</user_query>"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"role":"assistant","message":{{"content":[{{"type":"text","text":"Looking at auth.ts"}}]}}}}"#
        )
        .unwrap();
        let t = read_foreign_transcript("cursor", path.to_str().unwrap()).unwrap();
        assert!(!t.preview_only);
        assert_eq!(t.messages.len(), 2);
        assert_eq!(t.messages[0].role, "user");
        assert_eq!(t.messages[0].text, "Fix login");
        assert_eq!(t.messages[1].role, "assistant");
        assert_eq!(t.title.as_deref(), Some("Fix login"));
    }

    #[test]
    fn extracts_claude_turns() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("sess.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":"Add tests"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"Sure"}}]}}}}"#
        )
        .unwrap();
        let t = read_foreign_transcript("claude", path.to_str().unwrap()).unwrap();
        assert_eq!(t.messages.len(), 2);
        assert_eq!(t.messages[0].role, "user");
        assert_eq!(t.messages[0].text, "Add tests");
        assert_eq!(t.messages[1].role, "assistant");
        assert_eq!(t.messages[1].text, "Sure");
    }

    #[test]
    fn extracts_grok_array_user_and_string_assistant() {
        let dir = tempdir().unwrap();
        let sess = dir.path().join("019fc668-demo");
        std::fs::create_dir_all(&sess).unwrap();
        std::fs::write(
            sess.join("summary.json"),
            r#"{"info":{"id":"019fc668-demo","cwd":"/tmp"},"session_summary":"Nav rename","generated_title":"Nav rename"}"#,
        )
        .unwrap();
        let mut f = std::fs::File::create(sess.join("chat_history.jsonl")).unwrap();
        writeln!(
            f,
            r#"{{"type":"system","content":"You are Grok released by xAI. Long prompt…"}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","content":[{{"type":"text","text":"<user_info>\nOS Version: macos\n</user_info>"}}]}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","content":[{{"type":"text","text":"<system-reminder>\nskills…\n</system-reminder>"}}],"synthetic_reason":"system_reminder"}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"user","content":[{{"type":"text","text":"<user_query>\nFix the login bug\n</user_query>"}}]}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"reasoning","content":null}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","content":"Hi! How can I help you today?"}}"#,
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"assistant","content":"Looking at auth.ts"}}"#,
        )
        .unwrap();
        let summary = sess.join("summary.json");
        let t = read_foreign_transcript("grok", summary.to_str().unwrap()).unwrap();
        assert!(!t.preview_only);
        assert_eq!(t.messages.len(), 3);
        assert_eq!(t.messages[0].role, "user");
        assert_eq!(t.messages[0].text, "Fix the login bug");
        assert_eq!(t.messages[1].role, "assistant");
        assert_eq!(t.messages[1].text, "Hi! How can I help you today?");
        assert_eq!(t.messages[2].role, "assistant");
        assert_eq!(t.messages[2].text, "Looking at auth.ts");
        // Also accept chat_history.jsonl as the entry path.
        let hist = sess.join("chat_history.jsonl");
        let t2 = read_foreign_transcript("grok", hist.to_str().unwrap()).unwrap();
        assert_eq!(t2.messages.len(), 3);
        assert_eq!(t2.messages[0].role, "user");
    }

    #[test]
    fn extracts_codex_response_items() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rollout.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"Hello"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            f,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","content":[{{"type":"output_text","text":"Hi"}}]}}}}"#
        )
        .unwrap();
        let t = read_foreign_transcript("codex", path.to_str().unwrap()).unwrap();
        assert_eq!(t.messages.len(), 2);
        assert_eq!(t.messages[1].text, "Hi");
    }

    #[test]
    fn opencode_without_messages_is_preview_only() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("session.json");
        std::fs::write(
            &path,
            r#"{"id":"s1","title":"Wire resume","directory":"/tmp/demo"}"#,
        )
        .unwrap();
        let t = read_foreign_transcript("opencode", path.to_str().unwrap()).unwrap();
        assert!(t.preview_only);
        assert!(t.messages.is_empty());
        assert_eq!(t.title.as_deref(), Some("Wire resume"));
    }

    #[test]
    fn refuses_unknown_tool() {
        let err = read_foreign_transcript("nope", "/tmp/x").unwrap_err();
        assert!(err.contains("not supported"));
    }
}
