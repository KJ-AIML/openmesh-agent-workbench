//! Dev Track 0.1.3.7 Checkpoint F — evidence traceability and ambiguity boundary proofs.

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, load_continuity_input_snapshot,
    rebuild_current_state_projection, ContinuityDiagnostic, ContinuityDiagnosticKind,
    ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    CatchUpWindow, ContinuityConfidence, ContinuitySourceKind, ContinuityStateItem, EvidenceRef,
    PendingAttentionReason, ProducerRef, SourceCounts, WorkEvent, WorkSignal, WorkSignalKind,
    WORK_EVENT_PROTOCOL_VERSION,
};
use openmesh_core::events::ledger_dir;
use openmesh_core::promotion::{
    promotion_decisions_dir, PromotionDecision, PromotionDecisionRecord, PromotionKey,
    ProposedEventComposition, PROMOTED_EVENT_PROTOCOL_NOTE,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::get_project_dir;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
    evidence: Vec<EvidenceRef>,
) -> WorkSignal {
    WorkSignal {
        signal_id: id.into(),
        workspace_id: workspace_id.into(),
        producer,
        actor: openmesh_core::domain::ActorRef::Unknown,
        kind,
        summary: format!("summary for {id}"),
        timestamp: "2026-07-16T10:00:00Z".into(),
        evidence_refs: evidence,
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn sample_event(event_id: &str, workspace_id: &str, kind: &str, evidence_path: &str) -> WorkEvent {
    let json = serde_json::json!({
        "eventId": event_id,
        "workspaceId": workspace_id,
        "kind": kind,
        "summary": format!("event summary for {event_id}"),
        "timestamp": "2026-07-16T10:00:00Z",
        "evidence": [{
            "evidenceRef": { "type": "file-path", "value": evidence_path }
        }],
        "sensitivity": "private",
        "protocolVersion": WORK_EVENT_PROTOCOL_VERSION,
    });
    serde_json::from_value(json).expect("event json")
}

fn promote_record(workspace_id: &str, signal_ids: &[&str]) -> PromotionDecisionRecord {
    let ids: Vec<String> = signal_ids.iter().map(|s| (*s).to_string()).collect();
    let key = PromotionKey::from_inputs(workspace_id, &ids).unwrap();
    let proposed = ProposedEventComposition {
        kind: "work.decision".into(),
        summary: "promoted decision".into(),
        timestamp: "2026-07-16T10:00:00Z".into(),
        producer_signal_evidence_ids: ids.clone(),
        file_evidence_paths: vec!["docs/decision.md".into()],
        sensitivity: Sensitivity::Private,
        composition_note: PROMOTED_EVENT_PROTOCOL_NOTE.into(),
    };
    let decision = PromotionDecision::promote(key, ids, proposed);
    PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        None,
        "2026-07-16T10:00:00Z".into(),
    )
}

fn ambiguous_record(workspace_id: &str, signal_ids: &[&str]) -> PromotionDecisionRecord {
    let ids: Vec<String> = signal_ids.iter().map(|s| (*s).to_string()).collect();
    let key = PromotionKey::from_inputs(workspace_id, &ids).unwrap();
    let decision =
        PromotionDecision::ambiguous(key, ids, "conflicting evidence requires review".into());
    PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        None,
        "2026-07-16T10:00:00Z".into(),
    )
}

fn window_all() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-15T00:00:00Z".into(),
        until: "2026-07-16T23:59:59Z".into(),
    }
}

fn all_state_items(
    projection: &openmesh_core::domain::CurrentStateProjection,
) -> Vec<&ContinuityStateItem> {
    let s = &projection.sections;
    s.completed
        .iter()
        .chain(s.in_progress.iter())
        .chain(s.blocked.iter())
        .chain(s.decisions.iter())
        .chain(s.needs_attention.iter())
        .chain(s.still_open.iter())
        .collect()
}

fn all_catch_up_items(view: &openmesh_core::domain::CatchUpView) -> Vec<&ContinuityStateItem> {
    let s = &view.sections;
    s.completed
        .iter()
        .chain(s.changed.iter())
        .chain(s.blocked.iter())
        .chain(s.decided.iter())
        .chain(s.needs_attention.iter())
        .chain(s.still_open.iter())
        .collect()
}

fn evidence_values(refs: &[EvidenceRef]) -> Vec<String> {
    refs.iter()
        .map(|r| match r {
            EvidenceRef::FilePath(p) => p.clone(),
            EvidenceRef::ProducerSignal(id) => format!("signal:{id}"),
            EvidenceRef::GitState(_) => "git-state".into(),
            _ => "other-evidence".into(),
        })
        .collect()
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-evidence-boundary-{name}-{}-{unique}",
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
        "name": "Evidence Boundary Test",
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

fn write_processed_signal(project_path: &str, signal: &WorkSignal, name: &str) {
    let dir = get_project_dir(project_path).join("signals/processed");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{name}.json")),
        serde_json::to_string_pretty(signal).unwrap(),
    )
    .unwrap();
}

fn bucket_snapshot(project_path: &str) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = get_project_dir(project_path).join("signals").join(bucket);
        let count = if dir.exists() {
            fs::read_dir(dir).map(|e| e.count()).unwrap_or(0)
        } else {
            0
        };
        out.insert(bucket.to_string(), count);
    }
    out
}

#[test]
fn current_state_claims_preserve_evidence_refs() {
    let evidence_path = "docs/boundary-evidence.md";
    let signal = sample_signal(
        "ev-sig",
        "ws-ev",
        WorkSignalKind::Progress,
        ProducerRef::Git,
        vec![EvidenceRef::FilePath(evidence_path.into())],
    );
    let event = sample_event("ev-evt", "ws-ev", "work.progress", evidence_path);
    let snapshot = snapshot_with("ws-ev", vec![], vec![signal], vec![event], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    for item in all_state_items(&projection) {
        assert!(
            !item.evidence_refs.is_empty(),
            "section item {} must carry evidence refs",
            item.id
        );
    }
    let top = evidence_values(&projection.evidence_refs);
    assert!(top.iter().any(|v| v.contains("boundary-evidence")));
}

#[test]
fn catch_up_claims_preserve_evidence_refs() {
    let evidence_path = "docs/catchup-evidence.md";
    let signal = sample_signal(
        "cu-sig",
        "ws-cu",
        WorkSignalKind::Progress,
        ProducerRef::Reporter("cli".into()),
        vec![EvidenceRef::FilePath(evidence_path.into())],
    );
    let snapshot = snapshot_with("ws-cu", vec![], vec![signal], vec![], vec![], vec![]);
    let current = build_current_state_projection(&snapshot).expect("state");
    let view = build_catch_up_view(&snapshot, &current, &window_all()).expect("catch-up");

    for item in all_catch_up_items(&view) {
        assert!(
            !item.evidence_refs.is_empty(),
            "catch-up item {} must carry evidence refs",
            item.id
        );
    }
    assert!(evidence_values(&view.evidence_refs)
        .iter()
        .any(|v| v.contains("catchup-evidence")));
}

#[test]
fn pending_signal_does_not_become_confirmed_completed_claim() {
    let pending = sample_signal(
        "pend-milestone",
        "ws-pend",
        WorkSignalKind::Milestone,
        ProducerRef::Reporter("claude".into()),
        vec![EvidenceRef::FilePath("docs/milestone.md".into())],
    );
    let snapshot = snapshot_with("ws-pend", vec![pending], vec![], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert!(
        projection.sections.completed.is_empty(),
        "pending milestone must not appear as completed confirmed fact"
    );
    assert_eq!(projection.sections.still_open.len(), 1);
    let item = &projection.sections.still_open[0];
    assert_eq!(item.source, ContinuitySourceKind::PendingSignal);
    assert_eq!(item.unverified, Some(true));
    assert_eq!(item.confidence, ContinuityConfidence::Low);
}

#[test]
fn processed_signal_can_support_in_progress_but_not_fake_completion() {
    let progress = sample_signal(
        "proc-progress",
        "ws-proc",
        WorkSignalKind::Progress,
        ProducerRef::Git,
        vec![EvidenceRef::FilePath("docs/progress.md".into())],
    );
    let snapshot = snapshot_with("ws-proc", vec![], vec![progress], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert_eq!(projection.sections.in_progress.len(), 1);
    assert!(projection.sections.completed.is_empty());
    assert_ne!(
        projection.sections.in_progress[0].confidence,
        ContinuityConfidence::High
    );
}

#[test]
fn work_event_can_support_completed_claim() {
    let event = sample_event("done-1", "ws-evt", "work.completed", "docs/done.md");
    let snapshot = snapshot_with("ws-evt", vec![], vec![], vec![event], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert_eq!(projection.sections.completed.len(), 1);
    assert_eq!(
        projection.sections.completed[0].source,
        ContinuitySourceKind::WorkEvent
    );
    assert!(!projection.sections.completed[0].evidence_refs.is_empty());
}

#[test]
fn promotion_audit_can_support_decision_claim() {
    let record = promote_record("ws-audit", &["sig-dec"]);
    let snapshot = snapshot_with("ws-audit", vec![], vec![], vec![], vec![record], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert_eq!(projection.sections.decisions.len(), 1);
    assert_eq!(
        projection.sections.decisions[0].source,
        ContinuitySourceKind::PromotionAudit
    );
}

#[test]
fn ambiguous_promotion_audit_becomes_needs_attention_and_limitation() {
    let record = ambiguous_record("ws-amb-audit", &["sig-x"]);
    let snapshot = snapshot_with("ws-amb-audit", vec![], vec![], vec![], vec![record], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert_eq!(projection.sections.needs_attention.len(), 1);
    assert_eq!(
        projection.sections.needs_attention[0].confidence,
        ContinuityConfidence::Ambiguous
    );
    assert!(projection
        .pending_attention
        .iter()
        .any(|a| a.reason == PendingAttentionReason::AmbiguousPromotion));
}

#[test]
fn conflicting_inputs_remain_ambiguous_without_fake_certainty() {
    let mut complete = sample_signal(
        "rep-complete",
        "ws-conflict",
        WorkSignalKind::Milestone,
        ProducerRef::Reporter("claude".into()),
        vec![EvidenceRef::FilePath("docs/complete.md".into())],
    );
    let blocker = sample_event("blk-evt", "ws-conflict", "work.blocker", "docs/blocker.md");
    complete.correlation_hint = Some("track-0.1.3.7".into());
    let mut blocker_signal = sample_signal(
        "rep-blocker",
        "ws-conflict",
        WorkSignalKind::Blocker,
        ProducerRef::Reporter("claude".into()),
        vec![EvidenceRef::FilePath("docs/blocker-signal.md".into())],
    );
    blocker_signal.correlation_hint = Some("track-0.1.3.7".into());

    let snapshot = snapshot_with(
        "ws-conflict",
        vec![],
        vec![complete, blocker_signal],
        vec![blocker],
        vec![],
        vec![],
    );
    let projection = build_current_state_projection(&snapshot).expect("build");

    let ambiguous_items: Vec<_> = all_state_items(&projection)
        .into_iter()
        .filter(|i| i.confidence == ContinuityConfidence::Ambiguous)
        .collect();
    assert!(
        !ambiguous_items.is_empty(),
        "conflicting correlated inputs must remain ambiguous"
    );
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("ambiguous correlation hint")));
    assert!(
        !projection
            .sections
            .completed
            .iter()
            .any(|i| i.confidence == ContinuityConfidence::High),
        "must not fabricate high-confidence completion amid conflict"
    );
}

#[test]
fn diagnostics_surface_as_limitations_and_pending_attention() {
    let diagnostic = ContinuityDiagnostic {
        kind: ContinuityDiagnosticKind::SignalBucket,
        location: "Processed:broken.json".into(),
        message: "invalid JSON in processed bucket".into(),
    };
    let snapshot = snapshot_with("ws-diag", vec![], vec![], vec![], vec![], vec![diagnostic]);
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("invalid JSON")));
    assert!(!projection.pending_attention.is_empty());
}

#[test]
fn malformed_signal_record_is_not_silently_ignored() {
    let (_dir, project_path, _project_id) = create_test_project("malformed-signal");
    let processed = get_project_dir(&project_path).join("signals/processed");
    fs::create_dir_all(&processed).unwrap();
    fs::write(processed.join("broken.json"), "{ not valid json").unwrap();

    let snapshot = load_continuity_input_snapshot(&project_path).expect("load");
    assert_eq!(snapshot.diagnostics.len(), 1);
    let projection = build_current_state_projection(&snapshot).expect("build");
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("invalid JSON")));
}

#[test]
fn missing_input_sources_create_explicit_limitation() {
    let snapshot = snapshot_with("ws-empty-inputs", vec![], vec![], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("no continuity inputs")));
}

#[test]
fn empty_project_outputs_explicit_empty_state_not_fake_work() {
    let snapshot = snapshot_with("ws-empty", vec![], vec![], vec![], vec![], vec![]);
    let projection = build_current_state_projection(&snapshot).expect("build");
    assert!(all_state_items(&projection).is_empty());
    assert!(projection.pending_attention.is_empty());
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("no continuity inputs")));
}

#[test]
fn source_counts_explain_where_claims_came_from() {
    let pending = sample_signal(
        "src-pend",
        "ws-src",
        WorkSignalKind::ReviewRequired,
        ProducerRef::Reporter("claude".into()),
        vec![EvidenceRef::FilePath("docs/review.md".into())],
    );
    let processed = sample_signal(
        "src-proc",
        "ws-src",
        WorkSignalKind::Progress,
        ProducerRef::Git,
        vec![EvidenceRef::FilePath("docs/git.md".into())],
    );
    let event = sample_event("src-evt", "ws-src", "work.progress", "docs/event.md");
    let audit = ambiguous_record("ws-src", &["src-proc"]);
    let snapshot = snapshot_with(
        "ws-src",
        vec![pending],
        vec![processed],
        vec![event],
        vec![audit],
        vec![],
    );
    let projection = build_current_state_projection(&snapshot).expect("build");

    assert_eq!(projection.source_counts.pending_signals, 1);
    assert_eq!(projection.source_counts.processed_signals, 1);
    assert_eq!(projection.source_counts.work_events, 1);
    assert_eq!(projection.source_counts.promotion_audit_records, 1);
    assert_eq!(projection.source_counts.reporter_signals, 1);
    assert_eq!(projection.source_counts.git_signals, 1);
}

#[test]
fn evidence_boundary_tests_do_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("bucket-isolation");
    write_signal(
        &project_path,
        &sample_signal(
            "iso-pend",
            &project_id,
            WorkSignalKind::Progress,
            ProducerRef::Reporter("cli".into()),
            vec![EvidenceRef::FilePath("docs/iso.md".into())],
        ),
    )
    .expect("write pending");
    write_processed_signal(
        &project_path,
        &sample_signal(
            "iso-proc",
            &project_id,
            WorkSignalKind::Decision,
            ProducerRef::Git,
            vec![EvidenceRef::FilePath("docs/iso-git.md".into())],
        ),
        "iso-proc",
    );
    let before = bucket_snapshot(&project_path);

    let snapshot = load_continuity_input_snapshot(&project_path).expect("load");
    let projection = build_current_state_projection(&snapshot).expect("build");
    let view = build_catch_up_view(&snapshot, &projection, &window_all()).expect("catch-up");
    rebuild_current_state_projection(&project_path).expect("rebuild");

    assert_eq!(before, bucket_snapshot(&project_path));
    assert!(!view.summary.is_empty());
}

#[test]
fn evidence_boundary_tests_do_not_create_events_or_promotion_audit() {
    let (_dir, project_path, project_id) = create_test_project("no-write-side-effects");
    write_processed_signal(
        &project_path,
        &sample_signal(
            "nw-sig",
            &project_id,
            WorkSignalKind::Progress,
            ProducerRef::Git,
            vec![EvidenceRef::FilePath("docs/nw.md".into())],
        ),
        "nw-sig",
    );

    let snapshot = load_continuity_input_snapshot(&project_path).expect("load");
    let projection = build_current_state_projection(&snapshot).expect("build");
    let _view = build_catch_up_view(&snapshot, &projection, &window_all()).expect("catch-up");
    rebuild_current_state_projection(&project_path).expect("rebuild");

    assert!(!ledger_dir(&project_path).exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn checkpoint_f_does_not_touch_tauri_desktop_or_0_1_3_8() {
    let root = workspace_root();
    let forbidden = [
        "0.1.3.8",
        "append_correction",
        "dogfood_gate",
        "evidence_correction",
        "ContinuityIntelligence",
        "AXGA",
        "#[tauri::command]",
    ];
    let continuity_src = root.join("crates/openmesh-core/src/continuity");
    let cli_src = root.join("crates/openmesh-cli/src");
    for dir in [continuity_src, cli_src] {
        if !dir.exists() {
            continue;
        }
        let mut files = Vec::new();
        collect_rs_files(&dir, &mut files);
        for path in files {
            let content = fs::read_to_string(&path).expect("read source");
            for term in forbidden {
                assert!(
                    !content.contains(term),
                    "Checkpoint F scope must not reference `{term}`: {}",
                    path.display()
                );
            }
        }
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    if tauri_lib.exists() {
        let content = fs::read_to_string(&tauri_lib).expect("tauri lib");
        assert_eq!(
            content.matches("#[tauri::command]").count(),
            52,
            "Tauri command count must remain 52"
        );
        for term in ["run_state", "run_catch_up", "build_catch_up_view"] {
            assert!(
                !content.contains(term),
                "Desktop must not expose continuity CLI APIs (`{term}`)"
            );
        }
    }
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
