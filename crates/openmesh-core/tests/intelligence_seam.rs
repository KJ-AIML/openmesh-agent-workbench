//! Dev Track 0.1.3.5 Checkpoint F — intelligence seam + ambiguous handling tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{ActorRef, EvidenceRef, ProducerRef, WorkSignalKind};
use openmesh_core::events::{ledger_dir, list_events};
use openmesh_core::intelligence::{
    proposal_preserves_source_signal_refs, validate_proposal_contract, ContinuityIntelligence,
    NoopContinuityIntelligence, INTELLIGENCE_SEAM_CONTRACT_NOTE,
};
use openmesh_core::promotion::{
    ambiguous_case_from_request, apply_promotion_decision_with_intelligence,
    correlate_and_evaluate, get_decision_record, resolve_ambiguous_with_intelligence,
    PromotionApplyRequest, PromotionCase, PromotionOutcome, PromotionProposal,
    ProposedEventComposition, SignalRef, PROMOTED_EVENT_PROTOCOL_NOTE,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

struct PromoteTestDouble {
    composition: ProposedEventComposition,
}

impl ContinuityIntelligence for PromoteTestDouble {
    fn propose(
        &self,
        _case: &openmesh_core::promotion::AmbiguousPromotionCase,
    ) -> PromotionProposal {
        PromotionProposal {
            has_proposal: true,
            suggested_outcome: Some(PromotionOutcome::Promote),
            suggested_composition: Some(self.composition.clone()),
            rationale: Some("unit-test double only".into()),
        }
    }
}

fn signal(id: &str, kind: WorkSignalKind, summary: &str) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind,
        summary: summary.into(),
        producer: ProducerRef::Reporter("codex".into()),
        actor: ActorRef::Person("ter".into()),
        timestamp: "2026-07-15T20:00:00Z".into(),
        correlation_hint: Some("feat/seam".into()),
        evidence_refs: vec![EvidenceRef::ProducerSignal(id.into())],
    }
}

fn create_test_project(name: &str) -> (PathBuf, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-seam-f-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(project_dir.join(".openmesh")).unwrap();
    let project_id = format!("proj-{name}-{unique}");
    let now = "2026-07-08T00:00:00.000Z";
    let project_json = serde_json::json!({
        "id": project_id,
        "name": "Test",
        "folderPath": project_dir.to_str().unwrap(),
        "repoUrl": null,
        "defaultBranch": "main",
        "sprintSource": "none",
        "docsFolder": null,
        "terminalDir": null,
        "defaultAgentCli": null,
        "notes": null,
        "status": "active",
        "createdAt": now,
        "updatedAt": now,
    });
    fs::write(
        project_dir.join(".openmesh/project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();
    (dir, project_dir.to_string_lossy().into_owned())
}

fn ambiguous_apply_request(workspace_id: &str) -> PromotionApplyRequest {
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Handoff,
            "handoff summary one with enough detail",
        ),
        signal(
            "sig-b",
            WorkSignalKind::ReviewRequired,
            "review required summary two with enough detail",
        ),
    ];
    let batch = correlate_and_evaluate(workspace_id, &signals).unwrap();
    let correlated = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Ambiguous)
        .expect("ambiguous fixture");
    PromotionApplyRequest {
        workspace_id: workspace_id.to_string(),
        decision: correlated.decision.clone(),
        signals: correlated.group.signals.clone(),
        evidence: Some(correlated.evidence.clone()),
        recorded_at: "2026-07-15T20:01:00Z".into(),
    }
}

#[test]
fn noop_intelligence_is_side_effect_free() {
    let intelligence_rs = include_str!("../src/intelligence.rs");
    for forbidden in [
        "append_event",
        "write_decision_record",
        "process_pending",
        "std::fs",
        "reqwest",
        "openai",
        "axga",
    ] {
        assert!(
            !intelligence_rs.contains(forbidden),
            "intelligence seam must not reference `{forbidden}`"
        );
    }
    assert!(INTELLIGENCE_SEAM_CONTRACT_NOTE.contains("proposal-only"));
}

#[test]
fn intelligence_seam_returns_proposal_only() {
    let case = openmesh_core::promotion::AmbiguousPromotionCase {
        case: PromotionCase {
            workspace_id: "ws-seam".into(),
            signals: vec![signal(
                "sig-only",
                WorkSignalKind::Decision,
                "decision summary",
            )],
            correlation_hint: None,
        },
        reason: "needs seam".into(),
        qualification_notes: vec!["kind conflict".into()],
    };
    let proposal = NoopContinuityIntelligence.propose(&case);
    assert!(validate_proposal_contract(&proposal));
    assert!(!proposal.has_proposal);
    assert!(proposal.is_side_effect_free());
}

#[test]
fn ambiguous_case_remains_ambiguous_without_fake_certainty() {
    let (_dir, project_path) = create_test_project("stay-ambiguous");
    let project_id = openmesh_core::storage::read_project::<openmesh_core::storage::Project>(
        &project_path,
        "project.json",
    )
    .unwrap()
    .id;
    let request = ambiguous_apply_request(&project_id);

    let outcome = apply_promotion_decision_with_intelligence(
        &project_path,
        &request,
        &NoopContinuityIntelligence,
    )
    .unwrap();

    assert!(outcome.event.is_none());
    let record = get_decision_record(&project_path, &request.decision.promotion_key)
        .unwrap()
        .expect("audit");
    assert_eq!(record.outcome, PromotionOutcome::Ambiguous);
    assert!(record.ambiguous);
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn intelligence_seam_does_not_create_work_event() {
    let (_dir, project_path) = create_test_project("no-event");
    let project_id = openmesh_core::storage::read_project::<openmesh_core::storage::Project>(
        &project_path,
        "project.json",
    )
    .unwrap()
    .id;
    let request = ambiguous_apply_request(&project_id);

    apply_promotion_decision_with_intelligence(
        &project_path,
        &request,
        &NoopContinuityIntelligence,
    )
    .unwrap();

    assert!(!ledger_dir(&project_path).exists() || list_events(&project_path).unwrap().is_empty());
}

#[test]
fn intelligence_seam_does_not_write_promotion_audit_when_resolve_only() {
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Handoff,
            "handoff summary one with enough detail",
        ),
        signal(
            "sig-b",
            WorkSignalKind::ReviewRequired,
            "review required summary two with enough detail",
        ),
    ];
    let batch = correlate_and_evaluate("ws-resolve-only", &signals).unwrap();
    let correlated = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Ambiguous)
        .unwrap();
    let ambiguous = openmesh_core::promotion::ambiguous_case_from_decision(
        "ws-resolve-only",
        &correlated.decision,
        &correlated.group.signals,
    );
    let resolved = resolve_ambiguous_with_intelligence(
        &correlated.decision,
        &ambiguous,
        &NoopContinuityIntelligence,
    )
    .unwrap();
    assert_eq!(resolved.outcome, PromotionOutcome::Ambiguous);
}

#[test]
fn intelligence_seam_preserves_source_signal_refs() {
    let (_dir, project_path) = create_test_project("preserve-refs");
    let project_id = openmesh_core::storage::read_project::<openmesh_core::storage::Project>(
        &project_path,
        "project.json",
    )
    .unwrap()
    .id;
    let request = ambiguous_apply_request(&project_id);
    let ambiguous = ambiguous_case_from_request(&request);
    let proposal = NoopContinuityIntelligence.propose(&ambiguous);
    assert!(proposal_preserves_source_signal_refs(&proposal, &ambiguous));
    assert_eq!(
        ambiguous.case.signal_ids(),
        request.decision.source_signal_ids
    );

    let composition = ProposedEventComposition {
        kind: "work.decision".into(),
        summary: "test double promote".into(),
        timestamp: "2026-07-15T20:02:00Z".into(),
        producer_signal_evidence_ids: ambiguous.case.signal_ids(),
        file_evidence_paths: vec![],
        sensitivity: Sensitivity::Private,
        composition_note: PROMOTED_EVENT_PROTOCOL_NOTE.into(),
    };
    let test_double = PromoteTestDouble {
        composition: composition.clone(),
    };
    let promoted =
        resolve_ambiguous_with_intelligence(&request.decision, &ambiguous, &test_double).unwrap();
    assert_eq!(promoted.outcome, PromotionOutcome::Promote);
    assert!(proposal_preserves_source_signal_refs(
        &test_double.propose(&ambiguous),
        &ambiguous
    ));
}
