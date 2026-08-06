//! Durable Agent Chat sessions under `<project>/.openmesh/agent/chats/`.

use crate::storage::{atomic_write, get_project_dir};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChatMessage {
    pub id: String,
    pub role: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatImportProvenance {
    pub source: String,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredChatSession {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub title_is_default: bool,
    pub messages: Vec<StoredChatMessage>,
    pub created_at: i64,
    pub updated_at: i64,
    /// When this chat was seeded from a scanned foreign session (copy only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_from: Option<ChatImportProvenance>,
}

fn chats_path(project_path: &str) -> PathBuf {
    get_project_dir(project_path)
        .join("agent")
        .join("chats")
        .join("sessions.json")
}

pub fn load_chat_sessions(project_path: &str) -> Result<Vec<StoredChatSession>, String> {
    let path = chats_path(project_path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let sessions: Vec<StoredChatSession> =
        serde_json::from_str(&text).map_err(|e| format!("corrupt chat sessions: {e}"))?;
    Ok(sessions)
}

pub fn save_chat_sessions(
    project_path: &str,
    sessions: &[StoredChatSession],
) -> Result<(), String> {
    let path = chats_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let capped: Vec<_> = sessions.iter().take(50).cloned().collect();
    let json = serde_json::to_string_pretty(&capped).map_err(|e| e.to_string())?;
    atomic_write(&path, &json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_project;

    #[test]
    fn roundtrip_sessions() {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-chats-{}-{}",
            std::process::id(),
            1
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let project = dir.to_string_lossy().to_string();
        init_project(&project).unwrap();
        let sessions = vec![StoredChatSession {
            id: "chat-1".into(),
            title: "Hello".into(),
            title_is_default: false,
            messages: vec![StoredChatMessage {
                id: "m1".into(),
                role: "user".into(),
                text: "hi".into(),
                tool_calls: None,
                at: 1,
            }],
            created_at: 1,
            updated_at: 2,
            imported_from: Some(ChatImportProvenance {
                source: "cursor".into(),
                id: "abc".into(),
                path: Some("/tmp/abc.jsonl".into()),
            }),
        }];
        save_chat_sessions(&project, &sessions).unwrap();
        let loaded = load_chat_sessions(&project).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].title, "Hello");
        assert_eq!(
            loaded[0]
                .imported_from
                .as_ref()
                .map(|p| p.source.as_str()),
            Some("cursor")
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
