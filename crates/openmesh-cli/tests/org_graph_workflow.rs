//! Dev Track 0.1.19 — org graph CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-org-{label}-{}-{n}",
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
fn org_graph_show_after_team_init() {
    let p = temp_project("flow");
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
        &["team", "init", "--name", "Lab", "--owner-label", "Ter", "--json"],
        &p,
    )
    .status
    .success());

    let out = run(&["org", "graph", "show", "--json"], &p);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let g: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(g["nodes"].as_array().unwrap().len() >= 2);
    assert!(g["edges"].as_array().unwrap().len() >= 1);
}

#[test]
fn org_graph_requires_team() {
    let p = temp_project("no-team");
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
    let out = run(&["org", "graph", "show", "--json"], &p);
    assert!(!out.status.success());
}

#[test]
fn org_help() {
    let out = {
        let mut c = cli();
        c.args(["org", "graph", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    assert!(help.contains("show"));
}
