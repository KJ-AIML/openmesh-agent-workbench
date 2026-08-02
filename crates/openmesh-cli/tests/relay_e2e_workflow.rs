//! Dev Track 0.1.11 Checkpoint F — relay pack→approve→send→receive E2E.

use openmesh_core::domain::{EvidenceAttachment, EvidenceRef, WorkEvent};
use openmesh_core::events::append_event;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-relay-e2e-{label}-{}-{n}",
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
    for a in args {
        cmd.arg(a);
    }
    cmd.arg("--project").arg(project);
    cmd.output().unwrap()
}

fn workspace_id(project: &Path) -> String {
    let raw = std::fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[test]
fn relay_pack_approve_send_receive_audit() {
    let a = temp_project("sender");
    let b = temp_project("receiver");
    let ws = workspace_id(&a);
    append_event(
        &a.to_string_lossy(),
        &WorkEvent::new(
            "evt-relay-1",
            &ws,
            "work.completed",
            "Relay alpha ready",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("README.md".into()),
                observed_at: None,
            }],
            "2026-08-02T12:00:00Z",
        ),
    )
    .unwrap();

    assert!(run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &a
    )
    .status
    .success());
    let export = run(
        &[
            "mesh",
            "export",
            "--peer",
            "yo",
            "--envelope-id",
            "env-relay-1",
            "--since",
            "2026-08-01T00:00:00Z",
            "--json",
        ],
        &a,
    );
    assert!(export.status.success(), "{}", String::from_utf8_lossy(&export.stderr));

    let pack = run(
        &[
            "relay",
            "pack",
            "--envelope-id",
            "env-relay-1",
            "--package-id",
            "pkg-relay-1",
            "--json",
        ],
        &a,
    );
    assert!(pack.status.success(), "{}", String::from_utf8_lossy(&pack.stderr));
    assert!(a.join(".openmesh/relay/staging/pkg-relay-1.json").exists());

    // Unapproved send should fail
    let relay_root = std::env::temp_dir().join(format!(
        "openmesh-relay-root-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let _ = std::fs::create_dir_all(&relay_root);
    let bad_send = run(
        &[
            "relay",
            "send",
            "--id",
            "pkg-relay-1",
            "--relay-root",
            relay_root.to_str().unwrap(),
            "--json",
        ],
        &a,
    );
    assert!(!bad_send.status.success());

    let approve = run(
        &["relay", "approve", "--id", "pkg-relay-1", "--by", "ter", "--json"],
        &a,
    );
    assert!(approve.status.success(), "{}", String::from_utf8_lossy(&approve.stderr));
    assert!(a.join(".openmesh/relay/approved/pkg-relay-1.json").exists());

    let send = run(
        &[
            "relay",
            "send",
            "--id",
            "pkg-relay-1",
            "--relay-root",
            relay_root.to_str().unwrap(),
            "--json",
        ],
        &a,
    );
    assert!(send.status.success(), "{}", String::from_utf8_lossy(&send.stderr));
    assert!(relay_root.join("drop/pkg-relay-1.json").exists());
    assert!(a.join(".openmesh/relay/sent/pkg-relay-1.json").exists());

    let recv = run(
        &[
            "relay",
            "receive",
            "--id",
            "pkg-relay-1",
            "--relay-root",
            relay_root.to_str().unwrap(),
            "--json",
        ],
        &b,
    );
    assert!(recv.status.success(), "{}", String::from_utf8_lossy(&recv.stderr));
    assert!(b.join(".openmesh/relay/received/pkg-relay-1.json").exists());

    let audit = run(&["relay", "audit", "--json"], &a);
    assert!(audit.status.success());
    let events: Value = serde_json::from_slice(&audit.stdout).unwrap();
    let kinds: Vec<&str> = events
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"approved"));
    assert!(kinds.contains(&"sent"));
}

#[test]
fn relay_help_lists_commands() {
    let out = {
        let mut c = cli();
        c.args(["relay", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["pack", "approve", "send", "receive", "audit", "show"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
