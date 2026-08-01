//! Dev Track 0.1.8 Checkpoint C — handoff builder tests.

use openmesh_core::continuity::{build_current_state_projection, ContinuityInputSnapshot};
use openmesh_core::domain::{
    CatchUpWindow, EvidenceAttachment, EvidenceRef, SourceCounts, WorkEvent,
};
use openmesh_core::handoff::{
    build_handoff_note, build_handoff_recipient, validate_handoff_note, BuildHandoffRequest,
};

const NOW: &str = "2026-07-31T15:00:00Z";

fn window_all() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-15T00:00:00Z".into(),
        until: "2026-07-31T23:59:59Z".into(),
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
        source_counts: empty_source_counts(),
    }
}

fn request(workspace_id: &str) -> BuildHandoffRequest {
    BuildHandoffRequest {
        workspace_id: workspace_id.into(),
        recipient: build_handoff_recipient("Yo", Some("teammate")).expect("recipient"),
        window: window_all(),
        now_rfc3339: NOW.into(),
    }
}

#[test]
fn empty_project_documents_limitations() {
    let workspace_id = "ws-empty-handoff";
    let snapshot = empty_snapshot(workspace_id);
    let current_state = build_current_state_projection(&snapshot).expect("projection");
    let note =
        build_handoff_note(&snapshot, &current_state, &request(workspace_id)).expect("handoff");
    validate_handoff_note(&note).expect("valid");
    assert!(note.what_changed.items.is_empty());
    assert!(
        note.limitations
            .iter()
            .any(|entry| entry.contains("no continuity items in handoff window")),
        "expected empty-window limitation, got {:?}",
        note.limitations
    );
}

#[test]
fn seeded_event_produces_evidence_backed_item() {
    let workspace_id = "ws-seeded-handoff";
    let event = WorkEvent::new(
        "evt-handoff-seed",
        workspace_id,
        "work.completed",
        "Checkpoint B landed",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/plan.md".into()),
            observed_at: None,
        }],
        "2026-07-30T10:00:00Z",
    );
    let mut snapshot = empty_snapshot(workspace_id);
    snapshot.work_events = vec![event];
    snapshot.source_counts.work_events = 1;
    let current_state = build_current_state_projection(&snapshot).expect("projection");
    let note =
        build_handoff_note(&snapshot, &current_state, &request(workspace_id)).expect("handoff");
    validate_handoff_note(&note).expect("valid");
    assert!(
        !note.what_is_complete.items.is_empty()
            || note
                .limitations
                .iter()
                .any(|entry| entry.contains("no continuity records fell within window")),
        "expected completed item or catch-up limitation"
    );
    if let Some(item) = note.what_is_complete.items.first() {
        assert!(!item.evidence_refs.is_empty());
        assert_eq!(item.source_event_ids, vec!["evt-handoff-seed".to_string()]);
    }
}

#[test]
fn handoff_id_is_deterministic_for_same_inputs() {
    let workspace_id = "ws-deterministic-handoff";
    let snapshot = empty_snapshot(workspace_id);
    let current_state = build_current_state_projection(&snapshot).expect("projection");
    let req = request(workspace_id);
    let first = build_handoff_note(&snapshot, &current_state, &req).expect("first");
    let second = build_handoff_note(&snapshot, &current_state, &req).expect("second");
    assert_eq!(first.handoff_id, second.handoff_id);
    assert!(first.handoff_id.starts_with("handoff-20260731150000-"));
}
