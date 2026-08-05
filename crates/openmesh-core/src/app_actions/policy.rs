//! Mode × risk capability matrix for AppActions.

use super::types::{AppAction, ConfirmationPolicy, RiskClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentMode {
    Ask,
    Plan,
    Act,
    Delegate,
}

impl AgentMode {
    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "plan" => AgentMode::Plan,
            "act" => AgentMode::Act,
            "delegate" => AgentMode::Delegate,
            _ => AgentMode::Ask,
        }
    }
}

/// Whether this action may be attempted in the given mode (before confirm).
/// Write/external can still be *queued* for confirmation from any mode.
pub fn mode_allows(mode: AgentMode, action: &AppAction) -> bool {
    let risk = action.risk_class();
    match mode {
        AgentMode::Ask | AgentMode::Delegate => risk <= RiskClass::External,
        AgentMode::Plan => risk <= RiskClass::External,
        AgentMode::Act => risk <= RiskClass::Privileged,
    }
}

/// Whether dispatch may proceed given mode + confirm ticket.
pub fn may_dispatch(mode: AgentMode, action: &AppAction, confirmed: bool) -> Result<(), String> {
    if !mode_allows(mode, action) {
        return Err(format!(
            "mode {:?} cannot run {}",
            mode,
            action.label()
        ));
    }
    let policy = action.confirmation_policy();
    match policy {
        ConfirmationPolicy::None => Ok(()),
        ConfirmationPolicy::Soft | ConfirmationPolicy::Hard => {
            if confirmed {
                Ok(())
            } else {
                Err("confirmation_required".into())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ask_blocks_write_without_confirm() {
        let a = AppAction::CreateNote {
            title: Some("n".into()),
        };
        assert!(may_dispatch(AgentMode::Ask, &a, false).is_err());
        assert!(may_dispatch(AgentMode::Ask, &a, true).is_ok());
    }

    #[test]
    fn ask_allows_navigate() {
        let a = AppAction::Navigate {
            route: "/notes".into(),
        };
        assert!(may_dispatch(AgentMode::Ask, &a, false).is_ok());
    }

    #[test]
    fn recipe_needs_hard_confirm() {
        let a = AppAction::RunRecipe {
            recipe_id: "x".into(),
        };
        assert!(may_dispatch(AgentMode::Act, &a, false).is_err());
        assert!(may_dispatch(AgentMode::Act, &a, true).is_ok());
    }
}
