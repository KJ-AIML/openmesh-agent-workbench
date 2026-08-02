//! Dev Track 0.1.3.8 Checkpoint C — Current State correction visibility tests.

use openmesh_core::continuity::{
    build_current_state_projection, current_state_projection_path, load_continuity_input_snapshot,
    projections_dir, rebuild_current_state_projection, ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    ContinuityConfidence, ContinuitySourceKind, ContinuityStateItem, CurrentStateProjection,
    EvidenceAttachment, EvidenceRef, SourceCounts, WorkEvent, WORK_EVENT_CORRECTION_KIND,
};
use openmesh_core::events::{
    append_event, append_event_correction, ledger_dir, EventCorrectionRequest,
};
use openmesh_core::storage::get_project_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sample_evidence() -> Vec<EvidenceAttachment> {
    vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
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

fn snapshot_with_events(
    workspace_id: &str,
    work_events: Vec<WorkEvent>,
) -> ContinuityInputSnapshot {
    let mut source_counts = empty_source_counts();
    source_counts.work_events = work_events.len() as u32;
    ContinuityInputSnapshot {
        workspace_id: workspace_id.into(),
        loaded_at: "2026-07-17T03:00:00Z".into(),
        pending_signals: Vec::new(),
        processed_signals: Vec::new(),
        quarantine_signals: Vec::new(),
        duplicate_signals: Vec::new(),
        work_events,
        promotion_audit_records: Vec::new(),
        diagnostics: Vec::new(),
        source_counts,
    }
}

fn all_state_items(projection: &CurrentStateProjection) -> Vec<&ContinuityStateItem> {
    projection
        .sections
        .completed
        .iter()
        .chain(projection.sections.in_progress.iter())
        .chain(projection.sections.blocked.iter())
        .chain(projection.sections.decisions.iter())
        .chain(projection.sections.needs_attention.iter())
        .chain(projection.sections.still_open.iter())
        .collect()
}

fn event_item<'a>(
    projection: &'a CurrentStateProjection,
    event_id: &str,
) -> Option<&'a ContinuityStateItem> {
    all_state_items(projection)
        .into_iter()
        .find(|item| item.source_id == event_id && item.source == ContinuitySourceKind::WorkEvent)
}

fn evidence_contains_correction_ref(item: &ContinuityStateItem, correction_id: &str) -> bool {
    item.evidence_refs.iter().any(|evidence| match evidence {
        EvidenceRef::FilePath(path) => path.contains(correction_id),
        _ => false,
    })
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-current-state-corrections-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();
    let project_id = format!("proj-{name}-{unique}");
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
        "createdAt": "2026-07-17T03:00:00Z",
        "updatedAt": "2026-07-17T03:00:00Z",
    });
    fs::write(
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();
    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

fn bucket_snapshot(project_path: &str) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let root = get_project_dir(project_path).join("signals");
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = root.join(bucket);
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                let key = format!("{bucket}/{}", path.file_name().unwrap().to_string_lossy());
                out.insert(key, fs::read_to_string(&path).unwrap());
            }
        }
    }
    out
}

fn ledger_file_count(project_path: &str) -> usize {
    let dir = ledger_dir(project_path);
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0)
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn current_state_uses_effective_summary_for_corrected_event() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.summary, "Corrected summary");
    assert!(!item.summary.contains("Original summary"));
}

#[test]
fn current_state_uses_effective_kind_for_corrected_event() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.kind, "work.blocked");
    assert_eq!(projection.sections.blocked.len(), 1);
    assert_eq!(projection.sections.completed.len(), 0);
}

#[test]
fn current_state_suppresses_superseded_original_presentation() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    assert!(event_item(&projection, "evt-c1").is_none());
    let summaries: Vec<_> = all_state_items(&projection)
        .iter()
        .map(|item| item.summary.as_str())
        .collect();
    assert!(!summaries.contains(&"Original summary"));
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("evt-original") && l.contains("corrected")));
}

#[test]
fn current_state_preserves_correction_chain_evidence_refs() {
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
            "Latest correction",
            "2026-07-17T03:00:00Z",
        ),
    ];
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert!(evidence_contains_correction_ref(item, "evt-c1"));
    assert!(evidence_contains_correction_ref(item, "evt-c2"));
}

#[test]
fn current_state_marks_corrected_presentation_confidence_no_higher_than_medium() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.confidence, ContinuityConfidence::Medium);
}

#[test]
fn invalid_correction_does_not_hide_original_in_current_state() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-orphan",
            "evt-missing",
            WORK_EVENT_CORRECTION_KIND,
            "Orphan correction",
            "2026-07-17T02:00:00Z",
        ),
    ];
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.summary, "Original summary");
    assert_eq!(item.confidence, ContinuityConfidence::High);
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("invalid correction")));
}

#[test]
fn correction_cycle_surfaces_limitation_not_panic() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    assert!(event_item(&projection, "evt-original").is_some());
    assert!(projection
        .limitations
        .iter()
        .any(|l| l.contains("correction cycle")));
}

#[test]
fn multiple_corrections_latest_wins_in_current_state() {
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
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.summary, "Latest correction");
}

#[test]
fn correction_tie_break_is_deterministic_in_current_state() {
    let events = vec![
        base_event(
            "evt-original",
            "work.completed",
            "Original summary",
            "2026-07-17T01:00:00Z",
        ),
        correction_event(
            "evt-c-b",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Tie B",
            "2026-07-17T02:00:00Z",
        ),
        correction_event(
            "evt-c-a",
            "evt-original",
            WORK_EVENT_CORRECTION_KIND,
            "Tie A",
            "2026-07-17T02:00:00Z",
        ),
    ];
    let projection =
        build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("valid");
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.summary, "Tie B");
}

#[test]
fn rebuild_current_state_projection_reflects_corrections() {
    let (_dir, project_path, project_id) = create_test_project("rebuild-corrections");
    let original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    let mut original = original;
    original.workspace_id = project_id.clone();
    append_event(&project_path, &original).expect("append original");
    append_event_correction(
        &project_path,
        "evt-original",
        &EventCorrectionRequest {
            corrected_kind: "work.blocked".into(),
            corrected_summary: "Corrected via rebuild".into(),
            actor_label: None,
            timestamp: Some("2026-07-17T02:00:00Z".into()),
        },
    )
    .expect("append correction");

    let projection = rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(current_state_projection_path(&project_path).exists());
    let item = event_item(&projection, "evt-original").expect("original item");
    assert_eq!(item.summary, "Corrected via rebuild");
    assert_eq!(item.kind, "work.blocked");
}

#[test]
fn current_state_correction_visibility_does_not_mutate_event_ledger() {
    let (_dir, project_path, project_id) = create_test_project("ledger-immutable");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id;
    append_event(&project_path, &original).expect("append original");
    let before = fs::read_to_string(ledger_dir(&project_path).join("evt-original.json")).unwrap();
    let count_before = ledger_file_count(&project_path);

    let events = load_continuity_input_snapshot(&project_path)
        .expect("snapshot")
        .work_events;
    let _ = build_current_state_projection(&snapshot_with_events("ws-1", events)).expect("build");

    let after = fs::read_to_string(ledger_dir(&project_path).join("evt-original.json")).unwrap();
    assert_eq!(before, after);
    assert_eq!(count_before, ledger_file_count(&project_path));
}

#[test]
fn current_state_correction_visibility_does_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("signals-immutable");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id;
    append_event(&project_path, &original).expect("append original");
    let before = bucket_snapshot(&project_path);
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn current_state_correction_visibility_does_not_write_catch_up_files() {
    let (_dir, project_path, project_id) = create_test_project("no-catch-up");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id;
    append_event(&project_path, &original).expect("append original");
    rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(!projections_dir(&project_path)
        .join("catch-up-checkpoint.json")
        .exists());
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    assert!(serde_json::to_value(&snapshot)
        .unwrap()
        .as_object()
        .unwrap()
        .get("catchUpView")
        .is_none());
}

#[test]
fn checkpoint_c_does_not_touch_cli_tauri_or_0_1_4() {
    let root = workspace_root();
    let continuity_src = root.join("crates/openmesh-core/src/continuity");
    let forbidden = [
        "openmesh-cli",
        "tauri::",
        "#[tauri::command]",
        "0.1.4",
        "run_event_inspect",
        "run_event_correct",
    ];
    let mut files = Vec::new();
    collect_rs_files(&continuity_src, &mut files);
    for path in files {
        let content = fs::read_to_string(&path).expect("read source");
        for term in forbidden {
            assert!(
                !content.contains(term),
                "Checkpoint C continuity scope must not reference `{term}`: {}",
                path.display()
            );
        }
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        53,
        "Tauri command count must remain 53 (get_host_os)"
    );
}
