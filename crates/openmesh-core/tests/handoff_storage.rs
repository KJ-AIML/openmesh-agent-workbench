//! Dev Track 0.1.8 Checkpoint D — handoff storage tests.

use openmesh_core::continuity::{build_current_state_projection, ContinuityInputSnapshot};
use openmesh_core::domain::{CatchUpWindow, SourceCounts};
use openmesh_core::events::{get_event, list_events};
use openmesh_core::handoff::{
    approve_handoff_note, build_handoff_note, build_handoff_recipient, handoff_note_path,
    link_handoff_work_event, list_handoff_ids, read_handoff_note, write_handoff_note,
    BuildHandoffRequest, HandoffStatus, HandoffStorageError, WORK_EVENT_HANDOFF_KIND,
};
use openmesh_core::storage::{get_project_dir, init_project};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const NOW: &str = "2026-07-31T15:00:00Z";

fn temp_project(label: &str) -> (PathBuf, String, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-handoff-storage-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    let project_path = dir.to_string_lossy().to_string();
    let project_id = fs::read_to_string(dir.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("project id");
    (dir, project_path, project_id)
}

fn empty_snapshot(workspace_id: &str) -> ContinuityInputSnapshot {
    ContinuityInputSnapshot {
        workspace_id: workspace_id.into(),
        loaded_at: "2026-07-31T14:00:00Z".into(),
        pending_signals: vec![],
        processed_signals: vec![],
        quarantine_signals: vec![],
        duplicate_signals: vec![],
        work_events: vec![],
        promotion_audit_records: vec![],
        diagnostics: vec![],
        source_counts: SourceCounts {
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
        },
    }
}

fn sample_note(workspace_id: &str) -> openmesh_core::handoff::HandoffNote {
    let snapshot = empty_snapshot(workspace_id);
    let current_state = build_current_state_projection(&snapshot).expect("projection");
    let request = BuildHandoffRequest {
        workspace_id: workspace_id.into(),
        recipient: build_handoff_recipient("Yo", Some("teammate")).expect("recipient"),
        window: CatchUpWindow {
            since: "2026-07-24T00:00:00Z".into(),
            until: "2026-07-31T23:59:59Z".into(),
        },
        now_rfc3339: NOW.into(),
    };
    build_handoff_note(&snapshot, &current_state, &request).expect("build")
}

#[test]
fn write_and_read_roundtrip() {
    let (_dir, project_path, project_id) = temp_project("roundtrip");
    let note = sample_note(&project_id);
    write_handoff_note(&project_path, &note).expect("write");
    let loaded = read_handoff_note(&project_path, &note.handoff_id).expect("read");
    assert_eq!(loaded.handoff_id, note.handoff_id);
    assert_eq!(loaded.recipient.label, "Yo");
}

#[test]
fn handoff_persists_under_openmesh_handoff() {
    let (dir, project_path, project_id) = temp_project("path");
    let note = sample_note(&project_id);
    write_handoff_note(&project_path, &note).expect("write");
    let expected = handoff_note_path(&project_path, &note.handoff_id);
    assert!(expected.exists());
    assert_eq!(
        expected,
        dir.join(".openmesh/handoff")
            .join(format!("{}.json", note.handoff_id))
    );
    assert!(!get_project_dir(&project_path)
        .join("proxy/pending")
        .exists());
}

#[test]
fn list_handoff_ids_is_deterministic() {
    let (_dir, project_path, project_id) = temp_project("list");
    let first = sample_note(&project_id);
    write_handoff_note(&project_path, &first).expect("write first");
    let mut second = sample_note(&project_id);
    second.handoff_id = "handoff-zzzz".into();
    second.created_at = NOW.into();
    second.updated_at = NOW.into();
    write_handoff_note(&project_path, &second).expect("write second");
    assert_eq!(
        list_handoff_ids(&project_path).expect("list"),
        vec![first.handoff_id, second.handoff_id]
    );
}

#[test]
fn approve_sets_status_and_timestamps() {
    let (_dir, project_path, project_id) = temp_project("approve");
    let note = sample_note(&project_id);
    write_handoff_note(&project_path, &note).expect("write");
    let approved = approve_handoff_note(&project_path, &note.handoff_id, "2026-07-31T16:00:00Z")
        .expect("approve");
    assert_eq!(approved.status, HandoffStatus::Approved);
    assert_eq!(
        approved.approved_at.as_deref(),
        Some("2026-07-31T16:00:00Z")
    );
}

#[test]
fn link_appends_work_handoff_event() {
    let (_dir, project_path, project_id) = temp_project("link");
    let note = sample_note(&project_id);
    write_handoff_note(&project_path, &note).expect("write");
    let linked = link_handoff_work_event(&project_path, note).expect("link");
    assert!(linked.work_event_id.is_some());
    let event_id = linked.work_event_id.clone().expect("event id");
    let event = get_event(&project_path, &event_id)
        .expect("get")
        .expect("present");
    assert_eq!(event.kind, WORK_EVENT_HANDOFF_KIND);
    let events = list_events(&project_path).expect("events");
    assert_eq!(events.len(), 1);
    let reloaded = read_handoff_note(&project_path, &linked.handoff_id).expect("reload");
    assert_eq!(reloaded.work_event_id.as_deref(), Some(event_id.as_str()));
}

#[test]
fn link_is_idempotent_when_event_exists() {
    let (_dir, project_path, project_id) = temp_project("link-idempotent");
    let note = sample_note(&project_id);
    write_handoff_note(&project_path, &note).expect("write");
    let first = link_handoff_work_event(&project_path, note.clone()).expect("first link");
    let second = link_handoff_work_event(&project_path, first.clone());
    assert_eq!(
        second,
        Err(HandoffStorageError::AlreadyLinked(
            first.work_event_id.clone().expect("event id")
        ))
    );
}
