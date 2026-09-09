use super::corrections::ContinuityConfidence;
use super::events::{validate_utc_timestamp, EvidenceRef};
use super::profile::{
    contains_impersonation_claim, contains_secret_like_value, validate_authority_rule,
    validate_bounded_text, validate_communication_preferences, validate_decision_preferences,
    validate_evidence_policy, validate_privacy_rule, AuthorityRule, CommunicationPreferences,
    DecisionPreferences, DefaultRefusalRule, EvidencePolicy, PrivacyRule, MAX_PROFILE_LABEL_BYTES,
    MAX_PROFILE_TEXT_BYTES,
};
use super::projections::{
    validate_source_counts, CatchUpWindow, ContinuitySourceKind, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, SourceCounts, MAX_CATCH_UP_SUMMARY_BYTES,
    MAX_CONTINUITY_ITEM_EVIDENCE_REFS, MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES,
    MAX_LIMITATION_BYTES, MAX_NEXT_SUGGESTED_ATTENTION, MAX_PENDING_ATTENTION_PRIORITY,
    MIN_PENDING_ATTENTION_PRIORITY,
};
use super::signals::validate_evidence_ref;
use crate::context::Sensitivity;
use serde::{Deserialize, Serialize};

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
