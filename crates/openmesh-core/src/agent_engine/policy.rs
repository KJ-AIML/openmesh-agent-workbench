//! Shared Agent Engine policy seam (v0.2 A3).
//!
//! Every engine turn must present an [`AuthorizedAgentTurn`] minted here.
//! Callers cannot construct that token themselves; they must go through
//! [`authorize_agent_turn`].

use super::registry::tools_for_mode;
use super::types::DEFAULT_MAX_TOOL_ITERATIONS;
use serde::{Deserialize, Serialize};

/// Who initiated this Agent Engine turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentOrigin {
    LocalChat,
    LocalDelegate,
    LocalCLI,
    LanPeer,
    ContinuityQuery,
}

impl AgentOrigin {
    pub fn is_remote(self) -> bool {
        matches!(self, Self::LanPeer | Self::ContinuityQuery)
    }

    pub fn is_live_ask(self) -> bool {
        self.is_remote()
    }
}

/// Local operator vs network/continuity peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AgentScope {
    Local,
    Remote,
}

/// How configured secrets may be used for this turn.
///
/// The engine still needs a provider key to call the model. Remote origins
/// must not export secret material through tools or answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SecretPolicy {
    LocalConfigured,
    RemoteNoExport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequestContext {
    pub origin: AgentOrigin,
    pub project_path: String,
    /// Chat mode (`ask` / `plan` / `act` / `delegate`). Ignored for remote origins.
    pub mode: Option<String>,
}

impl AgentRequestContext {
    pub fn local_chat(project_path: impl Into<String>, mode: Option<&str>) -> Self {
        Self {
            origin: AgentOrigin::LocalChat,
            project_path: project_path.into(),
            mode: mode.map(|m| m.to_string()),
        }
    }

    pub fn local_cli(project_path: impl Into<String>) -> Self {
        Self {
            origin: AgentOrigin::LocalCLI,
            project_path: project_path.into(),
            mode: None,
        }
    }

    pub fn live_ask(origin: AgentOrigin, project_path: impl Into<String>) -> Self {
        Self {
            origin,
            project_path: project_path.into(),
            mode: None,
        }
    }
}

/// Capability token proving this turn passed the shared policy seam.
///
/// The inner fields are private so production callers cannot mint a more
/// permissive token than [`authorize_agent_turn`] allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedAgentTurn {
    origin: AgentOrigin,
    project_path: String,
    scope: AgentScope,
    secret_policy: SecretPolicy,
    tool_allowlist: Vec<String>,
    max_tool_iterations: u32,
}

impl AuthorizedAgentTurn {
    pub fn origin(&self) -> AgentOrigin {
        self.origin
    }

    pub fn project_path(&self) -> &str {
        &self.project_path
    }

    pub fn scope(&self) -> AgentScope {
        self.scope
    }

    pub fn secret_policy(&self) -> SecretPolicy {
        self.secret_policy
    }

    pub fn tool_allowlist(&self) -> &[String] {
        &self.tool_allowlist
    }

    pub fn max_tool_iterations(&self) -> u32 {
        self.max_tool_iterations
    }

    pub fn allows_mutating_tools(&self) -> bool {
        self.tool_allowlist.iter().any(|name| {
            matches!(
                name.as_str(),
                "propose_patch"
                    | "create_handoff_draft"
                    | "update_task"
                    | "link_session"
                    | "canvas_upsert_auto_ui"
            )
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AgentPolicyError {
    #[error("agent policy requires a project path")]
    EmptyProjectPath,
}

pub fn authorize_agent_turn(
    ctx: &AgentRequestContext,
) -> Result<AuthorizedAgentTurn, AgentPolicyError> {
    if ctx.project_path.trim().is_empty() {
        return Err(AgentPolicyError::EmptyProjectPath);
    }

    let scope = if ctx.origin.is_remote() {
        AgentScope::Remote
    } else {
        AgentScope::Local
    };
    let secret_policy = match scope {
        AgentScope::Local => SecretPolicy::LocalConfigured,
        AgentScope::Remote => SecretPolicy::RemoteNoExport,
    };

    let mode = ctx.mode.as_deref().unwrap_or("ask");
    let (tool_allowlist, max_tool_iterations) = match ctx.origin {
        AgentOrigin::LocalChat => {
            let iters = match mode.trim().to_ascii_lowercase().as_str() {
                "plan" | "act" => 5,
                "delegate" => 3,
                _ => 3,
            };
            (tools_for_mode(mode), iters)
        }
        AgentOrigin::LocalDelegate => (tools_for_mode("delegate"), 3),
        AgentOrigin::LocalCLI => (tools_for_mode("ask"), DEFAULT_MAX_TOOL_ITERATIONS),
        AgentOrigin::LanPeer | AgentOrigin::ContinuityQuery => (tools_for_mode("ask"), 3),
    };

    Ok(AuthorizedAgentTurn {
        origin: ctx.origin,
        project_path: ctx.project_path.trim().to_string(),
        scope,
        secret_policy,
        tool_allowlist,
        max_tool_iterations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_project_path_fails_closed() {
        let err = authorize_agent_turn(&AgentRequestContext {
            origin: AgentOrigin::LocalChat,
            project_path: "  ".into(),
            mode: None,
        })
        .unwrap_err();
        assert_eq!(err, AgentPolicyError::EmptyProjectPath);
    }

    #[test]
    fn remote_origins_are_read_only_even_if_act_mode_is_supplied() {
        for origin in [AgentOrigin::LanPeer, AgentOrigin::ContinuityQuery] {
            let auth = authorize_agent_turn(&AgentRequestContext {
                origin,
                project_path: "/tmp/ws".into(),
                mode: Some("act".into()),
            })
            .unwrap();
            assert_eq!(auth.scope(), AgentScope::Remote);
            assert_eq!(auth.secret_policy(), SecretPolicy::RemoteNoExport);
            assert!(!auth.allows_mutating_tools());
            assert!(auth.tool_allowlist().contains(&"read_file".into()));
            assert!(!auth.tool_allowlist().iter().any(|t| t == "propose_patch"));
            assert_eq!(auth.max_tool_iterations(), 3);
        }
    }

    #[test]
    fn local_chat_act_keeps_mutating_propose_tools() {
        let auth =
            authorize_agent_turn(&AgentRequestContext::local_chat("/tmp/ws", Some("act"))).unwrap();
        assert_eq!(auth.origin(), AgentOrigin::LocalChat);
        assert_eq!(auth.scope(), AgentScope::Local);
        assert!(auth.allows_mutating_tools());
        assert!(auth.tool_allowlist().iter().any(|t| t == "propose_patch"));
        assert_eq!(auth.max_tool_iterations(), 5);
    }

    #[test]
    fn local_cli_stays_ask_budget() {
        let auth = authorize_agent_turn(&AgentRequestContext::local_cli("/tmp/ws")).unwrap();
        assert_eq!(auth.origin(), AgentOrigin::LocalCLI);
        assert!(!auth.allows_mutating_tools());
        assert_eq!(auth.max_tool_iterations(), DEFAULT_MAX_TOOL_ITERATIONS);
    }

    #[test]
    fn local_delegate_matches_ask_tools() {
        let auth = authorize_agent_turn(&AgentRequestContext {
            origin: AgentOrigin::LocalDelegate,
            project_path: "/tmp/ws".into(),
            mode: Some("act".into()),
        })
        .unwrap();
        assert_eq!(auth.origin(), AgentOrigin::LocalDelegate);
        assert!(!auth.allows_mutating_tools());
    }
}
