//! Dev Track 0.1.3.7 Checkpoint C — Current State builder tests (temp projects / pure snapshots).

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::{
    build_current_state_projection, current_state_projection_path, load_continuity_input_snapshot,
    projections_dir, read_current_state_projection, rebuild_current_state_projection,
    write_current_state_projection, ContinuityDiagnostic, ContinuityDiagnosticKind,
    ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    ActorRef, ContinuityConfidence, ContinuitySourceKind, EvidenceRef, GitState, ProducerRef,
    SourceCounts, WorkEvent, WorkSignal, WorkSignalKind, MAX_PROJECTION_EVIDENCE_REFS,
    WORK_EVENT_PROTOCOL_VERSION, WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
};
use openmesh_core::promotion::{
    promotion_decisions_dir, PromotionDecision, PromotionDecisionRecord, PromotionKey,
    PromotionOutcome, PromotionReasonCode,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::get_project_dir;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn empty_source_counts() -> SourceCounts {
    SourceCounts {
        work_events: 0,
        processed_signals: 0,
        pending_signals: 0,
        promotion_audit_records: 0,
        quarantine_signals: 0,
        duplicate_signals: 0,
        reporter_signals: 0,
        git_signals: 0,
        heli_signals: 0,
        unknown_producer_signals: 0,
        other_producer_signals: 0,
    }
}

fn snapshot_with_counts(
    workspace_id: &str,
    pending: Vec<WorkSignal>,
    processed: Vec<WorkSignal>,
    work_events: Vec<WorkEvent>,
    promotion_audit_records: Vec<PromotionDecisionRecord>,
    diagnostics: Vec<ContinuityDiagnostic>,
) -> ContinuityInputSnapshot {
    let mut source_counts = empty_source_counts();
    source_counts.pending_signals = pending.len() as u32;
    source_counts.processed_signals = processed.len() as u32;
    source_counts.work_events = work_events.len() as u32;
    source_counts.promotion_audit_records = promotion_audit_records.len() as u32;
    for signal in pending.iter().chain(processed.iter()) {
        match signal.producer {
            ProducerRef::Reporter(_) => source_counts.reporter_signals += 1,
            ProducerRef::Git => source_counts.git_signals += 1,
            ProducerRef::Heli => source_counts.heli_signals += 1,
            ProducerRef::Native => source_counts.other_producer_signals += 1,
            _ => source_counts.unknown_producer_signals += 1,
        }
    }
    ContinuityInputSnapshot {
        workspace_id: workspace_id.into(),
        loaded_at: "2026-07-16T10:00:00Z".into(),
        pending_signals: pending,
        processed_signals: processed,
        quarantine_signals: Vec::new(),
        duplicate_signals: Vec::new(),
        work_events,
        promotion_audit_records,
        diagnostics,
        source_counts,
    }
}

fn sample_signal(
    id: &str,
    workspace_id: &str,
    kind: WorkSignalKind,
    producer: ProducerRef,
) -> WorkSignal {
    WorkSignal {
        signal_id: id.into(),
        workspace_id: workspace_id.into(),
        producer,
        actor: ActorRef::Unknown,
        kind,
        summary: format!("signal summary for {id}"),
        timestamp: "2026-07-16T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn sample_event(event_id: &str, workspace_id: &str, kind: &str, summary: &str) -> WorkEvent {
    let json = serde_json::json!({
        "eventId": event_id,
        "workspaceId": workspace_id,
        "kind": kind,
        "summary": summary,
        "timestamp": "2026-07-16T10:00:00Z",
        "evidence": [{
            "evidenceRef": { "type": "file-path", "value": "docs/overview.md" }
        }],
        "sensitivity": "private",
        "protocolVersion": WORK_EVENT_PROTOCOL_VERSION,
    });
    serde_json::from_value(json).expect("event json")
}

fn sample_promotion_record(
    workspace_id: &str,
    signal_ids: &[&str],
    outcome: PromotionOutcome,
) -> PromotionDecisionRecord {
    let ids: Vec<String> = signal_ids.iter().map(|s| (*s).to_string()).collect();
    let key = PromotionKey::from_inputs(workspace_id, &ids).unwrap();
    let decision = match outcome {
        PromotionOutcome::Suppress => {
            PromotionDecision::suppress(key.clone(), ids.clone(), PromotionReasonCode::ActivitySpam)
        }
        PromotionOutcome::Ambiguous => {
            PromotionDecision::ambiguous(key.clone(), ids.clone(), "needs review".into())
        }
        PromotionOutcome::Defer => PromotionDecision::defer(
            key.clone(),
            ids.clone(),
            PromotionReasonCode::MissingEvidence,
        ),
        PromotionOutcome::Promote => {
            PromotionDecision::defer(key.clone(), ids.clone(), PromotionReasonCode::Qualifies)
        }
    };
    PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        None,
        "2026-07-16T10:00:00Z".into(),
    )
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-current-state-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();
    let project_id = format!("proj-{name}-{unique}");
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
        "createdAt": "2026-07-16T10:00:00Z",
        "updatedAt": "2026-07-16T10:00:00Z",
    });
    fs::write(
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();
    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

fn write_processed_signal(project_path: &str, signal: &WorkSignal, filename: &str) {
    let dir = get_project_dir(project_path).join("signals/processed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{filename}.json")),
        serde_json::to_string(signal).unwrap(),
    )
    .unwrap();
}

fn bucket_snapshot(project_path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let root = get_project_dir(project_path).join("signals");
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = root.join(bucket);
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                let key = format!("{bucket}/{}", path.file_name().unwrap().to_string_lossy());
                out.insert(key, fs::read_to_string(&path).unwrap());
            }
        }
    }
    out
}

fn semantic_projection_key(
    projection: &openmesh_core::domain::CurrentStateProjection,
) -> serde_json::Value {
    serde_json::json!({
        "workspaceId": projection.workspace_id,
        "sections": projection.sections,
        "pendingAttention": projection.pending_attention,
        "sourceCounts": projection.source_counts,
        "evidenceRefs": projection.evidence_refs,
        "limitations": projection.limitations,
        "rebuildInputsHash": projection.rebuild_inputs_hash,
    })
}

#[test]
fn builds_current_state_from_empty_snapshot() {
    let snapshot = snapshot_with_counts("ws-empty", vec![], vec![], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.completed.len(), 0);
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("no continuity inputs")));
}

#[test]
fn builds_current_state_from_processed_signals() {
    let signal = sample_signal("proc-1", "ws-1", WorkSignalKind::Progress, ProducerRef::Git);
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.in_progress.len(), 1);
    assert_eq!(projection.sections.in_progress[0].source_id, "proc-1");
}

#[test]
fn builds_current_state_from_pending_signals() {
    let signal = sample_signal(
        "pend-1",
        "ws-1",
        WorkSignalKind::Progress,
        ProducerRef::Reporter("claude".into()),
    );
    let snapshot = snapshot_with_counts("ws-1", vec![signal], vec![], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.still_open.len(), 1);
    assert_eq!(projection.sections.still_open[0].unverified, Some(true));
}

#[test]
fn builds_current_state_from_work_events() {
    let event = sample_event("evt-1", "ws-1", "work.completed", "finished task");
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![], vec![event], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.completed.len(), 1);
    assert_eq!(
        projection.sections.completed[0].source,
        ContinuitySourceKind::WorkEvent
    );
}

#[test]
fn builds_current_state_from_promotion_audit() {
    let record = sample_promotion_record("ws-1", &["sig-a"], PromotionOutcome::Ambiguous);
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![], vec![], vec![record], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.needs_attention.len(), 1);
    assert_eq!(
        projection.sections.needs_attention[0].source,
        ContinuitySourceKind::PromotionAudit
    );
}

#[test]
fn maps_blockers_to_blocked_section_and_pending_attention() {
    let signal = sample_signal(
        "blk-1",
        "ws-1",
        WorkSignalKind::Blocker,
        ProducerRef::Reporter("claude".into()),
    );
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.blocked.len(), 1);
    assert!(projection
        .pending_attention
        .iter()
        .any(|a| a.reason == openmesh_core::domain::PendingAttentionReason::Blocker));
}

#[test]
fn maps_decisions_to_decisions_section() {
    let signal = sample_signal(
        "dec-1",
        "ws-1",
        WorkSignalKind::Decision,
        ProducerRef::Reporter("claude".into()),
    );
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.decisions.len(), 1);
}

#[test]
fn maps_progress_to_in_progress_section() {
    let signal = sample_signal("prog-1", "ws-1", WorkSignalKind::Progress, ProducerRef::Git);
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.in_progress.len(), 1);
    assert_eq!(projection.sections.in_progress[0].producer, "git");
}

#[test]
fn maps_unresolved_questions_to_needs_attention() {
    let signal = sample_signal(
        "q-1",
        "ws-1",
        WorkSignalKind::UnresolvedQuestion,
        ProducerRef::Reporter("claude".into()),
    );
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.sections.needs_attention.len(), 1);
}

#[test]
fn diagnostics_become_limitations_and_pending_attention() {
    let diagnostic = ContinuityDiagnostic {
        kind: ContinuityDiagnosticKind::SignalBucket,
        location: "Processed:broken.json".into(),
        message: "invalid JSON".into(),
    };
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![], vec![], vec![], vec![diagnostic]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("invalid JSON")));
    assert!(!projection.pending_attention.is_empty());
}

#[test]
fn ambiguous_inputs_remain_visible_without_fake_certainty() {
    let mut a = sample_signal("a-1", "ws-1", WorkSignalKind::Progress, ProducerRef::Git);
    let mut b = sample_signal("b-1", "ws-1", WorkSignalKind::Blocker, ProducerRef::Git);
    a.correlation_hint = Some("corr-1".into());
    b.correlation_hint = Some("corr-1".into());
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![a, b], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert!(projection
        .sections
        .in_progress
        .iter()
        .chain(projection.sections.blocked.iter())
        .any(|i| i.confidence == ContinuityConfidence::Ambiguous));
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("ambiguous correlation hint")));
}

#[test]
fn source_counts_are_preserved_in_projection() {
    let signal = sample_signal("git-1", "ws-1", WorkSignalKind::Progress, ProducerRef::Git);
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert_eq!(projection.source_counts.processed_signals, 1);
    assert_eq!(projection.source_counts.git_signals, 1);
}

#[test]
fn evidence_refs_are_preserved_and_bounded() {
    let mut signal = sample_signal("ev-1", "ws-1", WorkSignalKind::Progress, ProducerRef::Git);
    signal.protocol_version = WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE.into();
    signal.evidence_refs = vec![
        EvidenceRef::FilePath("a.md".into()),
        EvidenceRef::GitState(GitState {
            repo_id: "fnv1a-abc123def456".into(),
            branch: "main".into(),
            head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
            dirty: false,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            changed_paths: vec![],
            observed_at: "2026-07-16T10:00:00Z".into(),
            ahead: Some(0),
            behind: Some(0),
            base_ref: None,
            worktree_root: None,
        }),
    ];
    let snapshot = snapshot_with_counts("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("valid");
    assert!(!projection.evidence_refs.is_empty());
    assert!(projection.evidence_refs.len() <= MAX_PROJECTION_EVIDENCE_REFS);
}

#[test]
fn projection_validation_runs_before_write() {
    let (_dir, project_path, project_id) = create_test_project("invalid-write");
    let snapshot = snapshot_with_counts(&project_id, vec![], vec![], vec![], vec![], vec![]);
    let mut projection = build_current_state_projection(&snapshot).expect("valid build");
    projection.rebuild_inputs_hash = "not-a-valid-hash".into();
    let err = write_current_state_projection(&project_path, &projection).expect_err("reject");
    assert!(err.to_string().contains("validation") || err.to_string().contains("rebuild"));
    assert!(!current_state_projection_path(&project_path).exists());
}

#[test]
fn writes_current_state_projection_file() {
    let (_dir, project_path, project_id) = create_test_project("write-projection");
    let signal = sample_signal(
        "sig-1",
        &project_id,
        WorkSignalKind::Progress,
        ProducerRef::Git,
    );
    write_processed_signal(&project_path, &signal, "sig-1");
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(current_state_projection_path(&project_path).exists());
}

#[test]
fn reads_current_state_projection_file() {
    let (_dir, project_path, project_id) = create_test_project("read-projection");
    let signal = sample_signal(
        "sig-1",
        &project_id,
        WorkSignalKind::Progress,
        ProducerRef::Git,
    );
    write_processed_signal(&project_path, &signal, "sig-1");
    rebuild_current_state_projection(&project_path).expect("rebuild");
    let read = read_current_state_projection(&project_path).expect("read");
    assert_eq!(read.workspace_id, project_id);
    assert_eq!(read.sections.in_progress.len(), 1);
}

#[test]
fn rebuild_current_state_projection_is_idempotent() {
    let (_dir, project_path, project_id) = create_test_project("idempotent");
    let signal = sample_signal(
        "sig-1",
        &project_id,
        WorkSignalKind::Progress,
        ProducerRef::Git,
    );
    write_processed_signal(&project_path, &signal, "sig-1");
    rebuild_current_state_projection(&project_path).expect("first");
    let first = read_current_state_projection(&project_path).expect("read first");
    rebuild_current_state_projection(&project_path).expect("second");
    let second = read_current_state_projection(&project_path).expect("read second");
    assert_eq!(
        semantic_projection_key(&first),
        semantic_projection_key(&second)
    );
}

#[test]
fn projection_write_does_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("no-signal-mutation");
    let signal = sample_signal(
        "sig-1",
        &project_id,
        WorkSignalKind::Progress,
        ProducerRef::Git,
    );
    write_signal(
        &project_path,
        &sample_signal(
            "pend-1",
            &project_id,
            WorkSignalKind::ReviewRequired,
            ProducerRef::Reporter("claude".into()),
        ),
    )
    .unwrap();
    write_processed_signal(&project_path, &signal, "sig-1");
    let before = bucket_snapshot(&project_path);
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn projection_write_does_not_create_work_events_or_promotion_audit() {
    let (_dir, project_path, project_id) = create_test_project("no-event-audit");
    write_processed_signal(
        &project_path,
        &sample_signal(
            "sig-1",
            &project_id,
            WorkSignalKind::Progress,
            ProducerRef::Git,
        ),
        "sig-1",
    );
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(!get_project_dir(&project_path)
        .join("events/ledger")
        .exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn checkpoint_c_does_not_create_catch_up_view() {
    let (_dir, project_path, project_id) = create_test_project("no-catch-up");
    write_processed_signal(
        &project_path,
        &sample_signal(
            "sig-1",
            &project_id,
            WorkSignalKind::Progress,
            ProducerRef::Git,
        ),
        "sig-1",
    );
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(!projections_dir(&project_path)
        .join("catch-up-checkpoint.json")
        .exists());
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    assert!(serde_json::to_value(&snapshot)
        .unwrap()
        .as_object()
        .unwrap()
        .get("catchUpView")
        .is_none());
}
