//! Dev Track 0.1.3.7 Checkpoint A — Catch-up view contract tests (pure, no I/O).

use openmesh_core::domain::{
    is_supported_catch_up_view_protocol, pending_attention_priority_for_severity,
    validate_catch_up_view, validate_continuity_state_item, CatchUpSections, CatchUpView,
    CatchUpWindow, ContinuityConfidence, ContinuitySourceKind, ContinuityStateItem,
    ContinuityValidationError, EvidenceRef, PendingAttentionItem, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, CATCH_UP_VIEW_PROTOCOL_VERSION,
    MAX_CATCH_UP_SUMMARY_BYTES, MAX_NEXT_SUGGESTED_ATTENTION,
};

fn sample_item(source: ContinuitySourceKind, id: &str, source_id: &str) -> ContinuityStateItem {
    ContinuityStateItem {
        id: id.into(),
        summary: "Changed item".into(),
        kind: "progress".into(),
        source,
        source_id: source_id.into(),
        producer: "reporter:test".into(),
        timestamp: "2026-07-16T09:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath(
            "crates/openmesh-core/src/domain.rs".into(),
        )],
        confidence: ContinuityConfidence::High,
        correlation_hint: None,
        unverified: if source == ContinuitySourceKind::PendingSignal {
            Some(true)
        } else {
            None
        },
    }
}

fn empty_catch_up_sections() -> CatchUpSections {
    CatchUpSections {
        completed: vec![],
        changed: vec![],
        blocked: vec![],
        decided: vec![],
        needs_attention: vec![],
        still_open: vec![],
    }
}

fn sample_catch_up() -> CatchUpView {
    CatchUpView {
        workspace_id: "1783586870822-7352d".into(),
        generated_at: "2026-07-16T10:00:00Z".into(),
        protocol_version: CATCH_UP_VIEW_PROTOCOL_VERSION.into(),
        window: CatchUpWindow {
            since: "2026-07-15T10:00:00Z".into(),
            until: "2026-07-16T10:00:00Z".into(),
        },
        sections: CatchUpSections {
            changed: vec![sample_item(
                ContinuitySourceKind::WorkEvent,
                "event:evt-001",
                "evt-001",
            )],
            needs_attention: vec![sample_item(
                ContinuitySourceKind::PendingSignal,
                "signal:pending-001",
                "pending-001",
            )],
            ..empty_catch_up_sections()
        },
        summary: "1 changed; 1 needs attention".into(),
        next_suggested_attention: vec![PendingAttentionItem {
            id: "attention:pending-001".into(),
            summary: "Awaiting review".into(),
            reason: PendingAttentionReason::PendingSignal,
            source: ContinuitySourceKind::PendingSignal,
            source_id: "pending-001".into(),
            timestamp: "2026-07-16T09:45:00Z".into(),
            evidence_refs: vec![],
            status: PendingAttentionStatus::Open,
            severity: PendingAttentionSeverity::Medium,
            priority: pending_attention_priority_for_severity(PendingAttentionSeverity::Medium),
        }],
        evidence_refs: vec![EvidenceRef::FilePath(
            "crates/openmesh-core/src/domain.rs".into(),
        )],
        limitations: vec![],
    }
}

#[test]
fn catch_up_view_round_trips_json() {
    let view = sample_catch_up();
    let json = serde_json::to_string(&view).expect("serialize");
    let restored: CatchUpView = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, view);
    assert!(json.contains("\"nextSuggestedAttention\""));
    assert!(json.contains("\"stillOpen\""));
}

#[test]
fn catch_up_fixture_is_valid() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest_dir}/tests/fixtures/catch-up-valid.json");
    let fixture_json = std::fs::read_to_string(&fixture).expect("read fixture");
    let view: CatchUpView = serde_json::from_str(&fixture_json).expect("fixture");
    validate_catch_up_view(&view).expect("fixture valid");
}

#[test]
fn catch_up_protocol_1_0_is_supported() {
    assert!(is_supported_catch_up_view_protocol("1.0"));
    assert!(!is_supported_catch_up_view_protocol("1.1"));
}

#[test]
fn catch_up_rejects_inverted_window() {
    let mut view = sample_catch_up();
    view.window.since = "2026-07-17T10:00:00Z".into();
    assert!(matches!(
        validate_catch_up_view(&view),
        Err(ContinuityValidationError::CatchUpWindowInverted)
    ));
}

#[test]
fn catch_up_rejects_unknown_protocol() {
    let mut view = sample_catch_up();
    view.protocol_version = "0.5".into();
    assert!(matches!(
        validate_catch_up_view(&view),
        Err(ContinuityValidationError::UnsupportedProtocolVersion { .. })
    ));
}

#[test]
fn catch_up_bounds_summary_and_next_suggested_attention() {
    let mut view = sample_catch_up();
    view.summary = "x".repeat(MAX_CATCH_UP_SUMMARY_BYTES + 1);
    assert!(matches!(
        validate_catch_up_view(&view),
        Err(ContinuityValidationError::SummaryTooLong { .. })
    ));

    view = sample_catch_up();
    view.next_suggested_attention = (0..MAX_NEXT_SUGGESTED_ATTENTION + 1)
        .map(|i| PendingAttentionItem {
            id: format!("attention:{i}"),
            summary: format!("item {i}"),
            reason: PendingAttentionReason::PendingSignal,
            source: ContinuitySourceKind::PendingSignal,
            source_id: format!("pending-{i}"),
            timestamp: "2026-07-16T09:45:00Z".into(),
            evidence_refs: vec![],
            status: PendingAttentionStatus::Open,
            severity: PendingAttentionSeverity::Low,
            priority: 3,
        })
        .collect();
    assert!(matches!(
        validate_catch_up_view(&view),
        Err(ContinuityValidationError::TooManyNextSuggestedAttention { .. })
    ));
}

#[test]
fn catch_up_sections_use_six_fixed_keys() {
    let view = sample_catch_up();
    let json = serde_json::to_string(&view.sections).expect("serialize sections");
    for key in [
        "completed",
        "changed",
        "blocked",
        "decided",
        "needsAttention",
        "stillOpen",
    ] {
        assert!(
            json.contains(&format!("\"{key}\"")),
            "missing section {key}"
        );
    }
}

#[test]
fn catch_up_ambiguous_confidence_serializes() {
    let mut item = sample_item(ContinuitySourceKind::WorkEvent, "event:evt-002", "evt-002");
    item.confidence = ContinuityConfidence::Ambiguous;
    validate_continuity_state_item(&item).expect("ambiguous is valid");
    let json = serde_json::to_string(&item.confidence).expect("serialize confidence");
    assert_eq!(json, "\"ambiguous\"");
}

#[test]
fn catch_up_contract_types_are_pure_no_io() {
    let view = sample_catch_up();
    validate_catch_up_view(&view).expect("sample catch-up validates");
    let json = serde_json::to_string(&view).expect("serialize without filesystem writes");
    assert!(json.contains("\"protocolVersion\":\"1.0\""));
}
