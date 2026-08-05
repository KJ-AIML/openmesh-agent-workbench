//! Append-only AppAction audit ledger (in-memory + optional JSONL under project).

use super::types::{ActionIntent, ActionResult, ActionSource, AppAction};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionAuditEntry {
    pub id: String,
    pub at: u64,
    pub source: ActionSource,
    pub action: AppAction,
    pub ok: bool,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub undo_ref: Option<String>,
}

static MEMORY: Mutex<Vec<ActionAuditEntry>> = Mutex::new(Vec::new());

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn record_result(
    intent: &ActionIntent,
    result: &ActionResult,
    undo_ref: Option<String>,
) -> ActionAuditEntry {
    let entry = ActionAuditEntry {
        id: format!("act-{}", now_ms()),
        at: now_ms(),
        source: intent.source,
        action: intent.action.clone(),
        ok: result.ok,
        summary: result.summary.clone(),
        error: result.error.clone(),
        turn_id: intent.turn_id.clone(),
        undo_ref,
    };
    if let Ok(mut guard) = MEMORY.lock() {
        guard.push(entry.clone());
        if guard.len() > 200 {
            let drain = guard.len() - 200;
            guard.drain(0..drain);
        }
    }
    entry
}

pub fn list_memory(limit: usize) -> Vec<ActionAuditEntry> {
    MEMORY
        .lock()
        .map(|g| g.iter().rev().take(limit).cloned().collect())
        .unwrap_or_default()
}

pub fn clear_memory() {
    if let Ok(mut g) = MEMORY.lock() {
        g.clear();
    }
}

fn audit_path(project_path: &Path) -> PathBuf {
    project_path
        .join(".openmesh")
        .join("actions")
        .join("audit.jsonl")
}

pub fn append_to_project(project_path: &Path, entry: &ActionAuditEntry) -> Result<(), String> {
    let path = audit_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(entry).map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_actions::types::ActionSource;

    #[test]
    fn records_and_lists() {
        clear_memory();
        let intent = ActionIntent::new(
            AppAction::Navigate {
                route: "/docs".into(),
            },
            ActionSource::Voice,
        );
        let result = ActionResult::success("Docs", intent.action.clone());
        record_result(&intent, &result, None);
        let listed = list_memory(5);
        assert!(!listed.is_empty());
        assert!(listed[0].ok);
    }
}
