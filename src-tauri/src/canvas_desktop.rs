//! Tauri commands for OpenMesh Canvas domain (graph + Auto UI + freeform Board).

use openmesh_core::canvas::{
    add_node, connect_nodes, create_board, create_canvas, delete_auto_ui, delete_board, delete_node,
    list_auto_ui, list_boards, list_canvases, load_auto_ui, load_board, load_canvas,
    board_add_sticky, board_connect, save_board_scene, upsert_auto_ui, upsert_board,
    AutoUiDocument, BoardDocument, CanvasDocument,
};
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasDto {
    pub id: String,
    pub title: String,
    pub schema_version: String,
    pub nodes: Vec<openmesh_core::canvas::CanvasNode>,
    pub edges: Vec<openmesh_core::canvas::CanvasEdge>,
    pub updated_at: u64,
}

impl From<CanvasDocument> for CanvasDto {
    fn from(d: CanvasDocument) -> Self {
        Self {
            id: d.id,
            title: d.title,
            schema_version: d.schema_version,
            nodes: d.nodes,
            edges: d.edges,
            updated_at: d.updated_at,
        }
    }
}

#[tauri::command]
pub fn canvas_list(project_path: String) -> Result<Vec<CanvasDto>, String> {
    list_canvases(&PathBuf::from(project_path))
        .map(|v| v.into_iter().map(CanvasDto::from).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_create(project_path: String, title: String) -> Result<CanvasDto, String> {
    create_canvas(&PathBuf::from(project_path), title)
        .map(CanvasDto::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_load(project_path: String, id: String) -> Result<CanvasDto, String> {
    load_canvas(&PathBuf::from(project_path), &id)
        .map(CanvasDto::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_add_node(
    project_path: String,
    canvas_id: String,
    label: String,
    kind: Option<String>,
) -> Result<CanvasDto, String> {
    add_node(&PathBuf::from(project_path), &canvas_id, label, kind)
        .map(CanvasDto::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_connect(
    project_path: String,
    canvas_id: String,
    from: String,
    to: String,
) -> Result<CanvasDto, String> {
    connect_nodes(&PathBuf::from(project_path), &canvas_id, &from, &to)
        .map(CanvasDto::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_delete_node(
    project_path: String,
    canvas_id: String,
    node_id: String,
) -> Result<CanvasDto, String> {
    delete_node(&PathBuf::from(project_path), &canvas_id, &node_id)
        .map(CanvasDto::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_auto_ui_list(project_path: String) -> Result<Vec<AutoUiDocument>, String> {
    list_auto_ui(&PathBuf::from(project_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_auto_ui_load(project_path: String, id: String) -> Result<AutoUiDocument, String> {
    load_auto_ui(&PathBuf::from(project_path), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_auto_ui_upsert(
    project_path: String,
    document: Value,
) -> Result<AutoUiDocument, String> {
    upsert_auto_ui(&PathBuf::from(project_path), &document).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_auto_ui_delete(project_path: String, id: String) -> Result<(), String> {
    delete_auto_ui(&PathBuf::from(project_path), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_list(project_path: String) -> Result<Vec<BoardDocument>, String> {
    list_boards(&PathBuf::from(project_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_create(project_path: String, title: String) -> Result<BoardDocument, String> {
    create_board(&PathBuf::from(project_path), title).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_load(project_path: String, id: String) -> Result<BoardDocument, String> {
    load_board(&PathBuf::from(project_path), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_upsert(
    project_path: String,
    document: Value,
) -> Result<BoardDocument, String> {
    upsert_board(&PathBuf::from(project_path), &document).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_save_scene(
    project_path: String,
    id: String,
    scene: Value,
) -> Result<BoardDocument, String> {
    save_board_scene(&PathBuf::from(project_path), &id, &scene).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_delete(project_path: String, id: String) -> Result<(), String> {
    delete_board(&PathBuf::from(project_path), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_add_sticky(
    project_path: String,
    text: String,
    board_id: Option<String>,
) -> Result<BoardDocument, String> {
    board_add_sticky(
        &PathBuf::from(project_path),
        board_id.as_deref(),
        &text,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn canvas_board_connect(
    project_path: String,
    from: String,
    to: String,
    board_id: Option<String>,
) -> Result<BoardDocument, String> {
    board_connect(
        &PathBuf::from(project_path),
        board_id.as_deref(),
        &from,
        &to,
    )
    .map_err(|e| e.to_string())
}
