//! Undo stack for reversible AppActions.

use super::types::AppAction;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UndoFrame {
    pub id: String,
    pub forward: AppAction,
    pub inverse: AppAction,
    pub label: String,
}

static STACK: Mutex<Vec<UndoFrame>> = Mutex::new(Vec::new());

/// Best-effort inverse for reversible actions. Returns `None` if not undoable.
pub fn inverse_for(action: &AppAction) -> Option<AppAction> {
    match action {
        AppAction::Navigate { .. } => None, // navigation undo is session-history, not here
        AppAction::OpenPanel { panel } => Some(AppAction::ClosePanel {
            panel: panel.clone(),
        }),
        AppAction::ClosePanel { panel } => Some(AppAction::OpenPanel {
            panel: panel.clone(),
        }),
        AppAction::SetComposer { .. } => Some(AppAction::SetComposer {
            text: String::new(),
        }),
        AppAction::SetMode { .. } => Some(AppAction::SetMode {
            mode: "ask".into(),
        }),
        AppAction::OpenCanvas { .. } => Some(AppAction::Navigate {
            route: "/agent-chat".into(),
        }),
        AppAction::OpenBoard { .. } => Some(AppAction::Navigate {
            route: "/agent-chat".into(),
        }),
        AppAction::CanvasAddNode { .. } => None, // needs node id from apply result
        AppAction::CanvasConnect { from, to } => {
            // Best-effort: delete is not a connect inverse; leave for FE undo stack.
            let _ = (from, to);
            None
        }
        _ => None,
    }
}

pub fn push_undo(forward: AppAction, label: impl Into<String>) -> Option<UndoFrame> {
    let inverse = inverse_for(&forward)?;
    let frame = UndoFrame {
        id: format!("undo-{}", forward.label().len()),
        forward,
        inverse,
        label: label.into(),
    };
    if let Ok(mut g) = STACK.lock() {
        g.push(frame.clone());
        if g.len() > 40 {
            let drain = g.len() - 40;
            g.drain(0..drain);
        }
    }
    Some(frame)
}

pub fn pop_undo() -> Option<UndoFrame> {
    STACK.lock().ok().and_then(|mut g| g.pop())
}

pub fn peek_undo() -> Option<UndoFrame> {
    STACK.lock().ok().and_then(|g| g.last().cloned())
}

pub fn clear_undo() {
    if let Ok(mut g) = STACK.lock() {
        g.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_open_is_undoable() {
        clear_undo();
        let a = AppAction::OpenPanel {
            panel: "rail".into(),
        };
        assert!(push_undo(a, "Open rail").is_some());
        let frame = pop_undo().expect("frame");
        assert!(matches!(frame.inverse, AppAction::ClosePanel { .. }));
    }
}
