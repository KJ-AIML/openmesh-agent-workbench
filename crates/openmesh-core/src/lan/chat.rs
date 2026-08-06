//! Local + LAN human chat messages (trusted-LAN alpha).
//!
//! Messages are pushed over `POST /v1/chat/message` and stored under
//! `.openmesh/lan/chat/` so both peers keep a durable thread without cloud.

use crate::lan::contract::{LanChatMessage, LAN_CHAT_PROTOCOL, MAX_CHAT_TEXT_BYTES};
use crate::storage::get_project_dir;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use thiserror::Error;

const CHAT_DIR: &str = "lan/chat";
const MESSAGES_FILE: &str = "messages.jsonl";
const MAX_READ: usize = 500;

#[derive(Debug, Error)]
pub enum LanChatError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("io: {0}")]
    Io(String),
    #[error("decode: {0}")]
    Decode(String),
}

/// Direction relative to the local project store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanChatDirection {
    Inbound,
    Outbound,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredLanChatMessage {
    pub message: LanChatMessage,
    pub direction: LanChatDirection,
    /// Peer host:port for outbound, or remote peer id for inbound.
    pub peer_key: String,
    pub stored_at: String,
}

fn chat_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(CHAT_DIR)
}

fn messages_path(project_path: &str) -> PathBuf {
    chat_dir(project_path).join(MESSAGES_FILE)
}

pub fn validate_chat_message(msg: &LanChatMessage) -> Result<(), LanChatError> {
    if msg.protocol != LAN_CHAT_PROTOCOL {
        return Err(LanChatError::Validation(format!(
            "unsupported chat protocol {}",
            msg.protocol
        )));
    }
    if msg.message_id.trim().is_empty() || msg.message_id.len() > 128 {
        return Err(LanChatError::Validation("message_id invalid".into()));
    }
    if msg.from_peer_id.trim().is_empty() || msg.from_peer_id.len() > 128 {
        return Err(LanChatError::Validation("from_peer_id invalid".into()));
    }
    if msg.from_label.trim().is_empty() || msg.from_label.len() > 128 {
        return Err(LanChatError::Validation("from_label invalid".into()));
    }
    let text = msg.text.trim();
    if text.is_empty() {
        return Err(LanChatError::Validation("text empty".into()));
    }
    if msg.text.len() > MAX_CHAT_TEXT_BYTES {
        return Err(LanChatError::Validation(format!(
            "text exceeds {MAX_CHAT_TEXT_BYTES} bytes"
        )));
    }
    if msg.sent_at.trim().is_empty() {
        return Err(LanChatError::Validation("sent_at required".into()));
    }
    Ok(())
}

pub fn append_chat_message(
    project_path: &str,
    message: &LanChatMessage,
    direction: LanChatDirection,
    peer_key: &str,
) -> Result<StoredLanChatMessage, LanChatError> {
    validate_chat_message(message)?;
    if peer_key.trim().is_empty() {
        return Err(LanChatError::Validation("peer_key required".into()));
    }
    let stored = StoredLanChatMessage {
        message: message.clone(),
        direction,
        peer_key: peer_key.trim().to_string(),
        stored_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };
    let path = messages_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| LanChatError::Io(e.to_string()))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| LanChatError::Io(e.to_string()))?;
    let line = serde_json::to_string(&stored).map_err(|e| LanChatError::Decode(e.to_string()))?;
    writeln!(f, "{line}").map_err(|e| LanChatError::Io(e.to_string()))?;
    f.sync_all().map_err(|e| LanChatError::Io(e.to_string()))?;
    Ok(stored)
}

/// Newest-last list (chronological), optionally filtered by peer_key.
pub fn list_chat_messages(
    project_path: &str,
    peer_key: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<StoredLanChatMessage>, LanChatError> {
    let path = messages_path(project_path);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(&path).map_err(|e| LanChatError::Io(e.to_string()))?;
    let reader = BufReader::new(file);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| LanChatError::Io(e.to_string()))?;
        if line.trim().is_empty() {
            continue;
        }
        let row: StoredLanChatMessage =
            serde_json::from_str(&line).map_err(|e| LanChatError::Decode(e.to_string()))?;
        if let Some(want) = peer_key {
            if row.peer_key != want {
                continue;
            }
        }
        rows.push(row);
    }
    let cap = limit.unwrap_or(MAX_READ).min(MAX_READ);
    if rows.len() > cap {
        rows = rows.split_off(rows.len() - cap);
    }
    Ok(rows)
}

pub fn new_outbound_message(
    from_peer_id: &str,
    from_label: &str,
    text: &str,
    thread_id: Option<String>,
) -> LanChatMessage {
    let now = Utc::now();
    LanChatMessage {
        protocol: LAN_CHAT_PROTOCOL.into(),
        message_id: format!("chat-{}", now.format("%Y%m%dT%H%M%S%.3fZ")),
        from_peer_id: from_peer_id.to_string(),
        from_label: from_label.to_string(),
        text: text.to_string(),
        sent_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        thread_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_project;

    fn temp_project() -> String {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-chat-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn append_and_list_chat_roundtrip() {
        let project = temp_project();
        let msg = new_outbound_message("lan-a", "Alice", "hello team", None);
        append_chat_message(&project, &msg, LanChatDirection::Outbound, "127.0.0.1:41778")
            .unwrap();
        let inbound = LanChatMessage {
            protocol: LAN_CHAT_PROTOCOL.into(),
            message_id: "chat-in-1".into(),
            from_peer_id: "lan-b".into(),
            from_label: "Bob".into(),
            text: "hi back".into(),
            sent_at: "2026-08-06T01:00:00Z".into(),
            thread_id: None,
        };
        append_chat_message(&project, &inbound, LanChatDirection::Inbound, "lan-b").unwrap();
        let all = list_chat_messages(&project, None, None).unwrap();
        assert_eq!(all.len(), 2);
        let for_peer =
            list_chat_messages(&project, Some("127.0.0.1:41778"), None).unwrap();
        assert_eq!(for_peer.len(), 1);
        assert_eq!(for_peer[0].message.text, "hello team");
    }
}
