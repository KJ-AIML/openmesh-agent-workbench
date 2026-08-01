//! Dev Track 0.1.8 Checkpoint B — handoff scope helper tests.

use openmesh_core::handoff::{
    build_handoff_recipient, resolve_handoff_window, HandoffValidationError,
};

#[test]
fn recipient_accepts_label_and_role() {
    let recipient = build_handoff_recipient("Yo", Some("teammate")).expect("recipient");
    assert_eq!(recipient.label, "Yo");
    assert_eq!(recipient.role_label.as_deref(), Some("teammate"));
}

#[test]
fn recipient_rejects_empty_label() {
    assert_eq!(
        build_handoff_recipient("  ", None),
        Err(HandoffValidationError::EmptyRecipientLabel)
    );
}

#[test]
fn window_defaults_to_last_seven_days() {
    let window = resolve_handoff_window(None, None, "2026-07-31T12:00:00Z").expect("window");
    assert_eq!(window.until, "2026-07-31T12:00:00Z");
    assert_eq!(window.since, "2026-07-24T12:00:00Z");
}

#[test]
fn window_requires_both_bounds_when_partial() {
    assert!(matches!(
        resolve_handoff_window(Some("2026-07-24T00:00:00Z"), None, "2026-07-31T12:00:00Z"),
        Err(HandoffValidationError::InvalidWindow(_))
    ));
    assert!(matches!(
        resolve_handoff_window(None, Some("2026-07-31T12:00:00Z"), "2026-07-31T12:00:00Z"),
        Err(HandoffValidationError::InvalidWindow(_))
    ));
}

#[test]
fn window_rejects_inverted_bounds() {
    assert_eq!(
        resolve_handoff_window(
            Some("2026-07-31T12:00:00Z"),
            Some("2026-07-24T12:00:00Z"),
            "2026-07-31T12:00:00Z"
        ),
        Err(HandoffValidationError::WindowInverted)
    );
}

#[test]
fn explicit_window_passes_validation() {
    let window = resolve_handoff_window(
        Some("2026-07-24T00:00:00Z"),
        Some("2026-07-31T23:59:59Z"),
        "2026-07-31T12:00:00Z",
    )
    .expect("window");
    assert_eq!(window.since, "2026-07-24T00:00:00Z");
    assert_eq!(window.until, "2026-07-31T23:59:59Z");
}
