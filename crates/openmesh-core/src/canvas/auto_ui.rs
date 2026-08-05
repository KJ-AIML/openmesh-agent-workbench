//! OpenMesh Auto UI canvases — safe JSON UI documents agents can create.
//!
//! Not Cursor `.canvas.tsx` (that SDK only runs inside Cursor). Agents emit an
//! allowlisted component tree; the desktop app renders it with Vue components.
//! No arbitrary code execution, no HTML eval.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

pub const AUTO_UI_SCHEMA: &str = "openmesh.canvas/1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoUiDocument {
    /// Must be `openmesh.canvas/1`.
    pub schema: String,
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub blocks: Vec<AutoUiBlock>,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AutoUiBlock {
    #[serde(rename = "h1")]
    H1 { text: String },
    #[serde(rename = "h2")]
    H2 { text: String },
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        tone: Option<String>,
    },
    #[serde(rename = "callout")]
    Callout {
        text: String,
        #[serde(default = "default_info_tone")]
        tone: String,
    },
    #[serde(rename = "stat")]
    Stat {
        label: String,
        value: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        hint: Option<String>,
    },
    #[serde(rename = "stats")]
    Stats { items: Vec<AutoUiStatItem> },
    #[serde(rename = "table")]
    Table {
        columns: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    #[serde(rename = "pills")]
    Pills { items: Vec<AutoUiPill> },
    #[serde(rename = "todo")]
    Todo { items: Vec<AutoUiTodoItem> },
    #[serde(rename = "code")]
    Code {
        code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        language: Option<String>,
    },
    #[serde(rename = "divider")]
    Divider,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoUiStatItem {
    pub label: String,
    pub value: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoUiPill {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutoUiTodoItem {
    pub text: String,
    #[serde(default)]
    pub done: bool,
}

fn default_info_tone() -> String {
    "info".into()
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn auto_ui_dir(project_path: &Path) -> PathBuf {
    project_path.join(".openmesh").join("canvases").join("auto-ui")
}

fn auto_ui_file(project_path: &Path, id: &str) -> PathBuf {
    auto_ui_dir(project_path).join(format!("{id}.json"))
}

#[derive(Debug, thiserror::Error)]
pub enum AutoUiError {
    #[error("io: {0}")]
    Io(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

/// Validate and normalize a document (or raw JSON value) before persist.
pub fn parse_auto_ui_document(value: &Value) -> Result<AutoUiDocument, AutoUiError> {
    let mut doc: AutoUiDocument =
        serde_json::from_value(value.clone()).map_err(|e| AutoUiError::Invalid(e.to_string()))?;
    if doc.schema != AUTO_UI_SCHEMA {
        return Err(AutoUiError::Invalid(format!(
            "schema must be {AUTO_UI_SCHEMA}, got {}",
            doc.schema
        )));
    }
    if doc.title.trim().is_empty() {
        return Err(AutoUiError::Invalid("title is required".into()));
    }
    if doc.blocks.len() > 80 {
        return Err(AutoUiError::Invalid("too many blocks (max 80)".into()));
    }
    // Sanitize string lengths to keep disk/UI bounded.
    for block in &mut doc.blocks {
        clamp_block(block)?;
    }
    if doc.id.trim().is_empty() {
        doc.id = format!("aui-{}", now_ms());
    }
    // Keep ids filesystem-safe.
    let safe: String = doc
        .id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if safe.is_empty() {
        return Err(AutoUiError::Invalid("id is empty after sanitize".into()));
    }
    doc.id = safe;
    doc.updated_at = now_ms();
    Ok(doc)
}

fn clamp_block(block: &mut AutoUiBlock) -> Result<(), AutoUiError> {
    const MAX: usize = 8_000;
    match block {
        AutoUiBlock::H1 { text }
        | AutoUiBlock::H2 { text }
        | AutoUiBlock::Text { text, .. }
        | AutoUiBlock::Callout { text, .. } => {
            if text.len() > MAX {
                text.truncate(MAX);
            }
        }
        AutoUiBlock::Code { code, .. } => {
            if code.len() > MAX * 2 {
                code.truncate(MAX * 2);
            }
        }
        AutoUiBlock::Table { columns, rows } => {
            if columns.len() > 12 {
                return Err(AutoUiError::Invalid("table columns max 12".into()));
            }
            if rows.len() > 100 {
                rows.truncate(100);
            }
        }
        AutoUiBlock::Stats { items } => {
            if items.len() > 12 {
                items.truncate(12);
            }
        }
        AutoUiBlock::Pills { items } => {
            if items.len() > 24 {
                items.truncate(24);
            }
        }
        AutoUiBlock::Todo { items } => {
            if items.len() > 40 {
                items.truncate(40);
            }
        }
        AutoUiBlock::Stat { .. } | AutoUiBlock::Divider => {}
    }
    Ok(())
}

pub fn list_auto_ui(project_path: &Path) -> Result<Vec<AutoUiDocument>, AutoUiError> {
    let dir = auto_ui_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| AutoUiError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| AutoUiError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(doc) = load_auto_ui_path(&path) {
            out.push(doc);
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

pub fn load_auto_ui(project_path: &Path, id: &str) -> Result<AutoUiDocument, AutoUiError> {
    load_auto_ui_path(&auto_ui_file(project_path, id))
}

fn load_auto_ui_path(path: &Path) -> Result<AutoUiDocument, AutoUiError> {
    let raw = fs::read_to_string(path).map_err(|e| AutoUiError::NotFound(e.to_string()))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| AutoUiError::Invalid(e.to_string()))?;
    parse_auto_ui_document(&value)
}

pub fn upsert_auto_ui(
    project_path: &Path,
    value: &Value,
) -> Result<AutoUiDocument, AutoUiError> {
    let doc = parse_auto_ui_document(value)?;
    let dir = auto_ui_dir(project_path);
    fs::create_dir_all(&dir).map_err(|e| AutoUiError::Io(e.to_string()))?;
    let path = auto_ui_file(project_path, &doc.id);
    let raw =
        serde_json::to_string_pretty(&doc).map_err(|e| AutoUiError::Invalid(e.to_string()))?;
    fs::write(path, raw).map_err(|e| AutoUiError::Io(e.to_string()))?;
    Ok(doc)
}

pub fn delete_auto_ui(project_path: &Path, id: &str) -> Result<(), AutoUiError> {
    let path = auto_ui_file(project_path, id);
    if !path.exists() {
        return Err(AutoUiError::NotFound(id.into()));
    }
    fs::remove_file(path).map_err(|e| AutoUiError::Io(e.to_string()))
}

/// Tool helper: upsert from agent JSON args.
pub fn upsert_auto_ui_from_tool(project_path: &str, arguments_json: &str) -> Result<String, String> {
    let args: Value = serde_json::from_str(arguments_json).map_err(|e| e.to_string())?;
    let doc_val = if let Some(doc) = args.get("document") {
        doc.clone()
    } else if args.get("schema").is_some() {
        args.clone()
    } else {
        return Err("canvas_upsert_auto_ui requires document object (schema openmesh.canvas/1)".into());
    };
    let saved = upsert_auto_ui(Path::new(project_path), &doc_val).map_err(|e| e.to_string())?;
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "ok": true,
        "id": saved.id,
        "title": saved.title,
        "blockCount": saved.blocks.len(),
        "path": format!(".openmesh/canvases/auto-ui/{}.json", saved.id),
        "hint": "Open Canvas → Auto UI to view this artifact.",
    }))
    .unwrap_or_else(|_| "{}".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn upsert_and_list_roundtrip() {
        let dir = tempdir().unwrap();
        let project = dir.path();
        let doc = json!({
            "schema": AUTO_UI_SCHEMA,
            "id": "status-board",
            "title": "Sprint pulse",
            "summary": "Quick look",
            "blocks": [
                { "type": "h1", "text": "Sprint pulse" },
                { "type": "stat", "label": "Done", "value": "3/8" },
                { "type": "callout", "text": "Ship LAN ask next", "tone": "warn" }
            ]
        });
        let saved = upsert_auto_ui(project, &doc).unwrap();
        assert_eq!(saved.id, "status-board");
        let list = list_auto_ui(project).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Sprint pulse");
    }

    #[test]
    fn rejects_wrong_schema() {
        let err = parse_auto_ui_document(&json!({
            "schema": "cursor/canvas",
            "id": "x",
            "title": "Nope",
            "blocks": []
        }))
        .unwrap_err();
        assert!(err.to_string().contains("openmesh.canvas/1"));
    }

    #[test]
    fn does_not_false_accept_github_as_git() {
        // sanity: Auto UI path unrelated — ensure .github-like names ok in titles
        let doc = parse_auto_ui_document(&json!({
            "schema": AUTO_UI_SCHEMA,
            "id": "gh",
            "title": "GitHub notes",
            "blocks": [{ "type": "text", "text": "hello" }]
        }))
        .unwrap();
        assert_eq!(doc.title, "GitHub notes");
    }
}
