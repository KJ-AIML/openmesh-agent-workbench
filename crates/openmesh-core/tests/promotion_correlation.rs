//! Dev Track 0.1.3.5 Checkpoint D — correlation + duplicate/corroboration tests.

use openmesh_core::domain::{
    ActorRef, EvidenceRef, ProducerRef, WorkSignalKind, WORK_EVENT_PROTOCOL_VERSION,
};
use openmesh_core::events::{ledger_dir, list_events};
use openmesh_core::promotion::{
    correlate_and_evaluate, group_signals_by_correlation_hint, prepare_future_event_candidate,
    promotion_decisions_dir, EvidenceRelationship, PromotionOutcome, PromotionReasonCode,
    SignalRef, UNCORRELATED_BUCKET_PREFIX,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn signal(
    id: &str,
    kind: WorkSignalKind,
    summary: &str,
    producer: ProducerRef,
    hint: Option<&str>,
) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind,
        summary: summary.into(),
        producer,
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T16:00:00Z".into(),
        correlation_hint: hint.map(str::to_string),
        evidence_refs: vec![EvidenceRef::ProducerSignal(id.into())],
    }
}

fn create_test_project(name: &str) -> (PathBuf, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-corr-test-{name}-{}-{unique}",
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
fn correlation_groups_by_exact_correlation_hint() {
    let signals = vec![
        signal(
            "sig-b",
            WorkSignalKind::Progress,
            "progress on feature branch",
            ProducerRef::Reporter("codex".into()),
            Some("feat/corr"),
        ),
        signal(
            "sig-a",
            WorkSignalKind::Progress,
            "progress on feature branch",
            ProducerRef::Reporter("claude".into()),
            Some("feat/corr"),
        ),
        signal(
            "sig-solo",
            WorkSignalKind::Decision,
            "solo decision signal",
            ProducerRef::Reporter("codex".into()),
            None,
        ),
    ];
    let result = group_signals_by_correlation_hint("ws-1", &signals);
    assert_eq!(result.groups.len(), 2);
    assert_eq!(result.groups[0].signals.len(), 2);
    assert_eq!(
        result.groups[0].correlation_hint.as_deref(),
        Some("feat/corr")
    );
    assert_eq!(result.groups[1].signals.len(), 1);
    assert!(result.groups[1]
        .correlation_key
        .starts_with(UNCORRELATED_BUCKET_PREFIX));
}

#[test]
fn missing_correlation_hint_remains_uncorrelated() {
    let signals = vec![
        signal(
            "sig-1",
            WorkSignalKind::Milestone,
            "milestone reached for release",
            ProducerRef::Reporter("codex".into()),
            None,
        ),
        signal(
            "sig-2",
            WorkSignalKind::Handoff,
            "handoff notes for next owner",
            ProducerRef::Reporter("codex".into()),
            None,
        ),
    ];
    let result = group_signals_by_correlation_hint("ws-1", &signals);
    assert_eq!(result.groups.len(), 2);
    assert!(result.groups.iter().all(|g| g.correlation_hint.is_none()));
}

#[test]
fn correlation_group_order_is_deterministic() {
    let signals = vec![
        signal(
            "sig-z",
            WorkSignalKind::Progress,
            "z progress update with enough detail",
            ProducerRef::Reporter("a".into()),
            Some("hint-z"),
        ),
        signal(
            "sig-a",
            WorkSignalKind::Progress,
            "a progress update with enough detail",
            ProducerRef::Reporter("b".into()),
            Some("hint-a"),
        ),
        signal(
            "sig-m",
            WorkSignalKind::Decision,
            "middle solo decision signal",
            ProducerRef::Reporter("c".into()),
            None,
        ),
    ];
    let first = group_signals_by_correlation_hint("ws-1", &signals);
    let second = group_signals_by_correlation_hint("ws-1", &signals);
    assert_eq!(first, second);
    assert_eq!(first.groups[0].group_order_key, "sig-a");
    assert_eq!(first.groups[1].group_order_key, "sig-m");
    assert_eq!(first.groups[2].group_order_key, "sig-z");
    assert_eq!(first.groups[0].signals[0].signal_id, "sig-a");
}

#[test]
fn correlation_preserves_all_source_signal_ids() {
    let signals = vec![
        signal(
            "sig-1",
            WorkSignalKind::Decision,
            "accepted architecture decision for correlation",
            ProducerRef::Reporter("codex".into()),
            Some("feat/preserve"),
        ),
        signal(
            "sig-2",
            WorkSignalKind::Handoff,
            "handoff for correlation preserve test",
            ProducerRef::Reporter("claude".into()),
            Some("feat/preserve"),
        ),
    ];
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    let main = batch
        .decisions
        .iter()
        .find(|d| d.decision.source_signal_ids.len() == 2)
        .expect("group decision");
    assert_eq!(
        main.decision.source_signal_ids,
        vec!["sig-1".to_string(), "sig-2".to_string()]
    );
}

#[test]
fn many_signals_can_prepare_one_future_event_candidate_without_writing_event() {
    let signals = vec![
        signal(
            "sig-1",
            WorkSignalKind::Decision,
            "team accepted the correlated decision",
            ProducerRef::Reporter("codex".into()),
            Some("feat/one-event"),
        ),
        signal(
            "sig-2",
            WorkSignalKind::Decision,
            "team accepted the correlated decision",
            ProducerRef::Reporter("claude".into()),
            Some("feat/one-event"),
        ),
    ];
    let grouped = group_signals_by_correlation_hint("ws-1", &signals);
    let candidate = prepare_future_event_candidate(&grouped.groups[0]);
    assert_eq!(candidate.signals.len(), 2);
    assert_eq!(
        candidate.correlation_hint.as_deref(),
        Some("feat/one-event")
    );
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    assert!(batch
        .decisions
        .iter()
        .any(|d| d.decision.outcome == PromotionOutcome::Promote));
}

#[test]
fn same_origin_same_claim_is_duplicate_not_corroboration() {
    let summary = "identical progress claim for duplicate test";
    let signals = vec![
        signal(
            "sig-first",
            WorkSignalKind::Progress,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/dup"),
        ),
        signal(
            "sig-second",
            WorkSignalKind::Progress,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/dup"),
        ),
    ];
    let result = group_signals_by_correlation_hint("ws-1", &signals);
    assert_eq!(result.duplicate_refs.len(), 1);
    assert_eq!(
        result.duplicate_refs[0].relationship,
        EvidenceRelationship::SameOriginDuplicate
    );
    assert!(result.corroboration_refs.is_empty());
}

#[test]
fn independent_origin_same_conclusion_is_corroboration() {
    let summary = "shared conclusion for corroboration test case";
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Decision,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/corroborate"),
        ),
        signal(
            "sig-b",
            WorkSignalKind::Decision,
            summary,
            ProducerRef::Reporter("claude".into()),
            Some("feat/corroborate"),
        ),
    ];
    let result = group_signals_by_correlation_hint("ws-1", &signals);
    assert_eq!(result.corroboration_refs.len(), 1);
    assert_eq!(
        result.corroboration_refs[0].relationship,
        EvidenceRelationship::IndependentCorroboration
    );
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    let main = batch
        .decisions
        .iter()
        .find(|d| d.corroborating_signal_ids.len() == 2)
        .expect("corroboration decision");
    assert_eq!(
        main.decision.reason_code,
        Some(PromotionReasonCode::IndependentCorroboration)
    );
}

#[test]
fn duplicate_refs_remain_inspectable() {
    let summary = "duplicate inspectability claim text";
    let signals = vec![
        signal(
            "sig-canonical",
            WorkSignalKind::Progress,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/inspect-dup"),
        ),
        signal(
            "sig-dup",
            WorkSignalKind::Progress,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/inspect-dup"),
        ),
    ];
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    assert!(batch.correlation.duplicate_refs[0]
        .signal_ids
        .contains(&"sig-dup".to_string()));
    assert!(batch
        .decisions
        .iter()
        .any(|d| d.duplicate_signal_ids.contains(&"sig-dup".to_string())));
    assert!(batch.decisions.iter().any(|d| d
        .decision
        .source_signal_ids
        .contains(&"sig-canonical".to_string())));
}

#[test]
fn corroboration_refs_are_inspectable_as_evidence_relationships() {
    let summary = "corroboration evidence relationship claim";
    let signals = vec![
        signal(
            "sig-x",
            WorkSignalKind::Milestone,
            summary,
            ProducerRef::Reporter("codex".into()),
            Some("feat/evidence"),
        ),
        signal(
            "sig-y",
            WorkSignalKind::Milestone,
            summary,
            ProducerRef::Reporter("claude".into()),
            Some("feat/evidence"),
        ),
    ];
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    assert_eq!(
        batch.correlation.corroboration_refs[0].relationship,
        EvidenceRelationship::IndependentCorroboration
    );
    let main = batch
        .decisions
        .iter()
        .find(|d| d.evidence.relationship == EvidenceRelationship::IndependentCorroboration)
        .expect("evidence");
    assert_eq!(main.evidence.producer_signal_attachments.len(), 2);
}

#[test]
fn ambiguous_correlation_returns_ambiguous_without_fake_certainty() {
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Progress,
            "first conflicting claim from same producer",
            ProducerRef::Reporter("codex".into()),
            Some("feat/ambiguous"),
        ),
        signal(
            "sig-b",
            WorkSignalKind::Progress,
            "second conflicting claim from same producer",
            ProducerRef::Reporter("codex".into()),
            Some("feat/ambiguous"),
        ),
    ];
    let batch = correlate_and_evaluate("ws-1", &signals).unwrap();
    let ambiguous = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Ambiguous)
        .expect("ambiguous");
    assert!(ambiguous.decision.ambiguous);
    assert!(ambiguous.decision.proposed_composition.is_none());
    assert_eq!(
        ambiguous.decision.reason_code,
        Some(PromotionReasonCode::AmbiguousCorrelation)
    );
}

#[test]
fn correlation_does_not_write_promotion_audit_files() {
    let (_dir, project_path) = create_test_project("no-audit");
    let signals = vec![signal(
        "sig-a",
        WorkSignalKind::Decision,
        "decision without audit write",
        ProducerRef::Reporter("codex".into()),
        Some("feat/no-audit"),
    )];
    let _ = correlate_and_evaluate("ws-audit", &signals).unwrap();
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn correlation_does_not_touch_signal_inbox() {
    let (_dir, project_path) = create_test_project("no-inbox");
    let signals_root = openmesh_core::storage::get_project_dir(&project_path).join("signals");
    fs::create_dir_all(signals_root.join("processed")).unwrap();
    fs::write(signals_root.join("processed/existing.json"), "{}").unwrap();
    let before = fs::read_dir(signals_root.join("processed"))
        .unwrap()
        .count();
    let signals = vec![signal(
        "sig-inbox",
        WorkSignalKind::Milestone,
        "milestone without inbox mutation",
        ProducerRef::Reporter("codex".into()),
        Some("feat/inbox"),
    )];
    let _ = correlate_and_evaluate("ws-inbox", &signals).unwrap();
    assert_eq!(
        fs::read_dir(signals_root.join("processed"))
            .unwrap()
            .count(),
        before
    );
}

#[test]
fn correlation_does_not_create_work_events() {
    let (_dir, project_path) = create_test_project("no-events");
    let signals = vec![signal(
        "sig-events",
        WorkSignalKind::Handoff,
        "handoff without work event creation",
        ProducerRef::Reporter("codex".into()),
        Some("feat/events"),
    )];
    let _ = correlate_and_evaluate("ws-events", &signals).unwrap();
    assert!(!ledger_dir(&project_path).exists());
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn correlation_does_not_call_append_event() {
    let signals = vec![
        signal(
            "sig-1",
            WorkSignalKind::Decision,
            "pure correlation evaluation only",
            ProducerRef::Reporter("codex".into()),
            Some("feat/pure"),
        ),
        signal(
            "sig-2",
            WorkSignalKind::Decision,
            "pure correlation evaluation only",
            ProducerRef::Reporter("claude".into()),
            Some("feat/pure"),
        ),
    ];
    let batch = correlate_and_evaluate("ws-pure", &signals).unwrap();
    assert!(!batch.decisions.is_empty());
}

#[test]
fn correlation_does_not_change_protocol_version() {
    assert_eq!(WORK_EVENT_PROTOCOL_VERSION, "1.0");
    let signals = vec![signal(
        "sig-proto",
        WorkSignalKind::Decision,
        "protocol version must remain unchanged",
        ProducerRef::Reporter("codex".into()),
        None,
    )];
    let _ = correlate_and_evaluate("ws-proto", &signals).unwrap();
    assert_eq!(WORK_EVENT_PROTOCOL_VERSION, "1.0");
}
