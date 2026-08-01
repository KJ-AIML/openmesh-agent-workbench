//! Dev Track 0.1.8 Checkpoint A — Handoff Note contract tests (pure).

use openmesh_core::domain::{CatchUpWindow, EvidenceRef};
use openmesh_core::handoff::{
    validate_handoff_note, HandoffFreshness, HandoffNote, HandoffRecipient, HandoffSection,
    HandoffSectionItem, HandoffStatus, HandoffValidationError, HANDOFF_NOTE_PROTOCOL_VERSION,
    WORK_EVENT_HANDOFF_KIND,
};

fn sample_item(summary: &str) -> HandoffSectionItem {
    HandoffSectionItem {
        summary: summary.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/plan.md".into())],
        source_event_ids: vec!["evt-1".into()],
    }
}

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
            items: vec![sample_item("Authority gate landed in 0.1.7")],
        },
        what_is_complete: HandoffSection::default(),
        what_is_blocked: HandoffSection::default(),
        what_needs_review: HandoffSection::default(),
        open_questions: HandoffSection {
            items: vec![sample_item("Should Desktop UI follow in a later track?")],
        },
        safe_to_answer_context: HandoffSection::default(),
        next_suggested_step: HandoffSection {
            items: vec![sample_item("Start 0.1.8 builder from Current State")],
        },
        freshness: HandoffFreshness {
            generated_at: "2026-07-31T15:00:00Z".into(),
            window: CatchUpWindow {
                since: "2026-07-24T00:00:00Z".into(),
                until: "2026-07-31T23:59:59Z".into(),
            },
            age_seconds: 0,
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
fn valid_draft_note_passes() {
    validate_handoff_note(&sample_note()).expect("valid");
}

#[test]
fn work_event_handoff_kind_is_stable() {
    assert_eq!(WORK_EVENT_HANDOFF_KIND, "work.handoff");
}

#[test]
fn serde_roundtrip_preserves_note() {
    let note = sample_note();
    let json = serde_json::to_string(&note).expect("ser");
    let back: HandoffNote = serde_json::from_str(&json).expect("de");
    assert_eq!(back, note);
}

#[test]
fn deny_unknown_fields() {
    let json = r#"{
      "protocolVersion":"1.0",
      "handoffId":"h1",
      "workspaceId":"ws",
      "status":"draft",
      "recipient":{"label":"Yo"},
      "window":{"since":"2026-07-24T00:00:00Z","until":"2026-07-31T23:59:59Z"},
      "whatChanged":{"items":[]},
      "whatIsComplete":{"items":[]},
      "whatIsBlocked":{"items":[]},
      "whatNeedsReview":{"items":[]},
      "openQuestions":{"items":[]},
      "safeToAnswerContext":{"items":[]},
      "nextSuggestedStep":{"items":[]},
      "freshness":{"generatedAt":"2026-07-31T15:00:00Z","window":{"since":"2026-07-24T00:00:00Z","until":"2026-07-31T23:59:59Z"},"ageSeconds":0,"warnings":[]},
      "limitations":["empty sections documented"],
      "createdAt":"2026-07-31T15:00:00Z",
      "updatedAt":"2026-07-31T15:00:00Z",
      "extraField":true
    }"#;
    assert!(serde_json::from_str::<HandoffNote>(json).is_err());
}

#[test]
fn rejects_unsupported_protocol() {
    let mut note = sample_note();
    note.protocol_version = "0.9".into();
    assert!(matches!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::UnsupportedProtocolVersion { .. })
    ));
}

#[test]
fn rejects_empty_recipient() {
    let mut note = sample_note();
    note.recipient.label = "  ".into();
    assert_eq!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::EmptyRecipientLabel)
    );
}

#[test]
fn rejects_inverted_window() {
    let mut note = sample_note();
    note.window.since = "2026-07-31T00:00:00Z".into();
    note.window.until = "2026-07-24T00:00:00Z".into();
    assert_eq!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::WindowInverted)
    );
}

#[test]
fn approved_requires_approved_at() {
    let mut note = sample_note();
    note.status = HandoffStatus::Approved;
    assert_eq!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::ApprovedMissingApprovedAt)
    );
    note.approved_at = Some("2026-07-31T16:00:00Z".into());
    note.updated_at = "2026-07-31T16:00:00Z".into();
    validate_handoff_note(&note).expect("approved ok");
}

#[test]
fn draft_must_not_set_approved_at() {
    let mut note = sample_note();
    note.approved_at = Some("2026-07-31T16:00:00Z".into());
    assert_eq!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::DraftHasApprovedAt)
    );
}

#[test]
fn empty_sections_require_limitations() {
    let mut note = sample_note();
    note.what_changed = HandoffSection::default();
    note.open_questions = HandoffSection::default();
    note.next_suggested_step = HandoffSection::default();
    assert_eq!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::EmptyHandoffWithoutLimitations)
    );
    note.limitations = vec!["no continuity items in window".into()];
    validate_handoff_note(&note).expect("limitations allow empty");
}

#[test]
fn rejects_empty_item_summary() {
    let mut note = sample_note();
    note.what_changed.items[0].summary = " ".into();
    assert!(matches!(
        validate_handoff_note(&note),
        Err(HandoffValidationError::EmptyItemSummary { .. })
    ));
}

#[test]
fn rejects_unsafe_handoff_id_path_segments() {
    for unsafe_id in ["../escape", "handoff/evil", r"handoff\evil", ".."] {
        let mut note = sample_note();
        note.handoff_id = unsafe_id.into();
        assert_eq!(
            validate_handoff_note(&note),
            Err(HandoffValidationError::UnsafeHandoffId)
        );
    }
}
