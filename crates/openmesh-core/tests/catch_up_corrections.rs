//! Dev Track 0.1.3.8 Checkpoint D — Catch-up correction visibility tests.

use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, projections_dir, ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    CatchUpView, CatchUpWindow, ContinuityConfidence, ContinuitySourceKind, ContinuityStateItem,
    EvidenceAttachment, EvidenceRef, SourceCounts, WorkEvent, WORK_EVENT_CORRECTION_KIND,
};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::storage::get_project_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn window_all() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-15T00:00:00Z".into(),
        until: "2026-07-17T23:59:59Z".into(),
    }
}

fn window_before_correction() -> CatchUpWindow {
    CatchUpWindow {
        since: "2026-07-15T00:00:00Z".into(),
        until: "2026-07-17T01:30:00Z".into(),
    }
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

fn build_view(snapshot: &ContinuityInputSnapshot, window: &CatchUpWindow) -> CatchUpView {
    let current_state = build_current_state_projection(snapshot).expect("current state");
    build_catch_up_view(snapshot, &current_state, window).expect("catch-up view")
}

fn all_catch_up_items(view: &CatchUpView) -> Vec<&ContinuityStateItem> {
    view.sections
        .completed
        .iter()
        .chain(view.sections.changed.iter())
        .chain(view.sections.blocked.iter())
        .chain(view.sections.decided.iter())
        .chain(view.sections.needs_attention.iter())
        .chain(view.sections.still_open.iter())
        .collect()
}

fn event_item<'a>(view: &'a CatchUpView, event_id: &str) -> Option<&'a ContinuityStateItem> {
    all_catch_up_items(view)
        .into_iter()
        .find(|item| item.source_id == event_id && item.source == ContinuitySourceKind::WorkEvent)
}

fn changed_correction_item<'a>(
    view: &'a CatchUpView,
    correction_id: &str,
) -> Option<&'a ContinuityStateItem> {
    view.sections
        .changed
        .iter()
        .find(|item| item.source_id == correction_id)
}

fn evidence_contains_correction_ref(item: &ContinuityStateItem, event_id: &str) -> bool {
    item.evidence_refs.iter().any(|evidence| match evidence {
        EvidenceRef::FilePath(path) => path.contains(event_id),
        _ => false,
    })
}

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-catch-up-corrections-{name}-{}-{unique}",
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
fn catch_up_uses_effective_summary_for_corrected_event() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.summary, "Corrected summary");
}

#[test]
fn catch_up_uses_effective_kind_for_corrected_event() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.kind, "work.blocked");
    assert_eq!(view.sections.blocked.len(), 1);
}

#[test]
fn catch_up_suppresses_superseded_original_presentation() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let summaries: Vec<_> = all_catch_up_items(&view)
        .iter()
        .map(|item| item.summary.as_str())
        .collect();
    assert!(!summaries
        .iter()
        .any(|summary| summary.contains("Original summary")));
}

#[test]
fn correction_inside_window_appears_in_changed_section() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let changed = changed_correction_item(&view, "evt-c1").expect("changed item");
    assert_eq!(changed.correlation_hint.as_deref(), Some("evt-original"));
    assert_eq!(changed.kind, WORK_EVENT_CORRECTION_KIND);
    assert!(changed.summary.contains("Corrected summary"));
}

#[test]
fn correction_outside_window_does_not_create_changed_item() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_before_correction());
    assert!(changed_correction_item(&view, "evt-c1").is_none());
    let item = event_item(&view, "evt-original").expect("original still visible");
    assert_eq!(item.summary, "Corrected summary");
}

#[test]
fn catch_up_preserves_correction_chain_evidence_refs() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert!(evidence_contains_correction_ref(item, "evt-c1"));
    assert!(evidence_contains_correction_ref(item, "evt-c2"));
}

#[test]
fn catch_up_marks_corrected_presentation_confidence_no_higher_than_medium() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.confidence, ContinuityConfidence::Medium);
}

#[test]
fn invalid_correction_does_not_hide_original_in_catch_up() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.summary, "Original summary");
    assert_eq!(item.confidence, ContinuityConfidence::High);
    assert!(view
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    assert!(event_item(&view, "evt-original").is_some());
    assert!(view
        .limitations
        .iter()
        .any(|l| l.contains("correction cycle")));
}

#[test]
fn multiple_corrections_latest_wins_in_catch_up() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.summary, "Latest correction");
}

#[test]
fn correction_tie_break_is_deterministic_in_catch_up() {
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
    let snapshot = snapshot_with_events("ws-1", events);
    let view = build_view(&snapshot, &window_all());
    let item = event_item(&view, "evt-original").expect("original item");
    assert_eq!(item.summary, "Tie B");
}

#[test]
fn catch_up_correction_visibility_does_not_write_projection_files() {
    let (_dir, project_path, project_id) = create_test_project("no-projection");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id;
    append_event(&project_path, &original).expect("append");
    let snapshot = snapshot_with_events("ws-1", vec![original]);
    let _view = build_view(&snapshot, &window_all());
    assert!(!projections_dir(&project_path).exists());
}

#[test]
fn catch_up_correction_visibility_does_not_mutate_event_ledger() {
    let (_dir, project_path, project_id) = create_test_project("ledger-immutable");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id.clone();
    append_event(&project_path, &original).expect("append");
    let before = fs::read_to_string(ledger_dir(&project_path).join("evt-original.json")).unwrap();
    let snapshot = snapshot_with_events(&project_id, vec![original]);
    let _view = build_view(&snapshot, &window_all());
    let after = fs::read_to_string(ledger_dir(&project_path).join("evt-original.json")).unwrap();
    assert_eq!(before, after);
}

#[test]
fn catch_up_correction_visibility_does_not_mutate_signal_buckets() {
    let (_dir, project_path, project_id) = create_test_project("signals-immutable");
    let mut original = base_event(
        "evt-original",
        "work.completed",
        "Original summary",
        "2026-07-17T01:00:00Z",
    );
    original.workspace_id = project_id.clone();
    append_event(&project_path, &original).expect("append");
    let before = bucket_snapshot(&project_path);
    let snapshot = snapshot_with_events(&project_id, vec![original]);
    let _view = build_view(&snapshot, &window_all());
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn checkpoint_d_does_not_touch_cli_tauri_or_0_1_4() {
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
                "Checkpoint D continuity scope must not reference `{term}`: {}",
                path.display()
            );
        }
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        52,
        "Tauri command count must remain 52"
    );
}
