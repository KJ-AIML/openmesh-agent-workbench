//! Dev Track 0.1.16 — team cloud CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-team-cloud-{label}-{}-{n}",
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
fn team_cloud_init_show_sync_scaffold() {
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

    let init = run(
        &["team", "cloud", "init", "--mode", "local-sim", "--json"],
        &p,
    );
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let cfg: Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(cfg["selectiveSync"], true);
    assert_eq!(cfg["mode"], "local-sim");

    let show = run(&["team", "cloud", "show", "--json"], &p);
    assert!(show.status.success());

    let sync = run(&["team", "cloud", "sync-scaffold", "--json"], &p);
    assert!(
        sync.status.success(),
        "{}",
        String::from_utf8_lossy(&sync.stderr)
    );
    let plan: Value = serde_json::from_slice(&sync.stdout).unwrap();
    assert_eq!(plan["scaffoldOnly"], true);
    assert!(p.join(".openmesh/team-cloud/config.json").exists());
}

#[test]
fn team_cloud_requires_team_first() {
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
    let init = run(&["team", "cloud", "init", "--json"], &p);
    assert!(!init.status.success());
}

#[test]
fn team_cloud_help() {
    let out = {
        let mut c = cli();
        c.args(["team", "cloud", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["init", "show", "sync-scaffold"] {
        assert!(help.contains(cmd), "missing {cmd} in {help}");
    }
}
