//! Dev Track 0.1.8 Checkpoint E — handoff markdown projection tests.

use openmesh_core::domain::{CatchUpWindow, EvidenceRef};
use openmesh_core::handoff::{
    render_handoff_markdown, HandoffFreshness, HandoffNote, HandoffRecipient, HandoffSection,
    HandoffSectionItem, HandoffStatus, HANDOFF_NOTE_PROTOCOL_VERSION,
};

fn sample_note() -> HandoffNote {
    HandoffNote {
        protocol_version: HANDOFF_NOTE_PROTOCOL_VERSION.into(),
        handoff_id: "handoff-20260731-demo".into(),
        workspace_id: "ws-demo".into(),
        status: HandoffStatus::Draft,
        recipient: HandoffRecipient {
            label: "Yo".into(),
            role_label: Some("teammate".into()),
        },
        window: CatchUpWindow {
            since: "2026-07-24T00:00:00Z".into(),
            until: "2026-07-31T23:59:59Z".into(),
        },
        what_changed: HandoffSection {
            items: vec![HandoffSectionItem {
                summary: "Authority gate landed".into(),
                evidence_refs: vec![EvidenceRef::FilePath("docs/plan.md".into())],
                source_event_ids: vec!["evt-1".into()],
            }],
        },
        what_is_complete: HandoffSection::default(),
        what_is_blocked: HandoffSection::default(),
        what_needs_review: HandoffSection::default(),
        open_questions: HandoffSection::default(),
        safe_to_answer_context: HandoffSection::default(),
        next_suggested_step: HandoffSection::default(),
        freshness: HandoffFreshness {
            generated_at: "2026-07-31T15:00:00Z".into(),
            window: CatchUpWindow {
                since: "2026-07-24T00:00:00Z".into(),
                until: "2026-07-31T23:59:59Z".into(),
            },
            age_seconds: 42,
            warnings: vec![],
        },
        limitations: vec![],
        created_at: "2026-07-31T15:00:00Z".into(),
        updated_at: "2026-07-31T15:00:00Z".into(),
        approved_at: None,
        work_event_id: None,
    }
}

#[test]
fn markdown_is_deterministic() {
    let note = sample_note();
    let first = render_handoff_markdown(&note);
    let second = render_handoff_markdown(&note);
    assert_eq!(first, second);
}

#[test]
fn markdown_includes_status_recipient_and_evidence() {
    let markdown = render_handoff_markdown(&sample_note());
    assert!(markdown.contains("**Status:** draft"));
    assert!(markdown.contains("**Recipient:** Yo (teammate)"));
    assert!(markdown.contains("## What Changed"));
    assert!(markdown.contains("Authority gate landed"));
    assert!(markdown.contains("evidence: file:docs/plan.md"));
    assert!(markdown.contains("source_event_ids: evt-1"));
}

#[test]
fn approved_status_renders() {
    let mut note = sample_note();
    note.status = HandoffStatus::Approved;
    note.approved_at = Some("2026-07-31T16:00:00Z".into());
    let markdown = render_handoff_markdown(&note);
    assert!(markdown.contains("**Status:** approved"));
    assert!(markdown.contains("approved_at: 2026-07-31T16:00:00Z"));
}
