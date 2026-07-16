//! Dev Track 0.1.3.7 Checkpoint A — Current State projection contract tests (pure, no I/O).

use openmesh_core::domain::{
    is_supported_current_state_projection_protocol, pending_attention_priority_for_severity,
    validate_continuity_state_item, validate_current_state_projection,
    validate_pending_attention_item, validate_source_counts, ContinuityConfidence,
    ContinuitySourceKind, ContinuityStateItem, ContinuityValidationError, CurrentStateProjection,
    CurrentStateSections, EvidenceRef, PendingAttentionItem, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, SourceCounts,
    CURRENT_STATE_PROJECTION_PROTOCOL_VERSION, MAX_CONTINUITY_ITEM_EVIDENCE_REFS,
    MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES, MAX_PROJECTION_EVIDENCE_REFS,
    MAX_PROJECTION_LIMITATIONS,
};

fn sample_state_item(
    source: ContinuitySourceKind,
    id: &str,
    source_id: &str,
) -> ContinuityStateItem {
    ContinuityStateItem {
        id: id.into(),
        summary: "Bounded continuity item summary".into(),
        kind: "progress".into(),
        source,
        source_id: source_id.into(),
        producer: "git".into(),
        timestamp: "2026-07-16T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath(
            "crates/openmesh-core/src/domain.rs".into(),
        )],
        confidence: ContinuityConfidence::Medium,
        correlation_hint: None,
        unverified: if source == ContinuitySourceKind::PendingSignal {
            Some(true)
        } else {
            None
        },
    }
}

fn empty_sections() -> CurrentStateSections {
    CurrentStateSections {
        completed: vec![],
        in_progress: vec![],
        blocked: vec![],
        decisions: vec![],
        needs_attention: vec![],
        still_open: vec![],
    }
}

fn sample_projection() -> CurrentStateProjection {
    CurrentStateProjection {
        workspace_id: "1783586870822-7352d".into(),
        generated_at: "2026-07-16T10:00:00Z".into(),
        protocol_version: CURRENT_STATE_PROJECTION_PROTOCOL_VERSION.into(),
        sections: CurrentStateSections {
            in_progress: vec![sample_state_item(
                ContinuitySourceKind::ProcessedSignal,
                "signal:s-001",
                "s-001",
            )],
            ..empty_sections()
        },
        pending_attention: vec![],
        source_counts: SourceCounts {
            work_events: 0,
            processed_signals: 1,
            pending_signals: 0,
            promotion_audit_records: 0,
            quarantine_signals: 0,
            duplicate_signals: 0,
            reporter_signals: 0,
            git_signals: 1,
            heli_signals: 0,
            unknown_producer_signals: 0,
            other_producer_signals: 0,
        },
        evidence_refs: vec![EvidenceRef::FilePath(
            "crates/openmesh-core/src/domain.rs".into(),
        )],
        limitations: vec![],
        rebuild_inputs_hash: "fnv1a-abc123".into(),
    }
}

#[test]
fn current_state_projection_round_trips_json() {
    let projection = sample_projection();
    let json = serde_json::to_string(&projection).expect("serialize");
    let restored: CurrentStateProjection = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, projection);
    assert!(json.contains("\"inProgress\""));
    assert!(json.contains("\"pendingAttention\""));
    assert!(json.contains("\"sourceCounts\""));
}

#[test]
fn current_state_fixture_is_valid() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest_dir}/tests/fixtures/current-state-valid.json");
    let fixture_json = std::fs::read_to_string(&fixture).expect("read fixture");
    let projection: CurrentStateProjection = serde_json::from_str(&fixture_json).expect("fixture");
    validate_current_state_projection(&projection).expect("fixture valid");
}

#[test]
fn current_state_protocol_1_0_is_supported() {
    assert!(is_supported_current_state_projection_protocol("1.0"));
    assert!(!is_supported_current_state_projection_protocol("2.0"));
}

#[test]
fn current_state_rejects_unknown_protocol() {
    let mut projection = sample_projection();
    projection.protocol_version = "9.9".into();
    assert!(matches!(
        validate_current_state_projection(&projection),
        Err(ContinuityValidationError::UnsupportedProtocolVersion { .. })
    ));
}

#[test]
fn continuity_item_requires_stable_id_prefix() {
    let mut item = sample_state_item(ContinuitySourceKind::WorkEvent, "signal:wrong", "evt-001");
    assert!(matches!(
        validate_continuity_state_item(&item),
        Err(ContinuityValidationError::InvalidContinuityItem(_))
    ));
    item.id = "event:evt-001".into();
    validate_continuity_state_item(&item).expect("valid event id");
}

#[test]
fn continuity_item_unverified_only_for_pending_signal() {
    let mut item = sample_state_item(
        ContinuitySourceKind::ProcessedSignal,
        "signal:s-001",
        "s-001",
    );
    item.unverified = Some(true);
    assert!(matches!(
        validate_continuity_state_item(&item),
        Err(ContinuityValidationError::InvalidContinuityItem(_))
    ));
}

#[test]
fn continuity_item_bounds_evidence_refs() {
    let mut item = sample_state_item(ContinuitySourceKind::PendingSignal, "signal:s-002", "s-002");
    item.evidence_refs = (0..MAX_CONTINUITY_ITEM_EVIDENCE_REFS + 1)
        .map(|i| EvidenceRef::FilePath(format!("docs/file-{i}.md")))
        .collect();
    assert!(matches!(
        validate_continuity_state_item(&item),
        Err(ContinuityValidationError::TooManyEvidenceRefs { .. })
    ));
}

#[test]
fn current_state_bounds_projection_evidence_and_limitations() {
    let mut projection = sample_projection();
    projection.evidence_refs = (0..MAX_PROJECTION_EVIDENCE_REFS + 1)
        .map(|i| EvidenceRef::FilePath(format!("docs/file-{i}.md")))
        .collect();
    assert!(matches!(
        validate_current_state_projection(&projection),
        Err(ContinuityValidationError::TooManyEvidenceRefs { .. })
    ));

    projection = sample_projection();
    projection.limitations = (0..MAX_PROJECTION_LIMITATIONS + 1)
        .map(|i| format!("limit-{i}"))
        .collect();
    assert!(matches!(
        validate_current_state_projection(&projection),
        Err(ContinuityValidationError::TooManyLimitations { .. })
    ));
}

fn sample_pending_attention() -> PendingAttentionItem {
    PendingAttentionItem {
        id: "attention:001".into(),
        summary: "Review required".into(),
        reason: PendingAttentionReason::ReviewRequired,
        source: ContinuitySourceKind::PendingSignal,
        source_id: "pending-001".into(),
        timestamp: "2026-07-16T10:00:00Z".into(),
        evidence_refs: vec![],
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::High,
        priority: pending_attention_priority_for_severity(PendingAttentionSeverity::High),
    }
}

#[test]
fn source_counts_tracks_git_heli_reporter_signals() {
    let counts = SourceCounts {
        work_events: 2,
        processed_signals: 4,
        pending_signals: 1,
        promotion_audit_records: 0,
        quarantine_signals: 0,
        duplicate_signals: 0,
        reporter_signals: 2,
        git_signals: 1,
        heli_signals: 1,
        unknown_producer_signals: 1,
        other_producer_signals: 0,
    };
    validate_source_counts(&counts).expect("producer breakdown within signal totals");
    assert_eq!(counts.reporter_signals, 2);
    assert_eq!(counts.git_signals, 1);
    assert_eq!(counts.heli_signals, 1);
}

#[test]
fn source_counts_round_trip_preserves_producer_breakdown() {
    let counts = SourceCounts {
        work_events: 0,
        processed_signals: 3,
        pending_signals: 2,
        promotion_audit_records: 1,
        quarantine_signals: 0,
        duplicate_signals: 0,
        reporter_signals: 2,
        git_signals: 1,
        heli_signals: 1,
        unknown_producer_signals: 0,
        other_producer_signals: 1,
    };
    let json = serde_json::to_string(&counts).expect("serialize");
    let restored: SourceCounts = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, counts);
    assert!(json.contains("\"reporterSignals\""));
    assert!(json.contains("\"gitSignals\""));
    assert!(json.contains("\"heliSignals\""));
    validate_source_counts(&restored).expect("round-trip valid");
}

#[test]
fn source_counts_rejects_producer_breakdown_exceeding_signal_total() {
    let counts = SourceCounts {
        work_events: 0,
        processed_signals: 1,
        pending_signals: 0,
        promotion_audit_records: 0,
        quarantine_signals: 0,
        duplicate_signals: 0,
        reporter_signals: 1,
        git_signals: 1,
        heli_signals: 0,
        unknown_producer_signals: 0,
        other_producer_signals: 0,
    };
    assert!(matches!(
        validate_source_counts(&counts),
        Err(ContinuityValidationError::InvalidSourceCounts(_))
    ));
}

#[test]
fn pending_attention_has_status_and_severity() {
    let item = sample_pending_attention();
    validate_pending_attention_item(&item).expect("status and severity present");
    let json = serde_json::to_string(&item).expect("serialize");
    assert!(json.contains("\"status\":\"open\""));
    assert!(json.contains("\"severity\":\"high\""));
    assert!(json.contains("\"priority\":2"));
}

#[test]
fn pending_attention_rejects_invalid_status_or_severity() {
    let json = r#"{
        "id":"attention:bad",
        "summary":"bad",
        "reason":"pending-signal",
        "source":"pending-signal",
        "sourceId":"pending-bad",
        "timestamp":"2026-07-16T10:00:00Z",
        "evidenceRefs":[],
        "status":"not-a-status",
        "severity":"high",
        "priority":2
    }"#;
    let err = serde_json::from_str::<PendingAttentionItem>(json).expect_err("invalid status");
    assert!(err.to_string().contains("unknown variant"));

    let json = r#"{
        "id":"attention:bad",
        "summary":"bad",
        "reason":"pending-signal",
        "source":"pending-signal",
        "sourceId":"pending-bad",
        "timestamp":"2026-07-16T10:00:00Z",
        "evidenceRefs":[],
        "status":"open",
        "severity":"urgent",
        "priority":2
    }"#;
    let err = serde_json::from_str::<PendingAttentionItem>(json).expect_err("invalid severity");
    assert!(err.to_string().contains("unknown variant"));
}

#[test]
fn pending_attention_priority_does_not_replace_status_or_severity() {
    let mut item = sample_pending_attention();
    item.priority = 5;
    validate_pending_attention_item(&item).expect("priority may differ from severity mapping");
    assert_eq!(item.status, PendingAttentionStatus::Open);
    assert_eq!(item.severity, PendingAttentionSeverity::High);
    assert_ne!(
        item.priority,
        pending_attention_priority_for_severity(item.severity)
    );
}

#[test]
fn pending_attention_item_validates_priority_range() {
    let mut item = sample_pending_attention();
    item.priority = 6;
    assert!(matches!(
        validate_pending_attention_item(&item),
        Err(ContinuityValidationError::InvalidPendingAttentionItem(_))
    ));
}

#[test]
fn current_state_rejects_oversized_summary() {
    let mut item = sample_state_item(
        ContinuitySourceKind::ProcessedSignal,
        "signal:s-003",
        "s-003",
    );
    item.summary = "x".repeat(MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES + 1);
    assert!(matches!(
        validate_continuity_state_item(&item),
        Err(ContinuityValidationError::SummaryTooLong { .. })
    ));
}

#[test]
fn current_state_contract_types_are_pure_no_io() {
    let projection = sample_projection();
    validate_current_state_projection(&projection).expect("sample projection validates");
    let json = serde_json::to_string(&projection).expect("serialize without filesystem writes");
    assert!(json.contains("\"protocolVersion\":\"1.0\""));
}
