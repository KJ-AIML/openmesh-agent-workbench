//! Policy-aware AppAction dispatcher (core side).
//!
//! Navigation / compose intents are approved automatically.
//! Write / destructive / external intents require confirmation tickets (P3).

use super::audit::record_result;
use super::history::push_undo;
use super::policy::{may_dispatch, AgentMode};
use super::types::{ActionIntent, ActionResult, AppAction};

/// Host-side applicator (Desktop Vue / CLI stub).
pub trait AppActionHandler {
    fn apply(&mut self, action: &AppAction) -> Result<String, String>;
}

/// Dispatch an intent through risk policy, then apply via handler.
pub fn dispatch_intent<H: AppActionHandler>(
    intent: &ActionIntent,
    handler: &mut H,
    confirm_write: bool,
) -> ActionResult {
    dispatch_intent_in_mode(intent, handler, confirm_write, AgentMode::Ask)
}

pub fn dispatch_intent_in_mode<H: AppActionHandler>(
    intent: &ActionIntent,
    handler: &mut H,
    confirmed: bool,
    mode: AgentMode,
) -> ActionResult {
    if let Err(err) = may_dispatch(mode, &intent.action, confirmed) {
        let result = ActionResult::failure(
            format!("Blocked: {}", intent.action.label()),
            err,
        );
        record_result(intent, &result, None);
        return result;
    }

    let result = match handler.apply(&intent.action) {
        Ok(summary) => {
            let undo = push_undo(intent.action.clone(), &summary);
            let applied = ActionResult::success(summary, intent.action.clone());
            record_result(
                intent,
                &applied,
                undo.map(|u| u.id),
            );
            applied
        }
        Err(err) => {
            let failed = ActionResult::failure(intent.action.label(), err);
            record_result(intent, &failed, None);
            failed
        }
    };
    result
}

/// In-memory handler for unit tests / dry-run.
#[derive(Debug, Default)]
pub struct RecordingHandler {
    pub applied: Vec<AppAction>,
}

impl AppActionHandler for RecordingHandler {
    fn apply(&mut self, action: &AppAction) -> Result<String, String> {
        self.applied.push(action.clone());
        Ok(action.label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_actions::types::ActionSource;

    #[test]
    fn navigate_dispatches_without_confirm() {
        let intent = ActionIntent::new(
            AppAction::Navigate {
                route: "/notes".into(),
            },
            ActionSource::Voice,
        );
        let mut h = RecordingHandler::default();
        let result = dispatch_intent(&intent, &mut h, false);
        assert!(result.ok);
        assert_eq!(h.applied.len(), 1);
    }

    #[test]
    fn write_blocked_without_confirm() {
        let intent = ActionIntent::new(
            AppAction::CreateNote {
                title: Some("x".into()),
            },
            ActionSource::Chat,
        );
        let mut h = RecordingHandler::default();
        let result = dispatch_intent(&intent, &mut h, false);
        assert!(!result.ok);
        assert!(h.applied.is_empty());
    }

    #[test]
    fn write_allowed_with_confirm() {
        let intent = ActionIntent::new(
            AppAction::RunRecipe {
                recipe_id: "r1".into(),
            },
            ActionSource::Chat,
        );
        let mut h = RecordingHandler::default();
        let result = dispatch_intent(&intent, &mut h, true);
        assert!(result.ok);
    }
}
