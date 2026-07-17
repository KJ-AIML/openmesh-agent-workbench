//! Dev Track 0.1.3.8 Checkpoint B — correction ingestion tests.

use openmesh_core::domain::{
    ActorRef, EvidenceAttachment, EvidenceRef, WorkEvent, WORK_EVENT_PROTOCOL_VERSION_PROMOTED,
};
use openmesh_core::events::{
    append_event, append_event_correction, effective_kind, effective_summary, get_event,
    ledger_dir, list_events, read_event_file, EventCorrectionRequest, EventError,
};
use openmesh_core::promotion::promotion_decisions_dir;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-correction-ingest-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();

    let project_id = format!("proj-{name}-{unique}");
    let now = "2026-07-17T03:00:00.000Z";
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

fn sample_event(event_id: &str, workspace_id: &str) -> WorkEvent {
    WorkEvent::new(
        event_id,
        workspace_id,
        "work.completed",
        format!("summary for {event_id}"),
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    )
}

fn correction_event(
    event_id: &str,
    workspace_id: &str,
    target_id: &str,
    kind: &str,
    summary: &str,
    timestamp: &str,
) -> WorkEvent {
    let mut event = sample_event(event_id, workspace_id);
    event.kind = kind.into();
    event.summary = summary.into();
    event.timestamp = timestamp.into();
    event.corrects_event_id = Some(target_id.to_string());
    event.protocol_version = WORK_EVENT_PROTOCOL_VERSION_PROMOTED.to_string();
    event.actor = Some(ActorRef::Person("test-operator".into()));
    event
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

fn promotion_audit_file_count(project_path: &str) -> usize {
    let dir = promotion_decisions_dir(project_path);
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir).map(|rd| rd.count()).unwrap_or(0)
}

fn correction_request(kind: &str, summary: &str) -> EventCorrectionRequest {
    EventCorrectionRequest {
        corrected_kind: kind.into(),
        corrected_summary: summary.into(),
        actor_label: Some("cli-operator".into()),
        timestamp: Some("2026-07-17T02:00:00Z".into()),
    }
}

#[test]
fn append_correction_creates_new_event_with_corrects_event_id() {
    let (_dir, project_path, project_id) = create_test_project("creates");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    let result = append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    assert_eq!(
        result.correction_event.corrects_event_id.as_deref(),
        Some("evt-target")
    );
    assert!(get_event(&project_path, &result.correction_event.event_id)
        .unwrap()
        .is_some());
}

#[test]
fn append_correction_does_not_rewrite_original_event() {
    let (_dir, project_path, project_id) = create_test_project("no-rewrite");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    let original = get_event(&project_path, "evt-target").unwrap().unwrap();
    assert_eq!(original.kind, "work.completed");
    assert_eq!(original.summary, "summary for evt-target");
}

#[test]
fn append_correction_rejects_unknown_target() {
    let (_dir, project_path, _project_id) = create_test_project("unknown-target");
    let err = append_event_correction(
        &project_path,
        "evt-missing",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap_err();
    assert!(matches!(
        err,
        EventError::CorrectionTargetNotFound(ref id) if id == "evt-missing"
    ));
}

#[test]
fn append_correction_rejects_empty_summary() {
    let (_dir, project_path, project_id) = create_test_project("empty-summary");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    let err = append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "   "),
    )
    .unwrap_err();
    assert!(matches!(err, EventError::InvalidSemantics(_)));
}

#[test]
fn append_correction_rejects_invalid_cycle() {
    let (_dir, project_path, project_id) = create_test_project("cycle");
    append_event(&project_path, &sample_event("node", &project_id)).unwrap();
    append_event(
        &project_path,
        &correction_event(
            "corr-a",
            &project_id,
            "node",
            "correction",
            "A corrects node",
            "2026-07-17T01:10:00Z",
        ),
    )
    .unwrap();
    append_event(
        &project_path,
        &correction_event(
            "corr-b",
            &project_id,
            "corr-a",
            "correction",
            "B corrects A",
            "2026-07-17T01:11:00Z",
        ),
    )
    .unwrap();

    let corr_a_path = ledger_dir(&project_path).join("corr-a.json");
    let mut corr_a = read_event_file(&corr_a_path).unwrap();
    corr_a.corrects_event_id = Some("corr-b".into());
    fs::write(
        &corr_a_path,
        serde_json::to_string_pretty(&corr_a).expect("serialize corr-a"),
    )
    .unwrap();

    let err = append_event_correction(
        &project_path,
        "corr-a",
        &correction_request("correction", "Would close cycle"),
    )
    .unwrap_err();
    assert!(matches!(err, EventError::CorrectionCycle(_)));
}

#[test]
fn appended_correction_changes_effective_summary() {
    let (_dir, project_path, project_id) = create_test_project("effective-summary");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    assert_eq!(
        effective_summary(&project_path, "evt-target").unwrap(),
        Some("Corrected summary".into())
    );
}

#[test]
fn appended_correction_changes_effective_kind() {
    let (_dir, project_path, project_id) = create_test_project("effective-kind");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    assert_eq!(
        effective_kind(&project_path, "evt-target").unwrap(),
        Some("work.blocked".into())
    );
}

#[test]
fn multiple_appended_corrections_latest_wins() {
    let (_dir, project_path, project_id) = create_test_project("latest-wins");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();

    append_event_correction(
        &project_path,
        "evt-target",
        &EventCorrectionRequest {
            corrected_kind: "work.blocked".into(),
            corrected_summary: "Earlier correction".into(),
            actor_label: Some("cli-operator".into()),
            timestamp: Some("2026-07-17T02:00:00Z".into()),
        },
    )
    .unwrap();
    append_event_correction(
        &project_path,
        "evt-target",
        &EventCorrectionRequest {
            corrected_kind: "work.blocked".into(),
            corrected_summary: "Latest correction".into(),
            actor_label: Some("cli-operator".into()),
            timestamp: Some("2026-07-17T03:00:00Z".into()),
        },
    )
    .unwrap();

    assert_eq!(
        effective_summary(&project_path, "evt-target").unwrap(),
        Some("Latest correction".into())
    );
    assert_eq!(list_events(&project_path).unwrap().len(), 3);
}

#[test]
fn append_correction_does_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("signal-buckets");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();
    let before = signals_file_count(&project_path);

    append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    assert_eq!(signals_file_count(&project_path), before);
}

#[test]
fn append_correction_does_not_mutate_promotion_audit() {
    let (_dir, project_path, project_id) = create_test_project("promotion-audit");
    append_event(&project_path, &sample_event("evt-target", &project_id)).unwrap();
    let before = promotion_audit_file_count(&project_path);

    append_event_correction(
        &project_path,
        "evt-target",
        &correction_request("work.blocked", "Corrected summary"),
    )
    .unwrap();

    assert_eq!(promotion_audit_file_count(&project_path), before);
}
