//! Typed AppAction contracts for OpenMesh (JARVIS control plane).
//!
//! Agent Engine remains the sole planner. AppActions are the sole execution language
//! for in-app UI/domain changes — never pixel clicks.

pub mod audit;
pub mod dispatcher;
pub mod history;
pub mod policy;
pub mod routes;
pub mod types;

pub use audit::{append_to_project, list_memory, record_result, ActionAuditEntry};
pub use dispatcher::{dispatch_intent, AppActionHandler};
pub use history::{inverse_for, peek_undo, pop_undo, push_undo, UndoFrame};
pub use policy::{may_dispatch, mode_allows, AgentMode};
pub use routes::{label_for_route, normalize_ui_route, ui_navigate_json, ALLOWED_UI_ROUTES};
pub use types::{
    ActionIntent, ActionResult, ActionSource, AppAction, AppContext, ConfirmationPolicy, RiskClass,
};
