//! Dev Track 0.1.5 Checkpoint D — Proxy Context Pack policy validation (pure, no I/O).

use crate::context::Sensitivity;
use crate::context_pack_selection::canonical_evidence_ref_key;
use crate::domain::{
    deterministic_context_pack_id, is_supported_proxy_context_pack_protocol,
    is_supported_work_proxy_profile_version, validate_proxy_context_pack, validate_utc_timestamp,
    ContextPackAuthoritySummary, ContextPackCatchUp, ContextPackContinuityItem,
    ContextPackCurrentState, ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin,
    ContextPackFreshness, ContextPackItemProvenance, ContextPackRedactionSummary,
    ContextPackUnresolvedCategory, ContinuitySourceKind, PendingAttentionReason, ProxyContextPack,
    CONTEXT_PACK_EXECUTION_BOUNDARY, MAX_CONTEXT_PACK_DIAGNOSTICS, MAX_CONTEXT_PACK_EVIDENCE_INDEX,
    MAX_CONTEXT_PACK_LIMITATIONS, MAX_CONTEXT_PACK_UNRESOLVED_ITEMS,
};
use crate::domain::{
    ContextPackValidationError as StructuralValidationError, PendingAttentionStatus,
};

/// Inspectable pure validation error for Proxy Context Pack policy and safety checks.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextPackValidationError {
    #[error("unsupported protocol version")]
    UnsupportedProtocolVersion,
    #[error("invalid identity field")]
    InvalidIdentity,
    #[error("invalid build_inputs_hash format")]
    InvalidHashFormat,
    #[error("context_pack_id does not match build_inputs_hash")]
    ContextPackIdMismatch,
    #[error("invalid timestamp")]
    InvalidTimestamp,
    #[error("invalid catch-up window")]
    InvalidWindow,
    #[error("freshness metadata mismatch")]
    FreshnessMismatch,
    #[error("invalid authority summary")]
    InvalidAuthoritySummary,
    #[error("invalid privacy summary")]
    InvalidPrivacySummary,
    #[error("secret content detected on pack surface")]
    SecretContentDetected,
    #[error("unknown or ambiguous evidence sensitivity")]
    UnknownSensitivity,
    #[error("invalid evidence index")]
    InvalidEvidenceIndex,
    #[error("duplicate evidence identity")]
    DuplicateEvidenceIdentity,
    #[error("invalid provenance")]
    InvalidProvenance,
    #[error("invalid correction metadata")]
    InvalidCorrectionMetadata,
    #[error("collection bounds exceeded")]
    BoundsExceeded,
    #[error("limitations must not be empty")]
    MissingLimitations,
    #[error("invalid redaction summary")]
    InvalidRedactionSummary,
    #[error("forbidden runtime surface detected")]
    ForbiddenRuntimeSurface,
    #[error("structural validation failed")]
    Structural(StructuralValidationError),
}

impl ContextPackValidationError {
    /// Stable machine-readable category for CLI and diagnostics.
    pub fn category(&self) -> &'static str {
        match self {
            Self::UnsupportedProtocolVersion => "unsupported_protocol_version",
            Self::InvalidIdentity => "invalid_identity",
            Self::InvalidHashFormat => "invalid_hash_format",
            Self::ContextPackIdMismatch => "context_pack_id_mismatch",
            Self::InvalidTimestamp => "invalid_timestamp",
            Self::InvalidWindow => "invalid_window",
            Self::FreshnessMismatch => "freshness_mismatch",
            Self::InvalidAuthoritySummary => "invalid_authority_summary",
            Self::InvalidPrivacySummary => "invalid_privacy_summary",
            Self::SecretContentDetected => "secret_content_detected",
            Self::UnknownSensitivity => "unknown_sensitivity",
            Self::InvalidEvidenceIndex => "invalid_evidence_index",
            Self::DuplicateEvidenceIdentity => "duplicate_evidence_identity",
            Self::InvalidProvenance => "invalid_provenance",
            Self::InvalidCorrectionMetadata => "invalid_correction_metadata",
            Self::BoundsExceeded => "bounds_exceeded",
            Self::MissingLimitations => "missing_limitations",
            Self::InvalidRedactionSummary => "invalid_redaction_summary",
            Self::ForbiddenRuntimeSurface => "forbidden_runtime_surface",
            Self::Structural(_) => "structural_validation_failed",
        }
    }
}

/// Policy and safety validation layered on top of structural contracts.
pub fn validate_proxy_context_pack_policy(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    validate_protocol_and_identity(pack)?;
    validate_hash_and_pack_id(pack)?;
    validate_window_and_freshness(pack)?;
    validate_authority_policy(&pack.authority_summary)?;
    validate_privacy_surfaces(pack)?;
    validate_evidence_index_policy(&pack.evidence_index)?;
    validate_provenance_policy(pack)?;
    validate_correction_policy(pack)?;
    validate_bounds_policy(pack)?;
    validate_redaction_policy(&pack.redaction_summary)?;
    validate_no_forbidden_runtime_surfaces(pack)?;
    Ok(())
}

/// Complete validation: structural contracts plus policy/safety semantics.
pub fn validate_proxy_context_pack_complete(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    if let Err(err) = validate_proxy_context_pack(pack) {
        return Err(ContextPackValidationError::Structural(err));
    }
    validate_proxy_context_pack_policy(pack)
}

fn validate_protocol_and_identity(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    if !is_supported_proxy_context_pack_protocol(&pack.protocol_version) {
        return Err(ContextPackValidationError::UnsupportedProtocolVersion);
    }
    if pack.workspace_id.trim().is_empty()
        || pack.profile_id.trim().is_empty()
        || pack.context_pack_id.trim().is_empty()
        || pack.build_inputs_hash.trim().is_empty()
        || pack.owner_identity.owner_label.trim().is_empty()
    {
        return Err(ContextPackValidationError::InvalidIdentity);
    }
    if !is_supported_work_proxy_profile_version(&pack.profile_version) {
        return Err(ContextPackValidationError::InvalidIdentity);
    }
    for field in [
        &pack.owner_identity.owner_label,
        &pack.owner_identity.role_label,
    ] {
        if contains_impersonation_claim(field) {
            return Err(ContextPackValidationError::InvalidIdentity);
        }
    }
    Ok(())
}

fn validate_hash_and_pack_id(pack: &ProxyContextPack) -> Result<(), ContextPackValidationError> {
    if !is_valid_build_inputs_hash_format(&pack.build_inputs_hash) {
        return Err(ContextPackValidationError::InvalidHashFormat);
    }
    if !is_valid_context_pack_id_format(&pack.context_pack_id) {
        return Err(ContextPackValidationError::InvalidIdentity);
    }
    if pack.context_pack_id != deterministic_context_pack_id(&pack.build_inputs_hash) {
        return Err(ContextPackValidationError::ContextPackIdMismatch);
    }
    Ok(())
}

fn is_valid_build_inputs_hash_format(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("fnv1a-") else {
        return false;
    };
    hex.len() == 16
        && hex
            .chars()
            .all(|ch| ch.is_ascii_hexdigit() && !ch.is_uppercase())
}

fn is_valid_context_pack_id_format(value: &str) -> bool {
    let Some(hash) = value.strip_prefix("context-pack-fnv1a-") else {
        return false;
    };
    is_valid_build_inputs_hash_format(&format!("fnv1a-{hash}"))
}

fn validate_window_and_freshness(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    for ts in [
        &pack.generated_at,
        &pack.requested_window.since,
        &pack.requested_window.until,
    ] {
        validate_utc_timestamp(ts).map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    }
    let since = chrono::DateTime::parse_from_rfc3339(&pack.requested_window.since)
        .map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    let until = chrono::DateTime::parse_from_rfc3339(&pack.requested_window.until)
        .map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    if since > until {
        return Err(ContextPackValidationError::InvalidWindow);
    }
    validate_freshness_policy(&pack.freshness, &pack.requested_window, &pack.generated_at)
}

fn validate_freshness_policy(
    freshness: &ContextPackFreshness,
    window: &crate::domain::CatchUpWindow,
    generated_at: &str,
) -> Result<(), ContextPackValidationError> {
    for ts in [
        &freshness.snapshot_observed_at,
        &freshness.current_state_generated_at,
        &freshness.catch_up_since,
        &freshness.catch_up_until,
        &freshness.pack_generated_at,
    ] {
        validate_utc_timestamp(ts).map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    }
    if freshness.catch_up_since != window.since || freshness.catch_up_until != window.until {
        return Err(ContextPackValidationError::FreshnessMismatch);
    }
    if freshness.pack_generated_at != generated_at {
        return Err(ContextPackValidationError::FreshnessMismatch);
    }
    let observed = chrono::DateTime::parse_from_rfc3339(&freshness.snapshot_observed_at)
        .map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    let generated = chrono::DateTime::parse_from_rfc3339(&freshness.pack_generated_at)
        .map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
    let expected_age = generated
        .signed_duration_since(observed)
        .num_seconds()
        .max(0) as u64;
    if freshness.age_seconds != expected_age {
        return Err(ContextPackValidationError::FreshnessMismatch);
    }
    for warning in &freshness.warnings {
        if contains_secret_like_value(warning) {
            return Err(ContextPackValidationError::SecretContentDetected);
        }
    }
    Ok(())
}

fn validate_authority_policy(
    summary: &ContextPackAuthoritySummary,
) -> Result<(), ContextPackValidationError> {
    if summary.execution_boundary != CONTEXT_PACK_EXECUTION_BOUNDARY {
        return Err(ContextPackValidationError::InvalidAuthoritySummary);
    }
    let expected = crate::domain::proxy_context_pack_authority_ladder_levels();
    if summary.ladder_levels.len() != expected.len()
        || summary
            .ladder_levels
            .iter()
            .map(String::as_str)
            .ne(expected.iter().copied())
    {
        return Err(ContextPackValidationError::InvalidAuthoritySummary);
    }
    if summary.default_refusal_rules.is_empty() || summary.authority_rules.is_empty() {
        return Err(ContextPackValidationError::InvalidAuthoritySummary);
    }
    for rule in summary
        .authority_rules
        .iter()
        .flat_map(|rule| rule.description.iter().chain(rule.limitations.iter()))
    {
        if contains_forbidden_runtime_text(rule) {
            return Err(ContextPackValidationError::ForbiddenRuntimeSurface);
        }
    }
    for rule in &summary.default_refusal_rules {
        if contains_forbidden_runtime_text(&rule.statement) {
            return Err(ContextPackValidationError::ForbiddenRuntimeSurface);
        }
    }
    Ok(())
}

fn validate_privacy_surfaces(pack: &ProxyContextPack) -> Result<(), ContextPackValidationError> {
    scan_pack_surfaces_for_secrets(pack)?;
    for entry in &pack.privacy_summary.filtering_applied {
        if contains_secret_like_value(entry) {
            return Err(ContextPackValidationError::SecretContentDetected);
        }
    }
    Ok(())
}

fn scan_pack_surfaces_for_secrets(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    for item in all_continuity_items(&pack.current_state, &pack.catch_up) {
        scan_text_surface(&item.summary)?;
        scan_text_surface(&item.kind)?;
        for evidence in &item.evidence_refs {
            scan_evidence_ref(evidence)?;
        }
        if let Some(correction) = &item.correction {
            for event_id in &correction.correction_event_ids {
                scan_text_surface(event_id)?;
            }
        }
    }
    for item in pack
        .current_state
        .pending_attention
        .iter()
        .chain(pack.catch_up.next_suggested_attention.iter())
    {
        scan_text_surface(&item.summary)?;
        for evidence in &item.evidence_refs {
            scan_evidence_ref(evidence)?;
        }
    }
    for entry in &pack.evidence_index {
        if entry.sensitivity == Sensitivity::Secret {
            return Err(ContextPackValidationError::SecretContentDetected);
        }
        if !is_allowed_pack_sensitivity(&entry.sensitivity) {
            return Err(ContextPackValidationError::UnknownSensitivity);
        }
        scan_text_surface(&entry.label)?;
        scan_evidence_ref(&entry.evidence_ref)?;
    }
    for diagnostic in &pack.diagnostics {
        scan_text_surface(&diagnostic.code)?;
        scan_text_surface(&diagnostic.message)?;
    }
    for limitation in &pack.limitations {
        scan_text_surface(limitation)?;
    }
    for item in &pack.unresolved_items {
        scan_text_surface(&item.id)?;
        scan_text_surface(&item.summary)?;
    }
    for warning in &pack.freshness.warnings {
        scan_text_surface(warning)?;
    }
    Ok(())
}

fn is_allowed_pack_sensitivity(sensitivity: &Sensitivity) -> bool {
    matches!(
        sensitivity,
        Sensitivity::Public | Sensitivity::Team | Sensitivity::Private
    )
}

fn scan_text_surface(value: &str) -> Result<(), ContextPackValidationError> {
    if contains_secret_like_value(value) {
        return Err(ContextPackValidationError::SecretContentDetected);
    }
    Ok(())
}

fn scan_evidence_ref(
    evidence: &crate::domain::EvidenceRef,
) -> Result<(), ContextPackValidationError> {
    let encoded = serde_json::to_string(evidence)
        .map_err(|_| ContextPackValidationError::InvalidEvidenceIndex)?;
    if contains_secret_like_value(&encoded) {
        return Err(ContextPackValidationError::SecretContentDetected);
    }
    Ok(())
}

fn validate_evidence_index_policy(
    entries: &[ContextPackEvidenceIndexEntry],
) -> Result<(), ContextPackValidationError> {
    if entries.len() > MAX_CONTEXT_PACK_EVIDENCE_INDEX {
        return Err(ContextPackValidationError::BoundsExceeded);
    }
    let mut seen_ref_ids = std::collections::BTreeSet::new();
    let mut seen_canonical = std::collections::BTreeSet::new();
    for (index, entry) in entries.iter().enumerate() {
        if entry.origin != ContextPackEvidenceOrigin::ContinuityItem {
            return Err(ContextPackValidationError::InvalidEvidenceIndex);
        }
        if entry.sensitivity == Sensitivity::Secret
            || !is_allowed_pack_sensitivity(&entry.sensitivity)
        {
            return Err(ContextPackValidationError::UnknownSensitivity);
        }
        let expected_ref_id = format!("ref-{:03}", index + 1);
        if entry.ref_id != expected_ref_id {
            return Err(ContextPackValidationError::InvalidEvidenceIndex);
        }
        if !seen_ref_ids.insert(entry.ref_id.clone()) {
            return Err(ContextPackValidationError::DuplicateEvidenceIdentity);
        }
        let canonical = canonical_evidence_ref_key(&entry.evidence_ref)
            .map_err(|_| ContextPackValidationError::InvalidEvidenceIndex)?;
        if !seen_canonical.insert(canonical) {
            return Err(ContextPackValidationError::DuplicateEvidenceIdentity);
        }
        if let Some(timestamp) = &entry.timestamp {
            validate_utc_timestamp(timestamp)
                .map_err(|_| ContextPackValidationError::InvalidTimestamp)?;
        }
        if entry.label.eq_ignore_ascii_case("diagnostic-only") {
            return Err(ContextPackValidationError::InvalidEvidenceIndex);
        }
    }
    Ok(())
}

fn validate_provenance_policy(pack: &ProxyContextPack) -> Result<(), ContextPackValidationError> {
    for item in all_continuity_items(&pack.current_state, &pack.catch_up) {
        if item.source == ContinuitySourceKind::PendingSignal
            && item.provenance == ContextPackItemProvenance::Confirmed
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
        if item.provenance == ContextPackItemProvenance::DiagnosticOnly
            && item.source == ContinuitySourceKind::WorkEvent
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
    }
    for item in pack
        .current_state
        .pending_attention
        .iter()
        .chain(pack.catch_up.next_suggested_attention.iter())
    {
        if matches!(
            item.provenance,
            ContextPackItemProvenance::Confirmed | ContextPackItemProvenance::DiagnosticOnly
        ) && item.reason == PendingAttentionReason::PendingSignal
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
        if item.provenance == ContextPackItemProvenance::Confirmed
            && item.status != PendingAttentionStatus::Resolved
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
    }
    for item in &pack.unresolved_items {
        if item.category == ContextPackUnresolvedCategory::Unconfirmed
            && item.provenance == ContextPackItemProvenance::Confirmed
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
        if item.category == ContextPackUnresolvedCategory::Pending
            && item.provenance == ContextPackItemProvenance::Confirmed
        {
            return Err(ContextPackValidationError::InvalidProvenance);
        }
    }
    Ok(())
}

fn validate_correction_policy(pack: &ProxyContextPack) -> Result<(), ContextPackValidationError> {
    for item in all_continuity_items(&pack.current_state, &pack.catch_up) {
        if let Some(correction) = &item.correction {
            if correction.is_superseded_original {
                return Err(ContextPackValidationError::InvalidCorrectionMetadata);
            }
            if correction.is_corrected && correction.correction_event_ids.is_empty() {
                return Err(ContextPackValidationError::InvalidCorrectionMetadata);
            }
            if correction.is_superseded_original && correction.is_corrected {
                return Err(ContextPackValidationError::InvalidCorrectionMetadata);
            }
        }
    }
    Ok(())
}

fn validate_bounds_policy(pack: &ProxyContextPack) -> Result<(), ContextPackValidationError> {
    if pack.evidence_index.len() > MAX_CONTEXT_PACK_EVIDENCE_INDEX
        || pack.diagnostics.len() > MAX_CONTEXT_PACK_DIAGNOSTICS
        || pack.limitations.len() > MAX_CONTEXT_PACK_LIMITATIONS
        || pack.unresolved_items.len() > MAX_CONTEXT_PACK_UNRESOLVED_ITEMS
    {
        return Err(ContextPackValidationError::BoundsExceeded);
    }
    if pack.limitations.is_empty() {
        return Err(ContextPackValidationError::MissingLimitations);
    }
    let mut seen_diagnostics = std::collections::BTreeSet::new();
    for diagnostic in &pack.diagnostics {
        if !seen_diagnostics.insert(diagnostic.code.clone()) {
            return Err(ContextPackValidationError::BoundsExceeded);
        }
    }
    let mut seen_limitations = std::collections::BTreeSet::new();
    for limitation in &pack.limitations {
        let normalized = limitation.trim();
        if normalized.is_empty() {
            return Err(ContextPackValidationError::MissingLimitations);
        }
        if !seen_limitations.insert(normalized.to_string()) {
            return Err(ContextPackValidationError::BoundsExceeded);
        }
    }
    Ok(())
}

fn validate_redaction_policy(
    _summary: &ContextPackRedactionSummary,
) -> Result<(), ContextPackValidationError> {
    Ok(())
}

fn validate_no_forbidden_runtime_surfaces(
    pack: &ProxyContextPack,
) -> Result<(), ContextPackValidationError> {
    let encoded = serde_json::to_string(pack)
        .map_err(|_| ContextPackValidationError::ForbiddenRuntimeSurface)?;
    let lowered = encoded.to_ascii_lowercase();
    for marker in [
        "\"answertext\"",
        "\"generatedanswer\"",
        "\"approvalresult\"",
        "\"executedauthority\"",
        "\"humanconfirmed\"",
        "\"contextdocument\"",
        "\"documentannex\"",
        "\"contextindex\"",
        "\"agentcontextenabled\"",
        "\"strictfreshness\"",
        "\"authoritydecision\"",
    ] {
        if lowered.contains(marker) {
            return Err(ContextPackValidationError::ForbiddenRuntimeSurface);
        }
    }
    Ok(())
}

fn contains_forbidden_runtime_text(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    [
        "generated answer",
        "approval result",
        "executed authority",
        "human confirmed",
        "i already answered",
        "proxy answer text",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn contains_impersonation_claim(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    [
        "i am the human",
        "i am the owner",
        "speak as the human",
        "speak as the owner",
        "this proxy is the human",
        "this proxy is the owner",
        "impersonate the owner",
        "impersonate the human",
    ]
    .iter()
    .any(|phrase| normalized.contains(phrase))
}

fn contains_secret_like_value(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    [
        "api_key=",
        "apikey=",
        "password=",
        "secret=",
        "token=",
        "bearer ",
        "sk-live-",
        "sk-test-",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn all_continuity_items<'a>(
    current_state: &'a ContextPackCurrentState,
    catch_up: &'a ContextPackCatchUp,
) -> Vec<&'a ContextPackContinuityItem> {
    let sections = &current_state.sections;
    let mut items: Vec<&ContextPackContinuityItem> = sections
        .completed
        .iter()
        .chain(sections.in_progress.iter())
        .chain(sections.blocked.iter())
        .chain(sections.decisions.iter())
        .chain(sections.needs_attention.iter())
        .chain(sections.still_open.iter())
        .collect();
    let catch_up_sections = &catch_up.sections;
    items.extend(
        catch_up_sections
            .completed
            .iter()
            .chain(catch_up_sections.changed.iter())
            .chain(catch_up_sections.blocked.iter())
            .chain(catch_up_sections.decided.iter())
            .chain(catch_up_sections.needs_attention.iter())
            .chain(catch_up_sections.still_open.iter()),
    );
    items
}
