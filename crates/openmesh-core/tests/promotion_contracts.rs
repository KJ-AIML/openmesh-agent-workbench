//! Dev Track 0.1.3.5 Checkpoint A — pure promotion domain contract tests.
//!
//! No storage, inbox I/O, append_event, or protocol 1.1 validation.

use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{ActorRef, EvidenceRef, ProducerRef, WorkSignalKind};
use openmesh_core::intelligence::{ContinuityIntelligence, NoopContinuityIntelligence};
use openmesh_core::promotion::{
    promotion_key_material, AmbiguousPromotionCase, EvidenceRelationship, PromotionCase,
    PromotionDecision, PromotionEvidence, PromotionKey, PromotionOutcome, PromotionProposal,
    PromotionReasonCode, ProposedEventComposition, SignalRef, PROMOTED_EVENT_PROTOCOL_NOTE,
};

fn sample_signal(id: &str) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind: WorkSignalKind::Progress,
        summary: format!("summary for {id}"),
        producer: ProducerRef::Reporter("codex".into()),
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T12:00:00Z".into(),
        correlation_hint: Some("feat/openmesh-0.1.3.5".into()),
        evidence_refs: vec![EvidenceRef::ProducerSignal(id.into())],
    }
}

fn sample_key(ids: &[&str]) -> PromotionKey {
    PromotionKey::from_inputs(
        "ws-test",
        &ids.iter().map(|s| (*s).to_string()).collect::<Vec<_>>(),
    )
    .expect("valid key")
}

#[test]
fn promotion_outcome_taxonomy_is_exact() {
    let variants = [
        PromotionOutcome::Promote,
        PromotionOutcome::Suppress,
        PromotionOutcome::Defer,
        PromotionOutcome::Ambiguous,
    ];
    assert_eq!(variants.len(), 4);
    for (i, left) in variants.iter().enumerate() {
        for (j, right) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(left, right);
            } else {
                assert_ne!(left, right);
            }
        }
    }
}

#[test]
fn promotion_decision_can_represent_promote_without_writing_event() {
    let key = sample_key(&["sig-a"]);
    let proposed = ProposedEventComposition {
        kind: "work.completed".into(),
        summary: "checkpoint A contracts".into(),
        timestamp: "2026-07-15T13:00:00Z".into(),
        producer_signal_evidence_ids: vec!["sig-a".into()],
        file_evidence_paths: vec![],
        sensitivity: Sensitivity::Private,
        composition_note: PROMOTED_EVENT_PROTOCOL_NOTE.into(),
    };
    let decision = PromotionDecision::promote(key.clone(), vec!["sig-a".into()], proposed);

    assert_eq!(decision.outcome, PromotionOutcome::Promote);
    assert_eq!(decision.promotion_key, key);
    assert!(decision.proposed_composition.is_some());
    assert!(!decision.ambiguous);
    assert_eq!(
        decision
            .proposed_composition
            .as_ref()
            .unwrap()
            .composition_note,
        PROMOTED_EVENT_PROTOCOL_NOTE
    );
}

#[test]
fn promotion_decision_can_represent_suppress() {
    let key = sample_key(&["sig-spam"]);
    let decision = PromotionDecision::suppress(
        key,
        vec!["sig-spam".into()],
        PromotionReasonCode::ActivitySpam,
    );

    assert_eq!(decision.outcome, PromotionOutcome::Suppress);
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::ActivitySpam)
    );
    assert!(decision.proposed_composition.is_none());
    assert!(!decision.ambiguous);
}

#[test]
fn promotion_decision_can_represent_defer() {
    let key = sample_key(&["sig-missing"]);
    let decision = PromotionDecision::defer(
        key,
        vec!["sig-missing".into()],
        PromotionReasonCode::MissingEvidence,
    );

    assert_eq!(decision.outcome, PromotionOutcome::Defer);
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::MissingEvidence)
    );
    assert!(decision.proposed_composition.is_none());
}

#[test]
fn promotion_decision_can_represent_ambiguous_without_fake_certainty() {
    let key = sample_key(&["sig-x", "sig-y"]);
    let decision = PromotionDecision::ambiguous(
        key,
        vec!["sig-x".into(), "sig-y".into()],
        "kind conflict between progress and decision".into(),
    );

    assert_eq!(decision.outcome, PromotionOutcome::Ambiguous);
    assert!(decision.ambiguous);
    assert!(decision.proposed_composition.is_none());
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::SeamAmbiguous)
    );
    assert!(decision.reason_detail.is_some());
}

#[test]
fn promotion_evidence_can_represent_many_signals_one_event() {
    let evidence = PromotionEvidence {
        signal_refs: vec!["sig-1".into(), "sig-2".into(), "sig-3".into()],
        relationship: EvidenceRelationship::IndependentCorroboration,
        producer_signal_attachments: vec!["sig-1".into(), "sig-2".into(), "sig-3".into()],
    };

    assert_eq!(evidence.signal_refs.len(), 3);
    assert_eq!(evidence.producer_signal_attachments.len(), 3);
    assert_eq!(
        evidence.relationship,
        EvidenceRelationship::IndependentCorroboration
    );
}

#[test]
fn promotion_evidence_distinguishes_duplicate_from_corroboration_contracts() {
    let duplicate = PromotionEvidence {
        signal_refs: vec!["sig-a".into(), "sig-a-dup".into()],
        relationship: EvidenceRelationship::SameOriginDuplicate,
        producer_signal_attachments: vec!["sig-a".into()],
    };
    let corroboration = PromotionEvidence {
        signal_refs: vec!["sig-a".into(), "sig-b".into()],
        relationship: EvidenceRelationship::IndependentCorroboration,
        producer_signal_attachments: vec!["sig-a".into(), "sig-b".into()],
    };

    assert_ne!(duplicate.relationship, corroboration.relationship);
    assert_eq!(
        duplicate.relationship,
        EvidenceRelationship::SameOriginDuplicate
    );
    assert_eq!(
        corroboration.relationship,
        EvidenceRelationship::IndependentCorroboration
    );
}

#[test]
fn promotion_key_is_deterministic_or_validated_by_contract() {
    let material_a =
        promotion_key_material("ws-1", &["sig-b".into(), "sig-a".into()]).expect("material");
    let material_b =
        promotion_key_material("ws-1", &["sig-a".into(), "sig-b".into()]).expect("material");
    assert_eq!(material_a, material_b);

    let key = PromotionKey::from_material(material_a.clone()).expect("key");
    assert_eq!(key.as_str().len(), 64);
    assert_eq!(
        key,
        PromotionKey::from_inputs("ws-1", &["sig-a".into(), "sig-b".into()]).unwrap()
    );

    assert!(PromotionKey::from_inputs("", &["sig-a".into()]).is_err());
    assert!(PromotionKey::from_inputs("ws", &[]).is_err());
}

#[test]
fn intelligence_seam_contract_is_proposal_only_and_side_effect_free() {
    let case = AmbiguousPromotionCase {
        case: PromotionCase {
            workspace_id: "ws-test".into(),
            signals: vec![sample_signal("sig-1"), sample_signal("sig-2")],
            correlation_hint: Some("ambiguous-group".into()),
        },
        reason: "cannot resolve without intelligence".into(),
        qualification_notes: vec!["kind conflict".into()],
    };

    let seam = NoopContinuityIntelligence;
    let proposal = seam.propose(&case);

    assert!(!proposal.has_proposal);
    assert!(proposal.suggested_outcome.is_none());
    assert!(proposal.suggested_composition.is_none());
    assert!(proposal.is_side_effect_free());
    assert_eq!(proposal, PromotionProposal::none());
}

#[test]
fn boundary_guards_allow_core_promotion_but_block_product_surfaces() {
    let core_src = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(core_src.join("promotion.rs").exists());
    assert!(core_src.join("intelligence.rs").exists());

    for forbidden in [
        "correlation.rs",
        "suppression.rs",
        "current_state.rs",
        "projection.rs",
        "catch_up.rs",
    ] {
        assert!(
            !core_src.join(forbidden).exists(),
            "execution module must remain absent: {forbidden}"
        );
    }

    let workspace = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tauri_lib = workspace.join("src-tauri/src/lib.rs");
    let tauri = std::fs::read_to_string(tauri_lib).expect("tauri lib");
    for term in ["openmesh_core::promotion", "ContinuityIntelligence"] {
        assert!(
            !tauri.contains(term),
            "Tauri must not expose promotion surface `{term}`"
        );
    }

    let cli_main = workspace.join("crates/openmesh-cli/src/main.rs");
    if cli_main.exists() {
        let cli = std::fs::read_to_string(cli_main).expect("cli main");
        assert!(
            !cli.contains("openmesh_core::promotion"),
            "CLI must not reference promotion module"
        );
    }
}
