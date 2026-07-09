// Checkpoint C — end-to-end CLI -> inbox tests (approved plan §17/§19.F/G).
// Invokes the actual compiled binary against real temp projects.

use openmesh_core::storage::init_project;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-e2e-{label}-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init_project should succeed");
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn pending_dir(project: &Path) -> PathBuf {
    project.join(".openmesh").join("signals").join("pending")
}

fn signals_root(project: &Path) -> PathBuf {
    project.join(".openmesh").join("signals")
}

fn count_files(dir: &Path) -> usize {
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir).unwrap().count()
}

#[test]
fn progress_signal_succeeds_and_lands_in_pending() {
    let project = temp_project("progress");
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "made real progress",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("signal_id="));
    assert!(stdout.contains("kind=progress"));
    assert_eq!(count_files(&pending_dir(&project)), 1);
}

#[test]
fn decision_signal_succeeds_in_json_mode() {
    let project = temp_project("decision");
    let output = cli()
        .args([
            "signal",
            "decision",
            "--summary",
            "chose option A",
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON output");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["kind"], "decision");
    assert!(value["signal_id"].is_string());
    assert!(value["workspace_id"].is_string());
    assert_eq!(count_files(&pending_dir(&project)), 1);
}

#[test]
fn blocker_signal_succeeds() {
    let project = temp_project("blocker");
    let output = cli()
        .args([
            "signal",
            "blocker",
            "--summary",
            "hit a blocker",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(count_files(&pending_dir(&project)), 1);
}

#[test]
fn handoff_signal_succeeds() {
    let project = temp_project("handoff");
    let output = cli()
        .args([
            "signal",
            "handoff",
            "--summary",
            "handing off to next session",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(count_files(&pending_dir(&project)), 1);
}

#[test]
fn project_resolution_error_is_exit_1_and_creates_nothing() {
    let not_a_project = temp_project("not-init-parent").join("never-initialized");
    fs::create_dir_all(&not_a_project).unwrap();
    let output = cli()
        .args(["signal", "progress", "--summary", "x", "--project"])
        .arg(&not_a_project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("project-resolution"));
    assert!(!not_a_project.join(".openmesh").exists());
}

#[test]
fn project_resolution_error_json_mode() {
    let not_a_project = temp_project("not-init-json-parent").join("never-initialized");
    fs::create_dir_all(&not_a_project).unwrap();
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "x",
            "--json",
            "--project",
        ])
        .arg(&not_a_project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON output");
    assert_eq!(value["status"], "error");
    assert_eq!(value["category"], "project-resolution");
}

#[test]
fn invalid_signal_empty_summary_is_exit_3_and_creates_no_partial_record() {
    let project = temp_project("invalid-empty-summary");
    let output = cli()
        .args(["signal", "progress", "--summary", "   ", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid-signal"));
    // Zero partial inbox file after failure — every lifecycle bucket empty.
    let root = signals_root(&project);
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        assert_eq!(
            count_files(&root.join(bucket)),
            0,
            "bucket {bucket} must be empty"
        );
    }
}

#[test]
fn invalid_signal_json_mode_is_exit_3() {
    let project = temp_project("invalid-json-mode");
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "   ",
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).expect("valid JSON output");
    assert_eq!(value["status"], "error");
    assert_eq!(value["category"], "invalid-signal");
}

#[test]
fn missing_summary_usage_error_creates_nothing_and_has_no_json_guarantee() {
    let project = temp_project("usage-error");
    let output = cli()
        .args(["signal", "progress", "--json", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    // Pre-parse usage errors are clap-owned: plain stderr text, no JSON
    // guarantee, even though --json was passed (approved plan §9).
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(serde_json::from_str::<serde_json::Value>(stdout.trim()).is_err());
    assert_eq!(count_files(&pending_dir(&project)), 0);
}

/// Desktop-independence proof (§19.G): the CLI binary, invoked as a
/// genuinely separate process, writes successfully with zero coupling to
/// any Tauri/Desktop process — no IPC, no socket, no shared in-memory
/// state, so its success cannot depend on whether Desktop happens to be
/// running. This is what §19.G means by "already implicitly true of every
/// Rust cargo test run" — proving architectural independence, not gating
/// on Desktop's incidental state on whichever machine runs this suite (this
/// exact dev machine, e.g., already has OpenMesh Desktop open right now).
/// The mandatory *evidenced* Desktop-closed write, checked live around one
/// specific invocation, is the real dogfood's job (Checkpoint E, §12/§13).
#[test]
fn write_succeeds_as_a_standalone_process_independent_of_desktop() {
    let project = temp_project("desktop-independent");
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "written with desktop closed",
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(count_files(&pending_dir(&project)), 1);
}
