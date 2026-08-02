//! Dev Track 0.1.10 Checkpoint D — mesh import CLI workflow (two projects).

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
        "openmesh-cli-mesh-import-{label}-{}-{n}",
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
fn export_then_import_across_two_projects() {
    let ter = temp_project("ter");
    let yo = temp_project("yo");
    let ter_ws = workspace_id(&ter);

    append_event(
        &ter.to_string_lossy(),
        &WorkEvent::new(
            "evt-ter-1",
            &ter_ws,
            "work.completed",
            "Ready for mesh handoff",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("README.md".into()),
                observed_at: None,
            }],
            "2026-08-02T12:00:00Z",
        ),
    )
    .expect("event");

    let peer = run(
        &["mesh", "peer", "add", "--label", "Yo", "--id", "yo", "--json"],
        &ter,
    );
    assert!(peer.status.success());

    let export = run(
        &[
            "mesh",
            "export",
            "--peer",
            "yo",
            "--envelope-id",
            "env-ab-1",
            "--since",
            "2026-08-01T00:00:00Z",
            "--json",
        ],
        &ter,
    );
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let outbox = ter.join(".openmesh/mesh/outbox/env-ab-1.json");
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
    assert!(
        import.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&import.stderr),
        String::from_utf8_lossy(&import.stdout)
    );
    let env: Value = serde_json::from_slice(&import.stdout).expect("json");
    assert_eq!(env["envelopeId"], "env-ab-1");
    assert!(yo.join(".openmesh/mesh/inbox/env-ab-1.json").exists());
}

#[test]
fn mesh_import_help_present() {
    let out = {
        let mut cmd = cli();
        cmd.args(["mesh", "--help"]);
        cmd.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    assert!(help.contains("import"));
}
