//! Dev Track 0.1.3.7 Checkpoint D — Catch-up builder tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, projections_dir, ContinuityDiagnostic,
    ContinuityDiagnosticKind, ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    ActorRef, CatchUpWindow, ContinuityConfidence, CurrentStateProjection, EvidenceRef,
    ProducerRef, SourceCounts, WorkEvent, WorkSignal, WorkSignalKind, MAX_CATCH_UP_EVIDENCE_REFS,
    WORK_EVENT_PROTOCOL_VERSION,
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

fn window_all() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-15T00:00:00Z".into(),
        until: "2026-07-16T23:59:59Z".into(),
    }
}

fn window_narrow() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-16T09:00:00Z".into(),
        until: "2026-07-16T11:00:00Z".into(),
    }
}

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

fn snapshot_with(
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
    timestamp: &str,
) -> WorkSignal {
    WorkSignal {
        signal_id: id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("claude".into()),
        actor: ActorRef::Unknown,
        kind,
        summary: format!("summary for {id}"),
        timestamp: timestamp.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn sample_event(
    event_id: &str,
    workspace_id: &str,
    kind: &str,
    summary: &str,
    timestamp: &str,
) -> WorkEvent {
    let json = serde_json::json!({
        "eventId": event_id,
        "workspaceId": workspace_id,
        "kind": kind,
        "summary": summary,
        "timestamp": timestamp,
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
    recorded_at: &str,
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
        recorded_at.to_string(),
    )
}

fn current_state_for(snapshot: &ContinuityInputSnapshot) -> CurrentStateProjection {
    build_current_state_projection(snapshot).expect("current state")
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-catch-up-{name}-{}-{unique}",
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

#[test]
fn builds_empty_catch_up_view_with_explicit_empty_sections() {
    let snapshot = snapshot_with("ws-empty", vec![], vec![], vec![], vec![], vec![]);
    let current_state = current_state_for(&snapshot);
    let view = build_catch_up_view(&snapshot, &current_state, &window_all()).expect("build");
    assert_eq!(view.sections.completed.len(), 0);
    assert_eq!(view.sections.changed.len(), 0);
    assert_eq!(view.sections.blocked.len(), 0);
    assert_eq!(view.sections.decided.len(), 0);
    assert!(view.summary.contains("No changes found"));
}

#[test]
fn rejects_invalid_window_since_after_until() {
    let snapshot = snapshot_with("ws-1", vec![], vec![], vec![], vec![], vec![]);
    let current_state = current_state_for(&snapshot);
    let window = CatchUpWindow {
        since: "2026-07-16T12:00:00Z".into(),
        until: "2026-07-16T10:00:00Z".into(),
    };
    assert!(build_catch_up_view(&snapshot, &current_state, &window).is_err());
}

#[test]
fn includes_completed_records_in_completed_section() {
    let signal = sample_signal(
        "mile-1",
        "ws-1",
        WorkSignalKind::Milestone,
        "2026-07-16T10:00:00Z",
    );
    let event = sample_event(
        "evt-1",
        "ws-1",
        "work.completed",
        "done",
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![signal], vec![event], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert_eq!(view.sections.completed.len(), 2);
}

#[test]
fn includes_progress_and_scope_change_in_changed_section() {
    let progress = sample_signal(
        "prog-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let scope = sample_signal(
        "scope-1",
        "ws-1",
        WorkSignalKind::ScopeChange,
        "2026-07-16T10:30:00Z",
    );
    let snapshot = snapshot_with(
        "ws-1",
        vec![],
        vec![progress, scope],
        vec![],
        vec![],
        vec![],
    );
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert_eq!(view.sections.changed.len(), 2);
}

#[test]
fn includes_blockers_in_blocked_section() {
    let signal = sample_signal(
        "blk-1",
        "ws-1",
        WorkSignalKind::Blocker,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert_eq!(view.sections.blocked.len(), 1);
}

#[test]
fn includes_decisions_in_decided_section() {
    let signal = sample_signal(
        "dec-1",
        "ws-1",
        WorkSignalKind::Decision,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert_eq!(view.sections.decided.len(), 1);
}

#[test]
fn includes_pending_attention_in_needs_attention_section() {
    let pending = sample_signal(
        "pend-1",
        "ws-1",
        WorkSignalKind::ReviewRequired,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![pending], vec![], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(!view.sections.needs_attention.is_empty());
    assert!(!view.next_suggested_attention.is_empty());
}

#[test]
fn includes_unresolved_items_in_still_open_section() {
    let pending = sample_signal(
        "open-1",
        "ws-1",
        WorkSignalKind::SessionEnd,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![pending], vec![], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(!view.sections.still_open.is_empty());
}

#[test]
fn filters_records_by_since_until_window() {
    let inside = sample_signal(
        "in-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let outside = sample_signal(
        "out-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-15T08:00:00Z",
    );
    let snapshot = snapshot_with(
        "ws-1",
        vec![],
        vec![inside, outside],
        vec![],
        vec![],
        vec![],
    );
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_narrow())
        .expect("build");
    assert_eq!(view.sections.changed.len(), 1);
    assert_eq!(view.sections.changed[0].source_id, "in-1");
}

#[test]
fn preserves_evidence_refs_and_bounds_them() {
    let signal = sample_signal(
        "ev-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(!view.evidence_refs.is_empty());
    assert!(view.evidence_refs.len() <= MAX_CATCH_UP_EVIDENCE_REFS);
}

#[test]
fn preserves_current_state_limitations() {
    let snapshot = snapshot_with("ws-1", vec![], vec![], vec![], vec![], vec![]);
    let mut current_state = current_state_for(&snapshot);
    current_state
        .limitations
        .push("existing limitation from current state".into());
    let view = build_catch_up_view(&snapshot, &current_state, &window_all()).expect("build");
    assert!(view
        .limitations
        .iter()
        .any(|l| l.contains("existing limitation")));
}

#[test]
fn ambiguous_inputs_remain_visible_without_fake_certainty() {
    let mut a = sample_signal(
        "a-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let mut b = sample_signal(
        "b-1",
        "ws-1",
        WorkSignalKind::Blocker,
        "2026-07-16T10:05:00Z",
    );
    a.correlation_hint = Some("corr-1".into());
    b.correlation_hint = Some("corr-1".into());
    let snapshot = snapshot_with("ws-1", vec![], vec![a, b], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(view
        .sections
        .changed
        .iter()
        .chain(view.sections.blocked.iter())
        .any(|i| i.confidence == ContinuityConfidence::Ambiguous));
    assert!(view
        .limitations
        .iter()
        .any(|l| l.contains("ambiguous correlation hint")));
}

#[test]
fn diagnostics_become_needs_attention_and_limitations() {
    let diagnostic = ContinuityDiagnostic {
        kind: ContinuityDiagnosticKind::SignalBucket,
        location: "Processed:broken.json".into(),
        message: "invalid JSON".into(),
    };
    let snapshot = snapshot_with("ws-1", vec![], vec![], vec![], vec![], vec![diagnostic]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(view
        .sections
        .needs_attention
        .iter()
        .any(|i| i.summary.contains("invalid JSON")));
    assert!(view.limitations.iter().any(|l| l.contains("invalid JSON")));
}

#[test]
fn deterministic_summary_is_count_based_no_llm() {
    let signal = sample_signal(
        "prog-1",
        "ws-1",
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![signal], vec![], vec![], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert!(view.summary.contains("changed"));
    assert!(view.summary.chars().any(|c| c.is_ascii_digit()));
    assert!(!view.summary.to_lowercase().contains("llm"));
}

#[test]
fn catch_up_builder_does_not_write_projection_files() {
    let (_dir, project_path, project_id) = create_test_project("no-projection-write");
    let signal = sample_signal(
        "sig-1",
        &project_id,
        WorkSignalKind::Progress,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with(&project_id, vec![], vec![signal], vec![], vec![], vec![]);
    let current_state = current_state_for(&snapshot);
    let _view = build_catch_up_view(&snapshot, &current_state, &window_all()).expect("build");
    assert!(!projections_dir(&project_path).exists());
}

#[test]
fn catch_up_builder_does_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("no-bucket-mutation");
    write_signal(
        &project_path,
        &sample_signal(
            "pend-1",
            &project_id,
            WorkSignalKind::Progress,
            "2026-07-16T10:00:00Z",
        ),
    )
    .unwrap();
    let before = bucket_snapshot(&project_path);
    let snapshot = snapshot_with(
        &project_id,
        vec![sample_signal(
            "pend-1",
            &project_id,
            WorkSignalKind::Progress,
            "2026-07-16T10:00:00Z",
        )],
        vec![],
        vec![],
        vec![],
        vec![],
    );
    let _ = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all());
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn catch_up_builder_does_not_create_work_events_or_promotion_audit() {
    let (_dir, project_path, project_id) = create_test_project("no-events-audit");
    let snapshot = snapshot_with(
        &project_id,
        vec![],
        vec![sample_signal(
            "sig-1",
            &project_id,
            WorkSignalKind::Progress,
            "2026-07-16T10:00:00Z",
        )],
        vec![],
        vec![],
        vec![],
    );
    let _ = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all());
    assert!(!get_project_dir(&project_path)
        .join("events/ledger")
        .exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn checkpoint_e_cli_surface_does_not_add_tauri_commands() {
    let cli_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openmesh-cli/src");
    if cli_root.exists() {
        let main_rs = fs::read_to_string(cli_root.join("main.rs")).unwrap_or_default();
        assert!(
            main_rs.contains("catch_up"),
            "Checkpoint E must wire catch-up CLI"
        );
        assert!(
            main_rs.contains("state"),
            "Checkpoint E must wire state CLI"
        );
        assert!(
            !main_rs.contains("build_catch_up_view"),
            "CLI must call core through continuity modules, not inline builder"
        );
        let tauri_lib =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
        if tauri_lib.exists() {
            let tauri_rs = fs::read_to_string(&tauri_lib).unwrap_or_default();
            assert_eq!(
                tauri_rs.matches("#[tauri::command]").count(),
                52,
                "Tauri command count must remain 52"
            );
        }
    }
    let core_lib = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
        .expect("lib.rs");
    assert!(core_lib.contains("continuity"));
    assert!(!core_lib.contains("mod catch_up;"));
}

#[test]
fn promotion_audit_ambiguous_maps_to_needs_attention() {
    let record = sample_promotion_record(
        "ws-1",
        &["sig-a"],
        PromotionOutcome::Ambiguous,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![], vec![], vec![], vec![record], vec![]);
    let view = build_catch_up_view(&snapshot, &current_state_for(&snapshot), &window_all())
        .expect("build");
    assert_eq!(view.sections.needs_attention.len(), 1);
    assert_eq!(
        view.sections.needs_attention[0].confidence,
        ContinuityConfidence::Ambiguous
    );
}

#[test]
fn current_state_still_open_carried_into_catch_up() {
    let pending = sample_signal(
        "pend-open",
        "ws-1",
        WorkSignalKind::SessionEnd,
        "2026-07-16T10:00:00Z",
    );
    let snapshot = snapshot_with("ws-1", vec![pending], vec![], vec![], vec![], vec![]);
    let current_state = current_state_for(&snapshot);
    assert!(!current_state.sections.still_open.is_empty());
    let view = build_catch_up_view(&snapshot, &current_state, &window_all()).expect("build");
    assert!(view
        .sections
        .still_open
        .iter()
        .any(|i| i.source_id == "pend-open"));
}
