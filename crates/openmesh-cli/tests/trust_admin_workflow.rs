//! Dev Track 0.1.17 — trust-admin CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-trust-{label}-{}-{n}",
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

fn bootstrap_team(p: &Path) {
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
        &["team", "init", "--name", "Lab", "--owner-label", "Ter", "--json"],
        p,
    )
    .status
    .success());
}

#[test]
fn trust_admin_init_mode_allowlist_audit() {
    let p = temp_project("flow");
    bootstrap_team(&p);

    let init = run(&["trust-admin", "init", "--json"], &p);
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let policy: Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(policy["secretTopicsFailClosed"], true);
    assert_eq!(policy["allowSecretExport"], false);
    assert_eq!(policy["syncRequireSelective"], true);

    assert!(run(
        &[
            "trust-admin",
            "set-query-mode",
            "--mode",
            "allowlist-only",
            "--json",
        ],
        &p,
    )
    .status
    .success());

    assert!(run(
        &[
            "trust-admin",
            "allowlist",
            "add",
            "--member-id",
            "m-yo",
            "--peer",
            "yo",
            "--json",
        ],
        &p,
    )
    .status
    .success());

    let list = run(&["trust-admin", "allowlist", "list", "--json"], &p);
    assert!(list.status.success());
    let entries: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(entries.as_array().unwrap().len(), 1);

    let audit = run(&["trust-admin", "audit", "--json"], &p);
    assert!(audit.status.success());
    let events: Value = serde_json::from_slice(&audit.stdout).unwrap();
    assert!(events.as_array().unwrap().len() >= 2);

    assert!(p.join(".openmesh/trust-admin/policy.json").exists());
    assert!(p.join(".openmesh/trust-admin/audit.jsonl").exists());
}

#[test]
fn trust_admin_requires_team() {
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
    let init = run(&["trust-admin", "init", "--json"], &p);
    assert!(!init.status.success());
}

#[test]
fn trust_admin_help() {
    let out = {
        let mut c = cli();
        c.args(["trust-admin", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["init", "show", "set-query-mode", "allowlist", "audit"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
