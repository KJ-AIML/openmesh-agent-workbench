//! Dev Track 0.1.3.6 Checkpoint C — Heli evidence reader integration tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use openmesh_core::domain::{HeliProducerResult, ProducerSkipReason};
use openmesh_core::producers::read_heli_snapshot;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_workspace(label: &str) -> PathBuf {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("openmesh-heli-it-{label}-{id}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp workspace");
    dir
}

fn write_heli_state(ws: &Path, current: Option<&str>, decisions: Option<&str>) {
    let state = ws.join(".heli-harness/state");
    fs::create_dir_all(&state).expect("state dir");
    if let Some(text) = current {
        fs::write(state.join("current-task.md"), text).expect("current-task");
    }
    if let Some(text) = decisions {
        fs::write(state.join("decisions.md"), text).expect("decisions");
    }
}

fn snapshot_from(result: HeliProducerResult) -> openmesh_core::domain::HeliSnapshot {
    match result {
        HeliProducerResult::Snapshot(s) => s,
        other => panic!("expected snapshot, got {other:?}"),
    }
}

#[test]
fn heli_reader_current_task_present() {
    let ws = temp_workspace("task");
    write_heli_state(
        &ws,
        Some("# Dev Track 0.1.3.6\n\nCheckpoint C active\n"),
        None,
    );
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert!(snapshot
        .current_task_excerpt
        .as_deref()
        .unwrap()
        .contains("0.1.3.6"));
}

#[test]
fn heli_reader_decisions_present() {
    let ws = temp_workspace("decisions");
    write_heli_state(&ws, None, Some("## 2026-07-16\n\nTrack disposition\n"));
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert!(snapshot
        .decisions_tail_excerpt
        .as_deref()
        .unwrap()
        .contains("disposition"));
}

#[test]
fn heli_reader_both_present() {
    let ws = temp_workspace("both");
    write_heli_state(&ws, Some("Active track\n"), Some("## decision tail\n"));
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert!(snapshot.current_task_excerpt.is_some());
    assert!(snapshot.decisions_tail_excerpt.is_some());
}

#[test]
fn heli_reader_missing_harness_is_graceful_skip() {
    let ws = temp_workspace("no-harness");
    assert!(matches!(
        read_heli_snapshot(&ws),
        HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent)
    ));
}

#[test]
fn heli_reader_missing_state_directory_is_graceful_skip() {
    let ws = temp_workspace("no-state");
    fs::create_dir_all(ws.join(".heli-harness")).expect("harness root only");
    assert!(matches!(
        read_heli_snapshot(&ws),
        HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent)
    ));
}

#[test]
fn heli_reader_missing_optional_file_allows_partial_snapshot() {
    let ws = temp_workspace("partial");
    write_heli_state(&ws, Some("Only current task\n"), None);
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert!(snapshot.current_task_excerpt.is_some());
    assert!(snapshot.decisions_tail_excerpt.is_none());
}

#[test]
fn heli_reader_large_file_is_bounded_safely() {
    let ws = temp_workspace("large");
    let huge = "x".repeat(200_000);
    write_heli_state(&ws, Some(&huge), None);
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert!(snapshot.current_task_excerpt.as_ref().unwrap().len() <= 4096);
}

#[test]
fn heli_reader_does_not_mutate_source_files() {
    let ws = temp_workspace("readonly");
    write_heli_state(&ws, Some("immutable\n"), None);
    let path = ws.join(".heli-harness/state/current-task.md");
    let before = fs::read(&path).expect("read before");
    let _ = read_heli_snapshot(&ws);
    let after = fs::read(&path).expect("read after");
    assert_eq!(before, after);
}

#[test]
fn heli_reader_repeated_read_is_deterministic_for_fields() {
    let ws = temp_workspace("repeat");
    write_heli_state(&ws, Some("Deterministic\n"), Some("## tail\n"));
    let a = snapshot_from(read_heli_snapshot(&ws));
    let b = snapshot_from(read_heli_snapshot(&ws));
    assert_eq!(a.current_task_excerpt, b.current_task_excerpt);
    assert_eq!(a.decisions_tail_excerpt, b.decisions_tail_excerpt);
}

#[test]
fn heli_reader_accepts_unix_style_fixture_paths() {
    let ws = temp_workspace("unix-paths");
    write_heli_state(&ws, Some("unix\n"), None);
    let reports = ws.join(".heli-harness/state/reports");
    fs::create_dir_all(&reports).expect("reports");
    fs::write(reports.join("openmesh-0.1.3.6-plan.md"), "report").expect("report");
    let snapshot = snapshot_from(read_heli_snapshot(&ws));
    assert_eq!(
        snapshot.latest_report_path.as_deref(),
        Some(".heli-harness/state/reports/openmesh-0.1.3.6-plan.md")
    );
}

#[test]
fn heli_reader_no_executable_or_plugin_dependency() {
    let ws = temp_workspace("pure-files");
    write_heli_state(&ws, Some("file-only\n"), None);
    let result = read_heli_snapshot(&ws);
    assert!(matches!(result, HeliProducerResult::Snapshot(_)));
}
