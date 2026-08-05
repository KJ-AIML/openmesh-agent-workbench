//! Freeform Board documents — Excalidraw scene JSON under OpenMesh storage.
//!
//! Path: `<project>/.openmesh/canvases/boards/<id>.json`
//! Engine (`@excalidraw/excalidraw`) is renderer/editor only; OpenMesh owns persistence.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const BOARD_SCHEMA: &str = "openmesh.board/1";
pub const BOARD_ENGINE_EXCALIDRAW: &str = "excalidraw";

/// Soft cap so a single board cannot blow disk/IPC (bytes of pretty JSON).
const MAX_SCENE_JSON_BYTES: usize = 8_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BoardDocument {
    /// Must be `openmesh.board/1`.
    pub schema: String,
    pub id: String,
    pub title: String,
    /// Renderer/editor id — currently only `excalidraw`.
    pub engine: String,
    /// Opaque engine scene (Excalidraw elements/appState/files).
    pub scene: Value,
    #[serde(default)]
    pub updated_at: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum BoardError {
    #[error("io: {0}")]
    Io(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn boards_dir(project_path: &Path) -> PathBuf {
    project_path
        .join(".openmesh")
        .join("canvases")
        .join("boards")
}

/// Sanitize board ids for filesystem confinement (no path separators / traversal).
pub fn sanitize_board_id(id: &str) -> Result<String, BoardError> {
    let trimmed = id.trim();
    if trimmed.is_empty() {
        return Err(BoardError::Invalid("id is required".into()));
    }
    if trimmed.contains("..") || trimmed.contains('/') || trimmed.contains('\\') {
        return Err(BoardError::Invalid("id must not contain path segments".into()));
    }
    let safe: String = trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if safe.is_empty() || safe == "-" || safe.chars().all(|c| c == '-') {
        return Err(BoardError::Invalid("id is empty after sanitize".into()));
    }
    if safe.len() > 120 {
        return Err(BoardError::Invalid("id too long (max 120)".into()));
    }
    Ok(safe)
}

fn board_file(project_path: &Path, id: &str) -> Result<PathBuf, BoardError> {
    // Confinement: sanitize rejects `..`, `/`, `\`; file is always a single
    // basename under `.openmesh/canvases/boards/`.
    let safe = sanitize_board_id(id)?;
    Ok(boards_dir(project_path).join(format!("{safe}.json")))
}

fn empty_excalidraw_scene() -> Value {
    json!({
        "elements": [],
        "appState": {
            "viewBackgroundColor": "#ffffff"
        },
        "files": {}
    })
}

/// Validate and normalize a board document (or raw JSON value) before persist.
pub fn parse_board_document(value: &Value) -> Result<BoardDocument, BoardError> {
    let mut doc: BoardDocument =
        serde_json::from_value(value.clone()).map_err(|e| BoardError::Invalid(e.to_string()))?;
    if doc.schema != BOARD_SCHEMA {
        return Err(BoardError::Invalid(format!(
            "schema must be {BOARD_SCHEMA}, got {}",
            doc.schema
        )));
    }
    if doc.title.trim().is_empty() {
        return Err(BoardError::Invalid("title is required".into()));
    }
    if doc.title.len() > 200 {
        doc.title.truncate(200);
    }
    if doc.engine != BOARD_ENGINE_EXCALIDRAW {
        return Err(BoardError::Invalid(format!(
            "engine must be {BOARD_ENGINE_EXCALIDRAW} in v1"
        )));
    }
    if doc.id.trim().is_empty() {
        doc.id = format!("board-{}", now_ms());
    }
    doc.id = sanitize_board_id(&doc.id)?;
    if !doc.scene.is_object() {
        return Err(BoardError::Invalid("scene must be a JSON object".into()));
    }
    let scene_bytes = serde_json::to_vec(&doc.scene).map_err(|e| BoardError::Invalid(e.to_string()))?;
    if scene_bytes.len() > MAX_SCENE_JSON_BYTES {
        return Err(BoardError::Invalid(format!(
            "scene too large (max {MAX_SCENE_JSON_BYTES} bytes)"
        )));
    }
    // Preserve caller/file `updatedAt` on read; writers overwrite on upsert.
    Ok(doc)
}

pub fn list_boards(project_path: &Path) -> Result<Vec<BoardDocument>, BoardError> {
    let dir = boards_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| BoardError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| BoardError::Io(e.to_string()))?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(doc) = load_board_path(&path) {
            out.push(doc);
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

pub fn load_board(project_path: &Path, id: &str) -> Result<BoardDocument, BoardError> {
    let path = board_file(project_path, id)?;
    load_board_path(&path)
}

fn load_board_path(path: &Path) -> Result<BoardDocument, BoardError> {
    let raw = fs::read_to_string(path).map_err(|e| BoardError::NotFound(e.to_string()))?;
    let value: Value =
        serde_json::from_str(&raw).map_err(|e| BoardError::Invalid(e.to_string()))?;
    parse_board_document(&value)
}

pub fn create_board(
    project_path: &Path,
    title: impl Into<String>,
) -> Result<BoardDocument, BoardError> {
    let title = title.into();
    let doc = BoardDocument {
        schema: BOARD_SCHEMA.into(),
        id: format!("board-{}", now_ms()),
        title: if title.trim().is_empty() {
            "Untitled board".into()
        } else {
            title
        },
        engine: BOARD_ENGINE_EXCALIDRAW.into(),
        scene: empty_excalidraw_scene(),
        updated_at: now_ms(),
    };
    upsert_board(project_path, &serde_json::to_value(&doc).unwrap())
}

pub fn upsert_board(project_path: &Path, value: &Value) -> Result<BoardDocument, BoardError> {
    let mut doc = parse_board_document(value)?;
    doc.updated_at = now_ms();
    let path = board_file(project_path, &doc.id)?;
    let dir = boards_dir(project_path);
    fs::create_dir_all(&dir).map_err(|e| BoardError::Io(e.to_string()))?;
    let raw =
        serde_json::to_string_pretty(&doc).map_err(|e| BoardError::Invalid(e.to_string()))?;
    fs::write(&path, raw).map_err(|e| BoardError::Io(e.to_string()))?;
    Ok(doc)
}

pub fn save_board_scene(
    project_path: &Path,
    id: &str,
    scene: &Value,
) -> Result<BoardDocument, BoardError> {
    let mut doc = load_board(project_path, id)?;
    doc.scene = scene.clone();
    upsert_board(project_path, &serde_json::to_value(&doc).unwrap())
}

pub fn delete_board(project_path: &Path, id: &str) -> Result<(), BoardError> {
    let path = board_file(project_path, id)?;
    if !path.exists() {
        return Err(BoardError::NotFound(id.into()));
    }
    fs::remove_file(path).map_err(|e| BoardError::Io(e.to_string()))
}

fn element_id() -> String {
    format!("el-{}", now_ms())
}

fn resolve_board(
    project_path: &Path,
    board_id: Option<&str>,
) -> Result<BoardDocument, BoardError> {
    if let Some(id) = board_id.map(str::trim).filter(|s| !s.is_empty()) {
        return load_board(project_path, id);
    }
    let list = list_boards(project_path)?;
    if let Some(doc) = list.into_iter().next() {
        Ok(doc)
    } else {
        create_board(project_path, "Board")
    }
}

fn scene_elements_mut(doc: &mut BoardDocument) -> Result<&mut Vec<Value>, BoardError> {
    let scene = doc
        .scene
        .as_object_mut()
        .ok_or_else(|| BoardError::Invalid("scene must be object".into()))?;
    if !scene.contains_key("elements") {
        scene.insert("elements".into(), json!([]));
    }
    scene
        .get_mut("elements")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| BoardError::Invalid("scene.elements must be an array".into()))
}

/// Agent/human-safe: append a text sticky without dumping a full Excalidraw scene.
pub fn board_add_sticky(
    project_path: &Path,
    board_id: Option<&str>,
    text: &str,
) -> Result<BoardDocument, BoardError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(BoardError::Invalid("sticky text is required".into()));
    }
    if text.len() > 500 {
        return Err(BoardError::Invalid("sticky text too long (max 500)".into()));
    }
    let mut doc = resolve_board(project_path, board_id)?;
    let elements = scene_elements_mut(&mut doc)?;
    let n = elements.len() as f64;
    let id = element_id();
    elements.push(json!({
        "id": id,
        "type": "text",
        "x": 80.0 + (n % 5.0) * 40.0,
        "y": 80.0 + (n / 5.0).floor() * 60.0,
        "width": 240.0,
        "height": 48.0,
        "angle": 0,
        "strokeColor": "#1e1e1e",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 1,
        "strokeStyle": "solid",
        "roughness": 1,
        "opacity": 100,
        "text": text,
        "fontSize": 20,
        "fontFamily": 1,
        "textAlign": "left",
        "verticalAlign": "top",
        "containerId": null,
        "originalText": text,
        "lineHeight": 1.25,
        "baseline": 18,
        "isDeleted": false,
        "boundElements": null,
        "updated": now_ms(),
        "link": null,
        "locked": false,
        "version": 1,
        "versionNonce": now_ms() as u32,
        "groupIds": [],
        "frameId": null,
        "roundness": null,
        "seed": (now_ms() % 2_000_000_000) as u32,
        "index": null,
    }));
    upsert_board(project_path, &serde_json::to_value(&doc).unwrap())
}

fn find_text_element<'a>(elements: &'a [Value], label: &str) -> Option<&'a Value> {
    let needle = label.trim().to_ascii_lowercase();
    elements.iter().find(|el| {
        el.get("type").and_then(|t| t.as_str()) == Some("text")
            && el
                .get("text")
                .and_then(|t| t.as_str())
                .map(|t| t.trim().to_ascii_lowercase() == needle || t.to_ascii_lowercase().contains(&needle))
                .unwrap_or(false)
            && el.get("isDeleted").and_then(|d| d.as_bool()) != Some(true)
    })
}

/// Connect two labeled text stickies with an arrow (by label match).
pub fn board_connect(
    project_path: &Path,
    board_id: Option<&str>,
    from_label: &str,
    to_label: &str,
) -> Result<BoardDocument, BoardError> {
    let from_label = from_label.trim();
    let to_label = to_label.trim();
    if from_label.is_empty() || to_label.is_empty() {
        return Err(BoardError::Invalid("from and to labels are required".into()));
    }
    let mut doc = resolve_board(project_path, board_id)?;
    let elements = scene_elements_mut(&mut doc)?;
    let from = find_text_element(elements, from_label)
        .ok_or_else(|| BoardError::Invalid(format!("no sticky matching '{from_label}'")))?
        .clone();
    let to = find_text_element(elements, to_label)
        .ok_or_else(|| BoardError::Invalid(format!("no sticky matching '{to_label}'")))?
        .clone();
    let fx = from.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0)
        + from.get("width").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let fy = from.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0)
        + from.get("height").and_then(|v| v.as_f64()).unwrap_or(24.0) / 2.0;
    let tx = to.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let ty = to.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0)
        + to.get("height").and_then(|v| v.as_f64()).unwrap_or(24.0) / 2.0;
    let id = element_id();
    elements.push(json!({
        "id": id,
        "type": "arrow",
        "x": fx,
        "y": fy,
        "width": tx - fx,
        "height": ty - fy,
        "angle": 0,
        "strokeColor": "#1e1e1e",
        "backgroundColor": "transparent",
        "fillStyle": "solid",
        "strokeWidth": 2,
        "strokeStyle": "solid",
        "roughness": 1,
        "opacity": 100,
        "points": [[0, 0], [tx - fx, ty - fy]],
        "lastCommittedPoint": null,
        "startBinding": null,
        "endBinding": null,
        "startArrowhead": null,
        "endArrowhead": "arrow",
        "isDeleted": false,
        "boundElements": null,
        "updated": now_ms(),
        "link": null,
        "locked": false,
        "version": 1,
        "versionNonce": now_ms() as u32,
        "groupIds": [],
        "frameId": null,
        "roundness": { "type": 2 },
        "seed": (now_ms() % 2_000_000_000) as u32,
        "index": null,
    }));
    upsert_board(project_path, &serde_json::to_value(&doc).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn create_save_list_roundtrip() {
        let dir = tempdir().unwrap();
        let project = dir.path();
        let created = create_board(project, "Sketch").unwrap();
        assert_eq!(created.schema, BOARD_SCHEMA);
        assert_eq!(created.engine, BOARD_ENGINE_EXCALIDRAW);
        assert!(created.scene.get("elements").is_some());

        let scene = json!({
            "elements": [{ "id": "r1", "type": "rectangle", "x": 10, "y": 20 }],
            "appState": { "viewBackgroundColor": "#fafafa" },
            "files": {}
        });
        let saved = save_board_scene(project, &created.id, &scene).unwrap();
        assert_eq!(saved.scene["elements"][0]["id"], "r1");

        let list = list_boards(project).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Sketch");

        let loaded = load_board(project, &created.id).unwrap();
        assert_eq!(loaded.scene["appState"]["viewBackgroundColor"], "#fafafa");
    }

    #[test]
    fn rejects_path_traversal_id() {
        let err = sanitize_board_id("../escape").unwrap_err();
        assert!(err.to_string().contains("path"));
        let err = sanitize_board_id("a/b").unwrap_err();
        assert!(err.to_string().contains("path"));
    }

    #[test]
    fn rejects_wrong_schema() {
        let err = parse_board_document(&json!({
            "schema": "openmesh.canvas/1",
            "id": "x",
            "title": "Nope",
            "engine": "excalidraw",
            "scene": { "elements": [] }
        }))
        .unwrap_err();
        assert!(err.to_string().contains("openmesh.board/1"));
    }

    #[test]
    fn boards_live_under_boards_subdir() {
        let dir = tempdir().unwrap();
        let project = dir.path();
        let doc = create_board(project, "A").unwrap();
        let path = project
            .join(".openmesh")
            .join("canvases")
            .join("boards")
            .join(format!("{}.json", doc.id));
        assert!(path.is_file());
    }

    #[test]
    fn sticky_and_connect_ops() {
        let dir = tempdir().unwrap();
        let project = dir.path();
        let a = board_add_sticky(project, None, "API").unwrap();
        let b = board_add_sticky(project, Some(&a.id), "DB").unwrap();
        assert_eq!(b.scene["elements"].as_array().unwrap().len(), 2);
        let linked = board_connect(project, Some(&a.id), "API", "DB").unwrap();
        let els = linked.scene["elements"].as_array().unwrap();
        assert_eq!(els.len(), 3);
        assert_eq!(els[2]["type"], "arrow");
    }
}
