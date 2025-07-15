//! Dev Track 0.1.3.5 Checkpoint E2 — promotion application to WorkEvent ledger.

use openmesh_core::domain::{
    ActorRef, EvidenceAttachment, EvidenceRef, ProducerRef, WorkEvent, WorkSignalKind,
    WORK_EVENT_PROTOCOL_VERSION, WORK_EVENT_PROTOCOL_VERSION_PROMOTED,
};
use openmesh_core::events::{append_event, get_event, ledger_dir, list_events};
use openmesh_core::intelligence::NoopContinuityIntelligence;
use openmesh_core::promotion::{
    ambiguous_case_from_request, apply_correlation_decision, apply_promotion_decision,
    apply_promotion_decision_with_intelligence, correlate_and_evaluate, evaluate_promotion_case,
    get_decision_record, list_decision_records, promoted_event_id, write_decision_record,
    PromotedEventOutcome, PromotionApplyRequest, PromotionCase, PromotionDecisionRecord,
    PromotionOutcome, SignalRef, WriteDecisionOutcome, PROMOTED_EVENT_ID_PREFIX,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn signal(
    id: &str,
    kind: WorkSignalKind,
    summary: &str,
    actor: ActorRef,
    producer: ProducerRef,
    hint: Option<&str>,
    timestamp: &str,
) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind,
        summary: summary.into(),
        producer,
        actor,
        timestamp: timestamp.into(),
        correlation_hint: hint.map(str::to_string),
        evidence_refs: vec![EvidenceRef::ProducerSignal(id.into())],
    }
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-promote-e2-{name}-{}-{unique}",
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
        "name": "Test Project",
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

    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

fn promote_decision_signal(workspace_id: &str) -> (SignalRef, PromotionCase) {
    let sig = signal(
        "sig-promote-1",
        WorkSignalKind::Decision,
        "We decided to ship the promotion ledger path in 0.1.3.5",
        ActorRef::Person("ter".into()),
        ProducerRef::Reporter("codex".into()),
        None,
        "2026-07-15T18:00:00Z",
    );
    let case = PromotionCase {
        workspace_id: workspace_id.to_string(),
        signals: vec![sig.clone()],
        correlation_hint: None,
    };
    (sig, case)
}

fn apply_case_promote(
    project_path: &str,
    workspace_id: &str,
    case: &PromotionCase,
    recorded_at: &str,
) -> openmesh_core::promotion::ApplyPromotionOutcome {
    let decision = evaluate_promotion_case(case).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Promote);
    apply_promotion_decision(
        project_path,
        &PromotionApplyRequest {
            workspace_id: workspace_id.to_string(),
            decision,
            signals: case.signals.clone(),
            evidence: None,
            recorded_at: recorded_at.to_string(),
        },
    )
    .unwrap()
}

fn signals_file_count(project_path: &str) -> usize {
    let signals_root = openmesh_core::storage::get_project_dir(project_path).join("signals");
    if !signals_root.exists() {
        return 0;
    }
    let mut count = 0usize;
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = signals_root.join(bucket);
        if dir.exists() {
            count += fs::read_dir(dir).map(|rd| rd.count()).unwrap_or(0);
        }
    }
    count
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn promote_decision_appends_v1_1_work_event() {
    let (_dir, project_path, project_id) = create_test_project("append-v11");
    let (_sig, case) = promote_decision_signal(&project_id);

    let outcome = apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:00:00Z");
    assert!(matches!(outcome.audit, WriteDecisionOutcome::Created(_)));
    assert!(matches!(
        outcome.event,
        Some(PromotedEventOutcome::Created(_))
    ));

    let events = list_events(&project_path).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0].protocol_version,
        WORK_EVENT_PROTOCOL_VERSION_PROMOTED
    );
}

#[test]
fn promoted_work_event_has_required_actor() {
    let (_dir, project_path, project_id) = create_test_project("actor-required");
    let (_sig, case) = promote_decision_signal(&project_id);

    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:01:00Z");

    let event = list_events(&project_path).unwrap().pop().unwrap();
    assert!(matches!(event.actor, Some(ActorRef::Person(_))));
}

#[test]
fn promoted_work_event_uses_protocol_1_1_not_1_0() {
    let (_dir, project_path, project_id) = create_test_project("not-v10");
    let (_sig, case) = promote_decision_signal(&project_id);

    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:02:00Z");

    let event = list_events(&project_path).unwrap().pop().unwrap();
    assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION_PROMOTED);
    assert_ne!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
}

#[test]
fn promote_decision_writes_promotion_audit_record() {
    let (_dir, project_path, project_id) = create_test_project("audit-promote");
    let (_sig, case) = promote_decision_signal(&project_id);
    let decision = evaluate_promotion_case(&case).unwrap();

    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:03:00Z");

    let record = get_decision_record(&project_path, &decision.promotion_key)
        .unwrap()
        .expect("audit present");
    assert_eq!(record.outcome, PromotionOutcome::Promote);
    assert_eq!(record.source_signal_ids, decision.source_signal_ids);
}

#[test]
fn suppress_decision_writes_audit_but_no_work_event() {
    let (_dir, project_path, project_id) = create_test_project("suppress");
    let sig = signal(
        "sig-spam",
        WorkSignalKind::SessionEnd,
        "session ended",
        ActorRef::Unknown,
        ProducerRef::Reporter("codex".into()),
        None,
        "2026-07-15T18:00:00Z",
    );
    let case = PromotionCase {
        workspace_id: project_id.clone(),
        signals: vec![sig],
        correlation_hint: None,
    };
    let decision = evaluate_promotion_case(&case).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Suppress);

    let outcome = apply_promotion_decision(
        &project_path,
        &PromotionApplyRequest {
            workspace_id: project_id.clone(),
            decision: decision.clone(),
            signals: case.signals,
            evidence: None,
            recorded_at: "2026-07-15T19:04:00Z".into(),
        },
    )
    .unwrap();
    assert!(outcome.event.is_none());
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 1);
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn defer_decision_writes_audit_but_no_work_event() {
    let (_dir, project_path, project_id) = create_test_project("defer");
    let sig = signal(
        "sig-defer",
        WorkSignalKind::UnresolvedQuestion,
        "open question without evidence",
        ActorRef::Unknown,
        ProducerRef::Reporter("codex".into()),
        None,
        "2026-07-15T18:00:00Z",
    );
    let mut sig_no_evidence = sig.clone();
    sig_no_evidence.evidence_refs.clear();
    let case = PromotionCase {
        workspace_id: project_id.clone(),
        signals: vec![sig_no_evidence],
        correlation_hint: None,
    };
    let decision = evaluate_promotion_case(&case).unwrap();
    assert_eq!(decision.outcome, PromotionOutcome::Defer);

    let outcome = apply_promotion_decision(
        &project_path,
        &PromotionApplyRequest {
            workspace_id: project_id.clone(),
            decision,
            signals: case.signals,
            evidence: None,
            recorded_at: "2026-07-15T19:05:00Z".into(),
        },
    )
    .unwrap();
    assert!(outcome.event.is_none());
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 1);
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn ambiguous_decision_writes_audit_but_no_work_event() {
    let (_dir, project_path, project_id) = create_test_project("ambiguous");
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Handoff,
            "handoff summary one with enough detail",
            ActorRef::Person("a".into()),
            ProducerRef::Reporter("codex".into()),
            Some("feat/x"),
            "2026-07-15T18:00:00Z",
        ),
        signal(
            "sig-b",
            WorkSignalKind::ReviewRequired,
            "review required summary two with detail",
            ActorRef::Person("b".into()),
            ProducerRef::Reporter("claude".into()),
            Some("feat/x"),
            "2026-07-15T18:01:00Z",
        ),
    ];
    let batch = correlate_and_evaluate(&project_id, &signals).unwrap();
    let ambiguous = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Ambiguous)
        .expect("kind conflict yields ambiguous");

    let outcome =
        apply_correlation_decision(&project_path, ambiguous, "2026-07-15T19:06:00Z").unwrap();
    assert!(outcome.event.is_none());
    assert!(ambiguous.decision.ambiguous);
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 1);
    assert!(list_events(&project_path).unwrap().is_empty());
}

#[test]
fn same_promotion_key_rerun_creates_no_duplicate_audit_or_event() {
    let (_dir, project_path, project_id) = create_test_project("idempotent-rerun");
    let (_sig, case) = promote_decision_signal(&project_id);

    let first = apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:07:00Z");
    let second = apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:08:00Z");

    assert!(matches!(first.audit, WriteDecisionOutcome::Created(_)));
    assert!(matches!(second.audit, WriteDecisionOutcome::Existing(_)));
    assert!(matches!(
        first.event,
        Some(PromotedEventOutcome::Created(_))
    ));
    assert!(matches!(
        second.event,
        Some(PromotedEventOutcome::Existing(_))
    ));
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 1);
    assert_eq!(list_events(&project_path).unwrap().len(), 1);
}

#[test]
fn deterministic_event_id_is_derived_from_promotion_key() {
    let (_dir, project_path, project_id) = create_test_project("event-id");
    let (_sig, case) = promote_decision_signal(&project_id);
    let decision = evaluate_promotion_case(&case).unwrap();

    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:09:00Z");

    let event = list_events(&project_path).unwrap().pop().unwrap();
    assert!(event.event_id.starts_with(PROMOTED_EVENT_ID_PREFIX));
    assert_eq!(event.event_id, promoted_event_id(&decision.promotion_key));
}

#[test]
fn many_signals_can_promote_to_one_work_event() {
    let (_dir, project_path, project_id) = create_test_project("many-to-one");
    let signals = vec![
        signal(
            "sig-corr-a",
            WorkSignalKind::Progress,
            "feature branch progress with corroboration detail",
            ActorRef::Unknown,
            ProducerRef::Reporter("codex".into()),
            Some("feat/corr"),
            "2026-07-15T17:00:00Z",
        ),
        signal(
            "sig-corr-b",
            WorkSignalKind::Progress,
            "feature branch progress with corroboration detail",
            ActorRef::Unknown,
            ProducerRef::Reporter("claude".into()),
            Some("feat/corr"),
            "2026-07-15T18:00:00Z",
        ),
    ];
    let batch = correlate_and_evaluate(&project_id, &signals).unwrap();
    let promote = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Promote)
        .expect("corroboration promotes");

    apply_correlation_decision(&project_path, promote, "2026-07-15T19:10:00Z").unwrap();

    let events = list_events(&project_path).unwrap();
    assert_eq!(events.len(), 1);
    let producer_refs: Vec<_> = events[0]
        .evidence
        .iter()
        .filter_map(|a| match &a.evidence_ref {
            EvidenceRef::ProducerSignal(id) => Some(id.clone()),
            _ => None,
        })
        .collect();
    assert!(producer_refs.contains(&"sig-corr-a".to_string()));
    assert!(producer_refs.contains(&"sig-corr-b".to_string()));
}

#[test]
fn corroborating_signals_are_inspectable_in_event_evidence() {
    let (_dir, project_path, project_id) = create_test_project("corroboration-evidence");
    let signals = vec![
        signal(
            "sig-ev-a",
            WorkSignalKind::Milestone,
            "milestone reached on shared feature work",
            ActorRef::Unknown,
            ProducerRef::Reporter("codex".into()),
            Some("release-1"),
            "2026-07-15T17:30:00Z",
        ),
        signal(
            "sig-ev-b",
            WorkSignalKind::Milestone,
            "milestone reached on shared feature work",
            ActorRef::Unknown,
            ProducerRef::Reporter("claude".into()),
            Some("release-1"),
            "2026-07-15T18:30:00Z",
        ),
    ];
    let batch = correlate_and_evaluate(&project_id, &signals).unwrap();
    let promote = batch
        .decisions
        .iter()
        .find(|d| d.decision.outcome == PromotionOutcome::Promote)
        .unwrap();

    apply_correlation_decision(&project_path, promote, "2026-07-15T19:11:00Z").unwrap();

    let event = list_events(&project_path).unwrap().pop().unwrap();
    let ids: Vec<_> = event
        .evidence
        .iter()
        .filter_map(|a| match &a.evidence_ref {
            EvidenceRef::ProducerSignal(id) => Some(id.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"sig-ev-a"));
    assert!(ids.contains(&"sig-ev-b"));
}

#[test]
fn same_origin_duplicates_do_not_create_separate_events() {
    let (_dir, project_path, project_id) = create_test_project("same-origin-dup");
    let summary = "identical progress claim from same reporter";
    let signals = vec![
        signal(
            "sig-dup-a",
            WorkSignalKind::Progress,
            summary,
            ActorRef::Unknown,
            ProducerRef::Reporter("codex".into()),
            Some("feat/dup"),
            "2026-07-15T17:00:00Z",
        ),
        signal(
            "sig-dup-b",
            WorkSignalKind::Progress,
            summary,
            ActorRef::Unknown,
            ProducerRef::Reporter("codex".into()),
            Some("feat/dup"),
            "2026-07-15T17:01:00Z",
        ),
    ];
    let batch = correlate_and_evaluate(&project_id, &signals).unwrap();

    let promote_count = batch
        .decisions
        .iter()
        .filter(|d| d.decision.outcome == PromotionOutcome::Promote)
        .count();
    let suppress_count = batch
        .decisions
        .iter()
        .filter(|d| d.decision.outcome == PromotionOutcome::Suppress)
        .count();
    assert_eq!(promote_count, 1);
    assert_eq!(suppress_count, 1);

    for decision in &batch.decisions {
        apply_correlation_decision(&project_path, decision, "2026-07-15T19:12:00Z").unwrap();
    }

    assert_eq!(list_events(&project_path).unwrap().len(), 1);
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 2);
}

#[test]
fn missing_actor_prevents_invalid_work_event_write() {
    let (_dir, project_path, project_id) = create_test_project("missing-composition");
    let sig = signal(
        "sig-no-compose",
        WorkSignalKind::Decision,
        "decision without proposed composition path",
        ActorRef::Person("ter".into()),
        ProducerRef::Reporter("codex".into()),
        None,
        "2026-07-15T18:00:00Z",
    );
    let case = PromotionCase {
        workspace_id: project_id.clone(),
        signals: vec![sig.clone()],
        correlation_hint: None,
    };
    let mut decision = evaluate_promotion_case(&case).unwrap();
    decision.outcome = PromotionOutcome::Promote;
    decision.proposed_composition = None;

    let err = apply_promotion_decision(
        &project_path,
        &PromotionApplyRequest {
            workspace_id: project_id.clone(),
            decision,
            signals: case.signals,
            evidence: None,
            recorded_at: "2026-07-15T19:13:00Z".into(),
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("proposed_composition"));
    assert!(list_events(&project_path).unwrap().is_empty());
    assert!(list_decision_records(&project_path).unwrap().is_empty());
}

#[test]
fn promotion_apply_does_not_touch_signal_inbox() {
    let (_dir, project_path, project_id) = create_test_project("inbox-untouched");
    let signals_dir = openmesh_core::storage::get_project_dir(&project_path).join("signals");
    fs::create_dir_all(signals_dir.join("pending")).unwrap();
    fs::write(
        signals_dir.join("pending/inbox-signal.json"),
        r#"{"signalId":"x"}"#,
    )
    .unwrap();
    let before = signals_file_count(&project_path);

    let (_sig, case) = promote_decision_signal(&project_id);
    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:14:00Z");

    assert_eq!(signals_file_count(&project_path), before);
}

#[test]
fn promotion_apply_does_not_read_processed_bucket() {
    let (_dir, project_path, project_id) = create_test_project("processed-untouched");
    let processed =
        openmesh_core::storage::get_project_dir(&project_path).join("signals/processed");
    fs::create_dir_all(&processed).unwrap();
    let marker = processed.join("processed-marker.json");
    fs::write(&marker, r#"{"marker":true}"#).unwrap();
    let bytes_before = fs::read(&marker).unwrap();

    let (_sig, case) = promote_decision_signal(&project_id);
    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:15:00Z");

    assert_eq!(fs::read(&marker).unwrap(), bytes_before);
}

#[test]
fn promotion_apply_does_not_create_current_state_or_projection() {
    let (_dir, project_path, project_id) = create_test_project("no-projection");
    let om = openmesh_core::storage::get_project_dir(&project_path);

    let (_sig, case) = promote_decision_signal(&project_id);
    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:16:00Z");

    assert!(!om.join("current_state").exists());
    assert!(!om.join("projection").exists());
    assert!(!om.join("catch_me_up").exists());
}

#[test]
fn promotion_apply_does_not_add_cli_tauri_desktop_surface() {
    let root = workspace_root();
    let forbidden = [
        "apply_promotion_decision",
        "apply_correlation_decision",
        "compose_work_event_from_group",
    ];
    for surface in ["crates/openmesh-cli/src", "src-tauri/src", "frontend/src"] {
        let dir = root.join(surface);
        if !dir.exists() {
            continue;
        }
        let mut files = Vec::new();
        collect_rs_files(&dir, &mut files);
        for path in files {
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            for term in forbidden {
                assert!(
                    !content.contains(term),
                    "product surface must not reference promotion apply API `{term}`: {}",
                    path.display()
                );
            }
        }
    }
}

#[test]
fn old_v1_0_events_remain_unmodified_after_promotion() {
    let (_dir, project_path, project_id) = create_test_project("legacy-v10");
    let legacy_path = ledger_dir(&project_path).join("evt-legacy-v10.json");
    fs::create_dir_all(ledger_dir(&project_path)).unwrap();

    let mut legacy = WorkEvent::new(
        "evt-legacy-v10",
        &project_id,
        "work.progress",
        "Legacy 1.0 event before promotion apply.",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::ProducerSignal("sig-legacy".into()),
            observed_at: None,
        }],
        "2026-07-01T12:00:00Z",
    );
    legacy.protocol_version = WORK_EVENT_PROTOCOL_VERSION.to_string();
    legacy.actor = None;
    append_event(&project_path, &legacy).unwrap();
    let bytes_before = fs::read(&legacy_path).unwrap();

    let (_sig, case) = promote_decision_signal(&project_id);
    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T19:17:00Z");

    assert_eq!(fs::read(&legacy_path).unwrap(), bytes_before);
    let restored = get_event(&project_path, "evt-legacy-v10")
        .unwrap()
        .expect("legacy event");
    assert_eq!(restored.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
    assert!(restored.actor.is_none());
    assert_eq!(list_events(&project_path).unwrap().len(), 2);
}

#[test]
fn promote_audit_and_event_consistency_is_enforced() {
    let (_dir, project_path, project_id) = create_test_project("audit-event-consistency");
    let (_sig, case) = promote_decision_signal(&project_id);

    let outcome = apply_case_promote(&project_path, &project_id, &case, "2026-07-15T20:10:00Z");
    assert!(matches!(outcome.audit, WriteDecisionOutcome::Created(_)));
    assert!(matches!(
        outcome.event,
        Some(PromotedEventOutcome::Created(_))
    ));

    let decision = evaluate_promotion_case(&case).unwrap();
    let record = get_decision_record(&project_path, &decision.promotion_key)
        .unwrap()
        .expect("audit");
    assert_eq!(record.outcome, PromotionOutcome::Promote);
    assert_eq!(
        get_event(&project_path, &promoted_event_id(&decision.promotion_key))
            .unwrap()
            .is_some(),
        true
    );
}

#[test]
fn existing_promote_audit_without_event_is_not_silent_success() {
    let (_dir, project_path, project_id) = create_test_project("audit-without-event");
    let (_sig, case) = promote_decision_signal(&project_id);
    let decision = evaluate_promotion_case(&case).unwrap();
    let record = PromotionDecisionRecord::from_decision(
        project_id.clone(),
        decision.clone(),
        None,
        "2026-07-15T20:11:00Z".into(),
    );
    write_decision_record(&project_path, &record).unwrap();
    assert!(
        get_event(&project_path, &promoted_event_id(&decision.promotion_key))
            .unwrap()
            .is_none()
    );

    let outcome = apply_promotion_decision(
        &project_path,
        &PromotionApplyRequest {
            workspace_id: project_id.clone(),
            decision: decision.clone(),
            signals: case.signals.clone(),
            evidence: None,
            recorded_at: "2026-07-15T20:12:00Z".into(),
        },
    )
    .unwrap();

    assert!(matches!(
        outcome.event,
        Some(PromotedEventOutcome::Created(_)) | Some(PromotedEventOutcome::Existing(_))
    ));
    assert!(
        get_event(&project_path, &promoted_event_id(&decision.promotion_key))
            .unwrap()
            .is_some()
    );
}

#[test]
fn retry_after_existing_promote_event_is_idempotent() {
    let (_dir, project_path, project_id) = create_test_project("retry-idempotent");
    let (_sig, case) = promote_decision_signal(&project_id);
    let decision = evaluate_promotion_case(&case).unwrap();

    apply_case_promote(&project_path, &project_id, &case, "2026-07-15T20:13:00Z");
    let event_path = ledger_dir(&project_path).join(format!(
        "{}.json",
        promoted_event_id(&decision.promotion_key)
    ));
    let bytes_before = fs::read(&event_path).unwrap();

    let second = apply_case_promote(&project_path, &project_id, &case, "2026-07-15T20:14:00Z");
    assert!(matches!(
        second.event,
        Some(PromotedEventOutcome::Existing(_))
    ));
    assert_eq!(fs::read(&event_path).unwrap(), bytes_before);
    assert_eq!(list_events(&project_path).unwrap().len(), 1);
}

#[test]
fn non_promote_outcomes_have_audit_but_no_event() {
    let (_dir, project_path, project_id) = create_test_project("non-promote-bundle");
    let signals = vec![
        signal(
            "sig-a",
            WorkSignalKind::Handoff,
            "handoff summary one with enough detail",
            ActorRef::Person("a".into()),
            ProducerRef::Reporter("codex".into()),
            Some("feat/x"),
            "2026-07-15T18:00:00Z",
        ),
        signal(
            "sig-b",
            WorkSignalKind::ReviewRequired,
            "review required summary two with detail",
            ActorRef::Person("b".into()),
            ProducerRef::Reporter("claude".into()),
            Some("feat/x"),
            "2026-07-15T18:01:00Z",
        ),
    ];
    let batch = correlate_and_evaluate(&project_id, &signals).unwrap();
    for correlated in batch
        .decisions
        .iter()
        .filter(|d| d.decision.outcome != PromotionOutcome::Promote)
    {
        apply_correlation_decision(&project_path, correlated, "2026-07-15T20:15:00Z").unwrap();
    }

    assert!(list_events(&project_path).unwrap().is_empty());
    assert!(list_decision_records(&project_path)
        .unwrap()
        .iter()
        .all(|r| { r.outcome != PromotionOutcome::Promote }));
}

#[test]
fn ambiguous_apply_with_intelligence_stays_side_effect_free_on_noop() {
    let (_dir, project_path, project_id) = create_test_project("ambiguous-noop-apply");
    let request = {
        let signals = vec![
            signal(
                "sig-a",
                WorkSignalKind::Handoff,
                "handoff summary one with enough detail",
                ActorRef::Person("a".into()),
                ProducerRef::Reporter("codex".into()),
                Some("feat/x"),
                "2026-07-15T18:00:00Z",
            ),
            signal(
                "sig-b",
                WorkSignalKind::ReviewRequired,
                "review required summary two with detail",
                ActorRef::Person("b".into()),
                ProducerRef::Reporter("claude".into()),
                Some("feat/x"),
                "2026-07-15T18:01:00Z",
            ),
        ];
        let batch = correlate_and_evaluate(&project_id, &signals).unwrap();
        let correlated = batch
            .decisions
            .iter()
            .find(|d| d.decision.outcome == PromotionOutcome::Ambiguous)
            .unwrap();
        PromotionApplyRequest {
            workspace_id: project_id.clone(),
            decision: correlated.decision.clone(),
            signals: correlated.group.signals.clone(),
            evidence: Some(correlated.evidence.clone()),
            recorded_at: "2026-07-15T20:16:00Z".into(),
        }
    };

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
}
