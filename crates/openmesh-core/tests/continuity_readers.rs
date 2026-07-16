//! Dev Track 0.1.3.7 Checkpoint B — continuity reader tests (temp projects only).

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::{
    corrections_for_event, list_duplicate_signals, list_pending_signals, list_processed_signals,
    list_quarantine_signals, load_continuity_input_snapshot, load_promotion_audit_records,
    load_work_events, projections_dir, ContinuityDiagnosticKind, SignalBucket,
};
use openmesh_core::domain::{
    ActorRef, EvidenceRef, GitState, ProducerRef, WorkEvent, WorkSignal, WorkSignalKind,
    WORK_EVENT_PROTOCOL_VERSION, WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
};
use openmesh_core::events::ledger_dir;
use openmesh_core::promotion::{promotion_decisions_dir, write_decision_record, PromotionOutcome};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::get_project_dir;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-continuity-readers-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();

    let project_id = format!("proj-{name}-{unique}");
    let now = "2026-07-16T10:00:00Z";
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

fn sample_signal(
    id: &str,
    workspace_id: &str,
    producer: ProducerRef,
    protocol_version: &str,
    evidence_refs: Vec<EvidenceRef>,
) -> WorkSignal {
    WorkSignal {
        signal_id: id.to_string(),
        workspace_id: workspace_id.to_string(),
        producer,
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: format!("test signal {id}"),
        timestamp: "2026-07-16T10:00:00Z".to_string(),
        evidence_refs,
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: protocol_version.to_string(),
    }
}

fn sample_git_state() -> GitState {
    GitState {
        repo_id: "fnv1a-2ad3a48b04b15c64b82e2bc".into(),
        branch: "feat/openmesh-0.1.3".into(),
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
    }
}

fn signals_bucket_dir(project_path: &str, bucket: &str) -> PathBuf {
    get_project_dir(project_path).join("signals").join(bucket)
}

fn write_signal_to_bucket(
    project_path: &str,
    bucket: SignalBucket,
    signal: &WorkSignal,
    name: &str,
) {
    let bucket_name = match bucket {
        SignalBucket::Pending => "pending",
        SignalBucket::Processed => "processed",
        SignalBucket::Quarantine => "quarantine",
        SignalBucket::Duplicate => "duplicate",
    };
    let dir = signals_bucket_dir(project_path, bucket_name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join(format!("{name}.json")),
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

fn promotion_audit_snapshot(project_path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let dir = promotion_decisions_dir(project_path);
    if !dir.exists() {
        return out;
    }
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            out.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            );
        }
    }
    out
}

fn ledger_snapshot(project_path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let dir = ledger_dir(project_path);
    if !dir.exists() {
        return out;
    }
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            out.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            );
        }
    }
    out
}

fn sample_promotion_record(
    workspace_id: &str,
    signal_ids: &[&str],
    outcome: PromotionOutcome,
) -> openmesh_core::promotion::PromotionDecisionRecord {
    let ids: Vec<String> = signal_ids.iter().map(|s| (*s).to_string()).collect();
    let key = openmesh_core::promotion::PromotionKey::from_inputs(workspace_id, &ids).unwrap();
    let decision = match outcome {
        PromotionOutcome::Suppress => openmesh_core::promotion::PromotionDecision::suppress(
            key.clone(),
            ids.clone(),
            openmesh_core::promotion::PromotionReasonCode::ActivitySpam,
        ),
        _ => openmesh_core::promotion::PromotionDecision::defer(
            key.clone(),
            ids.clone(),
            openmesh_core::promotion::PromotionReasonCode::MissingEvidence,
        ),
    };
    openmesh_core::promotion::PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        None,
        "2026-07-16T10:00:00Z".to_string(),
    )
}

fn sample_event(event_id: &str, workspace_id: &str, summary: &str) -> WorkEvent {
    let json = serde_json::json!({
        "eventId": event_id,
        "workspaceId": workspace_id,
        "kind": "work.completed",
        "summary": summary,
        "timestamp": "2026-07-16T10:00:00Z",
        "evidence": [{
            "evidenceRef": { "type": "file-path", "value": "docs/overview.md" }
        }],
        "sensitivity": "private",
        "protocolVersion": WORK_EVENT_PROTOCOL_VERSION,
    });
    serde_json::from_value(json).expect("sample event json")
}

fn write_event_fixture(project_path: &str, event: &WorkEvent) {
    let ledger = ledger_dir(project_path);
    fs::create_dir_all(&ledger).unwrap();
    fs::write(
        ledger.join(format!("{}.json", event.event_id)),
        serde_json::to_string_pretty(event).unwrap(),
    )
    .unwrap();
}

#[test]
fn reads_pending_and_processed_signals_without_mutation() {
    let (_dir, project_path, project_id) = create_test_project("pending-processed");
    let pending = sample_signal(
        "pending-1",
        &project_id,
        ProducerRef::Reporter("claude".into()),
        "1.0",
        vec![EvidenceRef::FilePath("docs/a.md".into())],
    );
    let processed = sample_signal(
        "processed-1",
        &project_id,
        ProducerRef::Git,
        "1.0",
        vec![EvidenceRef::FilePath("crates/lib.rs".into())],
    );
    write_signal(&project_path, &pending).expect("pending write");
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Processed,
        &processed,
        "processed-1",
    );

    let before = bucket_snapshot(&project_path);
    let pending_loaded = list_pending_signals(&project_path).expect("pending load");
    let processed_loaded = list_processed_signals(&project_path).expect("processed load");
    let after = bucket_snapshot(&project_path);

    assert_eq!(before, after);
    assert_eq!(pending_loaded.signals.len(), 1);
    assert_eq!(processed_loaded.signals.len(), 1);
    assert_eq!(
        pending_loaded.signals[0].producer,
        ProducerRef::Reporter("claude".into())
    );
    assert_eq!(processed_loaded.signals[0].producer, ProducerRef::Git);
    assert_eq!(
        pending_loaded.signals[0].evidence_refs,
        vec![EvidenceRef::FilePath("docs/a.md".into())]
    );
    assert_eq!(pending_loaded.signals[0].timestamp, "2026-07-16T10:00:00Z");
}

#[test]
fn reads_quarantine_and_duplicate_buckets_without_mutation() {
    let (_dir, project_path, project_id) = create_test_project("quarantine-duplicate");
    let quarantine = sample_signal("q-1", &project_id, ProducerRef::Heli, "1.0", vec![]);
    let duplicate = sample_signal(
        "d-1",
        &project_id,
        ProducerRef::Reporter("codex".into()),
        "1.0",
        vec![],
    );
    write_signal_to_bucket(&project_path, SignalBucket::Quarantine, &quarantine, "q-1");
    write_signal_to_bucket(&project_path, SignalBucket::Duplicate, &duplicate, "d-1");

    let before = bucket_snapshot(&project_path);
    let quarantine_loaded = list_quarantine_signals(&project_path).expect("quarantine load");
    let duplicate_loaded = list_duplicate_signals(&project_path).expect("duplicate load");
    let after = bucket_snapshot(&project_path);

    assert_eq!(before, after);
    assert_eq!(quarantine_loaded.signals.len(), 1);
    assert_eq!(duplicate_loaded.signals.len(), 1);
    assert_eq!(quarantine_loaded.signals[0].producer, ProducerRef::Heli);
}

#[test]
fn signal_reader_orders_records_deterministically() {
    let (_dir, project_path, project_id) = create_test_project("signal-order");
    for name in ["zzz-last", "aaa-first", "mmm-middle"] {
        let signal = sample_signal(
            name,
            &project_id,
            ProducerRef::Reporter("claude".into()),
            "1.0",
            vec![],
        );
        write_signal_to_bucket(&project_path, SignalBucket::Processed, &signal, name);
    }

    let first = list_processed_signals(&project_path).expect("first load");
    let second = list_processed_signals(&project_path).expect("second load");
    let ids: Vec<_> = first.signals.iter().map(|s| s.signal_id.as_str()).collect();
    assert_eq!(ids, vec!["aaa-first", "mmm-middle", "zzz-last"]);
    assert_eq!(first.signals.len(), second.signals.len());
    let first_ids: Vec<_> = first.signals.iter().map(|s| s.signal_id.as_str()).collect();
    let second_ids: Vec<_> = second
        .signals
        .iter()
        .map(|s| s.signal_id.as_str())
        .collect();
    assert_eq!(first_ids, second_ids);
}

#[test]
fn signal_reader_preserves_protocol_1_0_and_1_1() {
    let (_dir, project_path, project_id) = create_test_project("protocols");
    let v10 = sample_signal(
        "sig-v10",
        &project_id,
        ProducerRef::Reporter("claude".into()),
        "1.0",
        vec![EvidenceRef::FilePath("docs/overview.md".into())],
    );
    let v11 = sample_signal(
        "sig-v11",
        &project_id,
        ProducerRef::Git,
        WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
        vec![EvidenceRef::GitState(sample_git_state())],
    );
    write_signal_to_bucket(&project_path, SignalBucket::Processed, &v10, "sig-v10");
    write_signal_to_bucket(&project_path, SignalBucket::Processed, &v11, "sig-v11");

    let loaded = list_processed_signals(&project_path).expect("load");
    assert_eq!(loaded.signals.len(), 2);
    assert!(loaded.signals.iter().any(|s| s.protocol_version == "1.0"));
    assert!(loaded.signals.iter().any(|s| s.protocol_version == "1.1"));
    assert!(loaded
        .signals
        .iter()
        .any(|s| matches!(&s.evidence_refs[0], EvidenceRef::GitState(_))));
}

#[test]
fn signal_reader_preserves_git_heli_reporter_producer_counts() {
    let (_dir, project_path, project_id) = create_test_project("producer-counts");
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Processed,
        &sample_signal(
            "r1",
            &project_id,
            ProducerRef::Reporter("a".into()),
            "1.0",
            vec![],
        ),
        "r1",
    );
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Processed,
        &sample_signal(
            "r2",
            &project_id,
            ProducerRef::Reporter("b".into()),
            "1.0",
            vec![],
        ),
        "r2",
    );
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Pending,
        &sample_signal("g1", &project_id, ProducerRef::Git, "1.0", vec![]),
        "g1",
    );
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Pending,
        &sample_signal("h1", &project_id, ProducerRef::Heli, "1.0", vec![]),
        "h1",
    );

    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    assert_eq!(snapshot.source_counts.reporter_signals, 2);
    assert_eq!(snapshot.source_counts.git_signals, 1);
    assert_eq!(snapshot.source_counts.heli_signals, 1);
    assert_eq!(snapshot.source_counts.processed_signals, 2);
    assert_eq!(snapshot.source_counts.pending_signals, 2);
}

#[test]
fn signal_reader_surfaces_malformed_records_as_diagnostics() {
    let (_dir, project_path, project_id) = create_test_project("malformed-signal");
    let valid = sample_signal(
        "valid-1",
        &project_id,
        ProducerRef::Reporter("claude".into()),
        "1.0",
        vec![],
    );
    write_signal_to_bucket(&project_path, SignalBucket::Processed, &valid, "valid-1");
    fs::create_dir_all(signals_bucket_dir(&project_path, "processed")).unwrap();
    fs::write(
        signals_bucket_dir(&project_path, "processed").join("broken.json"),
        "{not-json",
    )
    .unwrap();

    let loaded = list_processed_signals(&project_path).expect("load should not panic");
    assert_eq!(loaded.signals.len(), 1);
    assert_eq!(loaded.diagnostics.len(), 1);
    assert_eq!(
        loaded.diagnostics[0].kind,
        ContinuityDiagnosticKind::SignalBucket
    );
    assert!(loaded.diagnostics[0].message.contains("invalid JSON"));
}

#[test]
fn event_reader_loads_existing_work_events_read_only() {
    let (_dir, project_path, project_id) = create_test_project("events-read");
    let event = sample_event("evt-001", &project_id, "progress recorded");
    write_event_fixture(&project_path, &event);

    let before = ledger_snapshot(&project_path);
    let loaded = load_work_events(&project_path).expect("load events");
    let after = ledger_snapshot(&project_path);

    assert_eq!(before, after);
    assert_eq!(loaded.events.len(), 1);
    assert_eq!(loaded.events[0].event_id, "evt-001");
    assert_eq!(loaded.events[0].summary, "progress recorded");
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn event_reader_preserves_corrections_visibility() {
    let (_dir, project_path, project_id) = create_test_project("event-corrections");
    write_event_fixture(
        &project_path,
        &sample_event("evt-original", &project_id, "original summary"),
    );
    let correction_json = serde_json::json!({
        "eventId": "evt-corr",
        "workspaceId": project_id,
        "kind": "work.completed",
        "summary": "corrected summary",
        "timestamp": "2026-07-16T11:00:00Z",
        "evidence": [{
            "evidenceRef": { "type": "file-path", "value": "docs/overview.md" }
        }],
        "correctsEventId": "evt-original",
        "sensitivity": "private",
        "protocolVersion": WORK_EVENT_PROTOCOL_VERSION,
    });
    let correction: WorkEvent = serde_json::from_value(correction_json).unwrap();
    write_event_fixture(&project_path, &correction);

    let loaded = load_work_events(&project_path).expect("load");
    assert_eq!(loaded.events.len(), 2);
    let corrections = corrections_for_event(&loaded.events, "evt-original");
    assert_eq!(corrections.len(), 1);
    assert_eq!(corrections[0].event_id, "evt-corr");
    assert_eq!(
        corrections[0].corrects_event_id.as_deref(),
        Some("evt-original")
    );

    // Both original and correction remain visible in the read-only load.
    assert!(loaded
        .events
        .iter()
        .any(|e| e.event_id == "evt-original" && e.summary == "original summary"));
}

#[test]
fn promotion_audit_reader_missing_dir_is_empty() {
    let (_dir, project_path, _project_id) = create_test_project("audit-missing");
    let loaded = load_promotion_audit_records(&project_path).expect("load");
    assert!(loaded.records.is_empty());
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn promotion_audit_reader_loads_decisions_read_only() {
    let (_dir, project_path, project_id) = create_test_project("audit-read");
    let record = sample_promotion_record(&project_id, &["sig-a"], PromotionOutcome::Suppress);
    write_decision_record(&project_path, &record).expect("write audit");

    let before = promotion_audit_snapshot(&project_path);
    let loaded = load_promotion_audit_records(&project_path).expect("load");
    let after = promotion_audit_snapshot(&project_path);

    assert_eq!(before, after);
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.records[0].promotion_key, record.promotion_key);
    assert!(loaded.diagnostics.is_empty());
}

#[test]
fn promotion_audit_reader_surfaces_malformed_records_as_diagnostics() {
    let (_dir, project_path, project_id) = create_test_project("audit-malformed");
    let record = sample_promotion_record(&project_id, &["sig-good"], PromotionOutcome::Defer);
    write_decision_record(&project_path, &record).expect("write good");

    fs::write(
        promotion_decisions_dir(&project_path).join("bad-record.json"),
        r#"{"auditProtocolVersion":"nope"}"#,
    )
    .unwrap();

    let loaded = load_promotion_audit_records(&project_path).expect("load");
    assert_eq!(loaded.records.len(), 1);
    assert_eq!(loaded.diagnostics.len(), 1);
    assert_eq!(
        loaded.diagnostics[0].kind,
        ContinuityDiagnosticKind::PromotionAudit
    );
}

#[test]
fn unified_snapshot_contains_source_counts() {
    let (_dir, project_path, project_id) = create_test_project("snapshot-counts");
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Processed,
        &sample_signal("p1", &project_id, ProducerRef::Git, "1.0", vec![]),
        "p1",
    );
    write_event_fixture(
        &project_path,
        &sample_event("evt-1", &project_id, "one event"),
    );
    let record = sample_promotion_record(&project_id, &["p1"], PromotionOutcome::Defer);
    write_decision_record(&project_path, &record).unwrap();

    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    assert_eq!(snapshot.workspace_id, project_id);
    assert_eq!(snapshot.source_counts.work_events, 1);
    assert_eq!(snapshot.source_counts.processed_signals, 1);
    assert_eq!(snapshot.source_counts.git_signals, 1);
    assert_eq!(snapshot.source_counts.promotion_audit_records, 1);
}

#[test]
fn unified_snapshot_has_utc_loaded_at() {
    let (_dir, project_path, _project_id) = create_test_project("snapshot-loaded-at");
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    chrono::DateTime::parse_from_rfc3339(&snapshot.loaded_at).expect("loaded_at is RFC3339");
    assert!(
        snapshot.loaded_at.ends_with('Z') || snapshot.loaded_at.contains("+00:00"),
        "loaded_at must be UTC"
    );
}

#[test]
fn checkpoint_b_does_not_create_projection_files() {
    let (_dir, project_path, _project_id) = create_test_project("no-projections");
    let projections = projections_dir(&project_path);
    assert!(!projections.exists());
    let _snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    assert!(!projections.exists());
    assert!(!projections.join("current-state.json").exists());
}

#[test]
fn checkpoint_b_does_not_write_signals_events_or_audit() {
    let (_dir, project_path, project_id) = create_test_project("no-writes");
    write_signal_to_bucket(
        &project_path,
        SignalBucket::Processed,
        &sample_signal(
            "sig-1",
            &project_id,
            ProducerRef::Reporter("claude".into()),
            "1.0",
            vec![],
        ),
        "sig-1",
    );
    write_event_fixture(&project_path, &sample_event("evt-1", &project_id, "event"));
    write_decision_record(
        &project_path,
        &sample_promotion_record(&project_id, &["sig-1"], PromotionOutcome::Defer),
    )
    .unwrap();

    let signals_before = bucket_snapshot(&project_path);
    let events_before = ledger_snapshot(&project_path);
    let audit_before = promotion_audit_snapshot(&project_path);

    let _ = load_continuity_input_snapshot(&project_path).expect("snapshot");
    let _ = list_pending_signals(&project_path).expect("pending");
    let _ = load_work_events(&project_path).expect("events");
    let _ = load_promotion_audit_records(&project_path).expect("audit");

    assert_eq!(signals_before, bucket_snapshot(&project_path));
    assert_eq!(events_before, ledger_snapshot(&project_path));
    assert_eq!(audit_before, promotion_audit_snapshot(&project_path));
}
