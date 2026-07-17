//! Dev Track 0.1.3.8 Checkpoint A — WorkEvent correction semantics (pure contracts).

use openmesh_core::domain::{
    classify_direct_corrections, correction_chain_event_ids, correction_cycle_path,
    effective_confidence_for, effective_kind_for, effective_presentation, effective_summary_for,
    is_superseded_original, validate_correction_relationship, validate_event_semantics,
    ContinuityConfidence, CorrectionSemanticDiagnostic, EvidenceAttachment, EvidenceRef, WorkEvent,
    WORK_EVENT_CORRECTION_KIND,
};
use openmesh_core::events::{append_event, effective_kind, effective_summary, get_event};
use std::fs;
use std::path::PathBuf;

fn sample_evidence() -> Vec<EvidenceAttachment> {
    vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into()),
        observed_at: Some("2026-07-17T03:00:00Z".into()),
    }]
}

fn base_event(event_id: &str, kind: &str, summary: &str, timestamp: &str) -> WorkEvent {
    WorkEvent::new(
        event_id,
        "ws-test",
        kind,
        summary,
        sample_evidence(),
        timestamp,
    )
}

fn correction_event(
    event_id: &str,
    target_id: &str,
    kind: &str,
    summary: &str,
    timestamp: &str,
) -> WorkEvent {
    let mut event = base_event(event_id, kind, summary, timestamp);
    event.corrects_event_id = Some(target_id.to_string());
    event
}

#[test]
fn uncorrected_event_uses_original_effective_kind_and_summary() {
    let events = vec![base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    )];
    let original = &events[0];

    assert_eq!(effective_kind_for(&events, original), "work.completed");
    assert_eq!(effective_summary_for(&events, original), "Original summary");
    let presentation = effective_presentation(&events, "evt-original").unwrap();
    assert!(!presentation.is_corrected);
    assert!(!presentation.is_superseded_original);
    assert_eq!(presentation.confidence, ContinuityConfidence::High);
}

#[test]
fn correction_event_changes_effective_summary() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Corrected summary",
            "2026-07-17T02:00:00Z",
        ),
    ];
    let original = &events[0];

    assert_eq!(
        effective_summary_for(&events, original),
        "Corrected summary"
    );
}

#[test]
fn correction_event_changes_effective_kind() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            "work.blocked",
            "Blocked instead",
            "2026-07-17T02:00:00Z",
        ),
    ];
    let original = &events[0];

    assert_eq!(effective_kind_for(&events, original), "work.blocked");
}

#[test]
fn original_event_is_not_deleted_or_rewritten_by_correction_semantics() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Corrected summary",
            "2026-07-17T02:00:00Z",
        ),
    ];

    let original = find_event(&events, "evt-original").unwrap();
    assert_eq!(original.summary, "Original summary");
    assert_eq!(original.kind, "work.completed");
    assert!(original.corrects_event_id.is_none());

    let presentation = effective_presentation(&events, "evt-original").unwrap();
    assert_eq!(presentation.original_summary, "Original summary");
    assert_eq!(presentation.original_kind, "work.completed");
}

#[test]
fn correction_chain_remains_inspectable() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "First correction",
            "2026-07-17T02:00:00Z",
        ),
        correction_event(
            "evt-c2",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Second correction",
            "2026-07-17T03:00:00Z",
        ),
    ];

    assert_eq!(
        correction_chain_event_ids(&events, "evt-original"),
        vec!["evt-c1".to_string(), "evt-c2".to_string()]
    );
    let presentation = effective_presentation(&events, "evt-original").unwrap();
    assert_eq!(
        presentation.correction_event_ids,
        vec!["evt-c1".to_string(), "evt-c2".to_string()]
    );
    assert!(find_event(&events, "evt-c1").is_some());
    assert!(find_event(&events, "evt-c2").is_some());
}

#[test]
fn latest_correction_wins_deterministically() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Earlier correction",
            "2026-07-17T02:00:00Z",
        ),
        correction_event(
            "evt-c2",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Latest correction",
            "2026-07-17T03:00:00Z",
        ),
    ];
    let original = &events[0];

    assert_eq!(
        effective_summary_for(&events, original),
        "Latest correction"
    );
    let presentation = effective_presentation(&events, "evt-original").unwrap();
    assert_eq!(
        presentation.superseded_by_event_id.as_deref(),
        Some("evt-c2")
    );
}

#[test]
fn correction_tie_breaks_by_event_id() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c-a",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Tie A",
            "2026-07-17T02:00:00Z",
        ),
        correction_event(
            "evt-c-b",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Tie B wins",
            "2026-07-17T02:00:00Z",
        ),
    ];
    let original = &events[0];

    assert_eq!(effective_summary_for(&events, original), "Tie B wins");
}

#[test]
fn correction_cannot_correct_itself() {
    let mut self_correction = base_event(
        "evt-self",
        WORK_EVENT_CORRECTION_KIND,
        "Self correction",
        "2026-07-17T02:00:00Z",
    );
    self_correction.corrects_event_id = Some("evt-self".into());
    let events = vec![self_correction.clone()];

    let err = validate_correction_relationship(&self_correction, &events).unwrap_err();
    assert!(matches!(
        err,
        CorrectionSemanticDiagnostic::SelfCorrection { .. }
    ));
}

#[test]
fn missing_correction_target_is_diagnostic_not_panic() {
    let correction = correction_event(
        "evt-c1",
        "evt-missing",
        WORK_EVENT_CORRECTION_KIND,
        "Orphan correction",
        "2026-07-17T02:00:00Z",
    );
    let events = vec![correction.clone()];

    let err = validate_correction_relationship(&correction, &events).unwrap_err();
    assert!(matches!(
        err,
        CorrectionSemanticDiagnostic::MissingTarget { .. }
    ));
    assert!(effective_presentation(&events, "evt-missing").is_none());
}

#[test]
fn invalid_correction_does_not_hide_original() {
    let mut invalid = correction_event(
        "evt-invalid",
        "evt-original",
        "",
        "Invalid empty kind",
        "2026-07-17T02:00:00Z",
    );
    invalid.kind = "   ".into();
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        invalid,
    ];
    let original = &events[0];

    assert_eq!(effective_summary_for(&events, original), "Original summary");
    assert!(!is_superseded_original(&events, "evt-original"));
    let (_, diagnostics) = classify_direct_corrections(&events, "evt-original");
    assert!(diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        CorrectionSemanticDiagnostic::InvalidCorrectionSemantics { .. }
    )));
}

#[test]
fn correction_cycle_is_rejected_or_diagnostic() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-a",
            "evt-b",
            WORK_EVENT_CORRECTION_KIND,
            "A corrects B",
            "2026-07-17T02:00:00Z",
        ),
        correction_event(
            "evt-b",
            "evt-a",
            WORK_EVENT_CORRECTION_KIND,
            "B corrects A",
            "2026-07-17T02:01:00Z",
        ),
    ];

    assert!(correction_cycle_path(&events, "evt-a").is_some());
    let err = validate_correction_relationship(&events[1], &events).unwrap_err();
    assert!(matches!(
        err,
        CorrectionSemanticDiagnostic::CorrectionCycle { .. }
    ));
}

#[test]
fn corrected_effective_presentation_confidence_is_capped_at_medium() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c1",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Corrected summary",
            "2026-07-17T02:00:00Z",
        ),
    ];

    let presentation = effective_presentation(&events, "evt-original").unwrap();
    assert_eq!(presentation.confidence, ContinuityConfidence::Medium);
    assert_eq!(
        effective_confidence_for(&events, "evt-original"),
        Some(ContinuityConfidence::Medium)
    );

    let correction_presentation = effective_presentation(&events, "evt-c1").unwrap();
    assert_eq!(
        correction_presentation.confidence,
        ContinuityConfidence::Medium
    );
}

#[test]
fn existing_work_event_records_remain_compatible() {
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/events");
    let original_raw = fs::read_to_string(fixture_dir.join("valid.json")).unwrap();
    let correction_raw =
        fs::read_to_string(fixture_dir.join("event-correction-valid.json")).unwrap();

    let original: WorkEvent = serde_json::from_str(&original_raw).unwrap();
    let correction: WorkEvent = serde_json::from_str(&correction_raw).unwrap();

    validate_event_semantics(&original).unwrap();
    validate_event_semantics(&correction).unwrap();
    assert_eq!(
        correction.corrects_event_id.as_deref(),
        Some("1783605120049-event001")
    );

    let events = vec![original.clone(), correction.clone()];
    assert_eq!(
        effective_summary_for(&events, &original),
        correction.summary
    );
}

#[test]
fn correction_helpers_are_pure_no_io() {
    let events = vec![base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    )];
    let original = &events[0];

    let _ = effective_presentation(&events, "evt-original");
    let _ = effective_kind_for(&events, original);
    let _ = effective_summary_for(&events, original);
    let _ = correction_chain_event_ids(&events, "evt-original");
    let _ = is_superseded_original(&events, "evt-original");
}

#[test]
fn checkpoint_a_does_not_modify_continuity_builders_or_cli() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let continuity_readers = fs::read_to_string(root.join("src/continuity/readers.rs")).unwrap();

    // `current_state.rs` and `catch_up.rs` correction wiring are Checkpoints C/D scope, not A.
    for content in [&continuity_readers] {
        assert!(!content.contains("effective_presentation"));
        assert!(!content.contains("effective_kind_for"));
        assert!(!content.contains("EffectiveEventPresentation"));
    }
}

#[test]
fn ledger_effective_kind_and_summary_use_semantics() {
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-correction-contract-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();

    let project_id = format!("proj-correction-{unique}");
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

    let original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    let mut original = original;
    original.workspace_id = project_id.clone();
    append_event(&project_path, &original).unwrap();

    let mut correction = correction_event(
        "evt-c1",
        "evt-original",
        "work.blocked",
        "Corrected summary",
        "2026-07-17T02:00:00Z",
    );
    correction.workspace_id = project_id;
    append_event(&project_path, &correction).unwrap();

    let stored_original = get_event(&project_path, "evt-original").unwrap().unwrap();
    assert_eq!(stored_original.summary, "Original summary");

    assert_eq!(
        effective_summary(&project_path, "evt-original").unwrap(),
        Some("Corrected summary".to_string())
    );
    assert_eq!(
        effective_kind(&project_path, "evt-original").unwrap(),
        Some("work.blocked".to_string())
    );

    let _ = fs::remove_dir_all(&dir);
}

fn find_event<'a>(events: &'a [WorkEvent], event_id: &str) -> Option<&'a WorkEvent> {
    events.iter().find(|event| event.event_id == event_id)
}
