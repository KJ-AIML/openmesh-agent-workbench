// ============================================================================
// OpenMesh Work Continuity Domain Contracts — Dev Track 0.1.3.1
// ============================================================================
// Minimum ownership-boundary contracts only. No serialization schema frozen,
// no persistence, no promotion logic. See:
//   .heli-harness/state/reports/openmesh-0.1.3.1-execution-plan.md, section 5.
//
// What this module deliberately did NOT introduce in 0.1.3.1 (Category B):
//   CurrentStateProjection and PendingAttention were deferred to 0.1.3.7.
// Dev Track 0.1.3.6 Checkpoint A — `EvidenceRef::GitState` (WEC-33) and WorkSignal
// protocol `1.1` compatibility for Git evidence producers.
// Dev Track 0.1.3.7 Checkpoint A — `CurrentStateProjection`, `PendingAttentionItem`,
// and `CatchUpView` wire contracts (pure types + validation; no I/O).
//
// Dev Track 0.1.3.4 Checkpoint A hardens the serializable WorkEvent wire shape
// and EvidenceAttachment model. Ledger persistence is Checkpoint B.
// ============================================================================

pub mod corrections;
pub mod events;
pub mod profile;
pub mod signals;

pub use corrections::*;
pub use events::*;
pub use profile::*;
pub use signals::*;

use crate::context::Sensitivity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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

// ============================================================================
// Dev Track 0.1.5 Checkpoint A — Proxy Context Pack domain contracts (pure)
// ============================================================================
// Continuity-only `ProxyContextPack` v1.0 wire types and structural validation.
// No builder, selection, storage, CLI, or authority execution in this checkpoint.

/// Wire-schema version for `ProxyContextPack`.
pub const PROXY_CONTEXT_PACK_PROTOCOL_VERSION: &str = "1.0";

/// Fixed authority execution boundary recorded in every pack.
pub const CONTEXT_PACK_EXECUTION_BOUNDARY: &str =
    "policy-metadata-only; no runtime authority execution in 0.1.5";

pub const MAX_CONTEXT_PACK_ID_BYTES: usize = 256;
pub const MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES: usize = 64;
pub const MAX_CONTEXT_PACK_EVIDENCE_INDEX: usize = 128;
pub const MAX_CONTEXT_PACK_LIMITATIONS: usize = 64;
pub const MAX_CONTEXT_PACK_UNRESOLVED_ITEMS: usize = 32;
pub const MAX_CONTEXT_PACK_DIAGNOSTICS: usize = 32;
pub const MAX_CONTEXT_PACK_EVIDENCE_LABEL_BYTES: usize = 512;
pub const MAX_CONTEXT_PACK_DIAGNOSTIC_CODE_BYTES: usize = 128;
pub const MAX_CONTEXT_PACK_DIAGNOSTIC_MESSAGE_BYTES: usize = 512;
pub const MAX_CONTEXT_PACK_UNRESOLVED_SUMMARY_BYTES: usize = 512;
pub const MAX_CONTEXT_PACK_FRESHNESS_WARNING_BYTES: usize = 512;
pub const MAX_CONTEXT_PACK_FILTERING_APPLIED_ITEMS: usize = 32;
pub const MAX_CONTEXT_PACK_FILTERING_APPLIED_BYTES: usize = 256;

/// Returns true when `version` is a supported Proxy Context Pack protocol.
pub fn is_supported_proxy_context_pack_protocol(version: &str) -> bool {
    version == PROXY_CONTEXT_PACK_PROTOCOL_VERSION
}

/// Canonical five-level authority ladder wire values for context-pack metadata.
pub fn proxy_context_pack_authority_ladder_levels() -> [&'static str; 5] {
    [
        "can-answer",
        "can-suggest",
        "can-draft",
        "must-ask-human",
        "cannot-answer",
    ]
}

/// Deterministic context pack id contract from a non-empty `buildInputsHash`.
pub fn deterministic_context_pack_id(build_inputs_hash: &str) -> String {
    format!("context-pack-{build_inputs_hash}")
}

/// How a pack item entered the bounded context surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackItemProvenance {
    Confirmed,
    Pending,
    Unconfirmed,
    DiagnosticOnly,
}

/// Correction metadata carried alongside effective presentation in pack items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackCorrectionProvenance {
    pub is_corrected: bool,
    pub is_superseded_original: bool,
    pub correction_event_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub superseded_by_event_id: Option<String>,
}

/// Metadata-only owner identity — no impersonation or answer behavior.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackOwnerIdentity {
    pub owner_label: String,
    pub role_label: String,
}

/// Declarative authority policy metadata — not an executed decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackAuthoritySummary {
    pub authority_rules: Vec<AuthorityRule>,
    pub default_refusal_rules: Vec<DefaultRefusalRule>,
    pub ladder_levels: Vec<String>,
    pub execution_boundary: String,
}

/// Declarative privacy policy metadata — no excluded secret source identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackPrivacySummary {
    pub privacy_rules: Vec<PrivacyRule>,
    pub sensitive_topics: Vec<String>,
    pub filtering_applied: Vec<String>,
}

/// Continuity-only evidence index origin in v1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackEvidenceOrigin {
    ContinuityItem,
}

/// Included, non-secret continuity evidence reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackEvidenceIndexEntry {
    pub ref_id: String,
    pub evidence_ref: EvidenceRef,
    pub origin: ContextPackEvidenceOrigin,
    pub sensitivity: Sensitivity,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Aggregate omission counts only — never secret-identifying detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackRedactionSummary {
    pub secret_items_omitted: u32,
    pub policy_restricted_items_omitted: u32,
    pub malformed_items_omitted: u32,
    pub quarantined_items_omitted: u32,
    pub bounds_truncated_items: u32,
}

/// Bounded continuity item with explicit provenance for pack surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackContinuityItem {
    pub id: String,
    pub summary: String,
    pub kind: String,
    pub source: ContinuitySourceKind,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub confidence: ContinuityConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unverified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction: Option<ContextPackCorrectionProvenance>,
}

/// Pending-attention row with explicit pack provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackPendingAttentionItem {
    pub id: String,
    pub summary: String,
    pub reason: PendingAttentionReason,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub status: PendingAttentionStatus,
    pub severity: PendingAttentionSeverity,
    pub priority: u8,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// Sanitized Current State sections for pack embedding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackCurrentStateSections {
    pub completed: Vec<ContextPackContinuityItem>,
    pub in_progress: Vec<ContextPackContinuityItem>,
    pub blocked: Vec<ContextPackContinuityItem>,
    pub decisions: Vec<ContextPackContinuityItem>,
    pub needs_attention: Vec<ContextPackContinuityItem>,
    pub still_open: Vec<ContextPackContinuityItem>,
}

/// Sanitized Current State representation — not a raw `CurrentStateProjection`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackCurrentState {
    pub workspace_id: String,
    pub sections: ContextPackCurrentStateSections,
    pub pending_attention: Vec<ContextPackPendingAttentionItem>,
    pub limitations: Vec<String>,
}

/// Sanitized Catch-up sections for pack embedding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackCatchUpSections {
    pub completed: Vec<ContextPackContinuityItem>,
    pub changed: Vec<ContextPackContinuityItem>,
    pub blocked: Vec<ContextPackContinuityItem>,
    pub decided: Vec<ContextPackContinuityItem>,
    pub needs_attention: Vec<ContextPackContinuityItem>,
    pub still_open: Vec<ContextPackContinuityItem>,
}

/// Sanitized Catch-up representation — not a raw `CatchUpView`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackCatchUp {
    pub workspace_id: String,
    pub window: CatchUpWindow,
    pub sections: ContextPackCatchUpSections,
    pub summary: String,
    pub next_suggested_attention: Vec<ContextPackPendingAttentionItem>,
    pub limitations: Vec<String>,
}

/// Objective freshness metadata — no strict enforcement fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackFreshness {
    pub snapshot_observed_at: String,
    pub current_state_generated_at: String,
    pub catch_up_since: String,
    pub catch_up_until: String,
    pub pack_generated_at: String,
    pub age_seconds: u64,
    pub warnings: Vec<String>,
}

/// Bounded builder diagnostic — must not leak secret-identifying detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: ContextPackDiagnosticSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackDiagnosticSeverity {
    Info,
    Warning,
    Error,
}

/// Unresolved or unconfirmed continuity surface item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextPackUnresolvedItem {
    pub id: String,
    pub category: ContextPackUnresolvedCategory,
    pub summary: String,
    pub provenance: ContextPackItemProvenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContextPackUnresolvedCategory {
    MalformedEvidence,
    Quarantine,
    Pending,
    Unconfirmed,
    PartialContinuity,
    Truncation,
}

/// Bounded, inspectable Proxy Context Pack v1.0 (continuity-only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyContextPack {
    pub context_pack_id: String,
    pub workspace_id: String,
    pub profile_id: String,
    pub profile_version: String,
    pub protocol_version: String,
    pub generated_at: String,
    pub requested_window: CatchUpWindow,
    pub owner_identity: ContextPackOwnerIdentity,
    pub communication_preferences: CommunicationPreferences,
    pub decision_preferences: DecisionPreferences,
    pub authority_summary: ContextPackAuthoritySummary,
    pub privacy_summary: ContextPackPrivacySummary,
    pub evidence_policy: EvidencePolicy,
    pub current_state: ContextPackCurrentState,
    pub catch_up: ContextPackCatchUp,
    pub evidence_index: Vec<ContextPackEvidenceIndexEntry>,
    pub source_counts: SourceCounts,
    pub diagnostics: Vec<ContextPackDiagnostic>,
    pub limitations: Vec<String>,
    pub unresolved_items: Vec<ContextPackUnresolvedItem>,
    pub freshness: ContextPackFreshness,
    pub redaction_summary: ContextPackRedactionSummary,
    pub build_inputs_hash: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextPackValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("context_pack_id is empty after trim")]
    EmptyContextPackId,
    #[error("context_pack_id exceeds the {max}-byte bound")]
    ContextPackIdTooLong { max: usize },
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("profile_id is empty after trim")]
    EmptyProfileId,
    #[error("profile_version is empty after trim")]
    EmptyProfileVersion,
    #[error("build_inputs_hash is empty after trim")]
    EmptyBuildInputsHash,
    #[error("build_inputs_hash exceeds the {max}-byte bound")]
    BuildInputsHashTooLong { max: usize },
    #[error("timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("catch-up window since must be <= until")]
    CatchUpWindowInverted,
    #[error("limitations must not be empty")]
    EmptyLimitations,
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
    #[error("evidence_index exceeds the {max}-entry bound")]
    TooManyEvidenceIndexEntries { max: usize },
    #[error("duplicate evidence_index ref_id: {ref_id}")]
    DuplicateEvidenceIndexRefId { ref_id: String },
    #[error("evidence_index entry sensitivity must not be secret")]
    SecretEvidenceIndexEntry,
    #[error("diagnostics exceed the {max}-entry bound")]
    TooManyDiagnostics { max: usize },
    #[error("unresolved_items exceed the {max}-entry bound")]
    TooManyUnresolvedItems { max: usize },
    #[error("authority ladder must contain the exact five wire values")]
    InvalidAuthorityLadder,
    #[error("execution_boundary must be the frozen 0.1.5 metadata boundary")]
    InvalidExecutionBoundary,
    #[error("owner_identity is invalid: {0}")]
    InvalidOwnerIdentity(String),
    #[error("authority_summary is invalid: {0}")]
    InvalidAuthoritySummary(String),
    #[error("privacy_summary is invalid: {0}")]
    InvalidPrivacySummary(String),
    #[error("evidence_policy is invalid: {0}")]
    InvalidEvidencePolicy(String),
    #[error("current_state is invalid: {0}")]
    InvalidCurrentState(String),
    #[error("catch_up is invalid: {0}")]
    InvalidCatchUp(String),
    #[error("freshness is invalid: {0}")]
    InvalidFreshness(String),
    #[error("continuity item is invalid: {0}")]
    InvalidContinuityItem(String),
    #[error("pending item is invalid: {0}")]
    InvalidPendingItem(String),
    #[error("pending signal cannot be represented as confirmed")]
    PendingSignalRepresentedAsConfirmed,
    #[error("forbidden pack contract field present in serialized surface")]
    ForbiddenContractField,
}

/// Structural validation for `ProxyContextPack` v1.0 (pure, no I/O).
pub fn validate_proxy_context_pack(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    if !is_supported_proxy_context_pack_protocol(&pack.protocol_version) {
        return Err(ContextPackValidationError::UnsupportedProtocolVersion {
            found: pack.protocol_version.clone(),
            expected: PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
        });
    }
    validate_context_pack_id_field(&pack.context_pack_id)?;
    if pack.workspace_id.trim().is_empty() {
        return Err(ContextPackValidationError::EmptyWorkspaceId);
    }
    if pack.profile_id.trim().is_empty() {
        return Err(ContextPackValidationError::EmptyProfileId);
    }
    if pack.profile_id.len() > MAX_CONTEXT_PACK_ID_BYTES {
        return Err(ContextPackValidationError::ContextPackIdTooLong {
            max: MAX_CONTEXT_PACK_ID_BYTES,
        });
    }
    if pack.profile_version.trim().is_empty() {
        return Err(ContextPackValidationError::EmptyProfileVersion);
    }
    validate_build_inputs_hash_field(&pack.build_inputs_hash)?;
    validate_utc_timestamp(&pack.generated_at)
        .map_err(ContextPackValidationError::InvalidTimestamp)?;
    validate_context_pack_window(&pack.requested_window)?;
    validate_context_pack_owner_identity(&pack.owner_identity)?;
    validate_communication_preferences(&pack.communication_preferences)
        .map_err(|err| ContextPackValidationError::InvalidOwnerIdentity(err.to_string()))?;
    validate_decision_preferences(&pack.decision_preferences)
        .map_err(|err| ContextPackValidationError::InvalidOwnerIdentity(err.to_string()))?;
    validate_context_pack_authority_summary(&pack.authority_summary)?;
    validate_context_pack_privacy_summary(&pack.privacy_summary)?;
    validate_evidence_policy(&pack.evidence_policy)
        .map_err(|err| ContextPackValidationError::InvalidEvidencePolicy(err.to_string()))?;
    validate_context_pack_current_state(&pack.current_state, &pack.workspace_id)?;
    validate_context_pack_catch_up(&pack.catch_up, &pack.workspace_id, &pack.requested_window)?;
    validate_context_pack_evidence_index(&pack.evidence_index)?;
    validate_source_counts(&pack.source_counts)
        .map_err(|err| ContextPackValidationError::InvalidCurrentState(err.to_string()))?;
    validate_context_pack_diagnostics(&pack.diagnostics)?;
    validate_context_pack_limitations(&pack.limitations)?;
    validate_context_pack_unresolved_items(&pack.unresolved_items)?;
    validate_context_pack_freshness(&pack.freshness, &pack.requested_window, &pack.generated_at)?;
    validate_context_pack_redaction_summary(&pack.redaction_summary)?;
    if pack.context_pack_id != deterministic_context_pack_id(&pack.build_inputs_hash) {
        return Err(ContextPackValidationError::InvalidOwnerIdentity(
            "context_pack_id must match deterministic_context_pack_id(build_inputs_hash)".into(),
        ));
    }
    Ok(())
}

fn validate_context_pack_id_field(value: &str) -> Result<(), ContextPackValidationError> {
    if value.trim().is_empty() {
        return Err(ContextPackValidationError::EmptyContextPackId);
    }
    if value.len() > MAX_CONTEXT_PACK_ID_BYTES {
        return Err(ContextPackValidationError::ContextPackIdTooLong {
            max: MAX_CONTEXT_PACK_ID_BYTES,
        });
    }
    Ok(())
}

fn validate_build_inputs_hash_field(value: &str) -> Result<(), ContextPackValidationError> {
    if value.trim().is_empty() {
        return Err(ContextPackValidationError::EmptyBuildInputsHash);
    }
    if value.len() > MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES {
        return Err(ContextPackValidationError::BuildInputsHashTooLong {
            max: MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES,
        });
    }
    Ok(())
}

fn validate_context_pack_window(window: &CatchUpWindow) -> Result<(), ContextPackValidationError> {
    validate_utc_timestamp(&window.since).map_err(ContextPackValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&window.until).map_err(ContextPackValidationError::InvalidTimestamp)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|err| ContextPackValidationError::InvalidTimestamp(err.to_string()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|err| ContextPackValidationError::InvalidTimestamp(err.to_string()))?;
    if since > until {
        return Err(ContextPackValidationError::CatchUpWindowInverted);
    }
    Ok(())
}

fn validate_context_pack_owner_identity(
    identity: &ContextPackOwnerIdentity,
) -> Result<(), ContextPackValidationError> {
    if identity.owner_label.trim().is_empty() {
        return Err(ContextPackValidationError::InvalidOwnerIdentity(
            "owner_label is empty".into(),
        ));
    }
    if identity.owner_label.len() > MAX_PROFILE_LABEL_BYTES {
        return Err(ContextPackValidationError::InvalidOwnerIdentity(format!(
            "owner_label exceeds {} bytes",
            MAX_PROFILE_LABEL_BYTES
        )));
    }
    if identity.role_label.len() > MAX_PROFILE_LABEL_BYTES {
        return Err(ContextPackValidationError::InvalidOwnerIdentity(format!(
            "role_label exceeds {} bytes",
            MAX_PROFILE_LABEL_BYTES
        )));
    }
    for field in [&identity.owner_label, &identity.role_label] {
        if contains_impersonation_claim(field) {
            return Err(ContextPackValidationError::InvalidOwnerIdentity(
                "owner_identity must not contain impersonation claims".into(),
            ));
        }
    }
    Ok(())
}

fn validate_context_pack_authority_summary(
    summary: &ContextPackAuthoritySummary,
) -> Result<(), ContextPackValidationError> {
    if summary.execution_boundary != CONTEXT_PACK_EXECUTION_BOUNDARY {
        return Err(ContextPackValidationError::InvalidExecutionBoundary);
    }
    let expected = proxy_context_pack_authority_ladder_levels();
    if summary.ladder_levels.len() != expected.len()
        || summary
            .ladder_levels
            .iter()
            .map(String::as_str)
            .ne(expected.iter().copied())
    {
        return Err(ContextPackValidationError::InvalidAuthorityLadder);
    }
    if summary.authority_rules.is_empty() {
        return Err(ContextPackValidationError::InvalidAuthoritySummary(
            "authority_rules must not be empty".into(),
        ));
    }
    for rule in &summary.authority_rules {
        validate_authority_rule(rule)
            .map_err(|err| ContextPackValidationError::InvalidAuthoritySummary(err.to_string()))?;
    }
    if summary.default_refusal_rules.is_empty() {
        return Err(ContextPackValidationError::InvalidAuthoritySummary(
            "default_refusal_rules must not be empty".into(),
        ));
    }
    for rule in &summary.default_refusal_rules {
        validate_bounded_text(&rule.statement, MAX_PROFILE_TEXT_BYTES)
            .map_err(|err| ContextPackValidationError::InvalidAuthoritySummary(err.to_string()))?;
    }
    Ok(())
}

fn validate_context_pack_privacy_summary(
    summary: &ContextPackPrivacySummary,
) -> Result<(), ContextPackValidationError> {
    if summary.filtering_applied.len() > MAX_CONTEXT_PACK_FILTERING_APPLIED_ITEMS {
        return Err(ContextPackValidationError::InvalidPrivacySummary(format!(
            "filtering_applied exceeds {} items",
            MAX_CONTEXT_PACK_FILTERING_APPLIED_ITEMS
        )));
    }
    for entry in &summary.filtering_applied {
        validate_bounded_text(entry, MAX_CONTEXT_PACK_FILTERING_APPLIED_BYTES)
            .map_err(|err| ContextPackValidationError::InvalidPrivacySummary(err.to_string()))?;
        if contains_secret_like_value(entry) {
            return Err(ContextPackValidationError::InvalidPrivacySummary(
                "filtering_applied must not contain secret-identifying values".into(),
            ));
        }
    }
    for rule in &summary.privacy_rules {
        validate_privacy_rule(rule)
            .map_err(|err| ContextPackValidationError::InvalidPrivacySummary(err.to_string()))?;
    }
    for topic in &summary.sensitive_topics {
        validate_bounded_text(topic, MAX_PROFILE_TEXT_BYTES)
            .map_err(|err| ContextPackValidationError::InvalidPrivacySummary(err.to_string()))?;
    }
    Ok(())
}

fn validate_context_pack_current_state(
    state: &ContextPackCurrentState,
    workspace_id: &str,
) -> Result<(), ContextPackValidationError> {
    if state.workspace_id != workspace_id {
        return Err(ContextPackValidationError::InvalidCurrentState(
            "workspace_id mismatch".into(),
        ));
    }
    for item in state
        .sections
        .completed
        .iter()
        .chain(&state.sections.in_progress)
        .chain(&state.sections.blocked)
        .chain(&state.sections.decisions)
        .chain(&state.sections.needs_attention)
        .chain(&state.sections.still_open)
    {
        validate_context_pack_continuity_item(item)?;
    }
    for item in &state.pending_attention {
        validate_context_pack_pending_item(item)?;
    }
    validate_context_pack_limitations(&state.limitations)
        .map_err(|err| ContextPackValidationError::InvalidCurrentState(err.to_string()))?;
    Ok(())
}

fn validate_context_pack_catch_up(
    catch_up: &ContextPackCatchUp,
    workspace_id: &str,
    window: &CatchUpWindow,
) -> Result<(), ContextPackValidationError> {
    if catch_up.workspace_id != workspace_id {
        return Err(ContextPackValidationError::InvalidCatchUp(
            "workspace_id mismatch".into(),
        ));
    }
    if catch_up.window != *window {
        return Err(ContextPackValidationError::InvalidCatchUp(
            "catch_up.window must match requested_window".into(),
        ));
    }
    if catch_up.summary.len() > MAX_CATCH_UP_SUMMARY_BYTES {
        return Err(ContextPackValidationError::InvalidCatchUp(format!(
            "summary exceeds {} bytes",
            MAX_CATCH_UP_SUMMARY_BYTES
        )));
    }
    for item in catch_up
        .sections
        .completed
        .iter()
        .chain(&catch_up.sections.changed)
        .chain(&catch_up.sections.blocked)
        .chain(&catch_up.sections.decided)
        .chain(&catch_up.sections.needs_attention)
        .chain(&catch_up.sections.still_open)
    {
        validate_context_pack_continuity_item(item)?;
    }
    if catch_up.next_suggested_attention.len() > MAX_NEXT_SUGGESTED_ATTENTION {
        return Err(ContextPackValidationError::InvalidCatchUp(format!(
            "next_suggested_attention exceeds {}",
            MAX_NEXT_SUGGESTED_ATTENTION
        )));
    }
    for item in &catch_up.next_suggested_attention {
        validate_context_pack_pending_item(item)?;
    }
    validate_context_pack_limitations(&catch_up.limitations)
        .map_err(|err| ContextPackValidationError::InvalidCatchUp(err.to_string()))?;
    Ok(())
}

fn validate_context_pack_continuity_item(
    item: &ContextPackContinuityItem,
) -> Result<(), ContextPackValidationError> {
    if item.id.trim().is_empty() || item.summary.trim().is_empty() || item.kind.trim().is_empty() {
        return Err(ContextPackValidationError::InvalidContinuityItem(
            "id, summary, and kind are required".into(),
        ));
    }
    if item.summary.len() > MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES {
        return Err(ContextPackValidationError::InvalidContinuityItem(format!(
            "summary exceeds {} bytes",
            MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES
        )));
    }
    validate_utc_timestamp(&item.timestamp)
        .map_err(ContextPackValidationError::InvalidContinuityItem)?;
    if item.source == ContinuitySourceKind::PendingSignal
        && item.provenance == ContextPackItemProvenance::Confirmed
    {
        return Err(ContextPackValidationError::PendingSignalRepresentedAsConfirmed);
    }
    if item.evidence_refs.len() > MAX_CONTINUITY_ITEM_EVIDENCE_REFS {
        return Err(ContextPackValidationError::InvalidContinuityItem(format!(
            "evidence_refs exceed {}",
            MAX_CONTINUITY_ITEM_EVIDENCE_REFS
        )));
    }
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContextPackValidationError::InvalidContinuityItem(format!("evidence_refs: {err}"))
        })?;
    }
    if let Some(correction) = &item.correction {
        validate_context_pack_correction_provenance(correction)?;
        if correction.is_superseded_original {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "superseded originals must not appear in pack sections".into(),
            ));
        }
    }
    Ok(())
}

fn validate_context_pack_pending_item(
    item: &ContextPackPendingAttentionItem,
) -> Result<(), ContextPackValidationError> {
    if item.id.trim().is_empty() || item.summary.trim().is_empty() {
        return Err(ContextPackValidationError::InvalidPendingItem(
            "id and summary are required".into(),
        ));
    }
    validate_utc_timestamp(&item.timestamp)
        .map_err(ContextPackValidationError::InvalidPendingItem)?;
    if matches!(
        item.provenance,
        ContextPackItemProvenance::Confirmed | ContextPackItemProvenance::DiagnosticOnly
    ) && item.reason == PendingAttentionReason::PendingSignal
    {
        return Err(ContextPackValidationError::InvalidPendingItem(
            "pending-signal attention must remain pending or unconfirmed".into(),
        ));
    }
    if item.evidence_refs.len() > MAX_CONTINUITY_ITEM_EVIDENCE_REFS {
        return Err(ContextPackValidationError::InvalidPendingItem(
            "too many evidence_refs".into(),
        ));
    }
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence).map_err(|err| {
            ContextPackValidationError::InvalidPendingItem(format!("evidence_refs: {err}"))
        })?;
    }
    if item.priority < MIN_PENDING_ATTENTION_PRIORITY
        || item.priority > MAX_PENDING_ATTENTION_PRIORITY
    {
        return Err(ContextPackValidationError::InvalidPendingItem(
            "priority out of range".into(),
        ));
    }
    Ok(())
}

fn validate_context_pack_correction_provenance(
    correction: &ContextPackCorrectionProvenance,
) -> Result<(), ContextPackValidationError> {
    if correction.is_superseded_original && correction.is_corrected {
        return Err(ContextPackValidationError::InvalidContinuityItem(
            "item cannot be both corrected and superseded-original".into(),
        ));
    }
    for event_id in &correction.correction_event_ids {
        validate_context_pack_id_field(event_id).map_err(|_| {
            ContextPackValidationError::InvalidContinuityItem("invalid correction_event_id".into())
        })?;
    }
    Ok(())
}

fn validate_context_pack_evidence_index(
    entries: &[ContextPackEvidenceIndexEntry],
) -> Result<(), ContextPackValidationError> {
    if entries.len() > MAX_CONTEXT_PACK_EVIDENCE_INDEX {
        return Err(ContextPackValidationError::TooManyEvidenceIndexEntries {
            max: MAX_CONTEXT_PACK_EVIDENCE_INDEX,
        });
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in entries {
        if entry.sensitivity == Sensitivity::Secret {
            return Err(ContextPackValidationError::SecretEvidenceIndexEntry);
        }
        if entry.origin != ContextPackEvidenceOrigin::ContinuityItem {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "evidence_index origin must be continuity-item in v1.0".into(),
            ));
        }
        if entry.label.len() > MAX_CONTEXT_PACK_EVIDENCE_LABEL_BYTES {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "evidence_index label too long".into(),
            ));
        }
        if let Some(timestamp) = &entry.timestamp {
            validate_utc_timestamp(timestamp)
                .map_err(ContextPackValidationError::InvalidContinuityItem)?;
        }
        validate_evidence_ref(&entry.evidence_ref).map_err(|err| {
            ContextPackValidationError::InvalidContinuityItem(format!("evidence_ref: {err}"))
        })?;
        if !seen.insert(entry.ref_id.clone()) {
            return Err(ContextPackValidationError::DuplicateEvidenceIndexRefId {
                ref_id: entry.ref_id.clone(),
            });
        }
    }
    Ok(())
}

fn validate_context_pack_diagnostics(
    diagnostics: &[ContextPackDiagnostic],
) -> Result<(), ContextPackValidationError> {
    if diagnostics.len() > MAX_CONTEXT_PACK_DIAGNOSTICS {
        return Err(ContextPackValidationError::TooManyDiagnostics {
            max: MAX_CONTEXT_PACK_DIAGNOSTICS,
        });
    }
    for diagnostic in diagnostics {
        validate_bounded_text(&diagnostic.code, MAX_CONTEXT_PACK_DIAGNOSTIC_CODE_BYTES).map_err(
            |_| ContextPackValidationError::InvalidContinuityItem("diagnostic code".into()),
        )?;
        validate_bounded_text(
            &diagnostic.message,
            MAX_CONTEXT_PACK_DIAGNOSTIC_MESSAGE_BYTES,
        )
        .map_err(|_| {
            ContextPackValidationError::InvalidContinuityItem("diagnostic message".into())
        })?;
        if contains_secret_like_value(&diagnostic.message) {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "diagnostic must not leak secret-identifying detail".into(),
            ));
        }
    }
    Ok(())
}

fn validate_context_pack_limitations(
    limitations: &[String],
) -> Result<(), ContextPackValidationError> {
    if limitations.is_empty() {
        return Err(ContextPackValidationError::EmptyLimitations);
    }
    if limitations.len() > MAX_CONTEXT_PACK_LIMITATIONS {
        return Err(ContextPackValidationError::TooManyLimitations {
            max: MAX_CONTEXT_PACK_LIMITATIONS,
        });
    }
    for limitation in limitations {
        if limitation.len() > MAX_LIMITATION_BYTES {
            return Err(ContextPackValidationError::LimitationTooLong {
                max: MAX_LIMITATION_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_context_pack_unresolved_items(
    items: &[ContextPackUnresolvedItem],
) -> Result<(), ContextPackValidationError> {
    if items.len() > MAX_CONTEXT_PACK_UNRESOLVED_ITEMS {
        return Err(ContextPackValidationError::TooManyUnresolvedItems {
            max: MAX_CONTEXT_PACK_UNRESOLVED_ITEMS,
        });
    }
    for item in items {
        if item.id.trim().is_empty() || item.summary.trim().is_empty() {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "unresolved item id and summary are required".into(),
            ));
        }
        if item.summary.len() > MAX_CONTEXT_PACK_UNRESOLVED_SUMMARY_BYTES {
            return Err(ContextPackValidationError::InvalidContinuityItem(
                "unresolved summary too long".into(),
            ));
        }
    }
    Ok(())
}

fn validate_context_pack_freshness(
    freshness: &ContextPackFreshness,
    window: &CatchUpWindow,
    generated_at: &str,
) -> Result<(), ContextPackValidationError> {
    for ts in [
        &freshness.snapshot_observed_at,
        &freshness.current_state_generated_at,
        &freshness.catch_up_since,
        &freshness.catch_up_until,
        &freshness.pack_generated_at,
    ] {
        validate_utc_timestamp(ts).map_err(ContextPackValidationError::InvalidTimestamp)?;
    }
    if freshness.catch_up_since != window.since || freshness.catch_up_until != window.until {
        return Err(ContextPackValidationError::InvalidFreshness(
            "freshness window must match requested_window".into(),
        ));
    }
    if freshness.pack_generated_at != generated_at {
        return Err(ContextPackValidationError::InvalidFreshness(
            "pack_generated_at must match generated_at".into(),
        ));
    }
    if freshness.warnings.len() > MAX_CONTEXT_PACK_DIAGNOSTICS {
        return Err(ContextPackValidationError::InvalidFreshness(
            "too many freshness warnings".into(),
        ));
    }
    for warning in &freshness.warnings {
        validate_bounded_text(warning, MAX_CONTEXT_PACK_FRESHNESS_WARNING_BYTES)
            .map_err(|_| ContextPackValidationError::InvalidFreshness("warning too long".into()))?;
    }
    Ok(())
}

fn validate_context_pack_redaction_summary(
    summary: &ContextPackRedactionSummary,
) -> Result<(), ContextPackValidationError> {
    let _ = (
        summary.secret_items_omitted,
        summary.policy_restricted_items_omitted,
        summary.malformed_items_omitted,
        summary.quarantined_items_omitted,
        summary.bounds_truncated_items,
    );
    Ok(())
}

// ============================================================================
// Dev Track 0.1.6 Checkpoint A — proxy draft domain and wire contracts (pure)
// ============================================================================
// Structural validators only. No prompt composition, runtime adapters, CLI,
// persistence, or authority execution in this checkpoint.

/// Wire-schema version for `ProxyQuestion`.
pub const PROXY_QUESTION_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyDraft`.
pub const PROXY_DRAFT_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyDraftTraceMetadata`.
pub const PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyPromptBundle`.
pub const PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION: &str = "1.0";

/// Frozen `ProxyDraft.classification` wire value.
pub const PROXY_DRAFT_CLASSIFICATION: &str = "local-proxy-draft";

/// Frozen `ProxyDraft.authorityNotice` wire value.
pub const PROXY_DRAFT_AUTHORITY_NOTICE: &str =
    "Policy metadata only — no authority decision was executed.";

/// Frozen `ProxyDraft.executionBoundary` wire value.
pub const PROXY_DRAFT_EXECUTION_BOUNDARY: &str = "draft-only; no authority execution in 0.1.6";

/// Frozen bound: `ProxyQuestion.text` maximum UTF-8 byte length.
pub const MAX_PROXY_QUESTION_TEXT_BYTES: usize = 2048;

/// Frozen bound: `ProxyDraft.draftText` maximum UTF-8 byte length.
pub const MAX_PROXY_DRAFT_TEXT_BYTES: usize = 8192;

/// Frozen bound: `ProxyDraft.limitations` entry count.
pub const MAX_PROXY_DRAFT_LIMITATIONS: usize = 32;

pub const MAX_PROXY_QUESTION_ID_BYTES: usize = 256;
pub const MAX_PROXY_PROMPT_FIELD_BYTES: usize = 65_536;
pub const MAX_PROXY_RUNTIME_KIND_BYTES: usize = 64;
pub const MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES: usize = 64;
pub const MAX_PROXY_RUNTIME_MODEL_ID_BYTES: usize = 128;
pub const MAX_PROXY_TRACE_WORKSPACE_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_PROFILE_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_PROFILE_VERSION_BYTES: usize = 64;
pub const MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES: usize = MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES;
pub const MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES: usize = 64;
pub const MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES: usize = 32;
pub const MAX_PROXY_DRAFT_LIMITATION_BYTES: usize = MAX_LIMITATION_BYTES;

/// Returns true when `version` is a supported Proxy Question protocol.
pub fn is_supported_proxy_question_protocol(version: &str) -> bool {
    version == PROXY_QUESTION_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Draft protocol.
pub fn is_supported_proxy_draft_protocol(version: &str) -> bool {
    version == PROXY_DRAFT_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Draft trace metadata protocol.
pub fn is_supported_proxy_draft_trace_metadata_protocol(version: &str) -> bool {
    version == PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Prompt Bundle protocol.
pub fn is_supported_proxy_prompt_bundle_protocol(version: &str) -> bool {
    version == PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION
}

/// Ask-my-proxy question wire contract (identity generation deferred to Checkpoint B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyQuestion {
    pub protocol_version: String,
    pub question_id: String,
    pub text: String,
}

/// Immutable prompt bundle referenced by runtime requests (composition deferred to Checkpoint B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptBundle {
    pub protocol_version: String,
    pub system_message: String,
    pub context_json: String,
    pub user_message: String,
}

/// Provider-neutral runtime request contract (invocation deferred to Checkpoint C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyRuntimeRequest {
    pub prompt: ProxyPromptBundle,
    pub timeout_ms: u64,
    pub max_output_bytes: u32,
}

/// Runtime-owned draft output (safety validation deferred to Checkpoint D).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyRuntimeOutput {
    pub draft_text: String,
    pub provider_id: String,
    pub model_id: String,
    pub network_used: bool,
    pub duration_ms: u64,
}

/// Aggregate-only evidence summary under `ProxyDraft.trace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftEvidenceSummary {
    pub evidence_index_count: u32,
    pub source_counts: BTreeMap<String, u32>,
    pub secret_items_omitted: u32,
}

/// OpenMesh-owned trace metadata nested under `ProxyDraft.trace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftTraceMetadata {
    pub protocol_version: String,
    pub workspace_id: String,
    pub profile_id: String,
    pub profile_version: String,
    pub context_pack_id: String,
    pub build_inputs_hash: String,
    pub evidence_summary: ProxyDraftEvidenceSummary,
}

/// Runtime metadata shell included in `ProxyDraft`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftRuntimeMetadata {
    pub runtime_kind: String,
    pub provider_id: String,
    pub model_id: String,
    pub network_used: bool,
    pub duration_ms: u64,
}

/// Canonical proxy draft response wire contract v1.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraft {
    pub protocol_version: String,
    pub question_id: String,
    pub generated_at: String,
    pub classification: String,
    pub draft_text: String,
    pub authority_notice: String,
    pub execution_boundary: String,
    pub trace: ProxyDraftTraceMetadata,
    pub runtime: ProxyDraftRuntimeMetadata,
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyQuestionValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("question_id is empty after trim")]
    EmptyQuestionId,
    #[error("question_id exceeds the {max}-byte bound")]
    QuestionIdTooLong { max: usize },
    #[error("question_id format is invalid: {0}")]
    InvalidQuestionId(String),
    #[error("text is empty after normalization")]
    EmptyText,
    #[error("text exceeds the {max}-byte bound after normalization")]
    TextTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyPromptBundleValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("system_message is empty after trim")]
    EmptySystemMessage,
    #[error("user_message is empty after trim")]
    EmptyUserMessage,
    #[error("context_json is empty after trim")]
    EmptyContextJson,
    #[error("context_json is not valid JSON: {0}")]
    InvalidContextJson(String),
    #[error("prompt field exceeds the {max}-byte bound")]
    FieldTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyRuntimeRequestValidationError {
    #[error("timeout_ms must be greater than zero")]
    ZeroTimeout,
    #[error("max_output_bytes must be greater than zero")]
    ZeroMaxOutputBytes,
    #[error("max_output_bytes exceeds the {max}-byte bound")]
    MaxOutputBytesTooLarge { max: usize },
    #[error("prompt bundle is invalid: {0}")]
    InvalidPromptBundle(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyRuntimeOutputValidationError {
    #[error("draft_text is empty after trim")]
    EmptyDraftText,
    #[error("draft_text exceeds the {max}-byte bound")]
    DraftTextTooLong { max: usize },
    #[error("provider_id is empty after trim")]
    EmptyProviderId,
    #[error("model_id is empty after trim")]
    EmptyModelId,
    #[error("provider_id exceeds the {max}-byte bound")]
    ProviderIdTooLong { max: usize },
    #[error("model_id exceeds the {max}-byte bound")]
    ModelIdTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftEvidenceSummaryValidationError {
    #[error("source_counts exceed the {max}-entry bound")]
    TooManySourceCategories { max: usize },
    #[error("source_counts key is empty after trim")]
    EmptySourceCategory,
    #[error("source_counts key exceeds the {max}-byte bound")]
    SourceCategoryTooLong { max: usize },
    #[error("source_counts key is path-like or canonical-ref-like: {0}")]
    InvalidSourceCategory(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftTraceMetadataValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("profile_id is empty after trim")]
    EmptyProfileId,
    #[error("profile_version is empty after trim")]
    EmptyProfileVersion,
    #[error("context_pack_id is empty after trim")]
    EmptyContextPackId,
    #[error("build_inputs_hash is empty after trim")]
    EmptyBuildInputsHash,
    #[error("workspace_id exceeds the {max}-byte bound")]
    WorkspaceIdTooLong { max: usize },
    #[error("profile_id exceeds the {max}-byte bound")]
    ProfileIdTooLong { max: usize },
    #[error("profile_version exceeds the {max}-byte bound")]
    ProfileVersionTooLong { max: usize },
    #[error("context_pack_id exceeds the {max}-byte bound")]
    ContextPackIdTooLong { max: usize },
    #[error("build_inputs_hash exceeds the {max}-byte bound")]
    BuildInputsHashTooLong { max: usize },
    #[error("context_pack_id must start with context-pack-")]
    InvalidContextPackIdPrefix,
    #[error("evidence_summary is invalid: {0}")]
    InvalidEvidenceSummary(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftRuntimeMetadataValidationError {
    #[error("runtime_kind is empty after trim")]
    EmptyRuntimeKind,
    #[error("provider_id is empty after trim")]
    EmptyProviderId,
    #[error("model_id is empty after trim")]
    EmptyModelId,
    #[error("runtime_kind exceeds the {max}-byte bound")]
    RuntimeKindTooLong { max: usize },
    #[error("provider_id exceeds the {max}-byte bound")]
    ProviderIdTooLong { max: usize },
    #[error("model_id exceeds the {max}-byte bound")]
    ModelIdTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("question_id is invalid: {0}")]
    InvalidQuestionId(String),
    #[error("generated_at is invalid: {0}")]
    InvalidGeneratedAt(String),
    #[error("classification must be the frozen local-proxy-draft value")]
    InvalidClassification,
    #[error("authority_notice must be the frozen policy metadata notice")]
    InvalidAuthorityNotice,
    #[error("execution_boundary must be the frozen 0.1.6 draft-only boundary")]
    InvalidExecutionBoundary,
    #[error("draft_text is empty after trim")]
    EmptyDraftText,
    #[error("draft_text exceeds the {max}-byte bound")]
    DraftTextTooLong { max: usize },
    #[error("trace metadata is invalid: {0}")]
    InvalidTraceMetadata(String),
    #[error("runtime metadata is invalid: {0}")]
    InvalidRuntimeMetadata(String),
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation is empty after trim")]
    EmptyLimitation,
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
}

/// Normalize question text for validation (trim only; not serialized separately).
pub fn normalize_proxy_question_text(text: &str) -> String {
    text.trim().to_string()
}

/// Validate frozen `proxy-q-<nanos_hex>-<pid_hex>-<counter_hex>` question identity format.
pub fn validate_proxy_question_id(question_id: &str) -> Result<(), ProxyQuestionValidationError> {
    if question_id.trim().is_empty() {
        return Err(ProxyQuestionValidationError::EmptyQuestionId);
    }
    if question_id.len() > MAX_PROXY_QUESTION_ID_BYTES {
        return Err(ProxyQuestionValidationError::QuestionIdTooLong {
            max: MAX_PROXY_QUESTION_ID_BYTES,
        });
    }
    if question_id.chars().any(char::is_whitespace) {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must not contain whitespace".into(),
        ));
    }
    for forbidden in ['/', '\\', ':', '%'] {
        if question_id.contains(forbidden) {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(format!(
                "question_id must not contain '{forbidden}'"
            )));
        }
    }
    const PREFIX: &str = "proxy-q-";
    if !question_id.starts_with(PREFIX) {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must start with proxy-q-".into(),
        ));
    }
    let remainder = &question_id[PREFIX.len()..];
    let segments: Vec<&str> = remainder.split('-').collect();
    if segments.len() != 3 {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must contain exactly three lowercase hexadecimal segments after proxy-q-"
                .into(),
        ));
    }
    for segment in segments {
        if segment.is_empty() {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(
                "question_id segments must be non-empty".into(),
            ));
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, 'a'..='f'))
        {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(
                "question_id segments must be lowercase hexadecimal".into(),
            ));
        }
    }
    Ok(())
}

/// Structural validation for `ProxyQuestion` (pure, no I/O).
pub fn validate_proxy_question(
    question: &ProxyQuestion,
) -> Result<(), ProxyQuestionValidationError> {
    if !is_supported_proxy_question_protocol(&question.protocol_version) {
        return Err(ProxyQuestionValidationError::UnsupportedProtocolVersion {
            found: question.protocol_version.clone(),
            expected: PROXY_QUESTION_PROTOCOL_VERSION,
        });
    }
    validate_proxy_question_id(&question.question_id)?;
    let normalized = normalize_proxy_question_text(&question.text);
    if normalized.is_empty() {
        return Err(ProxyQuestionValidationError::EmptyText);
    }
    if normalized.len() > MAX_PROXY_QUESTION_TEXT_BYTES {
        return Err(ProxyQuestionValidationError::TextTooLong {
            max: MAX_PROXY_QUESTION_TEXT_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyPromptBundle` (pure, no I/O).
pub fn validate_proxy_prompt_bundle(
    bundle: &ProxyPromptBundle,
) -> Result<(), ProxyPromptBundleValidationError> {
    if !is_supported_proxy_prompt_bundle_protocol(&bundle.protocol_version) {
        return Err(
            ProxyPromptBundleValidationError::UnsupportedProtocolVersion {
                found: bundle.protocol_version.clone(),
                expected: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
            },
        );
    }
    validate_proxy_prompt_field(
        &bundle.system_message,
        ProxyPromptBundleValidationError::EmptySystemMessage,
    )?;
    validate_proxy_prompt_field(
        &bundle.user_message,
        ProxyPromptBundleValidationError::EmptyUserMessage,
    )?;
    if bundle.context_json.trim().is_empty() {
        return Err(ProxyPromptBundleValidationError::EmptyContextJson);
    }
    if bundle.context_json.len() > MAX_PROXY_PROMPT_FIELD_BYTES {
        return Err(ProxyPromptBundleValidationError::FieldTooLong {
            max: MAX_PROXY_PROMPT_FIELD_BYTES,
        });
    }
    serde_json::from_str::<serde_json::Value>(&bundle.context_json)
        .map_err(|err| ProxyPromptBundleValidationError::InvalidContextJson(err.to_string()))?;
    Ok(())
}

/// Structural validation for `ProxyRuntimeRequest` (pure, no I/O).
pub fn validate_proxy_runtime_request(
    request: &ProxyRuntimeRequest,
) -> Result<(), ProxyRuntimeRequestValidationError> {
    validate_proxy_prompt_bundle(&request.prompt)
        .map_err(|err| ProxyRuntimeRequestValidationError::InvalidPromptBundle(err.to_string()))?;
    if request.timeout_ms == 0 {
        return Err(ProxyRuntimeRequestValidationError::ZeroTimeout);
    }
    if request.max_output_bytes == 0 {
        return Err(ProxyRuntimeRequestValidationError::ZeroMaxOutputBytes);
    }
    if request.max_output_bytes as usize > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyRuntimeRequestValidationError::MaxOutputBytesTooLarge {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyRuntimeOutput` (pure, no I/O).
pub fn validate_proxy_runtime_output(
    output: &ProxyRuntimeOutput,
) -> Result<(), ProxyRuntimeOutputValidationError> {
    if output.draft_text.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyDraftText);
    }
    if output.draft_text.len() > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyRuntimeOutputValidationError::DraftTextTooLong {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    if output.provider_id.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyProviderId);
    }
    if output.model_id.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyModelId);
    }
    if output.provider_id.len() > MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES {
        return Err(ProxyRuntimeOutputValidationError::ProviderIdTooLong {
            max: MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES,
        });
    }
    if output.model_id.len() > MAX_PROXY_RUNTIME_MODEL_ID_BYTES {
        return Err(ProxyRuntimeOutputValidationError::ModelIdTooLong {
            max: MAX_PROXY_RUNTIME_MODEL_ID_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for aggregate-only `ProxyDraftEvidenceSummary` (pure, no I/O).
pub fn validate_proxy_draft_evidence_summary(
    summary: &ProxyDraftEvidenceSummary,
) -> Result<(), ProxyDraftEvidenceSummaryValidationError> {
    if summary.source_counts.len() > MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::TooManySourceCategories {
                max: MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES,
            },
        );
    }
    for key in summary.source_counts.keys() {
        validate_proxy_evidence_source_category_key(key)?;
    }
    let _ = (summary.evidence_index_count, summary.secret_items_omitted);
    Ok(())
}

/// Structural validation for `ProxyDraftTraceMetadata` (pure, no I/O).
pub fn validate_proxy_draft_trace_metadata(
    trace: &ProxyDraftTraceMetadata,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if !is_supported_proxy_draft_trace_metadata_protocol(&trace.protocol_version) {
        return Err(
            ProxyDraftTraceMetadataValidationError::UnsupportedProtocolVersion {
                found: trace.protocol_version.clone(),
                expected: PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION,
            },
        );
    }
    validate_proxy_trace_workspace_id(&trace.workspace_id)?;
    validate_proxy_trace_profile_id(&trace.profile_id)?;
    validate_proxy_trace_profile_version(&trace.profile_version)?;
    validate_proxy_trace_context_pack_id(&trace.context_pack_id)?;
    validate_proxy_trace_build_inputs_hash(&trace.build_inputs_hash)?;
    validate_proxy_draft_evidence_summary(&trace.evidence_summary).map_err(|err| {
        ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(err.to_string())
    })?;
    Ok(())
}

/// Structural validation for `ProxyDraftRuntimeMetadata` (pure, no I/O).
pub fn validate_proxy_draft_runtime_metadata(
    runtime: &ProxyDraftRuntimeMetadata,
) -> Result<(), ProxyDraftRuntimeMetadataValidationError> {
    if runtime.runtime_kind.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyRuntimeKind);
    }
    if runtime.provider_id.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyProviderId);
    }
    if runtime.model_id.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyModelId);
    }
    if runtime.runtime_kind.len() > MAX_PROXY_RUNTIME_KIND_BYTES {
        return Err(
            ProxyDraftRuntimeMetadataValidationError::RuntimeKindTooLong {
                max: MAX_PROXY_RUNTIME_KIND_BYTES,
            },
        );
    }
    if runtime.provider_id.len() > MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES {
        return Err(
            ProxyDraftRuntimeMetadataValidationError::ProviderIdTooLong {
                max: MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES,
            },
        );
    }
    if runtime.model_id.len() > MAX_PROXY_RUNTIME_MODEL_ID_BYTES {
        return Err(ProxyDraftRuntimeMetadataValidationError::ModelIdTooLong {
            max: MAX_PROXY_RUNTIME_MODEL_ID_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyDraft` v1.0 (pure, no I/O).
pub fn validate_proxy_draft(draft: &ProxyDraft) -> Result<(), ProxyDraftValidationError> {
    if !is_supported_proxy_draft_protocol(&draft.protocol_version) {
        return Err(ProxyDraftValidationError::UnsupportedProtocolVersion {
            found: draft.protocol_version.clone(),
            expected: PROXY_DRAFT_PROTOCOL_VERSION,
        });
    }
    validate_proxy_question_id(&draft.question_id)
        .map_err(|err| ProxyDraftValidationError::InvalidQuestionId(err.to_string()))?;
    validate_utc_timestamp(&draft.generated_at)
        .map_err(ProxyDraftValidationError::InvalidGeneratedAt)?;
    if draft.classification != PROXY_DRAFT_CLASSIFICATION {
        return Err(ProxyDraftValidationError::InvalidClassification);
    }
    if draft.authority_notice != PROXY_DRAFT_AUTHORITY_NOTICE {
        return Err(ProxyDraftValidationError::InvalidAuthorityNotice);
    }
    if draft.execution_boundary != PROXY_DRAFT_EXECUTION_BOUNDARY {
        return Err(ProxyDraftValidationError::InvalidExecutionBoundary);
    }
    if draft.draft_text.trim().is_empty() {
        return Err(ProxyDraftValidationError::EmptyDraftText);
    }
    if draft.draft_text.len() > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyDraftValidationError::DraftTextTooLong {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    validate_proxy_draft_trace_metadata(&draft.trace)
        .map_err(|err| ProxyDraftValidationError::InvalidTraceMetadata(err.to_string()))?;
    validate_proxy_draft_runtime_metadata(&draft.runtime)
        .map_err(|err| ProxyDraftValidationError::InvalidRuntimeMetadata(err.to_string()))?;
    if draft.limitations.len() > MAX_PROXY_DRAFT_LIMITATIONS {
        return Err(ProxyDraftValidationError::TooManyLimitations {
            max: MAX_PROXY_DRAFT_LIMITATIONS,
        });
    }
    for limitation in &draft.limitations {
        if limitation.trim().is_empty() {
            return Err(ProxyDraftValidationError::EmptyLimitation);
        }
        if limitation.len() > MAX_PROXY_DRAFT_LIMITATION_BYTES {
            return Err(ProxyDraftValidationError::LimitationTooLong {
                max: MAX_PROXY_DRAFT_LIMITATION_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_proxy_prompt_field(
    value: &str,
    empty_error: ProxyPromptBundleValidationError,
) -> Result<(), ProxyPromptBundleValidationError> {
    if value.trim().is_empty() {
        return Err(empty_error);
    }
    if value.len() > MAX_PROXY_PROMPT_FIELD_BYTES {
        return Err(ProxyPromptBundleValidationError::FieldTooLong {
            max: MAX_PROXY_PROMPT_FIELD_BYTES,
        });
    }
    Ok(())
}

fn validate_proxy_evidence_source_category_key(
    key: &str,
) -> Result<(), ProxyDraftEvidenceSummaryValidationError> {
    if key.trim().is_empty() {
        return Err(ProxyDraftEvidenceSummaryValidationError::EmptySourceCategory);
    }
    if key.len() > MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::SourceCategoryTooLong {
                max: MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES,
            },
        );
    }
    if key.contains('/') || key.contains('\\') {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::InvalidSourceCategory(
                "source category must not contain path separators".into(),
            ),
        );
    }
    if looks_like_canonical_ref_key(key) {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::InvalidSourceCategory(
                "source category must not look like a canonical ref".into(),
            ),
        );
    }
    Ok(())
}

fn looks_like_canonical_ref_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.starts_with("openmesh://")
        || lowered.contains("://")
        || lowered.starts_with("context-pack-")
        || lowered.starts_with("proxy-q-")
        || lowered.starts_with("evt-")
        || lowered.starts_with("ref-")
}

fn validate_proxy_trace_workspace_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyWorkspaceId);
    }
    if value.len() > MAX_PROXY_TRACE_WORKSPACE_ID_BYTES {
        return Err(ProxyDraftTraceMetadataValidationError::WorkspaceIdTooLong {
            max: MAX_PROXY_TRACE_WORKSPACE_ID_BYTES,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "workspace_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_profile_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyProfileId);
    }
    if value.len() > MAX_PROXY_TRACE_PROFILE_ID_BYTES {
        return Err(ProxyDraftTraceMetadataValidationError::ProfileIdTooLong {
            max: MAX_PROXY_TRACE_PROFILE_ID_BYTES,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "profile_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_profile_version(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyProfileVersion);
    }
    if value.len() > MAX_PROXY_TRACE_PROFILE_VERSION_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::ProfileVersionTooLong {
                max: MAX_PROXY_TRACE_PROFILE_VERSION_BYTES,
            },
        );
    }
    Ok(())
}

fn validate_proxy_trace_context_pack_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyContextPackId);
    }
    if value.len() > MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::ContextPackIdTooLong {
                max: MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES,
            },
        );
    }
    if !value.starts_with("context-pack-") {
        return Err(ProxyDraftTraceMetadataValidationError::InvalidContextPackIdPrefix);
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "context_pack_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_build_inputs_hash(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyBuildInputsHash);
    }
    if value.len() > MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::BuildInputsHashTooLong {
                max: MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES,
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(
        id: &str,
        producer: ProducerRef,
        actor: ActorRef,
        kind: WorkSignalKind,
        correlation_hint: Option<&str>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> WorkSignal {
        WorkSignal {
            signal_id: id.to_string(),
            workspace_id: "ws-1".to_string(),
            producer,
            actor,
            kind,
            summary: format!("signal {id}"),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            evidence_refs,
            correlation_hint: correlation_hint.map(|s| s.to_string()),
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
        }
    }

    fn attachment(evidence_ref: EvidenceRef) -> EvidenceAttachment {
        EvidenceAttachment {
            evidence_ref,
            observed_at: None,
        }
    }

    fn minimal_event(
        kind: &str,
        summary: &str,
        evidence_refs: Vec<EvidenceRef>,
        timestamp: &str,
    ) -> WorkEvent {
        WorkEvent::new(
            "evt-test-1",
            "ws-1",
            kind,
            summary,
            evidence_refs.into_iter().map(attachment).collect(),
            timestamp,
        )
    }

    // Category A — Positive Representability Tests (execution plan §6/§9.D1).

    /// WEC-29: an agent's completion claim, a verification result, and a
    /// human's acceptance exist as three separately-attributed records linked
    /// by a shared correlation hint — not one record with a boolean flag.
    #[test]
    fn wec_29_claim_verification_and_human_acceptance_are_distinct_records() {
        let claim = signal(
            "s-claim",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![],
        );
        let verification = signal(
            "s-verify",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![EvidenceRef::FilePath("tests/output.log".into())],
        );
        let acceptance = signal(
            "s-accept",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            Some("corr-1"),
            vec![EvidenceRef::ProducerSignal("s-verify".into())],
        );

        // All three share the same correlation hint, so they're correlatable...
        assert_eq!(claim.correlation_hint, verification.correlation_hint);
        assert_eq!(verification.correlation_hint, acceptance.correlation_hint);
        // ...but remain three distinct records, not one record with a status flag.
        assert_ne!(claim.signal_id, verification.signal_id);
        assert_ne!(verification.signal_id, acceptance.signal_id);
        // Acceptance is distinctly human-attributed; the claim is not.
        assert!(matches!(acceptance.actor, ActorRef::Person(_)));
        assert!(!matches!(claim.actor, ActorRef::Person(_)));
    }

    /// CC-1: many signals may support one WorkEvent, and producer count does
    /// not determine event count — both a single event with several evidence
    /// refs and several single-evidence events must be constructible.
    #[test]
    fn cc_1_many_signals_can_support_one_event_or_several() {
        let refs: Vec<EvidenceRef> = (0..4)
            .map(|i| EvidenceRef::ProducerSignal(format!("s-{i}")))
            .collect();

        let one_event_many_refs = minimal_event(
            "bugfix",
            "clear_project fixed",
            refs.clone(),
            "2026-07-08T00:00:00Z",
        );
        assert_eq!(one_event_many_refs.evidence.len(), 4);

        let four_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| {
                minimal_event(
                    "bugfix",
                    "clear_project fixed",
                    vec![r],
                    "2026-07-08T00:00:00Z",
                )
            })
            .collect();
        assert_eq!(four_events.len(), 4);
        assert!(four_events.iter().all(|e| e.evidence.len() == 1));
    }

    /// WEC-26 + WEC-32: same-producer duplicate vs. cross-producer
    /// corroboration must be distinguishable data, not the same shape.
    #[test]
    fn wec_26_32_duplicate_vs_corroborating_signals_are_distinguishable() {
        let duplicate_a = signal(
            "s-dup-a",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        let duplicate_b = signal(
            "s-dup-b",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        assert_eq!(duplicate_a.producer, duplicate_b.producer);

        let corroborating_a = signal(
            "s-corr-a",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        let corroborating_b = signal(
            "s-corr-b",
            ProducerRef::Git,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        assert_ne!(corroborating_a.producer, corroborating_b.producer);
        assert_eq!(
            corroborating_a.correlation_hint,
            corroborating_b.correlation_hint
        );
    }

    /// WEC-15: a claim existing only in conversation, not yet backed by
    /// durable evidence, is representable as a WorkSignal with no evidence
    /// refs — distinct from (and never auto-promoted to) a WorkEvent.
    #[test]
    fn wec_15_conversation_only_claim_has_no_evidence_refs() {
        let conversation_only = signal(
            "s-convo",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            None,
            vec![],
        );
        assert!(conversation_only.evidence_refs.is_empty());
    }

    /// WEC-30: a genuinely ambiguous case (one event or six?) must remain
    /// representable either way — the contracts must not resolve ambiguity.
    #[test]
    fn wec_30_ambiguous_one_event_or_six_both_remain_constructible() {
        let refs: Vec<EvidenceRef> = (0..6)
            .map(|i| EvidenceRef::ProducerSignal(format!("round-{i}")))
            .collect();

        let one_event = minimal_event("stabilized", "0.1.2.5 stabilized", refs.clone(), "t");
        assert_eq!(one_event.evidence.len(), 6);

        let six_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| minimal_event("bugfix-round", "one closure round", vec![r], "t"))
            .collect();
        assert_eq!(six_events.len(), 6);
    }

    // Generic structural test (not a Classification Pack case). WEC-33
    // (Git-state evidence representability) is implemented in Dev Track 0.1.3.6
    // Checkpoint A — see `producer_contracts.rs`.
    #[test]
    fn evidence_ref_variants_construct_and_compare() {
        let a = EvidenceRef::FilePath("docs/overview.md".into());
        let b = EvidenceRef::FilePath("docs/overview.md".into());
        let c = EvidenceRef::ProducerSignal("s-1".into());
        let d = EvidenceRef::GitState(sample_git_state());
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(c, d);
    }

    fn sample_git_state() -> GitState {
        GitState {
            repo_id: "fnv1a-abc123".into(),
            branch: "main".into(),
            head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
            dirty: true,
            staged_count: 0,
            unstaged_count: 1,
            untracked_count: 0,
            changed_paths: vec!["crates/openmesh-core/src/domain.rs".into()],
            observed_at: "2026-07-16T04:30:00Z".into(),
            ahead: None,
            behind: None,
            base_ref: None,
            worktree_root: None,
        }
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.2, Checkpoint A — protocol contract stabilization.
    // ------------------------------------------------------------------

    /// Full WorkSignal round-trips through JSON, preserving every field.
    #[test]
    fn work_signal_round_trips_through_json() {
        let original = signal(
            "s-roundtrip",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Person("ter".into()),
            WorkSignalKind::Handoff,
            Some("corr-rt"),
            vec![
                EvidenceRef::FilePath("docs/overview.md".into()),
                EvidenceRef::ProducerSignal("s-other".into()),
            ],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: WorkSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.signal_id, original.signal_id);
        assert_eq!(restored.workspace_id, original.workspace_id);
        assert_eq!(restored.producer, original.producer);
        assert_eq!(restored.actor, original.actor);
        assert_eq!(restored.kind, original.kind);
        assert_eq!(restored.summary, original.summary);
        assert_eq!(restored.timestamp, original.timestamp);
        assert_eq!(restored.evidence_refs, original.evidence_refs);
        assert_eq!(restored.correlation_hint, original.correlation_hint);
        assert_eq!(restored.sensitivity, original.sensitivity);
        assert_eq!(restored.protocol_version, original.protocol_version);
    }

    /// Every data-carrying enum variant round-trips, including all unit variants.
    #[test]
    fn producer_ref_variants_round_trip() {
        for p in [
            ProducerRef::Native,
            ProducerRef::Heli,
            ProducerRef::Git,
            ProducerRef::Reporter("codex".into()),
        ] {
            let json = serde_json::to_string(&p).expect("serialize");
            let back: ProducerRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, p);
        }
    }

    #[test]
    fn actor_ref_variants_round_trip() {
        for a in [
            ActorRef::Person("ter".into()),
            ActorRef::Device("laptop-1".into()),
            ActorRef::Proxy("proxy-1".into()),
            ActorRef::Unknown,
        ] {
            let json = serde_json::to_string(&a).expect("serialize");
            let back: ActorRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, a);
        }
    }

    #[test]
    fn evidence_ref_variants_round_trip() {
        for e in [
            EvidenceRef::FilePath("docs/overview.md".into()),
            EvidenceRef::ProducerSignal("s-1".into()),
            EvidenceRef::GitState(sample_git_state()),
        ] {
            let json = serde_json::to_string(&e).expect("serialize");
            let back: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, e);
        }
    }

    #[test]
    fn work_signal_kind_variants_round_trip() {
        for k in [
            WorkSignalKind::Progress,
            WorkSignalKind::Decision,
            WorkSignalKind::Blocker,
            WorkSignalKind::BlockerResolved,
            WorkSignalKind::ScopeChange,
            WorkSignalKind::Milestone,
            WorkSignalKind::ReviewRequired,
            WorkSignalKind::UnresolvedQuestion,
            WorkSignalKind::Handoff,
            WorkSignalKind::SessionEnd,
            WorkSignalKind::AgentSwitch,
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            let back: WorkSignalKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, k);
        }
    }

    /// Locks the exact wire shape approved in the 0.1.3.2 execution plan §3.12:
    /// camelCase fields, kebab-case kind, lowercase sensitivity, adjacently
    /// tagged data-carrying enums. Any accidental future drift in field
    /// naming, casing, or enum representation fails this test immediately.
    #[test]
    fn deserializes_the_canonical_example_json_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!("{}/tests/fixtures/signals/valid.json", manifest_dir);
        let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
        let s: WorkSignal = serde_json::from_str(&json).expect("deserialize fixture");

        assert_eq!(s.signal_id, "codex-2026-07-08-001");
        assert_eq!(s.workspace_id, "1720400000000-a1b2c3");
        assert_eq!(s.producer, ProducerRef::Reporter("codex".into()));
        assert_eq!(s.actor, ActorRef::Unknown);
        assert_eq!(s.kind, WorkSignalKind::Progress);
        assert_eq!(
            s.summary,
            "Implemented the shared continuity foundation and migrated context/index/ingestion into openmesh-core."
        );
        assert_eq!(s.timestamp, "2026-07-08T09:15:00Z");
        assert_eq!(
            s.evidence_refs,
            vec![EvidenceRef::FilePath(
                "crates/openmesh-core/src/domain.rs".into()
            )]
        );
        assert_eq!(s.correlation_hint, Some("corr-0.1.3.1".to_string()));
        assert_eq!(s.sensitivity, Sensitivity::Private);
        assert_eq!(s.protocol_version, "1.0");
        assert_eq!(WORK_SIGNAL_PROTOCOL_VERSION, "1.0");
    }

    /// A missing `sensitivity` field must deserialize as `Private` — proving
    /// the `#[serde(default)]` field attribute actually wires in the enum's
    /// own `#[default]`, not merely relying on it existing (approved plan §3.9).
    #[test]
    fn missing_sensitivity_field_defaults_to_private() {
        let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "progress",
            "summary": "no sensitivity field here",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "protocolVersion": "1.0"
        }"#;
        let s: WorkSignal = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.sensitivity, Sensitivity::Private);
    }

    /// An unrecognized `kind` string under the *current* protocol version is
    /// invalid current-version data, not a future-version compatibility case
    /// — it must fail strict deserialization (approved plan §3.4/§10),
    /// mirroring `context.rs`'s existing `deserialize_invalid_kind_fails`.
    /// This is the direct evidence motivating the one-field (not per-enum)
    /// preflight the classifier implements in Checkpoint C.
    #[test]
    fn unrecognized_kind_fails_strict_deserialize() {
        let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "not-a-real-kind",
            "summary": "bad kind",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "sensitivity": "private",
            "protocolVersion": "1.0"
        }"#;
        let result: Result<WorkSignal, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unrecognized kind should fail deserialization"
        );
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.4, Checkpoint A — WorkEvent wire contract.
    // ------------------------------------------------------------------

    fn sample_event() -> WorkEvent {
        WorkEvent {
            event_id: "1783605120049-event001".into(),
            workspace_id: "1783586870822-7352d".into(),
            kind: "work.completed".into(),
            summary: "Canonical WorkEvent for Checkpoint A.".into(),
            timestamp: "2026-07-15T07:00:00Z".into(),
            evidence: vec![
                EvidenceAttachment {
                    evidence_ref: EvidenceRef::FilePath(
                        "crates/openmesh-core/src/domain.rs".into(),
                    ),
                    observed_at: Some("2026-07-15T07:00:01Z".into()),
                },
                EvidenceAttachment {
                    evidence_ref: EvidenceRef::ProducerSignal("s-verify".into()),
                    observed_at: None,
                },
            ],
            corrects_event_id: None,
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_EVENT_PROTOCOL_VERSION.to_string(),
            actor: None,
        }
    }

    #[test]
    fn work_event_round_trips_through_json() {
        let original = sample_event();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, original);
    }

    #[test]
    fn evidence_attachment_round_trips_through_json() {
        let original = EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: Some("2026-07-15T07:00:00Z".into()),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: EvidenceAttachment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, original);
    }

    #[test]
    fn deserializes_the_canonical_event_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!("{manifest_dir}/tests/fixtures/events/valid.json");
        let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
        let event: WorkEvent = serde_json::from_str(&json).expect("deserialize fixture");

        assert_eq!(event.event_id, "1783605120049-event001");
        assert_eq!(event.workspace_id, "1783586870822-7352d");
        assert_eq!(event.kind, "work.completed");
        assert_eq!(
            event.summary,
            "Canonical WorkEvent fixture for Dev Track 0.1.3.4 Checkpoint A."
        );
        assert_eq!(event.timestamp, "2026-07-15T07:00:00Z");
        assert_eq!(event.evidence.len(), 2);
        assert_eq!(
            event.evidence[0].evidence_ref,
            EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into())
        );
        assert_eq!(
            event.evidence[0].observed_at.as_deref(),
            Some("2026-07-15T07:00:01Z")
        );
        assert_eq!(
            event.evidence[1].evidence_ref,
            EvidenceRef::ProducerSignal("s-verify".into())
        );
        assert!(event.evidence[1].observed_at.is_none());
        assert_eq!(event.corrects_event_id, None);
        assert_eq!(event.sensitivity, Sensitivity::Private);
        assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
    }

    #[test]
    fn work_event_missing_sensitivity_is_rejected() {
        let json = r#"{
            "eventId": "evt-1",
            "workspaceId": "ws-1",
            "kind": "work.completed",
            "summary": "completed the task",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [
                { "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }
            ],
            "protocolVersion": "1.0"
        }"#;
        let result: Result<WorkEvent, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "missing sensitivity must fail strict WorkEvent deserialization"
        );
    }

    #[test]
    fn work_event_corrects_event_id_round_trips() {
        let mut event = sample_event();
        event.corrects_event_id = Some("evt-original".into());
        let json = serde_json::to_string(&event).expect("serialize");
        let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.corrects_event_id.as_deref(), Some("evt-original"));
    }

    #[test]
    fn validate_event_semantics_accepts_valid_event() {
        validate_event_semantics(&sample_event()).expect("valid event");
    }

    #[test]
    fn validate_event_semantics_rejects_empty_evidence() {
        let mut event = sample_event();
        event.evidence.clear();
        assert_eq!(
            validate_event_semantics(&event),
            Err(EventValidationError::EmptyEvidence)
        );
    }

    #[test]
    fn validate_event_semantics_rejects_invalid_timestamp() {
        let mut event = sample_event();
        event.timestamp = "2026-07-15T07:00:00-05:00".into();
        assert!(matches!(
            validate_event_semantics(&event),
            Err(EventValidationError::InvalidTimestamp(_))
        ));
    }

    #[test]
    fn validate_event_semantics_rejects_wrong_protocol_version() {
        let mut event = sample_event();
        event.protocol_version = "99.0".into();
        assert_eq!(
            validate_event_semantics(&event),
            Err(EventValidationError::UnsupportedProtocolVersion {
                found: "99.0".into(),
            })
        );
    }

    #[test]
    fn validate_event_semantics_rejects_invalid_observed_at() {
        let mut event = sample_event();
        event.evidence[0].observed_at = Some("2026-07-15T07:00:01-05:00".into());
        assert!(matches!(
            validate_event_semantics(&event),
            Err(EventValidationError::InvalidObservedAt(_))
        ));
    }

    /// Legacy `1.0` WorkEvents omit `actor` on wire; `1.1` promoted events require it.
    #[test]
    fn work_event_v1_0_serializes_without_actor_on_wire() {
        let json = serde_json::to_string(&sample_event()).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let obj = value.as_object().expect("object");
        assert!(!obj.contains_key("actor"));
    }

    /// EvidenceRef includes pointer-only variants — `FilePath`, `ProducerSignal`,
    /// and bounded `GitState` metadata (no source/diff bodies).
    #[test]
    fn evidence_ref_includes_git_state_variant() {
        let variants = [
            EvidenceRef::FilePath("path".into()),
            EvidenceRef::ProducerSignal("s-1".into()),
            EvidenceRef::GitState(sample_git_state()),
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let restored: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored, variant);
        }
        match EvidenceRef::GitState(sample_git_state()) {
            EvidenceRef::FilePath(_)
            | EvidenceRef::ProducerSignal(_)
            | EvidenceRef::GitState(_) => {}
        }
    }
}
