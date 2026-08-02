//! Dev Track 0.1.10 Checkpoint B — mesh peer CLI workflow tests.

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-mesh-peer-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn run(args: &[&str], project: &Path) -> std::process::Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--project").arg(project);
    cmd.output().expect("spawn")
}

fn run_raw(args: &[&str]) -> std::process::Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("spawn")
}

#[test]
fn top_level_help_lists_mesh() {
    let help = String::from_utf8_lossy(&run_raw(&["--help"]).stdout).to_ascii_lowercase();
    assert!(help.contains("mesh"));
}

#[test]
fn mesh_peer_help_lists_add_list_show() {
    let help = String::from_utf8_lossy(&run_raw(&["mesh", "peer", "--help"]).stdout)
        .to_ascii_lowercase();
    for sub in ["add", "list", "show"] {
        assert!(help.contains(sub), "missing {sub}");
    }
}

#[test]
fn peer_add_list_show_json_workflow() {
    let project = temp_project("workflow");
    let add = run(
        &[
            "mesh",
            "peer",
            "add",
            "--label",
            "Yo",
            "--id",
            "yo",
            "--workspace-id",
            "ws-yo",
            "--json",
        ],
        &project,
    );
    assert!(
        add.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&add.stderr)
    );
    let added: Value = serde_json::from_slice(&add.stdout).expect("json");
    assert_eq!(added["peerId"], "yo");
    assert_eq!(added["label"], "Yo");
    assert!(project
        .join(".openmesh/mesh/peers/yo.json")
        .exists());

    let list = run(&["mesh", "peer", "list", "--json"], &project);
    assert!(list.status.success());
    let peers: Value = serde_json::from_slice(&list.stdout).expect("list json");
    assert_eq!(peers.as_array().unwrap().len(), 1);

    let show = run(&["mesh", "peer", "show", "--id", "yo", "--json"], &project);
    assert!(show.status.success());
    let peer: Value = serde_json::from_slice(&show.stdout).expect("show json");
    assert_eq!(peer["remoteWorkspaceId"], "ws-yo");
}

#[test]
fn peer_add_duplicate_is_conflict() {
    let project = temp_project("dup");
    let first = run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &project,
    );
    assert!(first.status.success());
    let second = run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &project,
    );
    assert_eq!(second.status.code(), Some(3));
    let err: Value = serde_json::from_slice(&second.stdout).expect("err json");
    assert_eq!(err["status"], "error");
    assert_eq!(err["category"], "conflict");
}
