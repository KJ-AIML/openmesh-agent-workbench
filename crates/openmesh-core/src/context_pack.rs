//! Dev Track 0.1.5 Checkpoint C — Proxy Context Pack composition (read-only project builder).

use crate::context::Sensitivity;
use crate::context_pack_selection::{
    build_context_pack_evidence_index, sanitize_context_pack_continuity_items,
    sanitize_context_pack_pending_items, ContextPackContinuityItemInput,
    ContextPackEvidenceCandidate, ContextPackEvidenceCandidateKind,
    ContextPackEvidenceSelectionOptions, ContextPackPendingAttentionItemInput,
};
use crate::context_pack_validation::validate_proxy_context_pack_complete;
use crate::continuity::{
    build_catch_up_view, build_current_state_projection, load_continuity_input_snapshot,
    ContinuityInputSnapshot, ContinuityReaderError,
};
use crate::domain::{
    deterministic_context_pack_id, effective_presentation,
    proxy_context_pack_authority_ladder_levels, validate_utc_timestamp, CatchUpSections,
    CatchUpView, CatchUpWindow, ContextPackAuthoritySummary, ContextPackCatchUp,
    ContextPackCatchUpSections, ContextPackCorrectionProvenance, ContextPackCurrentState,
    ContextPackCurrentStateSections, ContextPackDiagnostic, ContextPackDiagnosticSeverity,
    ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin, ContextPackFreshness,
    ContextPackItemProvenance, ContextPackOwnerIdentity, ContextPackPrivacySummary,
    ContextPackRedactionSummary, ContextPackUnresolvedCategory, ContextPackUnresolvedItem,
    ProxyContextPack, SourceCounts, WorkProxyProfile, CONTEXT_PACK_EXECUTION_BOUNDARY,
    MAX_CONTEXT_PACK_DIAGNOSTICS, MAX_CONTEXT_PACK_LIMITATIONS, MAX_CONTEXT_PACK_UNRESOLVED_ITEMS,
    PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use crate::domain::{
    ContinuitySourceKind, ContinuityStateItem, CurrentStateProjection, CurrentStateSections,
    EvidenceRef, PendingAttentionItem, PrivacyAllowedUse,
};
use crate::profile::{read_work_proxy_profile, ProfileError};

use serde::Serialize;
use std::collections::BTreeSet;

/// Deterministic builder options for Proxy Context Pack composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyContextPackBuildOptions {
    pub generated_at: String,
    pub selection: ContextPackEvidenceSelectionOptions,
}

impl Default for ProxyContextPackBuildOptions {
    fn default() -> Self {
        Self {
            generated_at: "2026-07-18T04:00:00Z".into(),
            selection: ContextPackEvidenceSelectionOptions::default(),
        }
    }
}

/// Already-loaded values for pure Proxy Context Pack composition.
#[derive(Debug, Clone)]
pub struct ProxyContextPackComposeInputs {
    pub profile: WorkProxyProfile,
    pub snapshot: ContinuityInputSnapshot,
    pub current_state: CurrentStateProjection,
    pub catch_up: CatchUpView,
    pub window: CatchUpWindow,
    pub generated_at: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextPackBuildError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("work proxy profile is missing")]
    ProfileMissing,
    #[error("profile error: {0}")]
    Profile(String),
    #[error("continuity snapshot error: {0}")]
    ContinuitySnapshot(String),
    #[error("current state build failed: {0}")]
    CurrentStateBuild(String),
    #[error("catch-up build failed: {0}")]
    CatchUpBuild(String),
    #[error("selection failed: {0}")]
    Selection(String),
    #[error("invalid catch-up window: {0}")]
    InvalidWindow(String),
    #[error("pack validation failed: {0}")]
    PackValidation(String),
    #[error("serialization failed: {0}")]
    Serialization(String),
}

/// Pure composition from already-loaded values.
pub fn compose_proxy_context_pack(
    inputs: &ProxyContextPackComposeInputs,
    options: &ProxyContextPackBuildOptions,
) -> Result<ProxyContextPack, ContextPackBuildError> {
    validate_window(&inputs.window)?;
    validate_utc_timestamp(&inputs.generated_at).map_err(ContextPackBuildError::InvalidWindow)?;
    validate_utc_timestamp(&inputs.current_state.generated_at)
        .map_err(ContextPackBuildError::CurrentStateBuild)?;
    validate_utc_timestamp(&inputs.catch_up.generated_at)
        .map_err(ContextPackBuildError::CatchUpBuild)?;

    if inputs.profile.workspace_id != inputs.snapshot.workspace_id {
        return Err(ContextPackBuildError::Profile(
            "profile workspace_id does not match continuity snapshot".into(),
        ));
    }

    let identity = assemble_pack_semantic_identity(inputs, options)?;
    let build_inputs_hash = hash_pack_semantic_identity(
        &inputs.profile,
        &inputs.snapshot,
        &inputs.window,
        options,
        &identity,
    )?;
    let context_pack_id = deterministic_context_pack_id(&build_inputs_hash);

    let freshness = build_freshness(
        &inputs.snapshot,
        &inputs.current_state,
        &inputs.window,
        &inputs.generated_at,
    )?;

    let pack = ProxyContextPack {
        context_pack_id,
        workspace_id: inputs.profile.workspace_id.clone(),
        profile_id: inputs.profile.profile_id.clone(),
        profile_version: inputs.profile.profile_version.clone(),
        protocol_version: PROXY_CONTEXT_PACK_PROTOCOL_VERSION.to_string(),
        generated_at: inputs.generated_at.clone(),
        requested_window: inputs.window.clone(),
        owner_identity: ContextPackOwnerIdentity {
            owner_label: inputs.profile.owner_label.clone(),
            role_label: inputs.profile.role_label.clone(),
        },
        communication_preferences: inputs.profile.communication_preferences.clone(),
        decision_preferences: inputs.profile.decision_preferences.clone(),
        authority_summary: ContextPackAuthoritySummary {
            authority_rules: inputs.profile.authority_rules.clone(),
            default_refusal_rules: inputs.profile.default_refusal_rules.clone(),
            ladder_levels: proxy_context_pack_authority_ladder_levels()
                .into_iter()
                .map(str::to_string)
                .collect(),
            execution_boundary: CONTEXT_PACK_EXECUTION_BOUNDARY.to_string(),
        },
        privacy_summary: ContextPackPrivacySummary {
            privacy_rules: inputs.profile.privacy_rules.clone(),
            sensitive_topics: inputs.profile.sensitive_topics.clone(),
            filtering_applied: filtering_applied_tags(&identity.redaction_summary),
        },
        evidence_policy: inputs.profile.evidence_policy.clone(),
        current_state: identity.current_state.state,
        catch_up: identity.catch_up.view,
        evidence_index: identity.evidence_index,
        source_counts: inputs.snapshot.source_counts.clone(),
        diagnostics: identity.diagnostics,
        limitations: identity.limitations,
        unresolved_items: identity.unresolved_items,
        freshness,
        redaction_summary: identity.redaction_summary,
        build_inputs_hash,
    };

    validate_proxy_context_pack_complete(&pack)
        .map_err(|err| ContextPackBuildError::PackValidation(err.category().to_string()))?;
    Ok(pack)
}

/// Read-only project-level builder. Performs no writes.
pub fn build_proxy_context_pack(
    project_path: &str,
    window: CatchUpWindow,
    options: ProxyContextPackBuildOptions,
) -> Result<ProxyContextPack, ContextPackBuildError> {
    let profile = read_work_proxy_profile(project_path).map_err(map_profile_error)?;
    let snapshot = load_continuity_input_snapshot(project_path).map_err(map_reader_error)?;
    let current_state = build_current_state_projection(&snapshot)
        .map_err(|err| ContextPackBuildError::CurrentStateBuild(err.to_string()))?;
    let catch_up = build_catch_up_view(&snapshot, &current_state, &window)
        .map_err(|err| ContextPackBuildError::CatchUpBuild(err.to_string()))?;

    compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window,
            generated_at: options.generated_at.clone(),
        },
        &options,
    )
}

struct SanitizedCurrentState {
    state: ContextPackCurrentState,
    redaction: ContextPackRedactionSummary,
    diagnostics: Vec<ContextPackDiagnostic>,
}

struct SanitizedCatchUp {
    view: ContextPackCatchUp,
    redaction: ContextPackRedactionSummary,
    diagnostics: Vec<ContextPackDiagnostic>,
}

fn sanitize_current_state(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    projection: &CurrentStateProjection,
) -> Result<SanitizedCurrentState, ContextPackBuildError> {
    let sections = sanitize_state_sections(profile, snapshot, &projection.sections)?;
    let pending_inputs: Vec<_> = projection
        .pending_attention
        .iter()
        .map(|item| pending_attention_to_input(profile, snapshot, item))
        .collect();
    let pending = sanitize_context_pack_pending_items(&pending_inputs)
        .map_err(|err| ContextPackBuildError::Selection(err.to_string()))?;

    let mut redaction = sections.redaction.clone();
    merge_redaction_into(&mut redaction, &pending.redaction_summary);
    let mut diagnostics = sections.diagnostics.clone();
    diagnostics.extend(pending.diagnostics.clone());
    diagnostics = bound_diagnostics(diagnostics);

    let limitations = merge_limitation_vecs(
        &projection.limitations,
        &["context pack metadata only; no answering runtime in 0.1.5".to_string()],
    );

    Ok(SanitizedCurrentState {
        state: ContextPackCurrentState {
            workspace_id: projection.workspace_id.clone(),
            sections: sections.sections,
            pending_attention: pending.items,
            limitations,
        },
        redaction,
        diagnostics,
    })
}

fn sanitize_catch_up(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    view: &CatchUpView,
    window: &CatchUpWindow,
) -> Result<SanitizedCatchUp, ContextPackBuildError> {
    let sections = sanitize_catch_up_sections(profile, snapshot, &view.sections)?;
    let pending_inputs: Vec<_> = view
        .next_suggested_attention
        .iter()
        .map(|item| pending_attention_to_input(profile, snapshot, item))
        .collect();
    let pending = sanitize_context_pack_pending_items(&pending_inputs)
        .map_err(|err| ContextPackBuildError::Selection(err.to_string()))?;

    let mut redaction = sections.redaction.clone();
    merge_redaction_into(&mut redaction, &pending.redaction_summary);
    let mut diagnostics = sections.diagnostics.clone();
    diagnostics.extend(pending.diagnostics.clone());
    diagnostics = bound_diagnostics(diagnostics);

    let limitations = merge_limitation_vecs(
        &view.limitations,
        &["context pack metadata only; no answering runtime in 0.1.5".to_string()],
    );

    Ok(SanitizedCatchUp {
        view: ContextPackCatchUp {
            workspace_id: view.workspace_id.clone(),
            window: window.clone(),
            sections: sections.sections,
            summary: view.summary.clone(),
            next_suggested_attention: pending.items,
            limitations,
        },
        redaction,
        diagnostics,
    })
}

struct SanitizedSections<T> {
    sections: T,
    redaction: ContextPackRedactionSummary,
    diagnostics: Vec<ContextPackDiagnostic>,
}

fn sanitize_state_sections(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    sections: &CurrentStateSections,
) -> Result<SanitizedSections<ContextPackCurrentStateSections>, ContextPackBuildError> {
    let mut aggregate = empty_redaction();
    let mut diagnostics = Vec::new();

    let mut sanitize_vec = |items: &[ContinuityStateItem]| -> Result<_, ContextPackBuildError> {
        let inputs: Vec<_> = items
            .iter()
            .map(|item| continuity_item_to_input(profile, snapshot, item))
            .collect();
        let result = sanitize_context_pack_continuity_items(&inputs)
            .map_err(|err| ContextPackBuildError::Selection(err.to_string()))?;
        merge_redaction_into(&mut aggregate, &result.redaction_summary);
        diagnostics.extend(result.diagnostics);
        Ok(result.items)
    };

    Ok(SanitizedSections {
        sections: ContextPackCurrentStateSections {
            completed: sanitize_vec(&sections.completed)?,
            in_progress: sanitize_vec(&sections.in_progress)?,
            blocked: sanitize_vec(&sections.blocked)?,
            decisions: sanitize_vec(&sections.decisions)?,
            needs_attention: sanitize_vec(&sections.needs_attention)?,
            still_open: sanitize_vec(&sections.still_open)?,
        },
        redaction: aggregate,
        diagnostics: bound_diagnostics(diagnostics),
    })
}

fn sanitize_catch_up_sections(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    sections: &CatchUpSections,
) -> Result<SanitizedSections<ContextPackCatchUpSections>, ContextPackBuildError> {
    let mut aggregate = empty_redaction();
    let mut diagnostics = Vec::new();

    let mut sanitize_vec = |items: &[ContinuityStateItem]| -> Result<_, ContextPackBuildError> {
        let inputs: Vec<_> = items
            .iter()
            .map(|item| continuity_item_to_input(profile, snapshot, item))
            .collect();
        let result = sanitize_context_pack_continuity_items(&inputs)
            .map_err(|err| ContextPackBuildError::Selection(err.to_string()))?;
        merge_redaction_into(&mut aggregate, &result.redaction_summary);
        diagnostics.extend(result.diagnostics);
        Ok(result.items)
    };

    Ok(SanitizedSections {
        sections: ContextPackCatchUpSections {
            completed: sanitize_vec(&sections.completed)?,
            changed: sanitize_vec(&sections.changed)?,
            blocked: sanitize_vec(&sections.blocked)?,
            decided: sanitize_vec(&sections.decided)?,
            needs_attention: sanitize_vec(&sections.needs_attention)?,
            still_open: sanitize_vec(&sections.still_open)?,
        },
        redaction: aggregate,
        diagnostics: bound_diagnostics(diagnostics),
    })
}

fn empty_redaction() -> ContextPackRedactionSummary {
    ContextPackRedactionSummary {
        secret_items_omitted: 0,
        policy_restricted_items_omitted: 0,
        malformed_items_omitted: 0,
        quarantined_items_omitted: 0,
        bounds_truncated_items: 0,
    }
}

fn continuity_item_to_input(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    item: &ContinuityStateItem,
) -> ContextPackContinuityItemInput {
    let resolved_sensitivity = sensitivity_for_item(snapshot, item);
    let unknown_sensitivity = resolved_sensitivity.is_none();
    let sensitivity = resolved_sensitivity.unwrap_or(Sensitivity::Private);
    ContextPackContinuityItemInput {
        id: item.id.clone(),
        summary: item.summary.clone(),
        kind: item.kind.clone(),
        source: item.source,
        provenance: provenance_for_item(item),
        timestamp: item.timestamp.clone(),
        evidence_refs: item.evidence_refs.clone(),
        confidence: item.confidence,
        unverified: item.unverified,
        correction: correction_for_item(snapshot, item),
        sensitivity: sensitivity.clone(),
        policy_restricted: unknown_sensitivity || is_policy_restricted(profile, item, &sensitivity),
        malformed: unknown_sensitivity,
        quarantined: false,
    }
}

fn pending_attention_to_input(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    item: &PendingAttentionItem,
) -> ContextPackPendingAttentionItemInput {
    let pseudo = ContinuityStateItem {
        id: item.id.clone(),
        summary: item.summary.clone(),
        kind: "pending.attention".into(),
        source: item.source,
        source_id: item.source_id.clone(),
        producer: String::new(),
        timestamp: item.timestamp.clone(),
        evidence_refs: item.evidence_refs.clone(),
        confidence: crate::domain::ContinuityConfidence::Medium,
        correlation_hint: None,
        unverified: Some(true),
    };
    let resolved_sensitivity = sensitivity_for_pending_attention(snapshot, item);
    let unknown_sensitivity = resolved_sensitivity.is_none();
    let sensitivity = resolved_sensitivity.unwrap_or(Sensitivity::Private);
    ContextPackPendingAttentionItemInput {
        id: item.id.clone(),
        summary: item.summary.clone(),
        reason: item.reason,
        provenance: provenance_for_pending(item),
        timestamp: item.timestamp.clone(),
        status: item.status,
        severity: item.severity,
        priority: item.priority,
        evidence_refs: item.evidence_refs.clone(),
        sensitivity: sensitivity.clone(),
        policy_restricted: unknown_sensitivity
            || is_policy_restricted(profile, &pseudo, &sensitivity),
        malformed: unknown_sensitivity,
        quarantined: false,
    }
}

fn provenance_for_item(item: &ContinuityStateItem) -> ContextPackItemProvenance {
    if item.source == ContinuitySourceKind::PendingSignal {
        return ContextPackItemProvenance::Pending;
    }
    if item.unverified == Some(true) {
        return ContextPackItemProvenance::Unconfirmed;
    }
    ContextPackItemProvenance::Confirmed
}

fn provenance_for_pending(item: &PendingAttentionItem) -> ContextPackItemProvenance {
    if item.source == ContinuitySourceKind::PendingSignal {
        ContextPackItemProvenance::Pending
    } else if item.reason == crate::domain::PendingAttentionReason::AmbiguousPromotion {
        ContextPackItemProvenance::Unconfirmed
    } else {
        ContextPackItemProvenance::Pending
    }
}

fn sensitivity_for_item(
    snapshot: &ContinuityInputSnapshot,
    item: &ContinuityStateItem,
) -> Option<Sensitivity> {
    match item.source {
        ContinuitySourceKind::WorkEvent => snapshot
            .work_events
            .iter()
            .find(|event| event.event_id == item.source_id)
            .map(|event| event.sensitivity.clone()),
        ContinuitySourceKind::ProcessedSignal | ContinuitySourceKind::PendingSignal => snapshot
            .processed_signals
            .iter()
            .chain(snapshot.pending_signals.iter())
            .find(|signal| signal.signal_id == item.source_id)
            .map(|signal| signal.sensitivity.clone()),
        ContinuitySourceKind::PromotionAudit => snapshot
            .promotion_audit_records
            .iter()
            .find(|record| record.promotion_key.as_str() == item.source_id)
            .and_then(|record| {
                record.source_signal_ids.first().and_then(|signal_id| {
                    snapshot
                        .processed_signals
                        .iter()
                        .chain(snapshot.pending_signals.iter())
                        .find(|signal| signal.signal_id == *signal_id)
                        .map(|signal| signal.sensitivity.clone())
                })
            }),
    }
}

fn sensitivity_for_evidence_ref(
    snapshot: &ContinuityInputSnapshot,
    evidence_ref: &EvidenceRef,
) -> Option<Sensitivity> {
    let mut found: Option<Sensitivity> = None;
    for event in &snapshot.work_events {
        if event
            .evidence
            .iter()
            .any(|attachment| &attachment.evidence_ref == evidence_ref)
        {
            let sensitivity = event.sensitivity.clone();
            if let Some(existing) = &found {
                if *existing != sensitivity {
                    return None;
                }
            } else {
                found = Some(sensitivity);
            }
        }
    }
    for signal in snapshot
        .processed_signals
        .iter()
        .chain(snapshot.pending_signals.iter())
    {
        if signal
            .evidence_refs
            .iter()
            .any(|candidate| candidate == evidence_ref)
        {
            let sensitivity = signal.sensitivity.clone();
            if let Some(existing) = &found {
                if *existing != sensitivity {
                    return None;
                }
            } else {
                found = Some(sensitivity);
            }
        }
    }
    found
}

fn sensitivity_for_pending_attention(
    snapshot: &ContinuityInputSnapshot,
    item: &PendingAttentionItem,
) -> Option<Sensitivity> {
    match item.source {
        ContinuitySourceKind::WorkEvent => snapshot
            .work_events
            .iter()
            .find(|event| event.event_id == item.source_id)
            .map(|event| event.sensitivity.clone()),
        ContinuitySourceKind::ProcessedSignal | ContinuitySourceKind::PendingSignal => snapshot
            .processed_signals
            .iter()
            .chain(snapshot.pending_signals.iter())
            .find(|signal| signal.signal_id == item.source_id)
            .map(|signal| signal.sensitivity.clone()),
        ContinuitySourceKind::PromotionAudit => snapshot
            .promotion_audit_records
            .iter()
            .find(|record| record.promotion_key.as_str() == item.source_id)
            .and_then(|record| {
                record.source_signal_ids.first().and_then(|signal_id| {
                    snapshot
                        .processed_signals
                        .iter()
                        .chain(snapshot.pending_signals.iter())
                        .find(|signal| signal.signal_id == *signal_id)
                        .map(|signal| signal.sensitivity.clone())
                })
            }),
    }
}

fn is_policy_restricted(
    profile: &WorkProxyProfile,
    item: &ContinuityStateItem,
    sensitivity: &Sensitivity,
) -> bool {
    if *sensitivity == Sensitivity::Secret {
        return true;
    }
    let topic = format!("{}.{}", item.kind, item.summary);
    profile.privacy_rules.iter().any(|rule| {
        (topic.contains(&rule.topic) || rule.topic == "*")
            && rule.allowed_use == PrivacyAllowedUse::ExcludeFromAnswers
    })
}

fn correction_for_item(
    snapshot: &ContinuityInputSnapshot,
    item: &ContinuityStateItem,
) -> Option<ContextPackCorrectionProvenance> {
    if item.source != ContinuitySourceKind::WorkEvent {
        return None;
    }
    let presentation = effective_presentation(&snapshot.work_events, &item.source_id)?;
    if presentation.is_superseded_original {
        return None;
    }
    if !presentation.is_corrected && presentation.correction_event_ids.is_empty() {
        return None;
    }
    Some(ContextPackCorrectionProvenance {
        is_corrected: presentation.is_corrected,
        is_superseded_original: false,
        correction_event_ids: presentation.correction_event_ids,
        superseded_by_event_id: presentation.superseded_by_event_id,
    })
}

fn build_evidence_candidates(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    catch_up: &CatchUpView,
) -> Vec<ContextPackEvidenceCandidate> {
    let mut candidates = Vec::new();
    for item in all_state_items(current_state) {
        push_item_candidates(profile, snapshot, item, &mut candidates);
    }
    for item in all_catch_up_items(catch_up) {
        push_item_candidates(profile, snapshot, item, &mut candidates);
    }
    for evidence_ref in current_state
        .evidence_refs
        .iter()
        .chain(catch_up.evidence_refs.iter())
    {
        let Some(sensitivity) = sensitivity_for_evidence_ref(snapshot, evidence_ref) else {
            continue;
        };
        if sensitivity == Sensitivity::Secret {
            candidates.push(ContextPackEvidenceCandidate {
                evidence_ref: evidence_ref.clone(),
                origin: ContextPackEvidenceOrigin::ContinuityItem,
                sensitivity,
                safe_label: "continuity evidence".into(),
                timestamp: None,
                provenance: ContextPackItemProvenance::Confirmed,
                correction: None,
                policy_eligible: false,
                kind: ContextPackEvidenceCandidateKind::Normal,
            });
            continue;
        }
        candidates.push(ContextPackEvidenceCandidate {
            evidence_ref: evidence_ref.clone(),
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity,
            safe_label: "continuity evidence".into(),
            timestamp: None,
            provenance: ContextPackItemProvenance::Confirmed,
            correction: None,
            policy_eligible: true,
            kind: ContextPackEvidenceCandidateKind::Normal,
        });
    }
    candidates.sort_by(|left, right| {
        crate::context_pack_selection::canonical_evidence_ref_key(&left.evidence_ref)
            .unwrap_or_default()
            .cmp(
                &crate::context_pack_selection::canonical_evidence_ref_key(&right.evidence_ref)
                    .unwrap_or_default(),
            )
    });
    candidates
}

fn push_item_candidates(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    item: &ContinuityStateItem,
    out: &mut Vec<ContextPackEvidenceCandidate>,
) {
    let input = continuity_item_to_input(profile, snapshot, item);
    if input.sensitivity == Sensitivity::Secret || input.policy_restricted {
        return;
    }
    let provenance = input.provenance;
    let correction = input.correction.clone();
    for evidence_ref in &item.evidence_refs {
        out.push(ContextPackEvidenceCandidate {
            evidence_ref: evidence_ref.clone(),
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity: input.sensitivity.clone(),
            safe_label: "continuity evidence".into(),
            timestamp: Some(item.timestamp.clone()),
            provenance,
            correction: correction.clone(),
            policy_eligible: !input.policy_restricted,
            kind: ContextPackEvidenceCandidateKind::Normal,
        });
    }
}

fn all_state_items(projection: &CurrentStateProjection) -> Vec<&ContinuityStateItem> {
    let sections = &projection.sections;
    sections
        .completed
        .iter()
        .chain(sections.in_progress.iter())
        .chain(sections.blocked.iter())
        .chain(sections.decisions.iter())
        .chain(sections.needs_attention.iter())
        .chain(sections.still_open.iter())
        .collect()
}

fn all_catch_up_items(view: &CatchUpView) -> Vec<&ContinuityStateItem> {
    let sections = &view.sections;
    sections
        .completed
        .iter()
        .chain(sections.changed.iter())
        .chain(sections.blocked.iter())
        .chain(sections.decided.iter())
        .chain(sections.needs_attention.iter())
        .chain(sections.still_open.iter())
        .collect()
}

struct PackSemanticIdentityParts {
    current_state: SanitizedCurrentState,
    catch_up: SanitizedCatchUp,
    evidence_index: Vec<ContextPackEvidenceIndexEntry>,
    redaction_summary: ContextPackRedactionSummary,
    diagnostics: Vec<ContextPackDiagnostic>,
    limitations: Vec<String>,
    unresolved_items: Vec<ContextPackUnresolvedItem>,
}

/// Canonical pack-safe semantic material hashed for deterministic build identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BuildInputsSemanticFingerprint {
    protocol_version: String,
    profile: WorkProxyProfile,
    since: String,
    until: String,
    selection: ContextPackEvidenceSelectionOptions,
    current_state: ContextPackCurrentState,
    catch_up: ContextPackCatchUp,
    evidence_index: Vec<ContextPackEvidenceIndexEntry>,
    source_counts: SourceCounts,
    diagnostics: Vec<ContextPackDiagnostic>,
    limitations: Vec<String>,
    unresolved_items: Vec<ContextPackUnresolvedItem>,
    redaction_summary: ContextPackRedactionSummary,
}

fn assemble_pack_semantic_identity(
    inputs: &ProxyContextPackComposeInputs,
    options: &ProxyContextPackBuildOptions,
) -> Result<PackSemanticIdentityParts, ContextPackBuildError> {
    let current_state =
        sanitize_current_state(&inputs.profile, &inputs.snapshot, &inputs.current_state)?;
    let catch_up = sanitize_catch_up(
        &inputs.profile,
        &inputs.snapshot,
        &inputs.catch_up,
        &inputs.window,
    )?;

    let evidence_candidates = build_evidence_candidates(
        &inputs.profile,
        &inputs.snapshot,
        &inputs.current_state,
        &inputs.catch_up,
    );
    let evidence = build_context_pack_evidence_index(&evidence_candidates, &options.selection)
        .map_err(|err| ContextPackBuildError::Selection(err.to_string()))?;

    let redaction_summary = merge_redaction(
        &current_state.redaction,
        &catch_up.redaction,
        &evidence.redaction_summary,
    );
    let mut diagnostics = merge_diagnostics(
        &inputs.snapshot,
        &current_state.diagnostics,
        &catch_up.diagnostics,
        &evidence.diagnostics,
    );
    let limitations = merge_limitations(&inputs.profile, &inputs.current_state, &inputs.catch_up);
    let unresolved_items =
        build_unresolved_items(&inputs.current_state, &inputs.catch_up, &inputs.snapshot);

    if redaction_summary.secret_items_omitted > 0
        && !diagnostics
            .iter()
            .any(|diag| diag.code == "secret-evidence-omitted")
    {
        push_diagnostic(
            &mut diagnostics,
            "secret-evidence-omitted",
            "secret evidence omitted from context pack surfaces",
            ContextPackDiagnosticSeverity::Info,
        );
    }

    Ok(PackSemanticIdentityParts {
        current_state,
        catch_up,
        evidence_index: evidence.evidence_index,
        redaction_summary,
        diagnostics,
        limitations,
        unresolved_items,
    })
}

fn hash_pack_semantic_identity(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    window: &CatchUpWindow,
    options: &ProxyContextPackBuildOptions,
    parts: &PackSemanticIdentityParts,
) -> Result<String, ContextPackBuildError> {
    let fingerprint = BuildInputsSemanticFingerprint {
        protocol_version: PROXY_CONTEXT_PACK_PROTOCOL_VERSION.to_string(),
        profile: profile.clone(),
        since: window.since.clone(),
        until: window.until.clone(),
        selection: options.selection.clone(),
        current_state: parts.current_state.state.clone(),
        catch_up: parts.catch_up.view.clone(),
        evidence_index: parts.evidence_index.clone(),
        source_counts: snapshot.source_counts.clone(),
        diagnostics: parts.diagnostics.clone(),
        limitations: parts.limitations.clone(),
        unresolved_items: parts.unresolved_items.clone(),
        redaction_summary: parts.redaction_summary.clone(),
    };
    let material = serde_json::to_string(&fingerprint)
        .map_err(|err| ContextPackBuildError::Serialization(err.to_string()))?;
    Ok(format!("fnv1a-{}", fnv1a_hex(&material)))
}

pub fn compute_build_inputs_hash(
    profile: &WorkProxyProfile,
    snapshot: &ContinuityInputSnapshot,
    window: &CatchUpWindow,
    options: &ProxyContextPackBuildOptions,
) -> Result<String, ContextPackBuildError> {
    validate_window(window)?;
    if profile.workspace_id != snapshot.workspace_id {
        return Err(ContextPackBuildError::Profile(
            "profile workspace_id does not match continuity snapshot".into(),
        ));
    }
    let current_state = build_current_state_projection(snapshot)
        .map_err(|err| ContextPackBuildError::CurrentStateBuild(err.to_string()))?;
    let catch_up = build_catch_up_view(snapshot, &current_state, window)
        .map_err(|err| ContextPackBuildError::CatchUpBuild(err.to_string()))?;
    let inputs = ProxyContextPackComposeInputs {
        profile: profile.clone(),
        snapshot: snapshot.clone(),
        current_state,
        catch_up,
        window: window.clone(),
        generated_at: options.generated_at.clone(),
    };
    let identity = assemble_pack_semantic_identity(&inputs, options)?;
    hash_pack_semantic_identity(profile, snapshot, window, options, &identity)
}

fn build_freshness(
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    window: &CatchUpWindow,
    generated_at: &str,
) -> Result<ContextPackFreshness, ContextPackBuildError> {
    let observed = chrono::DateTime::parse_from_rfc3339(&snapshot.loaded_at)
        .map_err(|err| ContextPackBuildError::Serialization(err.to_string()))?;
    let generated = chrono::DateTime::parse_from_rfc3339(generated_at)
        .map_err(|err| ContextPackBuildError::Serialization(err.to_string()))?;
    let age_seconds = generated
        .signed_duration_since(observed)
        .num_seconds()
        .max(0) as u64;
    Ok(ContextPackFreshness {
        snapshot_observed_at: snapshot.loaded_at.clone(),
        current_state_generated_at: current_state.generated_at.clone(),
        catch_up_since: window.since.clone(),
        catch_up_until: window.until.clone(),
        pack_generated_at: generated_at.to_string(),
        age_seconds,
        warnings: Vec::new(),
    })
}

fn build_unresolved_items(
    current_state: &CurrentStateProjection,
    catch_up: &CatchUpView,
    snapshot: &ContinuityInputSnapshot,
) -> Vec<ContextPackUnresolvedItem> {
    let mut items = Vec::new();
    for pending in current_state
        .pending_attention
        .iter()
        .chain(catch_up.next_suggested_attention.iter())
    {
        if items.len() >= MAX_CONTEXT_PACK_UNRESOLVED_ITEMS {
            break;
        }
        items.push(ContextPackUnresolvedItem {
            id: pending.id.clone(),
            category: if pending.source == ContinuitySourceKind::PendingSignal {
                ContextPackUnresolvedCategory::Pending
            } else {
                ContextPackUnresolvedCategory::Unconfirmed
            },
            summary: truncate_unresolved_summary(&pending.summary),
            provenance: provenance_for_pending(pending),
        });
    }
    if !snapshot.quarantine_signals.is_empty() && items.len() < MAX_CONTEXT_PACK_UNRESOLVED_ITEMS {
        items.push(ContextPackUnresolvedItem {
            id: "unresolved-quarantine".into(),
            category: ContextPackUnresolvedCategory::Quarantine,
            summary: "quarantined continuity inputs present".into(),
            provenance: ContextPackItemProvenance::DiagnosticOnly,
        });
    }
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn merge_limitations(
    profile: &WorkProxyProfile,
    current_state: &CurrentStateProjection,
    catch_up: &CatchUpView,
) -> Vec<String> {
    let mut merged = merge_limitation_vecs(&profile.limitations, &current_state.limitations);
    merged = merge_limitation_vecs(&merged, &catch_up.limitations);
    merged = merge_limitation_vecs(
        &merged,
        &["context pack metadata only; no answering runtime in 0.1.5".to_string()],
    );
    merged.truncate(MAX_CONTEXT_PACK_LIMITATIONS);
    merged
}

fn merge_limitation_vecs(base: &[String], extra: &[String]) -> Vec<String> {
    let mut set: BTreeSet<String> = BTreeSet::new();
    for entry in base.iter().chain(extra.iter()) {
        let trimmed = entry.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }
    set.into_iter().collect()
}

fn merge_diagnostics(
    snapshot: &ContinuityInputSnapshot,
    current: &[ContextPackDiagnostic],
    catch_up: &[ContextPackDiagnostic],
    selection: &[ContextPackDiagnostic],
) -> Vec<ContextPackDiagnostic> {
    let mut diagnostics = Vec::new();
    if !snapshot.diagnostics.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "continuity-reader",
            "continuity reader reported non-fatal diagnostics",
            ContextPackDiagnosticSeverity::Info,
        );
    }
    for diag in current
        .iter()
        .chain(catch_up.iter())
        .chain(selection.iter())
    {
        if diagnostics.len() >= MAX_CONTEXT_PACK_DIAGNOSTICS {
            break;
        }
        if !diagnostics
            .iter()
            .any(|existing| existing.code == diag.code)
        {
            diagnostics.push(diag.clone());
        }
    }
    bound_diagnostics(diagnostics)
}

fn merge_redaction(
    left: &ContextPackRedactionSummary,
    middle: &ContextPackRedactionSummary,
    right: &ContextPackRedactionSummary,
) -> ContextPackRedactionSummary {
    let mut merged = left.clone();
    merge_redaction_into(&mut merged, middle);
    merge_redaction_into(&mut merged, right);
    merged
}

fn merge_redaction_into(
    target: &mut ContextPackRedactionSummary,
    other: &ContextPackRedactionSummary,
) {
    target.secret_items_omitted = target
        .secret_items_omitted
        .saturating_add(other.secret_items_omitted);
    target.policy_restricted_items_omitted = target
        .policy_restricted_items_omitted
        .saturating_add(other.policy_restricted_items_omitted);
    target.malformed_items_omitted = target
        .malformed_items_omitted
        .saturating_add(other.malformed_items_omitted);
    target.quarantined_items_omitted = target
        .quarantined_items_omitted
        .saturating_add(other.quarantined_items_omitted);
    target.bounds_truncated_items = target
        .bounds_truncated_items
        .saturating_add(other.bounds_truncated_items);
}

fn filtering_applied_tags(redaction: &ContextPackRedactionSummary) -> Vec<String> {
    let mut tags = Vec::new();
    if redaction.secret_items_omitted > 0 {
        tags.push("secret-evidence-omitted".into());
    }
    if redaction.policy_restricted_items_omitted > 0 {
        tags.push("policy-restricted-omitted".into());
    }
    if redaction.bounds_truncated_items > 0 {
        tags.push("bounds-truncated".into());
    }
    tags
}

fn bound_diagnostics(mut diagnostics: Vec<ContextPackDiagnostic>) -> Vec<ContextPackDiagnostic> {
    diagnostics.sort_by(|left, right| left.code.cmp(&right.code));
    diagnostics.truncate(MAX_CONTEXT_PACK_DIAGNOSTICS);
    diagnostics
}

fn push_diagnostic(
    diagnostics: &mut Vec<ContextPackDiagnostic>,
    code: &str,
    message: &str,
    severity: ContextPackDiagnosticSeverity,
) {
    if diagnostics.len() >= MAX_CONTEXT_PACK_DIAGNOSTICS {
        return;
    }
    diagnostics.push(ContextPackDiagnostic {
        code: code.into(),
        message: message.into(),
        severity,
    });
}

fn truncate_unresolved_summary(summary: &str) -> String {
    const MAX: usize = 512;
    if summary.len() <= MAX {
        summary.to_string()
    } else {
        summary.chars().take(MAX).collect()
    }
}

fn validate_window(window: &CatchUpWindow) -> Result<(), ContextPackBuildError> {
    validate_utc_timestamp(&window.since).map_err(ContextPackBuildError::InvalidWindow)?;
    validate_utc_timestamp(&window.until).map_err(ContextPackBuildError::InvalidWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|err| ContextPackBuildError::InvalidWindow(err.to_string()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|err| ContextPackBuildError::InvalidWindow(err.to_string()))?;
    if since > until {
        return Err(ContextPackBuildError::InvalidWindow(
            "since must be <= until".into(),
        ));
    }
    Ok(())
}

fn fnv1a_hex(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

fn map_profile_error(err: ProfileError) -> ContextPackBuildError {
    match err {
        ProfileError::ProfileMissing => ContextPackBuildError::ProfileMissing,
        ProfileError::ProjectNotInitialized(path) => {
            ContextPackBuildError::ProjectNotInitialized(path)
        }
        other => ContextPackBuildError::Profile(other.to_string()),
    }
}

fn map_reader_error(err: ContinuityReaderError) -> ContextPackBuildError {
    match err {
        ContinuityReaderError::ProjectNotInitialized(path) => {
            ContextPackBuildError::ProjectNotInitialized(path)
        }
        other => ContextPackBuildError::ContinuitySnapshot(other.to_string()),
    }
}
