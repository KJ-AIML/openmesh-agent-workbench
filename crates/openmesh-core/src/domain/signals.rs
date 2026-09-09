use super::events::{
    validate_repo_relative_path, validate_utc_timestamp, ActorRef, EvidenceRef, GitState,
    ProducerRef, MAX_EVENT_ID_BYTES, MAX_GIT_STATE_BASE_REF_BYTES, MAX_GIT_STATE_BRANCH_BYTES,
    MAX_GIT_STATE_CHANGED_PATHS, MAX_GIT_STATE_HEAD_BYTES, MAX_GIT_STATE_REPO_ID_BYTES,
    MAX_GIT_STATE_WORKTREE_ROOT_BYTES,
};
use crate::context::Sensitivity;
use serde::{Deserialize, Serialize};

/// Wire-schema version for the Work Signal Protocol (Dev Track 0.1.3.2). Any
/// wire-incompatible evolution (including a new enum variant on WorkSignalKind,
/// ProducerRef, ActorRef, EvidenceRef, or Sensitivity) must bump this constant —
/// see the approved 0.1.3.2 execution plan §3.10/§10 for the compatibility rule.
pub const WORK_SIGNAL_PROTOCOL_VERSION: &str = "1.0";

/// WorkSignal wire schema when `EvidenceRef::GitState` is present (Dev Track 0.1.3.6).
pub const WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE: &str = "1.1";

/// Returns true when `version` is a supported on-disk WorkSignal protocol.
pub fn is_supported_work_signal_protocol(version: &str) -> bool {
    version == WORK_SIGNAL_PROTOCOL_VERSION
        || version == WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SignalValidationError {
    #[error("unsupported protocol_version {found}; accepted versions are 1.0 and 1.1")]
    UnsupportedProtocolVersion { found: String },
    #[error("protocol_version 1.0 must not include git-state evidence")]
    Protocol10WithGitState,
    #[error("invalid evidence: {0}")]
    InvalidEvidence(String),
    #[error("invalid git-state evidence: {0}")]
    InvalidGitState(String),
}

/// Returns true when `signal` carries any `EvidenceRef::GitState` attachment.
pub fn signal_has_git_state_evidence(signal: &WorkSignal) -> bool {
    signal
        .evidence_refs
        .iter()
        .any(|ev| matches!(ev, EvidenceRef::GitState(_)))
}

/// Validate a `GitState` evidence payload.
pub fn validate_git_state(state: &GitState) -> Result<(), SignalValidationError> {
    if state.repo_id.trim().is_empty() {
        return Err(SignalValidationError::InvalidGitState(
            "repo_id is empty".into(),
        ));
    }
    if state.repo_id.len() > MAX_GIT_STATE_REPO_ID_BYTES {
        return Err(SignalValidationError::InvalidGitState(format!(
            "repo_id exceeds {MAX_GIT_STATE_REPO_ID_BYTES} bytes"
        )));
    }
    if !state.repo_id.starts_with("fnv1a-") {
        return Err(SignalValidationError::InvalidGitState(
            "repo_id must start with fnv1a-".into(),
        ));
    }
    if !state.repo_id[6..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(SignalValidationError::InvalidGitState(
            "repo_id suffix must be lowercase hex".into(),
        ));
    }

    if state.branch.len() > MAX_GIT_STATE_BRANCH_BYTES {
        return Err(SignalValidationError::InvalidGitState(format!(
            "branch exceeds {MAX_GIT_STATE_BRANCH_BYTES} bytes"
        )));
    }

    if state.head.len() != MAX_GIT_STATE_HEAD_BYTES {
        return Err(SignalValidationError::InvalidGitState(format!(
            "head must be exactly {MAX_GIT_STATE_HEAD_BYTES} hex characters"
        )));
    }
    if !state.head.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(SignalValidationError::InvalidGitState(
            "head must be hexadecimal".into(),
        ));
    }

    if state.changed_paths.len() > MAX_GIT_STATE_CHANGED_PATHS {
        return Err(SignalValidationError::InvalidGitState(format!(
            "changed_paths exceeds {MAX_GIT_STATE_CHANGED_PATHS} entries"
        )));
    }
    for path in &state.changed_paths {
        validate_repo_relative_path(path, "changed_paths entry").map_err(|msg| {
            SignalValidationError::InvalidGitState(format!("changed_paths: {msg}"))
        })?;
    }

    validate_utc_timestamp(&state.observed_at)
        .map_err(|msg| SignalValidationError::InvalidGitState(format!("observed_at: {msg}")))?;

    if let Some(base_ref) = &state.base_ref {
        if base_ref.len() > MAX_GIT_STATE_BASE_REF_BYTES {
            return Err(SignalValidationError::InvalidGitState(format!(
                "base_ref exceeds {MAX_GIT_STATE_BASE_REF_BYTES} bytes"
            )));
        }
    }
    if let Some(root) = &state.worktree_root {
        if root.len() > MAX_GIT_STATE_WORKTREE_ROOT_BYTES {
            return Err(SignalValidationError::InvalidGitState(format!(
                "worktree_root exceeds {MAX_GIT_STATE_WORKTREE_ROOT_BYTES} bytes"
            )));
        }
    }

    Ok(())
}

/// Validate one `EvidenceRef`, including nested `GitState` when present.
pub fn validate_evidence_ref(evidence: &EvidenceRef) -> Result<(), SignalValidationError> {
    match evidence {
        EvidenceRef::FilePath(path) => validate_repo_relative_path(path, "file-path")
            .map_err(|msg| SignalValidationError::InvalidEvidence(format!("file-path: {msg}"))),
        EvidenceRef::ProducerSignal(id) => {
            if id.trim().is_empty() {
                return Err(SignalValidationError::InvalidEvidence(
                    "producer-signal id is empty".into(),
                ));
            }
            if id.len() > MAX_EVENT_ID_BYTES {
                return Err(SignalValidationError::InvalidEvidence(format!(
                    "producer-signal id exceeds {MAX_EVENT_ID_BYTES} bytes"
                )));
            }
            Ok(())
        }
        EvidenceRef::GitState(state) => validate_git_state(state),
    }
}

/// Validate WorkSignal protocol/evidence compatibility and every evidence ref.
pub fn validate_work_signal_semantics(signal: &WorkSignal) -> Result<(), SignalValidationError> {
    validate_work_signal_protocol_evidence_compatibility(signal)?;
    for evidence in &signal.evidence_refs {
        validate_evidence_ref(evidence)?;
    }
    Ok(())
}

fn validate_work_signal_protocol_evidence_compatibility(
    signal: &WorkSignal,
) -> Result<(), SignalValidationError> {
    let has_git_state = signal_has_git_state_evidence(signal);
    match signal.protocol_version.as_str() {
        WORK_SIGNAL_PROTOCOL_VERSION => {
            if has_git_state {
                Err(SignalValidationError::Protocol10WithGitState)
            } else {
                Ok(())
            }
        }
        WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE => {
            if has_git_state {
                Ok(())
            } else {
                // 1.1 without git-state is allowed for forward compatibility.
                Ok(())
            }
        }
        found => Err(SignalValidationError::UnsupportedProtocolVersion {
            found: found.to_string(),
        }),
    }
}

/// WorkSignal's semantic kind. Fixed to the categories already settled in
/// Development Spec v1.6 Decision 1 / the Continuity Runtime Architecture §6 —
/// not an open/extensible list, since that list is already canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkSignalKind {
    Progress,
    Decision,
    Blocker,
    BlockerResolved,
    ScopeChange,
    Milestone,
    ReviewRequired,
    UnresolvedQuestion,
    Handoff,
    SessionEnd,
    AgentSwitch,
}

/// An unpromoted claim/observation entering the continuity domain. Not yet
/// durable truth — only the Deterministic Continuity Pipeline (0.1.3.5, not
/// implemented here) decides whether this becomes a WorkEvent.
///
/// `producer` identifies which system/integration emitted this signal.
/// `actor` identifies whose claim or action this represents (an agent, a
/// human, or unknown) — a distinct field, not folded into `producer`. This
/// distinction is what lets an agent's completion claim, a verification
/// result, and a human's acceptance exist as separately-attributed records
/// (Classification Pack case WEC-29) instead of being inferred from evidence
/// presence alone.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkSignal {
    pub signal_id: String,
    pub workspace_id: String,
    pub producer: ProducerRef,
    pub actor: ActorRef,
    pub kind: WorkSignalKind,
    pub summary: String,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    /// Missing on read defaults to `Sensitivity::Private` via `#[serde(default)]`
    /// — the enum's own `#[default]` attribute alone does not apply to a missing
    /// *struct field*; this attribute is what actually wires it in (approved plan §3.9).
    #[serde(default)]
    pub sensitivity: Sensitivity,
    pub protocol_version: String,
}
