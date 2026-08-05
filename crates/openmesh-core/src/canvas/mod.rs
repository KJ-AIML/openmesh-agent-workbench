//! OpenMesh Canvas domain — Network graph, Auto UI artifacts, and freeform Boards.

pub mod auto_ui;
pub mod board;
pub mod model;
pub mod service;

pub use auto_ui::{
    delete_auto_ui, list_auto_ui, load_auto_ui, parse_auto_ui_document, upsert_auto_ui,
    upsert_auto_ui_from_tool, AutoUiBlock, AutoUiDocument, AutoUiError, AUTO_UI_SCHEMA,
};
pub use board::{
    board_add_sticky, board_connect, create_board, delete_board, list_boards, load_board,
    parse_board_document, save_board_scene, sanitize_board_id, upsert_board, BoardDocument,
    BoardError, BOARD_ENGINE_EXCALIDRAW, BOARD_SCHEMA,
};
pub use model::{CanvasDocument, CanvasEdge, CanvasNode, CanvasRevision};
pub use service::{
    add_node, connect_nodes, create_canvas, delete_node, fit_hint, list_canvases, load_canvas,
    save_canvas, CanvasError,
};
