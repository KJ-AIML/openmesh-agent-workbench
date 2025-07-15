//! Dev Track 0.1.3.5 Checkpoint E1 — WorkEvent protocol 1.1 compatibility tests.

use openmesh_core::domain::{
    validate_event_semantics, ActorRef, EvidenceAttachment, EvidenceRef, ProducerRef, WorkEvent,
    WorkSignalKind, WORK_EVENT_PROTOCOL_VERSION, WORK_EVENT_PROTOCOL_VERSION_PROMOTED,
};
use openmesh_core::events::{
    append_event, classify_ledger_record, ledger_dir, list_events, validate_ledger,
    LedgerClassification,
};
use openmesh_core::promotion::{correlate_and_evaluate, SignalRef};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn v1_0_event() -> WorkEvent {
    let mut event = WorkEvent::new(
        "evt-v1-0",
        "ws-proto",
        "work.completed",
        "legacy event without actor",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/a.md".into()),
            observed_at: None,
        }],
        "2026-07-15T09:00:00Z",
    );
    event.actor = None;
    event
}

fn v1_1_event() -> WorkEvent {
    let mut event = WorkEvent::new(
        "evt-v1-1",
        "ws-proto",
        "work.decision",
        "promoted event with actor",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::ProducerSignal("sig-1".into()),
            observed_at: None,
        }],
        "2026-07-15T09:01:00Z",
    );
    event.protocol_version = WORK_EVENT_PROTOCOL_VERSION_PROMOTED.to_string();
    event.actor = Some(ActorRef::Person("ter".into()));
    event
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-proto-e1-{name}-{}-{unique}",
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
    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

#[test]
fn work_event_v1_0_without_actor_remains_valid() {
    let event = v1_0_event();
    validate_event_semantics(&event).expect("v1.0 without actor is valid");
}

#[test]
fn work_event_v1_1_with_actor_is_valid() {
    let event = v1_1_event();
    validate_event_semantics(&event).expect("v1.1 with actor is valid");
}

#[test]
fn work_event_v1_1_missing_actor_is_rejected() {
    let mut event = v1_1_event();
    event.actor = None;
    assert!(validate_event_semantics(&event).is_err());
}

#[test]
fn unknown_protocol_version_still_rejected_or_quarantined() {
    let mut event = v1_0_event();
    event.protocol_version = "99.0".into();
    assert!(validate_event_semantics(&event).is_err());

    let (_dir, project_path, project_id) = create_test_project("unknown-version");
    let mut legacy = v1_0_event();
    legacy.workspace_id = project_id.clone();
    legacy.event_id = "evt-known".into();
    append_event(&project_path, &legacy).unwrap();

    let raw = format!(
        r#"{{
            "eventId": "evt-unknown",
            "workspaceId": "{project_id}",
            "kind": "work.completed",
            "summary": "unknown protocol",
            "timestamp": "2026-07-15T09:02:00Z",
            "evidence": [{{ "evidenceRef": {{ "type": "file-path", "value": "docs/a.md" }} }}],
            "protocolVersion": "99.0",
            "sensitivity": "private"
        }}"#
    );
    fs::create_dir_all(ledger_dir(&project_path)).unwrap();
    fs::write(ledger_dir(&project_path).join("evt-unknown.json"), raw).unwrap();

    let report = validate_ledger(&project_path).unwrap();
    assert_eq!(report.valid.len(), 1);
    assert_eq!(report.quarantined.len(), 1);
    assert!(matches!(
        report.quarantined[0].classification,
        LedgerClassification::UnsupportedVersion(_)
    ));
}

#[test]
fn ledger_accepts_v1_0_and_v1_1_records() {
    let (_dir, project_path, project_id) = create_test_project("dual-version");
    let mut v10 = v1_0_event();
    v10.workspace_id = project_id.clone();
    v10.event_id = "evt-legacy".into();
    append_event(&project_path, &v10).unwrap();

    let mut v11 = v1_1_event();
    v11.workspace_id = project_id;
    v11.event_id = "evt-promoted".into();
    append_event(&project_path, &v11).unwrap();

    let events = list_events(&project_path).unwrap();
    assert_eq!(events.len(), 2);
}

#[test]
fn ledger_quarantines_unknown_protocol_after_dual_version_update() {
    let (_dir, project_path, project_id) = create_test_project("quarantine-unknown");
    let mut v11 = v1_1_event();
    v11.workspace_id = project_id.clone();
    v11.event_id = "evt-v11".into();
    append_event(&project_path, &v11).unwrap();

    fs::write(
        ledger_dir(&project_path).join("evt-bad.json"),
        r#"{
            "eventId": "evt-bad",
            "workspaceId": "ws",
            "kind": "work.completed",
            "summary": "bad",
            "timestamp": "2026-07-15T09:03:00Z",
            "evidence": [{ "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }],
            "protocolVersion": "2.0",
            "sensitivity": "private"
        }"#,
    )
    .unwrap();

    let project = openmesh_core::storage::read_project::<openmesh_core::storage::Project>(
        &project_path,
        "project.json",
    )
    .unwrap();
    let err = classify_ledger_record(&ledger_dir(&project_path).join("evt-bad.json"), &project)
        .unwrap_err();
    assert!(matches!(err, LedgerClassification::UnsupportedVersion(_)));
}

#[test]
fn old_valid_fixture_remains_v1_0_compatible() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let json = fs::read_to_string(format!("{manifest_dir}/tests/fixtures/events/valid.json"))
        .expect("read fixture");
    let event: WorkEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
    assert!(event.actor.is_none());
    validate_event_semantics(&event).expect("fixture remains valid");
}

#[test]
fn promoted_valid_fixture_uses_v1_1_with_actor() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let json = fs::read_to_string(format!(
        "{manifest_dir}/tests/fixtures/events/valid-v1.1.json"
    ))
    .expect("read fixture");
    let event: WorkEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION_PROMOTED);
    assert!(matches!(event.actor, Some(ActorRef::Person(_))));
    validate_event_semantics(&event).expect("v1.1 fixture is valid");
}

#[test]
fn no_migration_or_rewrite_of_v1_0_records() {
    let (_dir, project_path, project_id) = create_test_project("no-migration");
    let mut v10 = v1_0_event();
    v10.workspace_id = project_id.clone();
    v10.event_id = "evt-keep".into();
    append_event(&project_path, &v10).unwrap();

    let path = ledger_dir(&project_path).join("evt-keep.json");
    let bytes_before = fs::read(&path).unwrap();

    let mut v11 = v1_1_event();
    v11.workspace_id = project_id;
    v11.event_id = "evt-new".into();
    append_event(&project_path, &v11).unwrap();

    let bytes_after = fs::read(&path).unwrap();
    assert_eq!(bytes_before, bytes_after);
    assert_eq!(list_events(&project_path).unwrap().len(), 2);
}

#[test]
fn checkpoint_e2_apply_promotion_uses_append_event() {
    let signals = vec![SignalRef {
        signal_id: "sig-only".into(),
        kind: WorkSignalKind::Decision,
        summary: "correlation path remains pure before apply".into(),
        producer: ProducerRef::Reporter("codex".into()),
        actor: ActorRef::Unknown,
        timestamp: "2026-07-15T09:04:00Z".into(),
        correlation_hint: None,
        evidence_refs: vec![EvidenceRef::ProducerSignal("sig-only".into())],
    }];
    let _ = correlate_and_evaluate("ws-promo", &signals).unwrap();
    let promotion_rs = include_str!("../src/promotion.rs");
    assert!(promotion_rs.contains("fn apply_promotion_decision"));
    assert!(promotion_rs.contains("append_event"));
}
