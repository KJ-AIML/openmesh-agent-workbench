use super::corrections::ContinuityConfidence;
use super::events::{validate_utc_timestamp, EvidenceRef};
use super::signals::validate_evidence_ref;
use serde::{Deserialize, Serialize};

// ============================================================================
// Dev Track 0.1.3.7 Checkpoint A — Current State & Catch-up domain contracts
// ============================================================================

/// Wire-schema version for persisted Current State projections.
pub const CURRENT_STATE_PROJECTION_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for on-demand Catch-up views.
pub const CATCH_UP_VIEW_PROTOCOL_VERSION: &str = "1.0";

pub const MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES: usize = 512;
pub const MAX_CONTINUITY_ITEM_EVIDENCE_REFS: usize = 8;
pub const MAX_PROJECTION_EVIDENCE_REFS: usize = 64;
pub const MAX_CATCH_UP_EVIDENCE_REFS: usize = 64;
pub const MAX_PROJECTION_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 512;
pub const MAX_CATCH_UP_SUMMARY_BYTES: usize = 1024;
pub const MAX_NEXT_SUGGESTED_ATTENTION: usize = 5;
pub const MAX_REBUILD_INPUTS_HASH_BYTES: usize = 64;
pub const MIN_PENDING_ATTENTION_PRIORITY: u8 = 1;
pub const MAX_PENDING_ATTENTION_PRIORITY: u8 = 5;

/// Returns true when `version` is a supported Current State projection protocol.
pub fn is_supported_current_state_projection_protocol(version: &str) -> bool {
    version == CURRENT_STATE_PROJECTION_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Catch-up view protocol.
pub fn is_supported_catch_up_view_protocol(version: &str) -> bool {
    version == CATCH_UP_VIEW_PROTOCOL_VERSION
}

/// Which durable record a continuity item was derived from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuitySourceKind {
    WorkEvent,
    ProcessedSignal,
    PendingSignal,
    PromotionAudit,
}

/// Why an item appears in Pending Attention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingAttentionReason {
    PendingSignal,
    ReviewRequired,
    Blocker,
    UnresolvedQuestion,
    AmbiguousPromotion,
    SuppressedPromotion,
}

/// Counts of read-only inputs used to build a projection or catch-up view.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceCounts {
    pub work_events: u32,
    pub processed_signals: u32,
    pub pending_signals: u32,
    pub promotion_audit_records: u32,
    pub quarantine_signals: u32,
    pub duplicate_signals: u32,
    pub reporter_signals: u32,
    pub git_signals: u32,
    pub heli_signals: u32,
    pub unknown_producer_signals: u32,
    pub other_producer_signals: u32,
}

/// One evidence-backed continuity item in a Current State or Catch-up section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinuityStateItem {
    pub id: String,
    pub summary: String,
    pub kind: String,
    pub source: ContinuitySourceKind,
    pub source_id: String,
    pub producer: String,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub confidence: ContinuityConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unverified: Option<bool>,
}

/// Fixed Current State sections (Product Bible §7.5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrentStateSections {
    pub completed: Vec<ContinuityStateItem>,
    pub in_progress: Vec<ContinuityStateItem>,
    pub blocked: Vec<ContinuityStateItem>,
    pub decisions: Vec<ContinuityStateItem>,
    pub needs_attention: Vec<ContinuityStateItem>,
    pub still_open: Vec<ContinuityStateItem>,
}

/// Rebuildable view of where work stands now (Runtime Architecture §19).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CurrentStateProjection {
    pub workspace_id: String,
    pub generated_at: String,
    pub protocol_version: String,
    pub sections: CurrentStateSections,
    pub pending_attention: Vec<PendingAttentionItem>,
    pub source_counts: SourceCounts,
    pub evidence_refs: Vec<EvidenceRef>,
    pub limitations: Vec<String>,
    pub rebuild_inputs_hash: String,
}

/// Lifecycle state for a pending-attention item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingAttentionStatus {
    Open,
    Acknowledged,
    Resolved,
    Deferred,
}

/// Urgency for a pending-attention item — distinct from sort `priority`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingAttentionSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Work that currently needs a person (Product Bible §7.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingAttentionItem {
    pub id: String,
    pub summary: String,
    pub reason: PendingAttentionReason,
    pub source: ContinuitySourceKind,
    pub source_id: String,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub status: PendingAttentionStatus,
    pub severity: PendingAttentionSeverity,
    pub priority: u8,
}

/// Recommended sort priority for a severity level (1 = highest urgency).
pub fn pending_attention_priority_for_severity(severity: PendingAttentionSeverity) -> u8 {
    match severity {
        PendingAttentionSeverity::Critical => 1,
        PendingAttentionSeverity::High => 2,
        PendingAttentionSeverity::Medium => 3,
        PendingAttentionSeverity::Low => 4,
    }
}

/// Catch-up time window bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatchUpWindow {
    pub since: String,
    pub until: String,
}

/// Fixed Catch-up sections (Development Spec v1.6 §0.1.3.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatchUpSections {
    pub completed: Vec<ContinuityStateItem>,
    pub changed: Vec<ContinuityStateItem>,
    pub blocked: Vec<ContinuityStateItem>,
    pub decided: Vec<ContinuityStateItem>,
    pub needs_attention: Vec<ContinuityStateItem>,
    pub still_open: Vec<ContinuityStateItem>,
}

/// On-demand local catch-up view — not canonical storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatchUpView {
    pub workspace_id: String,
    pub generated_at: String,
    pub protocol_version: String,
    pub window: CatchUpWindow,
    pub sections: CatchUpSections,
    pub summary: String,
    pub next_suggested_attention: Vec<PendingAttentionItem>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContinuityValidationError {
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("invalid source_counts: {0}")]
    InvalidSourceCounts(String),
    #[error("invalid continuity item: {0}")]
    InvalidContinuityItem(String),
    #[error("invalid pending attention item: {0}")]
    InvalidPendingAttentionItem(String),
    #[error("invalid catch-up window: {0}")]
    InvalidCatchUpWindow(String),
    #[error("rebuild_inputs_hash is invalid: {0}")]
    InvalidRebuildInputsHash(String),
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
    #[error("evidence_refs exceed the {max}-entry bound")]
    TooManyEvidenceRefs { max: usize },
    #[error("summary exceeds the {max}-byte bound")]
    SummaryTooLong { max: usize },
    #[error("next_suggested_attention exceeds the {max}-entry bound")]
    TooManyNextSuggestedAttention { max: usize },
    #[error("catch-up window since must be <= until")]
    CatchUpWindowInverted,
}

/// Validate a `ContinuityStateItem` wire record.
pub fn validate_continuity_state_item(
    item: &ContinuityStateItem,
) -> Result<(), ContinuityValidationError> {
    if item.id.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "id is empty".into(),
        ));
    }
    validate_continuity_item_id(&item.id, item.source)?;
    if item.summary.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "summary is empty".into(),
        ));
    }
    if item.summary.len() > MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES {
        return Err(ContinuityValidationError::SummaryTooLong {
            max: MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES,
        });
    }
    if item.kind.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "kind is empty".into(),
        ));
    }
    if item.source_id.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "source_id is empty".into(),
        ));
    }
    if item.producer.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "producer is empty".into(),
        ));
    }
    validate_utc_timestamp(&item.timestamp).map_err(ContinuityValidationError::InvalidTimestamp)?;
    if item.evidence_refs.len() > MAX_CONTINUITY_ITEM_EVIDENCE_REFS {
        return Err(ContinuityValidationError::TooManyEvidenceRefs {
            max: MAX_CONTINUITY_ITEM_EVIDENCE_REFS,
        });
    }
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContinuityValidationError::InvalidContinuityItem(format!("evidence_refs: {err}"))
        })?;
    }
    if item.unverified == Some(true) && item.source != ContinuitySourceKind::PendingSignal {
        return Err(ContinuityValidationError::InvalidContinuityItem(
            "unverified may be true only for pending-signal items".into(),
        ));
    }
    if let Some(hint) = &item.correlation_hint {
        if hint.trim().is_empty() {
            return Err(ContinuityValidationError::InvalidContinuityItem(
                "correlation_hint is empty".into(),
            ));
        }
    }
    Ok(())
}

/// Validate producer-level and bucket-level `SourceCounts`.
pub fn validate_source_counts(counts: &SourceCounts) -> Result<(), ContinuityValidationError> {
    let producer_total = counts.reporter_signals as u64
        + counts.git_signals as u64
        + counts.heli_signals as u64
        + counts.unknown_producer_signals as u64
        + counts.other_producer_signals as u64;
    let signal_total = counts.processed_signals as u64 + counts.pending_signals as u64;
    if producer_total > signal_total {
        return Err(ContinuityValidationError::InvalidSourceCounts(
            "producer signal breakdown exceeds processed + pending signal count".into(),
        ));
    }
    Ok(())
}

/// Validate a `PendingAttentionItem` wire record.
pub fn validate_pending_attention_item(
    item: &PendingAttentionItem,
) -> Result<(), ContinuityValidationError> {
    if item.id.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidPendingAttentionItem(
            "id is empty".into(),
        ));
    }
    if item.summary.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidPendingAttentionItem(
            "summary is empty".into(),
        ));
    }
    if item.summary.len() > MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES {
        return Err(ContinuityValidationError::SummaryTooLong {
            max: MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES,
        });
    }
    if item.source_id.trim().is_empty() {
        return Err(ContinuityValidationError::InvalidPendingAttentionItem(
            "source_id is empty".into(),
        ));
    }
    validate_utc_timestamp(&item.timestamp).map_err(ContinuityValidationError::InvalidTimestamp)?;
    if item.evidence_refs.len() > MAX_CONTINUITY_ITEM_EVIDENCE_REFS {
        return Err(ContinuityValidationError::TooManyEvidenceRefs {
            max: MAX_CONTINUITY_ITEM_EVIDENCE_REFS,
        });
    }
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContinuityValidationError::InvalidPendingAttentionItem(format!("evidence_refs: {err}"))
        })?;
    }
    if item.priority < MIN_PENDING_ATTENTION_PRIORITY
        || item.priority > MAX_PENDING_ATTENTION_PRIORITY
    {
        return Err(ContinuityValidationError::InvalidPendingAttentionItem(
            format!(
                "priority must be between {MIN_PENDING_ATTENTION_PRIORITY} and {MAX_PENDING_ATTENTION_PRIORITY}"
            ),
        ));
    }
    Ok(())
}

/// Validate a persisted `CurrentStateProjection`.
pub fn validate_current_state_projection(
    projection: &CurrentStateProjection,
) -> Result<(), ContinuityValidationError> {
    if projection.workspace_id.trim().is_empty() {
        return Err(ContinuityValidationError::EmptyWorkspaceId);
    }
    validate_utc_timestamp(&projection.generated_at)
        .map_err(ContinuityValidationError::InvalidTimestamp)?;
    if !is_supported_current_state_projection_protocol(&projection.protocol_version) {
        return Err(ContinuityValidationError::UnsupportedProtocolVersion {
            found: projection.protocol_version.clone(),
            expected: CURRENT_STATE_PROJECTION_PROTOCOL_VERSION,
        });
    }
    validate_current_state_sections(&projection.sections)?;
    for item in &projection.pending_attention {
        validate_pending_attention_item(item)?;
    }
    validate_source_counts(&projection.source_counts)?;
    if projection.evidence_refs.len() > MAX_PROJECTION_EVIDENCE_REFS {
        return Err(ContinuityValidationError::TooManyEvidenceRefs {
            max: MAX_PROJECTION_EVIDENCE_REFS,
        });
    }
    for evidence in &projection.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContinuityValidationError::InvalidContinuityItem(format!("evidence_refs: {err}"))
        })?;
    }
    validate_limitations(&projection.limitations)?;
    validate_rebuild_inputs_hash(&projection.rebuild_inputs_hash)?;
    Ok(())
}

/// Validate an on-demand `CatchUpView`.
pub fn validate_catch_up_view(view: &CatchUpView) -> Result<(), ContinuityValidationError> {
    if view.workspace_id.trim().is_empty() {
        return Err(ContinuityValidationError::EmptyWorkspaceId);
    }
    validate_utc_timestamp(&view.generated_at)
        .map_err(ContinuityValidationError::InvalidTimestamp)?;
    if !is_supported_catch_up_view_protocol(&view.protocol_version) {
        return Err(ContinuityValidationError::UnsupportedProtocolVersion {
            found: view.protocol_version.clone(),
            expected: CATCH_UP_VIEW_PROTOCOL_VERSION,
        });
    }
    validate_catch_up_window(&view.window)?;
    validate_catch_up_sections(&view.sections)?;
    if view.summary.len() > MAX_CATCH_UP_SUMMARY_BYTES {
        return Err(ContinuityValidationError::SummaryTooLong {
            max: MAX_CATCH_UP_SUMMARY_BYTES,
        });
    }
    if view.next_suggested_attention.len() > MAX_NEXT_SUGGESTED_ATTENTION {
        return Err(ContinuityValidationError::TooManyNextSuggestedAttention {
            max: MAX_NEXT_SUGGESTED_ATTENTION,
        });
    }
    for item in &view.next_suggested_attention {
        validate_pending_attention_item(item)?;
    }
    if view.evidence_refs.len() > MAX_CATCH_UP_EVIDENCE_REFS {
        return Err(ContinuityValidationError::TooManyEvidenceRefs {
            max: MAX_CATCH_UP_EVIDENCE_REFS,
        });
    }
    for evidence in &view.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContinuityValidationError::InvalidContinuityItem(format!("evidence_refs: {err}"))
        })?;
    }
    validate_limitations(&view.limitations)?;
    Ok(())
}

fn validate_current_state_sections(
    sections: &CurrentStateSections,
) -> Result<(), ContinuityValidationError> {
    for item in sections
        .completed
        .iter()
        .chain(&sections.in_progress)
        .chain(&sections.blocked)
        .chain(&sections.decisions)
        .chain(&sections.needs_attention)
        .chain(&sections.still_open)
    {
        validate_continuity_state_item(item)?;
    }
    Ok(())
}

fn validate_catch_up_sections(sections: &CatchUpSections) -> Result<(), ContinuityValidationError> {
    for item in sections
        .completed
        .iter()
        .chain(&sections.changed)
        .chain(&sections.blocked)
        .chain(&sections.decided)
        .chain(&sections.needs_attention)
        .chain(&sections.still_open)
    {
        validate_continuity_state_item(item)?;
    }
    Ok(())
}

fn validate_catch_up_window(window: &CatchUpWindow) -> Result<(), ContinuityValidationError> {
    validate_utc_timestamp(&window.since)
        .map_err(ContinuityValidationError::InvalidCatchUpWindow)?;
    validate_utc_timestamp(&window.until)
        .map_err(ContinuityValidationError::InvalidCatchUpWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|_| ContinuityValidationError::InvalidCatchUpWindow(window.since.clone()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|_| ContinuityValidationError::InvalidCatchUpWindow(window.until.clone()))?;
    if since > until {
        return Err(ContinuityValidationError::CatchUpWindowInverted);
    }
    Ok(())
}

fn validate_limitations(limitations: &[String]) -> Result<(), ContinuityValidationError> {
    if limitations.len() > MAX_PROJECTION_LIMITATIONS {
        return Err(ContinuityValidationError::TooManyLimitations {
            max: MAX_PROJECTION_LIMITATIONS,
        });
    }
    for limitation in limitations {
        if limitation.len() > MAX_LIMITATION_BYTES {
            return Err(ContinuityValidationError::LimitationTooLong {
                max: MAX_LIMITATION_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_rebuild_inputs_hash(hash: &str) -> Result<(), ContinuityValidationError> {
    if !hash.starts_with("fnv1a-") {
        return Err(ContinuityValidationError::InvalidRebuildInputsHash(
            "must start with fnv1a-".into(),
        ));
    }
    if hash.len() > MAX_REBUILD_INPUTS_HASH_BYTES {
        return Err(ContinuityValidationError::InvalidRebuildInputsHash(
            format!("exceeds {MAX_REBUILD_INPUTS_HASH_BYTES} bytes"),
        ));
    }
    if !hash[6..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ContinuityValidationError::InvalidRebuildInputsHash(
            "suffix must be lowercase hex".into(),
        ));
    }
    Ok(())
}

fn validate_continuity_item_id(
    id: &str,
    source: ContinuitySourceKind,
) -> Result<(), ContinuityValidationError> {
    let expected_prefix = match source {
        ContinuitySourceKind::WorkEvent => "event:",
        ContinuitySourceKind::ProcessedSignal | ContinuitySourceKind::PendingSignal => "signal:",
        ContinuitySourceKind::PromotionAudit => "audit:",
    };
    if !id.starts_with(expected_prefix) || id.len() <= expected_prefix.len() {
        return Err(ContinuityValidationError::InvalidContinuityItem(format!(
            "id must use stable prefix {expected_prefix}"
        )));
    }
    Ok(())
}
