//! Dev Track 0.1.10 Checkpoint C — mesh export CLI workflow.

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
        "openmesh-cli-mesh-export-{label}-{}-{n}",
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
fn mesh_export_help_present() {
    let help = String::from_utf8_lossy(&{
        let mut cmd = cli();
        cmd.args(["mesh", "--help"]);
        cmd.output().unwrap().stdout
    })
    .to_ascii_lowercase();
    assert!(help.contains("export"));
    assert!(help.contains("peer"));
}

#[test]
fn mesh_export_cli_writes_outbox_json() {
    let project = temp_project("cli-export");
    let ws = workspace_id(&project);
    append_event(
        &project.to_string_lossy(),
        &WorkEvent::new(
            "evt-cli-mesh",
            &ws,
            "work.completed",
            "Mesh export ready",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("docs/development/openmesh-0.1.10-execution-plan.md".into()),
                observed_at: None,
            }],
            "2026-08-02T12:00:00Z",
        ),
    )
    .expect("event");

    let peer = run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &project,
    );
    assert!(peer.status.success(), "{}", String::from_utf8_lossy(&peer.stderr));

    let export = run(
        &[
            "mesh",
            "export",
            "--peer",
            "yo",
            "--envelope-id",
            "env-cli-1",
            "--since",
            "2026-08-01T00:00:00Z",
            "--json",
        ],
        &project,
    );
    assert!(
        export.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&export.stderr),
        String::from_utf8_lossy(&export.stdout)
    );
    let env: Value = serde_json::from_slice(&export.stdout).expect("json");
    assert_eq!(env["envelopeId"], "env-cli-1");
    assert_eq!(env["toPeer"]["label"], "Yo");
    assert!(project
        .join(".openmesh/mesh/outbox/env-cli-1.json")
        .exists());
}
