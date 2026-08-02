//! Dev Track 0.1.20 — pilot CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-pilot-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).unwrap();
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn run(args: &[&str], project: &Path) -> std::process::Output {
    let mut cmd = cli();
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--project").arg(project);
    cmd.output().unwrap()
}

#[test]
fn pilot_check_before_team_not_ready() {
    let p = temp_project("bare");
    assert!(run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Ter",
            "--role-label",
            "Owner",
            "--json",
        ],
        &p,
    )
    .status
    .success());
    let out = run(&["pilot", "check", "--json"], &p);
    // exit 2 when not pilot_ready
    assert_eq!(out.status.code(), Some(2), "{}", String::from_utf8_lossy(&out.stderr));
    let pack: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(pack["pilotReady"], false);
    assert!(pack["failCount"].as_u64().unwrap() >= 1);
    assert!(p.join(".openmesh/pilot/pack.json").exists());
}

#[test]
fn pilot_check_with_team_and_trust_ready() {
    let p = temp_project("ready");
    assert!(run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Ter",
            "--role-label",
            "Owner",
            "--json",
        ],
        &p,
    )
    .status
    .success());
    assert!(run(
        &["team", "init", "--name", "Pilot", "--owner-label", "Ter", "--json"],
        &p,
    )
    .status
    .success());
    assert!(run(&["trust-admin", "init", "--json"], &p).status.success());
    let out = run(&["pilot", "check", "--json"], &p);
    assert!(
        out.status.success(),
        "code={:?} err={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let pack: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(pack["pilotReady"], true);
    assert_eq!(pack["failCount"], 0);
    assert!(pack["threatNotes"].as_array().unwrap().len() >= 1);
    assert!(pack["runbook"].as_array().unwrap().len() >= 1);
}

#[test]
fn pilot_help() {
    let out = {
        let mut c = cli();
        c.args(["pilot", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["check", "show", "runbook", "threats"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
