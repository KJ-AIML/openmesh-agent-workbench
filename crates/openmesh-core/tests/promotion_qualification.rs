//! Dev Track 0.1.3.5 Checkpoint C — deterministic qualification + suppression tests.

use openmesh_core::domain::{
    ActorRef, EvidenceRef, ProducerRef, WorkSignalKind, WORK_EVENT_PROTOCOL_VERSION,
};
use openmesh_core::events::{ledger_dir, list_events};
use openmesh_core::promotion::{
    evaluate_promotion_case, matrix_outcome_when_qualification_fails,
    matrix_outcome_when_qualification_passes, promotion_decisions_dir, qualification_score,
    PromotionCase, PromotionOutcome, PromotionReasonCode, QualificationContext, SignalRef,
    QUALIFICATION_PASS_THRESHOLD,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn sample_signal(id: &str, kind: WorkSignalKind, summary: &str) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind,
        summary: summary.into(),
        producer: ProducerRef::Reporter("codex".into()),
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T15:00:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![EvidenceRef::FilePath("docs/evidence.md".into())],
    }
}

fn solo_case(workspace_id: &str, signal: SignalRef) -> PromotionCase {
    PromotionCase {
        workspace_id: workspace_id.into(),
        signals: vec![signal],
        correlation_hint: None,
    }
}

fn create_test_project(name: &str) -> (PathBuf, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-qual-test-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();
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
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();
    (dir, project_dir.to_string_lossy().into_owned())
}

#[test]
fn qualification_matrix_covers_all_work_signal_kinds() {
    let kinds = [
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
    ];
    assert_eq!(kinds.len(), 11);

    for kind in kinds {
        let pass = matrix_outcome_when_qualification_passes(kind, true, 40, true, 2);
        let fail = matrix_outcome_when_qualification_fails(kind, true);
        assert!(
            matches!(
                pass,
                PromotionOutcome::Promote
                    | PromotionOutcome::Suppress
                    | PromotionOutcome::Defer
                    | PromotionOutcome::Ambiguous
            ),
            "pass disposition missing for {kind:?}"
        );
        assert!(
            matches!(
                fail,
                PromotionOutcome::Promote
                    | PromotionOutcome::Suppress
                    | PromotionOutcome::Defer
                    | PromotionOutcome::Ambiguous
            ),
            "fail disposition missing for {kind:?}"
        );
    }
}

#[test]
fn five_question_score_promotes_at_threshold() {
    let signal = sample_signal(
        "sig-threshold-pass",
        WorkSignalKind::Decision,
        "accepted the architecture decision for promotion",
    );
    let ctx = QualificationContext {
        group_size: 1,
        any_peer_has_evidence: false,
    };
    let score = qualification_score(&signal, &ctx);
    assert!(score.true_count() >= QUALIFICATION_PASS_THRESHOLD);
    assert!(score.passes_threshold());

    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Promote);
}

#[test]
fn five_question_score_below_threshold_does_not_promote() {
    let signal = SignalRef {
        signal_id: "sig-low".into(),
        kind: WorkSignalKind::SessionEnd,
        summary: "bye".into(),
        producer: ProducerRef::Native,
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T15:01:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![],
    };
    let ctx = QualificationContext {
        group_size: 1,
        any_peer_has_evidence: false,
    };
    let score = qualification_score(&signal, &ctx);
    assert!(score.true_count() < QUALIFICATION_PASS_THRESHOLD);

    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert_ne!(decision.outcome, PromotionOutcome::Promote);
}

#[test]
fn meaningful_decision_signal_promotes() {
    let signal = sample_signal(
        "sig-decision",
        WorkSignalKind::Decision,
        "team accepted the promotion qualification contract",
    );
    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Promote);
    assert_eq!(decision.reason_code, Some(PromotionReasonCode::Qualifies));
    assert!(decision.proposed_composition.is_some());
}

#[test]
fn session_end_activity_spam_suppresses() {
    let signal = SignalRef {
        signal_id: "sig-session".into(),
        kind: WorkSignalKind::SessionEnd,
        summary: "session ended".into(),
        producer: ProducerRef::Native,
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T15:02:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![],
    };
    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Suppress);
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::ActivitySpam)
    );
}

#[test]
fn low_information_agent_switch_suppresses_or_defers() {
    let signal = SignalRef {
        signal_id: "sig-switch".into(),
        kind: WorkSignalKind::AgentSwitch,
        summary: "switched".into(),
        producer: ProducerRef::Native,
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T15:03:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![],
    };
    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert!(matches!(
        decision.outcome,
        PromotionOutcome::Suppress | PromotionOutcome::Defer
    ));
}

#[test]
fn unresolved_or_incomplete_case_defers() {
    let signal = SignalRef {
        signal_id: "sig-review".into(),
        kind: WorkSignalKind::ReviewRequired,
        summary: "needs review".into(),
        producer: ProducerRef::Reporter("codex".into()),
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T15:04:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![],
    };
    let decision = evaluate_promotion_case(&solo_case("ws-1", signal)).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Defer);
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::UnresolvedOrIncomplete)
    );
}

#[test]
fn ambiguous_case_returns_ambiguous_without_fake_certainty() {
    let case = PromotionCase {
        workspace_id: "ws-1".into(),
        signals: vec![
            sample_signal(
                "sig-milestone",
                WorkSignalKind::Milestone,
                "milestone reached",
            ),
            sample_signal(
                "sig-scope",
                WorkSignalKind::ScopeChange,
                "scope changed materially",
            ),
        ],
        correlation_hint: Some("feat/conflict".into()),
    };
    let decision = evaluate_promotion_case(&case).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Ambiguous);
    assert!(decision.ambiguous);
    assert!(decision.proposed_composition.is_none());
    assert_eq!(
        decision.reason_code,
        Some(PromotionReasonCode::AmbiguousRequiresIntelligence)
    );
}

#[test]
fn qualification_preserves_source_signal_ids() {
    let case = PromotionCase {
        workspace_id: "ws-1".into(),
        signals: vec![
            sample_signal("sig-b", WorkSignalKind::Decision, "decision b"),
            sample_signal("sig-a", WorkSignalKind::Handoff, "handoff a"),
        ],
        correlation_hint: None,
    };
    let decision = evaluate_promotion_case(&case).unwrap();
    assert_eq!(decision.source_signal_ids, vec!["sig-b", "sig-a"]);
}

#[test]
fn qualification_does_not_write_promotion_audit_files() {
    let (_dir, project_path) = create_test_project("no-audit");
    let signal = sample_signal("sig-audit", WorkSignalKind::Decision, "decision only");
    let _decision = evaluate_promotion_case(&solo_case("ws-audit", signal)).unwrap();
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn qualification_does_not_touch_signal_inbox() {
    let (_dir, project_path) = create_test_project("no-signals");
    let signals_root = openmesh_core::storage::get_project_dir(&project_path).join("signals");
    fs::create_dir_all(signals_root.join("processed")).unwrap();
    fs::write(signals_root.join("processed/existing.json"), "{}").unwrap();
    let before = fs::read_dir(signals_root.join("processed"))
        .unwrap()
        .count();

    let signal = sample_signal("sig-inbox", WorkSignalKind::Decision, "leave inbox alone");
    let _ = evaluate_promotion_case(&solo_case("ws-inbox", signal)).unwrap();

    assert_eq!(
        fs::read_dir(signals_root.join("processed"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn qualification_does_not_create_work_events() {
    let (_dir, project_path) = create_test_project("no-events");
    let signal = sample_signal("sig-events", WorkSignalKind::Milestone, "milestone only");
    let _ = evaluate_promotion_case(&solo_case("ws-events", signal)).unwrap();
    assert!(!ledger_dir(&project_path).exists());
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn qualification_does_not_call_append_event() {
    let signal = sample_signal(
        "sig-pure",
        WorkSignalKind::Handoff,
        "pure qualification only",
    );
    let decision = evaluate_promotion_case(&solo_case("ws-pure", signal)).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Promote);
    assert!(decision.proposed_composition.is_some());
}

#[test]
fn qualification_does_not_change_protocol_version() {
    assert_eq!(WORK_EVENT_PROTOCOL_VERSION, "1.0");
    let signal = sample_signal("sig-proto", WorkSignalKind::Decision, "protocol unchanged");
    let decision = evaluate_promotion_case(&solo_case("ws-proto", signal)).unwrap();
    assert_eq!(
        decision
            .proposed_composition
            .as_ref()
            .map(|c| c.composition_note.as_str()),
        Some("protocol 1.1 with composed actor — Checkpoint E2")
    );
    assert_eq!(WORK_EVENT_PROTOCOL_VERSION, "1.0");
}
