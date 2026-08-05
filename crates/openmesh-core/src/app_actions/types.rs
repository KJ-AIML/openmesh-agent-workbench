//! Typed action intents for the OpenMesh AppAction bus.

use serde::{Deserialize, Serialize};

/// Where an action originated.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ActionSource {
    Voice,
    Chat,
    Recipe,
    System,
}

/// Risk class for confirmation / audit policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum RiskClass {
    Read,
    Navigate,
    Compose,
    Write,
    /// External side effects (recipe run, session resume).
    External,
    Destructive,
    Privileged,
}

/// Confirmation policy for an intent before dispatch.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConfirmationPolicy {
    None,
    Soft,
    Hard,
}

/// Canonical AppAction variants the UI can apply.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AppAction {
    #[serde(rename = "navigate")]
    Navigate { route: String },
    #[serde(rename = "openPanel")]
    OpenPanel { panel: String },
    #[serde(rename = "closePanel")]
    ClosePanel { panel: String },
    #[serde(rename = "setComposer")]
    SetComposer { text: String },
    #[serde(rename = "focusComposer")]
    FocusComposer,
    #[serde(rename = "setMode")]
    SetMode { mode: String },
    #[serde(rename = "selectSession")]
    SelectSession { session_id: String },
    #[serde(rename = "createNote")]
    CreateNote { title: Option<String> },
    #[serde(rename = "openNote")]
    OpenNote { note_id: String },
    #[serde(rename = "openSprint")]
    OpenSprint { sprint_id: Option<String> },
    #[serde(rename = "createSprint")]
    CreateSprint { name: Option<String> },
    #[serde(rename = "runRecipe")]
    RunRecipe { recipe_id: String },
    #[serde(rename = "stopRecipe")]
    StopRecipe { run_key: String },
    #[serde(rename = "openCanvas")]
    OpenCanvas { canvas_id: Option<String> },
    /// Open Canvas → Board (freeform Excalidraw surface). Thin stub in v1.
    #[serde(rename = "openBoard")]
    OpenBoard { board_id: Option<String> },
    /// Add a text sticky on the freeform Board (not Network graph).
    #[serde(rename = "boardAddSticky")]
    BoardAddSticky {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        board_id: Option<String>,
    },
    /// Connect two stickies by label match.
    #[serde(rename = "boardConnect")]
    BoardConnect {
        from: String,
        to: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        board_id: Option<String>,
    },
    #[serde(rename = "canvasAddNode")]
    CanvasAddNode {
        label: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        kind: Option<String>,
    },
    #[serde(rename = "canvasConnect")]
    CanvasConnect { from: String, to: String },
    #[serde(rename = "canvasDeleteNode")]
    CanvasDeleteNode { node_id: String },
    #[serde(rename = "canvasFitView")]
    CanvasFitView,
    #[serde(rename = "noop")]
    Noop { reason: Option<String> },
}

impl AppAction {
    pub fn risk_class(&self) -> RiskClass {
        match self {
            AppAction::Navigate { .. }
            | AppAction::OpenPanel { .. }
            | AppAction::ClosePanel { .. }
            | AppAction::FocusComposer
            | AppAction::SelectSession { .. }
            | AppAction::OpenSprint { .. }
            | AppAction::OpenNote { .. }
            | AppAction::OpenCanvas { .. }
            | AppAction::OpenBoard { .. }
            | AppAction::CanvasFitView
            | AppAction::Noop { .. } => RiskClass::Navigate,
            AppAction::SetComposer { .. }
            | AppAction::SetMode { .. }
            | AppAction::CanvasAddNode { .. }
            | AppAction::CanvasConnect { .. }
            | AppAction::BoardConnect { .. } => RiskClass::Compose,
            AppAction::CreateNote { .. }
            | AppAction::CreateSprint { .. }
            | AppAction::BoardAddSticky { .. } => RiskClass::Write,
            AppAction::RunRecipe { .. } | AppAction::StopRecipe { .. } => RiskClass::External,
            AppAction::CanvasDeleteNode { .. } => RiskClass::Destructive,
        }
    }

    pub fn confirmation_policy(&self) -> ConfirmationPolicy {
        match self {
            // Board mutations stay soft-gated even when compose-classed.
            AppAction::BoardConnect { .. } => ConfirmationPolicy::Soft,
            _ => match self.risk_class() {
                RiskClass::Read | RiskClass::Navigate | RiskClass::Compose => {
                    ConfirmationPolicy::None
                }
                RiskClass::Write => ConfirmationPolicy::Soft,
                RiskClass::External | RiskClass::Destructive | RiskClass::Privileged => {
                    ConfirmationPolicy::Hard
                }
            },
        }
    }

    pub fn label(&self) -> String {
        match self {
            AppAction::Navigate { route } => format!("Navigate to {route}"),
            AppAction::OpenPanel { panel } => format!("Open panel {panel}"),
            AppAction::ClosePanel { panel } => format!("Close panel {panel}"),
            AppAction::SetComposer { .. } => "Set composer text".into(),
            AppAction::FocusComposer => "Focus composer".into(),
            AppAction::SetMode { mode } => format!("Set mode {mode}"),
            AppAction::SelectSession { .. } => "Select chat session".into(),
            AppAction::CreateNote { .. } => "Create note".into(),
            AppAction::OpenNote { .. } => "Open note".into(),
            AppAction::OpenSprint { .. } => "Open sprint".into(),
            AppAction::CreateSprint { .. } => "Create sprint".into(),
            AppAction::RunRecipe { recipe_id } => format!("Run recipe {recipe_id}"),
            AppAction::StopRecipe { .. } => "Stop recipe".into(),
            AppAction::OpenCanvas { .. } => "Open canvas".into(),
            AppAction::OpenBoard { .. } => "Open board".into(),
            AppAction::BoardAddSticky { text, .. } => format!("Board sticky: {text}"),
            AppAction::BoardConnect { from, to, .. } => {
                format!("Board connect {from} → {to}")
            }
            AppAction::CanvasAddNode { label, .. } => format!("Add canvas node {label}"),
            AppAction::CanvasConnect { from, to } => format!("Connect {from} → {to}"),
            AppAction::CanvasDeleteNode { node_id } => format!("Delete node {node_id}"),
            AppAction::CanvasFitView => "Fit canvas view".into(),
            AppAction::Noop { reason } => reason.clone().unwrap_or_else(|| "No operation".into()),
        }
    }
}

/// Intent envelope produced by Agent Engine tool steps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionIntent {
    pub action: AppAction,
    pub source: ActionSource,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

impl ActionIntent {
    pub fn new(action: AppAction, source: ActionSource) -> Self {
        Self {
            action,
            source,
            turn_id: None,
            rationale: None,
        }
    }

    pub fn with_turn(mut self, turn_id: impl Into<String>) -> Self {
        self.turn_id = Some(turn_id.into());
        self
    }
}

/// Result of applying (or rejecting) an action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActionResult {
    pub ok: bool,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied: Option<AppAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ActionResult {
    pub fn success(summary: impl Into<String>, applied: AppAction) -> Self {
        Self {
            ok: true,
            summary: summary.into(),
            applied: Some(applied),
            error: None,
        }
    }

    pub fn failure(summary: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            ok: false,
            summary: summary.into(),
            applied: None,
            error: Some(error.into()),
        }
    }
}

/// Lightweight app context for `app.get_context`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AppContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_session_id: Option<String>,
    #[serde(default)]
    pub open_panels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canvas_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_is_low_risk() {
        let a = AppAction::Navigate {
            route: "/agent-chat".into(),
        };
        assert_eq!(a.risk_class(), RiskClass::Navigate);
        assert_eq!(a.confirmation_policy(), ConfirmationPolicy::None);
    }

    #[test]
    fn write_needs_soft_confirm() {
        let a = AppAction::CreateNote {
            title: Some("x".into()),
        };
        assert_eq!(a.risk_class(), RiskClass::Write);
        assert_eq!(a.confirmation_policy(), ConfirmationPolicy::Soft);
    }

    #[test]
    fn recipe_is_external_hard() {
        let a = AppAction::RunRecipe {
            recipe_id: "cargo-test".into(),
        };
        assert_eq!(a.risk_class(), RiskClass::External);
        assert_eq!(a.confirmation_policy(), ConfirmationPolicy::Hard);
    }

    #[test]
    fn action_intent_roundtrip_json() {
        let intent = ActionIntent::new(
            AppAction::Navigate {
                route: "/notes".into(),
            },
            ActionSource::Voice,
        )
        .with_turn("t1");
        let json = serde_json::to_string(&intent).unwrap();
        let back: ActionIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, intent);
    }
}
