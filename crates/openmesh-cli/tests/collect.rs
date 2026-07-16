//! Dev Track 0.1.3.6 Checkpoint E — collect command e2e tests.

use openmesh_core::storage::init_project;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-collect-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn pending_dir(project: &Path) -> PathBuf {
    project.join(".openmesh/signals/pending")
}

fn pending_count(project: &Path) -> usize {
    let dir = pending_dir(project);
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir).map(|e| e.count()).unwrap_or(0)
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn init_git(project: &Path) {
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["init"])
        .status()
        .expect("git init");
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["config", "user.email", "collect@test.openmesh"])
        .status()
        .expect("email");
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["config", "user.name", "Collect Test"])
        .status()
        .expect("name");
    fs::write(project.join("README.md"), "v1\n").unwrap();
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["add", "README.md"])
        .status()
        .unwrap();
    Command::new("git")
        .arg("-C")
        .arg(project)
        .args(["commit", "-m", "initial"])
        .status()
        .unwrap();
}

#[test]
fn collect_git_parser_rejects_unknown_subcommand() {
    let output = cli()
        .args(["collect", "unknown", "--project", "/tmp/x"])
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn collect_git_end_to_end_writes_pending_signal() {
    if !git_available() {
        eprintln!("SKIP: git unavailable");
        return;
    }
    let project = temp_project("git-e2e");
    init_git(&project);
    let output = cli()
        .args(["collect", "git", "--project"])
        .arg(&project)
        .arg("--correlation-hint")
        .arg("collect-git-e2e")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(pending_count(&project), 1);
    let pending = fs::read_dir(pending_dir(&project))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let json = fs::read_to_string(pending).unwrap();
    assert!(json.contains("protocolVersion") && json.contains("1.1"));
    assert!(json.contains("git-state"));
    assert!(json.contains("collect-git-e2e"));
}

#[test]
fn collect_heli_end_to_end_writes_pending_signal() {
    let project = temp_project("heli-e2e");
    let state = project.join(".heli-harness/state");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("current-task.md"), "Collect heli e2e\n").unwrap();
    let output = cli()
        .args(["collect", "heli", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(pending_count(&project), 1);
}

#[test]
fn collect_heli_absent_exits_zero_without_pending_write() {
    let project = temp_project("heli-absent");
    let output = cli()
        .args(["collect", "heli", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("heli: absent"));
    assert_eq!(pending_count(&project), 0);
}

#[test]
fn collect_git_non_git_project_returns_non_zero() {
    if !git_available() {
        return;
    }
    let project = temp_project("non-git");
    let output = cli()
        .args(["collect", "git", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(pending_count(&project), 0);
}

#[test]
fn collect_json_output_includes_signal_id() {
    let project = temp_project("json");
    let state = project.join(".heli-harness/state");
    fs::create_dir_all(&state).unwrap();
    fs::write(state.join("current-task.md"), "json mode\n").unwrap();
    let output = cli()
        .args(["collect", "heli", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(parsed["status"], "ok");
    assert!(parsed["signalId"].is_string());
}
