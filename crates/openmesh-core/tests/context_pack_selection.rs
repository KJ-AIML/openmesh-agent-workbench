//! Dev Track 0.1.5 Checkpoint B — deterministic context selection tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_selection::{
    build_context_pack_evidence_index, canonical_evidence_ref_key,
    sanitize_context_pack_continuity_items, sanitize_context_pack_pending_items,
    ContextPackContinuityItemInput, ContextPackEvidenceCandidate, ContextPackEvidenceCandidateKind,
    ContextPackEvidenceSelectionOptions, ContextPackPendingAttentionItemInput,
    ContextPackSelectionError,
};
use openmesh_core::domain::{
    ContextPackCorrectionProvenance, ContextPackEvidenceOrigin, ContextPackItemProvenance,
    ContinuityConfidence, ContinuitySourceKind, EvidenceRef, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, MAX_CONTEXT_PACK_DIAGNOSTICS,
    MAX_CONTEXT_PACK_EVIDENCE_INDEX,
};
use std::fs;
use std::path::PathBuf;

fn file_candidate(path: &str, label: &str) -> ContextPackEvidenceCandidate {
    ContextPackEvidenceCandidate {
        evidence_ref: EvidenceRef::FilePath(path.into()),
        origin: ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: Sensitivity::Private,
        safe_label: label.into(),
        timestamp: Some("2026-07-17T10:00:00Z".into()),
        provenance: ContextPackItemProvenance::Confirmed,
        correction: None,
        policy_eligible: true,
        kind: ContextPackEvidenceCandidateKind::Normal,
    }
}

fn build(
    candidates: &[ContextPackEvidenceCandidate],
) -> openmesh_core::context_pack_selection::ContextPackEvidenceSelectionResult {
    build_context_pack_evidence_index(candidates, &ContextPackEvidenceSelectionOptions::default())
        .expect("selection succeeds")
}

#[test]
fn evidence_selection_is_deterministic_across_input_order() {
    let a = file_candidate("docs/a.md", "alpha");
    let b = file_candidate("docs/b.md", "beta");
    let first = build(&[a.clone(), b.clone()]);
    let second = build(&[b, a]);
    assert_eq!(first, second);
}

#[test]
fn evidence_selection_deduplicates_identical_refs() {
    let candidate = file_candidate("docs/readme.md", "one");
    let result = build(&[candidate.clone(), candidate]);
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(result.redaction_summary.secret_items_omitted, 0);
}

#[test]
fn evidence_selection_assigns_stable_ref_ids_after_sorting() {
    let result = build(&[
        file_candidate("docs/z.md", "z"),
        file_candidate("docs/a.md", "a"),
    ]);
    assert_eq!(result.evidence_index[0].ref_id, "ref-001");
    assert_eq!(result.evidence_index[1].ref_id, "ref-002");
    assert_eq!(
        result.evidence_index[0].evidence_ref,
        EvidenceRef::FilePath("docs/a.md".into())
    );
}

#[test]
fn evidence_selection_contains_continuity_origins_only() {
    let result = build(&[file_candidate("docs/readme.md", "readme")]);
    assert_eq!(
        result.evidence_index[0].origin,
        ContextPackEvidenceOrigin::ContinuityItem
    );
}

#[test]
fn evidence_selection_omits_secret_candidates_completely() {
    let mut secret = file_candidate("docs/secret.md", "secret");
    secret.sensitivity = Sensitivity::Secret;
    let result = build(&[secret]);
    assert!(result.evidence_index.is_empty());
}

#[test]
fn secret_omission_retains_no_ref_label_path_or_timestamp() {
    let mut secret = file_candidate("secrets/credentials.env", "credential path");
    secret.sensitivity = Sensitivity::Secret;
    let result = build(&[secret]);
    let json = serde_json::to_string(&result.evidence_index).expect("serialize")
        + &serde_json::to_string(&result.redaction_summary).expect("serialize")
        + &serde_json::to_string(&result.diagnostics).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in ["credentials", "credential path", "2026-07-17", "secrets/"] {
        assert!(!lowered.contains(forbidden), "leaked {forbidden}");
    }
}

#[test]
fn secret_omission_increments_aggregate_count_only() {
    let mut secret = file_candidate("docs/secret.md", "secret");
    secret.sensitivity = Sensitivity::Secret;
    let result = build(&[secret]);
    assert_eq!(result.redaction_summary.secret_items_omitted, 1);
    assert_eq!(result.diagnostics.len(), 0);
}

#[test]
fn policy_restricted_candidate_is_omitted_without_identity_leak() {
    let mut restricted = file_candidate("docs/private.md", "private topic");
    restricted.policy_eligible = false;
    let result = build(&[restricted]);
    assert!(result.evidence_index.is_empty());
    assert_eq!(result.redaction_summary.policy_restricted_items_omitted, 1);
    let json = serde_json::to_string(&result.evidence_index).expect("serialize")
        + &serde_json::to_string(&result.redaction_summary).expect("serialize");
    assert!(!json.contains("private topic"));
}

#[test]
fn malformed_candidate_is_diagnostic_not_fatal() {
    let mut malformed = file_candidate("docs/readme.md", "readme");
    malformed.kind = ContextPackEvidenceCandidateKind::Malformed;
    let result = build(&[malformed, file_candidate("docs/ok.md", "ok")]);
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(result.redaction_summary.malformed_items_omitted, 1);
}

#[test]
fn quarantined_candidate_is_diagnostic_not_fatal() {
    let mut quarantined = file_candidate("docs/readme.md", "readme");
    quarantined.kind = ContextPackEvidenceCandidateKind::Quarantined;
    let result = build(&[quarantined, file_candidate("docs/ok.md", "ok")]);
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(result.redaction_summary.quarantined_items_omitted, 1);
}

#[test]
fn malformed_and_quarantined_diagnostics_are_non_identifying() {
    let mut malformed = file_candidate("secrets/leak.md", "leak");
    malformed.kind = ContextPackEvidenceCandidateKind::Malformed;
    let mut quarantined = file_candidate("secrets/leak.md", "leak");
    quarantined.kind = ContextPackEvidenceCandidateKind::Quarantined;
    let result = build(&[malformed, quarantined]);
    let json = serde_json::to_string(&result.diagnostics).expect("serialize");
    assert!(!json.contains("secrets/leak"));
    assert!(!json.contains("leak"));
}

#[test]
fn evidence_selection_respects_128_item_bound() {
    let candidates: Vec<_> = (0..MAX_CONTEXT_PACK_EVIDENCE_INDEX + 5)
        .map(|index| file_candidate(&format!("docs/file-{index:03}.md"), "label"))
        .collect();
    let result = build(&candidates);
    assert_eq!(result.evidence_index.len(), MAX_CONTEXT_PACK_EVIDENCE_INDEX);
}

#[test]
fn bound_truncation_is_deterministic() {
    let candidates: Vec<_> = (0..130)
        .map(|index| file_candidate(&format!("docs/file-{index:03}.md"), "label"))
        .collect();
    let first = build(&candidates);
    let mut shuffled = candidates;
    shuffled.reverse();
    let second = build(&shuffled);
    assert_eq!(first.evidence_index, second.evidence_index);
    assert_eq!(first.redaction_summary.bounds_truncated_items, 2);
}

#[test]
fn bound_truncation_records_aggregate_count() {
    let candidates: Vec<_> = (0..130)
        .map(|index| file_candidate(&format!("docs/file-{index:03}.md"), "label"))
        .collect();
    let result = build(&candidates);
    assert_eq!(result.redaction_summary.bounds_truncated_items, 2);
    assert!(result
        .diagnostics
        .iter()
        .any(|diag| diag.code == "evidence-truncated"));
}

#[test]
fn diagnostics_respect_32_item_bound() {
    let candidates: Vec<_> = (0..40)
        .map(|_| {
            let mut malformed = file_candidate("docs/readme.md", "readme");
            malformed.kind = ContextPackEvidenceCandidateKind::Malformed;
            malformed
        })
        .collect();
    let result = build(&candidates);
    assert!(result.diagnostics.len() <= MAX_CONTEXT_PACK_DIAGNOSTICS);
}

#[test]
fn duplicate_with_secret_classification_is_fully_omitted() {
    let mut secret = file_candidate("docs/shared.md", "secret label");
    secret.sensitivity = Sensitivity::Secret;
    let public = file_candidate("docs/shared.md", "public label");
    let result = build(&[secret, public]);
    assert!(result.evidence_index.is_empty());
    assert_eq!(result.redaction_summary.secret_items_omitted, 2);
    let json = serde_json::to_string(&result.evidence_index).expect("serialize")
        + &serde_json::to_string(&result.redaction_summary).expect("serialize");
    assert!(!json.contains("public label"));
}

#[test]
fn duplicate_with_policy_restriction_uses_most_restrictive_result() {
    let mut restricted = file_candidate("docs/shared.md", "restricted");
    restricted.policy_eligible = false;
    let allowed = file_candidate("docs/shared.md", "allowed");
    let result = build(&[restricted, allowed]);
    assert!(result.evidence_index.is_empty());
    assert_eq!(result.redaction_summary.policy_restricted_items_omitted, 1);
}

#[test]
fn duplicate_order_does_not_change_restrictive_resolution() {
    let mut restricted = file_candidate("docs/shared.md", "restricted");
    restricted.policy_eligible = false;
    let allowed = file_candidate("docs/shared.md", "allowed");
    let first = build(&[restricted.clone(), allowed.clone()]);
    let second = build(&[allowed, restricted]);
    assert_eq!(first.redaction_summary, second.redaction_summary);
    assert!(first.evidence_index.is_empty());
}

#[test]
fn labels_do_not_control_evidence_identity() {
    let a = file_candidate("docs/shared.md", "zzz");
    let b = file_candidate("docs/shared.md", "aaa");
    let result = build(&[a, b]);
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(result.evidence_index[0].label, "aaa");
}

#[test]
fn timestamps_do_not_control_evidence_identity() {
    let mut later = file_candidate("docs/shared.md", "same");
    later.timestamp = Some("2026-07-18T00:00:00Z".into());
    let mut earlier = file_candidate("docs/shared.md", "same");
    earlier.timestamp = Some("2026-07-17T00:00:00Z".into());
    let result = build(&[later, earlier]);
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(
        result.evidence_index[0].timestamp.as_deref(),
        Some("2026-07-17T00:00:00Z")
    );
}

#[test]
fn pending_candidate_remains_pending() {
    let mut pending = file_candidate("docs/pending.md", "pending");
    pending.provenance = ContextPackItemProvenance::Pending;
    let result = build(&[pending]);
    assert_eq!(result.evidence_index.len(), 1);
}

#[test]
fn unconfirmed_candidate_remains_unconfirmed() {
    let mut unconfirmed = file_candidate("docs/unconfirmed.md", "unconfirmed");
    unconfirmed.provenance = ContextPackItemProvenance::Unconfirmed;
    let result = build(&[unconfirmed]);
    assert_eq!(result.evidence_index.len(), 1);
}

#[test]
fn evidence_index_presence_does_not_confirm_pending_signal() {
    let mut pending = file_candidate("docs/pending.md", "pending");
    pending.provenance = ContextPackItemProvenance::Pending;
    let result = build(&[pending]);
    assert_ne!(result.evidence_index[0].label, "confirmed");
}

#[test]
fn selection_performs_no_signal_promotion() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_selection.rs"),
    )
    .expect("read selection module");
    for forbidden in [
        "promote",
        "processed_signals",
        "pending_signals",
        "write_signal",
    ] {
        assert!(
            !module.contains(forbidden),
            "selection must not perform signal promotion via {forbidden}"
        );
    }
}

#[test]
fn effective_corrected_presentation_is_supported() {
    let corrected = ContextPackEvidenceCandidate {
        evidence_ref: EvidenceRef::FilePath("docs/readme.md".into()),
        origin: ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: Sensitivity::Private,
        safe_label: "corrected summary".into(),
        timestamp: Some("2026-07-17T10:00:00Z".into()),
        provenance: ContextPackItemProvenance::Confirmed,
        correction: Some(ContextPackCorrectionProvenance {
            is_corrected: true,
            is_superseded_original: false,
            correction_event_ids: vec!["evt-correct-1".into()],
            superseded_by_event_id: None,
        }),
        policy_eligible: true,
        kind: ContextPackEvidenceCandidateKind::Normal,
    };
    let result = build(&[corrected]);
    assert_eq!(result.evidence_index.len(), 1);
}

#[test]
fn superseded_raw_presentation_is_not_reintroduced() {
    let superseded = ContextPackEvidenceCandidate {
        evidence_ref: EvidenceRef::FilePath("docs/readme.md".into()),
        origin: ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: Sensitivity::Private,
        safe_label: "raw".into(),
        timestamp: Some("2026-07-17T10:00:00Z".into()),
        provenance: ContextPackItemProvenance::Confirmed,
        correction: Some(ContextPackCorrectionProvenance {
            is_corrected: false,
            is_superseded_original: true,
            correction_event_ids: vec![],
            superseded_by_event_id: Some("evt-new".into()),
        }),
        policy_eligible: true,
        kind: ContextPackEvidenceCandidateKind::Normal,
    };
    let result = build(&[superseded]);
    assert!(result.evidence_index.is_empty());
}

#[test]
fn correction_provenance_is_preserved() {
    let input = ContextPackContinuityItemInput {
        id: "item-1".into(),
        summary: "corrected item".into(),
        kind: "work.completed".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/readme.md".into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: Some(ContextPackCorrectionProvenance {
            is_corrected: true,
            is_superseded_original: false,
            correction_event_ids: vec!["evt-correct-1".into()],
            superseded_by_event_id: None,
        }),
        sensitivity: Sensitivity::Private,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert!(result.items[0].correction.as_ref().unwrap().is_corrected);
}

#[test]
fn invalid_correction_original_can_remain_representable() {
    let input = ContextPackContinuityItemInput {
        id: "item-1".into(),
        summary: "original remains visible".into(),
        kind: "work.completed".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/readme.md".into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: Some(ContextPackCorrectionProvenance {
            is_corrected: false,
            is_superseded_original: false,
            correction_event_ids: vec![],
            superseded_by_event_id: None,
        }),
        sensitivity: Sensitivity::Private,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert_eq!(result.items.len(), 1);
}

#[test]
fn sanitized_current_state_omits_secret_derived_content() {
    let input = ContextPackContinuityItemInput {
        id: "secret-item".into(),
        summary: "secret summary".into(),
        kind: "work.blocked".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("secrets/credentials.env".into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: None,
        sensitivity: Sensitivity::Secret,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert!(result.items.is_empty());
    assert_eq!(result.redaction_summary.secret_items_omitted, 1);
}

#[test]
fn sanitized_catch_up_omits_secret_derived_content() {
    let input = ContextPackPendingAttentionItemInput {
        id: "pending-secret".into(),
        summary: "secret pending".into(),
        reason: PendingAttentionReason::PendingSignal,
        provenance: ContextPackItemProvenance::Pending,
        timestamp: "2026-07-17T10:00:00Z".into(),
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::High,
        priority: 1,
        evidence_refs: vec![EvidenceRef::FilePath("secrets/credentials.env".into())],
        sensitivity: Sensitivity::Secret,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_pending_items(&[input]).expect("sanitize");
    assert!(result.items.is_empty());
    assert_eq!(result.redaction_summary.secret_items_omitted, 1);
}

#[test]
fn sanitized_items_preserve_pending_provenance() {
    let input = ContextPackContinuityItemInput {
        id: "pending-item".into(),
        summary: "pending work".into(),
        kind: "work.in-progress".into(),
        source: ContinuitySourceKind::PendingSignal,
        provenance: ContextPackItemProvenance::Pending,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/readme.md".into())],
        confidence: ContinuityConfidence::Medium,
        unverified: Some(true),
        correction: None,
        sensitivity: Sensitivity::Private,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert_eq!(
        result.items[0].provenance,
        ContextPackItemProvenance::Pending
    );
}

#[test]
fn sanitized_items_preserve_correction_provenance() {
    let input = ContextPackContinuityItemInput {
        id: "item-1".into(),
        summary: "corrected".into(),
        kind: "work.completed".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/readme.md".into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: Some(ContextPackCorrectionProvenance {
            is_corrected: true,
            is_superseded_original: false,
            correction_event_ids: vec!["evt-1".into()],
            superseded_by_event_id: None,
        }),
        sensitivity: Sensitivity::Private,
        policy_restricted: false,
        malformed: false,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert!(result.items[0].correction.as_ref().unwrap().is_corrected);
}

#[test]
fn redaction_summary_contains_counts_only() {
    let mut secret = file_candidate("docs/secret.md", "secret");
    secret.sensitivity = Sensitivity::Secret;
    let json = serde_json::to_string(&build(&[secret]).redaction_summary).expect("serialize");
    for forbidden in ["secret.md", "label", "path", "timestamp", "ref-"] {
        assert!(!json.contains(forbidden), "summary leaked {forbidden}");
    }
}

#[test]
fn redaction_summary_is_deterministic() {
    let candidates = vec![file_candidate("docs/a.md", "a"), {
        let mut secret = file_candidate("docs/b.md", "b");
        secret.sensitivity = Sensitivity::Secret;
        secret
    }];
    let first = build(&candidates);
    let second = build(&candidates.iter().rev().cloned().collect::<Vec<_>>());
    assert_eq!(first.redaction_summary, second.redaction_summary);
}

#[test]
fn selection_output_contains_no_secret_identifying_metadata() {
    let mut secret = file_candidate("secrets/credentials.env", "credential label");
    secret.sensitivity = Sensitivity::Secret;
    let result = build(&[secret]);
    let json = serde_json::to_string(&result.evidence_index).expect("serialize")
        + &serde_json::to_string(&result.redaction_summary).expect("serialize")
        + &serde_json::to_string(&result.diagnostics).expect("serialize");
    assert!(!json.contains("credentials"));
    assert!(!json.contains("credential label"));
}

#[test]
fn selection_output_contains_no_answer_or_authority_execution() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_selection.rs"),
    )
    .expect("read selection module");
    for forbidden in [
        "resolve_profile_authority",
        "generate_answer",
        "ask_my_proxy",
        "ask-my-proxy",
        "ProxyPolicyResult",
    ] {
        assert!(!module.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn selection_module_performs_no_filesystem_io() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_selection.rs"),
    )
    .expect("read selection module");
    for forbidden in [
        "std::fs",
        "read_to_string",
        "OpenOptions",
        "write(",
        "read_project",
    ] {
        assert!(
            !module.contains(forbidden),
            "selection must not use {forbidden}"
        );
    }
}

#[test]
fn selection_module_does_not_build_current_state_or_catch_up() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_selection.rs"),
    )
    .expect("read selection module");
    for forbidden in [
        "build_current_state_projection",
        "build_catch_up_view",
        "CurrentStateProjection",
        "CatchUpView",
    ] {
        assert!(
            !module.contains(forbidden),
            "selection must not call {forbidden}"
        );
    }
}

#[test]
fn selection_module_does_not_touch_profile_or_continuity_modules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for rel in [
        "profile.rs",
        "profile_validation.rs",
        "continuity/current_state.rs",
        "continuity/catch_up.rs",
    ] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        assert!(!content.contains("context_pack_selection"));
    }
}

#[test]
fn checkpoint_b_selection_module_exists() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(root.join("context_pack_selection.rs").exists());
}

#[test]
fn checkpoint_b_does_not_start_ask_my_proxy() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_selection.rs"),
    )
    .expect("read selection module");
    assert!(!module.to_ascii_lowercase().contains("ask-my-proxy"));
    assert!(!module.to_ascii_lowercase().contains("ask my proxy"));
}

#[test]
fn checkpoint_b_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    let content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    assert!(!content.contains("context_pack_selection"));
    assert!(!content.contains("ProxyContextPack"));
}

#[test]
fn canonical_identity_matches_serialized_evidence_ref() {
    let evidence_ref = EvidenceRef::FilePath("docs/readme.md".into());
    let key = canonical_evidence_ref_key(&evidence_ref).expect("canonical key");
    assert!(key.contains("docs/readme.md"));
}

#[test]
fn invalid_timestamp_returns_pure_selection_error() {
    let mut candidate = file_candidate("docs/readme.md", "readme");
    candidate.timestamp = Some("not-a-timestamp".into());
    let err = build_context_pack_evidence_index(
        &[candidate],
        &ContextPackEvidenceSelectionOptions::default(),
    )
    .expect_err("invalid timestamp");
    assert!(matches!(
        err,
        ContextPackSelectionError::InvalidTimestamp(_)
    ));
}

#[test]
fn unknown_sensitivity_is_never_defaulted_to_private_or_public() {
    let input = ContextPackContinuityItemInput {
        id: "unknown-sensitivity-item".into(),
        summary: "item with unresolved sensitivity".into(),
        kind: "work.in-progress".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/unknown.md".into())],
        confidence: ContinuityConfidence::Medium,
        unverified: None,
        correction: None,
        sensitivity: Sensitivity::Private,
        policy_restricted: true,
        malformed: true,
        quarantined: false,
    };
    let result = sanitize_context_pack_continuity_items(&[input]).expect("sanitize");
    assert!(result.items.is_empty());
    assert_eq!(result.redaction_summary.malformed_items_omitted, 1);
    let json = serde_json::to_string(&result.items).expect("serialize")
        + &serde_json::to_string(&result.diagnostics).expect("serialize");
    assert!(!json.contains("unknown.md"));
    assert!(!json.contains("\"private\""));
    assert!(!json.contains("\"public\""));
}

#[test]
fn ambiguous_evidence_candidate_is_omitted_fail_closed() {
    let mut ambiguous = file_candidate("docs/shared.md", "ambiguous label");
    ambiguous.policy_eligible = false;
    ambiguous.kind = ContextPackEvidenceCandidateKind::Malformed;
    let result = build_context_pack_evidence_index(
        &[ambiguous, file_candidate("docs/ok.md", "ok")],
        &ContextPackEvidenceSelectionOptions::default(),
    )
    .expect("selection succeeds");
    assert_eq!(result.evidence_index.len(), 1);
    assert_eq!(result.evidence_index[0].label, "ok");
    assert_eq!(result.redaction_summary.malformed_items_omitted, 1);
    let json = serde_json::to_string(&result.evidence_index).expect("serialize");
    assert!(!json.contains("ambiguous label"));
    assert!(!json.contains("shared.md"));
}
