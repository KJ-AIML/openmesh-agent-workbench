//! Dev Track 0.1.21 — RC CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-rc-{label}-{}-{n}",
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

fn bootstrap_ready(p: &Path) {
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
        p,
    )
    .status
    .success());
    assert!(run(
        &["team", "init", "--name", "RC", "--owner-label", "Ter", "--json"],
        p,
    )
    .status
    .success());
    assert!(run(&["trust-admin", "init", "--json"], p).status.success());
}

#[test]
fn rc_check_not_ready_without_team() {
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
    let out = run(&["rc", "check", "--json"], &p);
    assert_eq!(out.status.code(), Some(2));
    let pack: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(pack["rcReady"], false);
    assert!(pack["p0FailCount"].as_u64().unwrap() >= 1);
}

#[test]
fn rc_check_ready_with_team_trust() {
    let p = temp_project("ready");
    bootstrap_ready(&p);
    let out = run(&["rc", "check", "--json"], &p);
    assert!(
        out.status.success(),
        "{:?}",
        String::from_utf8_lossy(&out.stderr)
    );
    let pack: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(pack["rcReady"], true);
    assert_eq!(pack["p0FailCount"], 0);
    assert_eq!(pack["p1FailCount"], 0);
    assert_eq!(pack["freezePolicy"]["featuresFrozen"], true);
    assert!(pack["regressionMatrix"].as_array().unwrap().len() >= 3);
    assert!(p.join(".openmesh/rc/pack.json").exists());
}

#[test]
fn rc_matrix_and_freeze() {
    let p = temp_project("matrix");
    bootstrap_ready(&p);
    let m = run(&["rc", "matrix", "--json"], &p);
    assert!(m.status.success());
    let f = run(&["rc", "freeze-policy", "--json"], &p);
    assert!(f.status.success());
    let pol: Value = serde_json::from_slice(&f.stdout).unwrap();
    assert_eq!(pol["featuresFrozen"], true);
}

#[test]
fn rc_help() {
    let out = {
        let mut c = cli();
        c.args(["rc", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["check", "show", "matrix", "freeze-policy"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
