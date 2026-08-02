//! Dev Track 0.1.15 — team CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-team-{label}-{}-{n}",
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
fn team_init_member_add_list_show() {
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
        &p
    )
    .status
    .success());

    let init = run(
        &["team", "init", "--name", "Lab Team", "--owner-label", "Ter", "--json"],
        &p,
    );
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let ws: Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(ws["displayName"], "Lab Team");
    assert!(ws["members"].as_array().unwrap().len() >= 1);

    assert!(run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &p
    )
    .status
    .success());

    let add = run(
        &[
            "team",
            "member",
            "add",
            "--label",
            "Yo",
            "--id",
            "m-yo",
            "--peer",
            "yo",
            "--role",
            "member",
            "--json",
        ],
        &p,
    );
    assert!(add.status.success(), "{}", String::from_utf8_lossy(&add.stderr));

    let list = run(&["team", "member", "list", "--json"], &p);
    assert!(list.status.success());
    let members: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(members.as_array().unwrap().len(), 2);

    let show = run(&["team", "show", "--json"], &p);
    assert!(show.status.success());
    assert!(p.join(".openmesh/team/workspace.json").exists());
}

#[test]
fn team_help() {
    let out = {
        let mut c = cli();
        c.args(["team", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["init", "show", "member", "query"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
