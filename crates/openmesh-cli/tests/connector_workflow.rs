//! Dev Track 0.1.18 — connector CLI workflow.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-connector-{label}-{}-{n}",
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
fn connector_register_list_collect() {
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

    let reg = run(
        &[
            "connector",
            "register",
            "--id",
            "gh-lab",
            "--name",
            "Lab GH",
            "--kind",
            "github-stub",
            "--ref",
            "acme/lab",
            "--json",
        ],
        &p,
    );
    assert!(
        reg.status.success(),
        "{}",
        String::from_utf8_lossy(&reg.stderr)
    );
    let d: Value = serde_json::from_slice(&reg.stdout).unwrap();
    assert_eq!(d["role"], "evidence-producer-only");
    assert_eq!(d["kind"], "github-stub");

    let list = run(&["connector", "list", "--json"], &p);
    assert!(list.status.success());
    let arr: Value = serde_json::from_slice(&list.stdout).unwrap();
    assert_eq!(arr.as_array().unwrap().len(), 1);

    let collect = run(&["connector", "collect", "--id", "gh-lab", "--json"], &p);
    assert!(
        collect.status.success(),
        "{}",
        String::from_utf8_lossy(&collect.stderr)
    );
    let run_v: Value = serde_json::from_slice(&collect.stdout).unwrap();
    assert_eq!(run_v["evidenceOnly"], true);
    assert!(run_v["items"].as_array().unwrap().len() >= 1);
    assert!(p.join(".openmesh/connectors/registry.json").exists());
}

#[test]
fn connector_help() {
    let out = {
        let mut c = cli();
        c.args(["connector", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["register", "list", "show", "collect"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
