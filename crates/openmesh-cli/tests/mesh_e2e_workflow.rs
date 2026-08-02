//! Dev Track 0.1.10 Checkpoint F — full two-person mesh E2E via CLI.

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
        "openmesh-cli-mesh-e2e-{label}-{}-{n}",
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

fn workspace_id(project: &Path) -> String {
    let raw = std::fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[test]
fn two_person_mesh_peer_export_import_list_show() {
    let ter = temp_project("ter");
    let yo = temp_project("yo");
    let ter_ws = workspace_id(&ter);

    append_event(
        &ter.to_string_lossy(),
        &WorkEvent::new(
            "evt-e2e-1",
            &ter_ws,
            "work.completed",
            "Two-person mesh prototype ready",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("crates/openmesh-core/src/mesh/mod.rs".into()),
                observed_at: None,
            }],
            "2026-08-02T12:00:00Z",
        ),
    )
    .expect("event");

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
            "yo",
            "--envelope-id",
            "env-e2e-1",
            "--since",
            "2026-08-01T00:00:00Z",
            "--json",
        ],
        &ter,
    );
    assert!(export.status.success(), "{}", String::from_utf8_lossy(&export.stderr));
    let outbox = ter.join(".openmesh/mesh/outbox/env-e2e-1.json");
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
        &yo,
    );
    assert!(import.status.success(), "{}", String::from_utf8_lossy(&import.stderr));
    assert!(yo.join(".openmesh/mesh/inbox/env-e2e-1.json").exists());

    let list = run(&["mesh", "list", "--mailbox", "inbox", "--json"], &yo);
    assert!(list.status.success());
    let rows: Value = serde_json::from_slice(&list.stdout).unwrap();
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["envelopeId"], "env-e2e-1");
    assert_eq!(arr[0]["attributedTo"], "local"); // Ter's default from-label without profile

    let show = run(
        &["mesh", "show", "--id", "env-e2e-1", "--mailbox", "inbox", "--json"],
        &yo,
    );
    assert!(show.status.success());
    let env: Value = serde_json::from_slice(&show.stdout).unwrap();
    assert_eq!(env["envelopeId"], "env-e2e-1");
    assert_eq!(env["mailbox"], "inbox");
    assert!(
        env["evidenceItems"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false)
            || env["limitations"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false)
    );

    // Exporter can list outbox
    let out_list = run(&["mesh", "list", "--mailbox", "outbox", "--json"], &ter);
    assert!(out_list.status.success());
    let out_rows: Value = serde_json::from_slice(&out_list.stdout).unwrap();
    assert_eq!(out_rows.as_array().unwrap().len(), 1);
}
