//! Dev Track 0.1.14 — Ter × Yo mesh query E2E (offline peer ask, read-only).

use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-ter-yo-{label}-{}-{n}",
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
fn ter_asks_yo_offline_proxy_read_only() {
    let ter = temp_project("ter");
    let yo = temp_project("yo");

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
        &ter
    )
    .status
    .success());
    assert!(run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Yo",
            "--role-label",
            "Collaborator",
            "--json",
        ],
        &yo
    )
    .status
    .success());

    assert!(run(
        &["signal", "progress", "--summary", "Yo finished offline work", "--json"],
        &yo
    )
    .status
    .success());
    assert!(run(
        &["signal", "decision", "--summary", "Ship mesh query alpha", "--json"],
        &yo
    )
    .status
    .success());
    assert!(run(&["state", "--rebuild", "--json"], &yo).status.success());

    assert!(run(
        &["mesh", "peer", "add", "--label", "Ter", "--id", "ter", "--json"],
        &yo
    )
    .status
    .success());
    assert!(run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &ter
    )
    .status
    .success());

    let export = run(
        &[
            "mesh",
            "export",
            "--peer",
            "ter",
            "--envelope-id",
            "env-yo-teryo-1",
            "--since",
            "2026-07-01T00:00:00Z",
            "--json",
        ],
        &yo,
    );
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let outbox = yo.join(".openmesh/mesh/outbox/env-yo-teryo-1.json");
    assert!(outbox.exists());

    let import = run(
        &[
            "mesh",
            "import",
            "--file",
            outbox.to_str().unwrap(),
            "--register-peer",
            "--json",
        ],
        &ter,
    );
    assert!(
        import.status.success(),
        "{}",
        String::from_utf8_lossy(&import.stderr)
    );

    // Yo is offline — Ter queries Yo's proxy from imported evidence only.
    let query = run(
        &[
            "mesh",
            "query",
            "--peer",
            "yo",
            "--question",
            "What did Yo finish while offline?",
            "--tier",
            "low-impact",
            "--query-id",
            "mq-teryo-1",
            "--json",
        ],
        &ter,
    );
    assert!(
        query.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&query.stderr),
        String::from_utf8_lossy(&query.stdout)
    );
    let ans: Value = serde_json::from_slice(&query.stdout).unwrap();
    assert_eq!(ans["queryId"], "mq-teryo-1");
    assert_eq!(ans["readOnly"], true);
    assert_eq!(ans["refused"], false);
    assert_eq!(ans["peerLabel"], "Yo");
    let statement = ans["freshness"]["statement"].as_str().unwrap_or("");
    assert!(
        statement.to_ascii_lowercase().contains("fresh")
            || statement.to_ascii_lowercase().contains("stale")
            || statement.to_ascii_lowercase().contains("read-only"),
        "missing freshness: {statement}"
    );
    let text = ans["answerText"].as_str().unwrap_or("");
    assert!(
        text.contains("read-only") || text.contains("Remote peer"),
        "answer should declare remote read-only: {text}"
    );
    assert!(
        ans["envelopeIds"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "expected envelope ids"
    );

    // Ledger must not auto-merge foreign evidence as local WorkEvents.
    let events_dir = ter.join(".openmesh/events/ledger");
    if events_dir.exists() {
        let count = std::fs::read_dir(&events_dir).unwrap().count();
        assert_eq!(count, 0, "remote query must not write work events");
    }
    assert!(ter
        .join(".openmesh/mesh/queries/mq-teryo-1.json")
        .exists());
}

#[test]
fn mesh_query_refuses_without_peer_envelopes() {
    let ter = temp_project("ter-empty");
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
        &ter
    )
    .status
    .success());
    assert!(run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &ter
    )
    .status
    .success());

    let query = run(
        &[
            "mesh",
            "query",
            "--peer",
            "yo",
            "--question",
            "Anything?",
            "--tier",
            "standard",
            "--json",
        ],
        &ter,
    );
    assert!(query.status.success());
    let ans: Value = serde_json::from_slice(&query.stdout).unwrap();
    assert_eq!(ans["refused"], true);
    assert_eq!(ans["readOnly"], true);
}

#[test]
fn mesh_query_help() {
    let out = {
        let mut c = cli();
        c.args(["mesh", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    assert!(help.contains("query"), "mesh help missing query");
}
