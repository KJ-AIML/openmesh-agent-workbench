use crate::context::Sensitivity;
use serde::{Deserialize, Serialize};

/// Wire-schema version for the Canonical WorkEvent protocol (Dev Track 0.1.3.4).
pub const WORK_EVENT_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for promotion-composed WorkEvents (Dev Track 0.1.3.5 E1).
pub const WORK_EVENT_PROTOCOL_VERSION_PROMOTED: &str = "1.1";

/// Returns true when `version` is a supported on-disk WorkEvent protocol.
pub fn is_supported_work_event_protocol(version: &str) -> bool {
    version == WORK_EVENT_PROTOCOL_VERSION || version == WORK_EVENT_PROTOCOL_VERSION_PROMOTED
}

/// Frozen bound: `event_id` maximum length (approved 0.1.3.4 plan §3.1).
pub const MAX_EVENT_ID_BYTES: usize = 256;

/// Frozen bound: WorkEvent `summary` maximum length (approved 0.1.3.4 plan §3.1).
pub const MAX_EVENT_SUMMARY_BYTES: usize = 4096;

/// Frozen bound: `GitState.repo_id` maximum length (approved 0.1.3.6 plan §3.1).
pub const MAX_GIT_STATE_REPO_ID_BYTES: usize = 32;
/// Frozen bound: `GitState.branch` maximum length.
pub const MAX_GIT_STATE_BRANCH_BYTES: usize = 256;
/// Frozen bound: `GitState.head` — full Git SHA-1 hex length.
pub const MAX_GIT_STATE_HEAD_BYTES: usize = 40;
/// Frozen bound: each repo-relative path in `GitState.changed_paths`.
pub const MAX_GIT_STATE_PATH_BYTES: usize = 512;
/// Frozen bound: number of entries in `GitState.changed_paths`.
pub const MAX_GIT_STATE_CHANGED_PATHS: usize = 64;
/// Frozen bound: optional `GitState.base_ref` length.
pub const MAX_GIT_STATE_BASE_REF_BYTES: usize = 256;
/// Frozen bound: optional `GitState.worktree_root` length.
pub const MAX_GIT_STATE_WORKTREE_ROOT_BYTES: usize = 1024;

/// Which system/integration emitted a WorkSignal — for later dedup/correlation
/// (Classification Pack cases WEC-26/WEC-32). Distinct from `ActorRef`: this is
/// about the producer, not who the claim/action belongs to.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum ProducerRef {
    /// OpenMesh's own native producer (e.g. manual checkpoints, snapshot creation).
    Native,
    /// Heli, read independently as an evidence producer.
    Heli,
    /// Local Git evidence (owned by 0.1.3.6 — this variant exists so ProducerRef
    /// can already be typed against it; no Git-specific producer logic runs yet).
    Git,
    /// An external agent Reporter Skill, identified by agent/tool name.
    Reporter(String),
}

/// Who a WorkSignal's claim or action is attributed to — distinct from
/// `ProducerRef` (which system emitted it). A bare identity discriminant only;
/// not the "Person and Proxy Identity" / role / responsibility profile system,
/// which is 0.1.4's job ("My Work Proxy Profile"). No authority logic here.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum ActorRef {
    Person(String),
    Device(String),
    Proxy(String),
    Unknown,
}

/// Pointer-only Git evidence — metadata about local repository state, never full
/// source code or patch bodies (approved 0.1.3.6 plan §3.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitState {
    pub repo_id: String,
    pub branch: String,
    pub head: String,
    pub dirty: bool,
    pub staged_count: u32,
    pub unstaged_count: u32,
    pub untracked_count: u32,
    pub changed_paths: Vec<String>,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ahead: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behind: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree_root: Option<String>,
}

/// Pure producer contract — local Git snapshot before WorkSignal composition (Checkpoint B).
pub type GitSnapshot = GitState;

/// Pure producer contract — bounded Heli harness state excerpt (Checkpoint C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeliSnapshot {
    pub current_task_excerpt: Option<String>,
    pub decisions_tail_excerpt: Option<String>,
    pub latest_report_path: Option<String>,
    pub observed_at: String,
}

/// Why a producer chose not to emit a WorkSignal (no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerSkipReason {
    HeliAbsent,
    GitNotRepository,
    GitUnavailable,
}

/// Git producer failure classification (no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitProducerError {
    GitNotAvailable,
    NotARepository,
    ReadFailed(String),
}

/// Heli producer failure classification (no I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeliProducerError {
    ReadFailed(String),
}

/// Pure producer result envelope for Git (Checkpoint B).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitProducerResult {
    Snapshot(GitSnapshot),
    Skip(ProducerSkipReason),
    Err(GitProducerError),
}

/// Pure producer result envelope for Heli (Checkpoint C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeliProducerResult {
    Snapshot(HeliSnapshot),
    Skip(ProducerSkipReason),
    Err(HeliProducerError),
}

/// Why OpenMesh believes a claim/event — a reference, not the evidence content
/// itself. Deliberately `#[non_exhaustive]` so a future Git-ref variant (needed
/// by 0.1.3.6, Classification Pack case WEC-33) can be added without a breaking
/// change to any downstream consumer of this crate.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "kebab-case")]
pub enum EvidenceRef {
    /// A file/path reference (e.g. a relative path within a project).
    FilePath(String),
    /// A reference to another WorkSignal by its `signal_id` (corroboration).
    ProducerSignal(String),
    /// Bounded local Git repository snapshot metadata (WEC-33, Dev Track 0.1.3.6).
    GitState(GitState),
}

/// Deterministically bound `changed_paths` to the frozen maximum.
pub fn bound_git_changed_paths(mut paths: Vec<String>) -> Vec<String> {
    paths.sort();
    paths.truncate(MAX_GIT_STATE_CHANGED_PATHS);
    paths
}

pub(crate) fn validate_repo_relative_path(path: &str, label: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err(format!("{label} is empty"));
    }
    if path.len() > MAX_GIT_STATE_PATH_BYTES {
        return Err(format!("{label} exceeds {MAX_GIT_STATE_PATH_BYTES} bytes"));
    }
    if path.contains('\\') {
        return Err(format!("{label} must use forward slashes"));
    }
    if path.starts_with('/') || path.starts_with("../") || path.contains("/../") {
        return Err(format!("{label} must be repo-relative"));
    }
    Ok(())
}

/// Evidence pointer plus optional observation metadata for a canonical WorkEvent.
///
/// `evidence_ref` is where the fact lives; `observed_at` is when/how OpenMesh
/// (or a producer via append) came to know it. These are intentionally not
/// collapsed (Dev Spec 0.1.3.4 / Classification Pack model pressure).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceAttachment {
    pub evidence_ref: EvidenceRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<String>,
}

/// A durable, evidence-backed meaningful transition. `evidence` is a **list**,
/// not a single reference, so that many signals may support one WorkEvent
/// (Classification Pack case CC-1) without forcing a cardinality.
///
/// `actor` is required on wire for `protocolVersion = "1.1"` promoted events and
/// absent for legacy `1.0` records (Dev Track 0.1.3.5 Checkpoint E1).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkEvent {
    pub event_id: String,
    pub workspace_id: String,
    pub kind: String,
    pub summary: String,
    pub timestamp: String,
    pub evidence: Vec<EvidenceAttachment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrects_event_id: Option<String>,
    pub sensitivity: Sensitivity,
    pub protocol_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<ActorRef>,
}

impl WorkEvent {
    pub fn new(
        event_id: impl Into<String>,
        workspace_id: impl Into<String>,
        kind: impl Into<String>,
        summary: impl Into<String>,
        evidence: Vec<EvidenceAttachment>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            workspace_id: workspace_id.into(),
            kind: kind.into(),
            summary: summary.into(),
            timestamp: timestamp.into(),
            evidence,
            corrects_event_id: None,
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_EVENT_PROTOCOL_VERSION.to_string(),
            actor: None,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EventValidationError {
    #[error("event_id is empty after trim")]
    EmptyEventId,
    #[error("event_id exceeds the {max}-byte bound")]
    EventIdTooLong { max: usize },
    #[error("event_id contains a control character")]
    EventIdControlChar,
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("kind is empty after trim")]
    EmptyKind,
    #[error("summary is empty after trim")]
    EmptySummary,
    #[error("summary exceeds the {max}-byte bound")]
    SummaryTooLong { max: usize },
    #[error("timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("evidence must not be empty for a canonical WorkEvent")]
    EmptyEvidence,
    #[error("unsupported protocol_version {found}; accepted versions are 1.0 and 1.1")]
    UnsupportedProtocolVersion { found: String },
    #[error("protocol_version 1.1 requires actor")]
    MissingActorOnPromotedEvent,
    #[error("protocol_version 1.0 must not include actor")]
    ActorNotAllowedOnLegacyProtocol,
    #[error("corrects_event_id is empty after trim")]
    EmptyCorrectsEventId,
    #[error("corrects_event_id exceeds the {max}-byte bound")]
    CorrectsEventIdTooLong { max: usize },
    #[error("corrects_event_id contains a control character")]
    CorrectsEventIdControlChar,
    #[error("evidence observed_at is invalid: {0}")]
    InvalidObservedAt(String),
}

/// Shared semantic validation for WorkEvent records. Used by the future ledger
/// append path (Checkpoint B) and classification (Checkpoint C).
pub fn validate_event_semantics(event: &WorkEvent) -> Result<(), EventValidationError> {
    validate_id_field(&event.event_id, IdField::EventId)?;
    if event.workspace_id.trim().is_empty() {
        return Err(EventValidationError::EmptyWorkspaceId);
    }
    if event.kind.trim().is_empty() {
        return Err(EventValidationError::EmptyKind);
    }
    if event.summary.trim().is_empty() {
        return Err(EventValidationError::EmptySummary);
    }
    if event.summary.len() > MAX_EVENT_SUMMARY_BYTES {
        return Err(EventValidationError::SummaryTooLong {
            max: MAX_EVENT_SUMMARY_BYTES,
        });
    }
    validate_utc_timestamp(&event.timestamp).map_err(EventValidationError::InvalidTimestamp)?;
    if event.evidence.is_empty() {
        return Err(EventValidationError::EmptyEvidence);
    }
    for attachment in &event.evidence {
        if let Some(observed_at) = &attachment.observed_at {
            validate_utc_timestamp(observed_at).map_err(EventValidationError::InvalidObservedAt)?;
        }
    }
    if event.protocol_version == WORK_EVENT_PROTOCOL_VERSION {
        if event.actor.is_some() {
            return Err(EventValidationError::ActorNotAllowedOnLegacyProtocol);
        }
    } else if event.protocol_version == WORK_EVENT_PROTOCOL_VERSION_PROMOTED {
        if event.actor.is_none() {
            return Err(EventValidationError::MissingActorOnPromotedEvent);
        }
    } else {
        return Err(EventValidationError::UnsupportedProtocolVersion {
            found: event.protocol_version.clone(),
        });
    }
    if let Some(corrects) = &event.corrects_event_id {
        validate_id_field(corrects, IdField::CorrectsEventId)?;
    }
    Ok(())
}

enum IdField {
    EventId,
    CorrectsEventId,
}

fn validate_id_field(value: &str, field: IdField) -> Result<(), EventValidationError> {
    if value.trim().is_empty() {
        return Err(match field {
            IdField::EventId => EventValidationError::EmptyEventId,
            IdField::CorrectsEventId => EventValidationError::EmptyCorrectsEventId,
        });
    }
    if value.len() > MAX_EVENT_ID_BYTES {
        return Err(match field {
            IdField::EventId => EventValidationError::EventIdTooLong {
                max: MAX_EVENT_ID_BYTES,
            },
            IdField::CorrectsEventId => EventValidationError::CorrectsEventIdTooLong {
                max: MAX_EVENT_ID_BYTES,
            },
        });
    }
    if value.chars().any(|c| c.is_control()) {
        return Err(match field {
            IdField::EventId => EventValidationError::EventIdControlChar,
            IdField::CorrectsEventId => EventValidationError::CorrectsEventIdControlChar,
        });
    }
    Ok(())
}

/// RFC 3339 UTC only (`Z` or `+00:00`; reject `-00:00` and non-UTC offsets).
pub fn validate_utc_timestamp(timestamp: &str) -> Result<(), String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map_err(|_| format!("timestamp is not valid RFC 3339: {timestamp}"))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!("timestamp offset must be UTC: {timestamp}"));
    }
    if timestamp.trim_end().ends_with("-00:00") {
        return Err(format!(
            "timestamp offset -00:00 is not an approved UTC representation (only Z and +00:00 are): {timestamp}"
        ));
    }
    Ok(())
}
