//! Dev Track 0.1.12 — online-proxy CLI workflow.

use openmesh_core::domain::{
    default_work_proxy_profile, deterministic_work_proxy_profile_id, EvidenceAttachment,
    EvidenceRef, WorkEvent,
};
use openmesh_core::events::append_event;
use openmesh_core::profile::write_work_proxy_profile;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-online-proxy-{label}-{}-{n}",
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

fn workspace_id(project: &Path) -> String {
    let raw = std::fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

fn seed_profile_and_event(project: &Path) {
    let ws = workspace_id(project);
    let path = project.to_string_lossy().to_string();
    let profile = default_work_proxy_profile(
        &ws,
        deterministic_work_proxy_profile_id(&ws),
        "Ter",
        "Owner",
        "2026-08-02T18:00:00Z",
    );
    write_work_proxy_profile(&path, &profile).expect("profile");
    append_event(
        &path,
        &WorkEvent::new(
            "evt-online-1",
            &ws,
            "work.completed",
            "Online proxy alpha ready",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("README.md".into()),
                observed_at: None,
            }],
            "2026-08-02T17:00:00Z",
        ),
    )
    .unwrap();
}

#[test]
fn online_proxy_init_status_ask_show() {
    let project = temp_project("flow");
    seed_profile_and_event(&project);

    let init = run(
        &["online-proxy", "init", "--owner-label", "Ter", "--json"],
        &project,
    );
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
    assert!(project.join(".openmesh/online-proxy/config.json").exists());

    let status = run(&["online-proxy", "status", "--json"], &project);
    assert!(status.status.success());
    let cfg: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(cfg["ownerLabel"], "Ter");

    let ask = run(
        &[
            "online-proxy",
            "ask",
            "--question",
            "What is the status?",
            "--tier",
            "low-impact",
            "--answer-id",
            "ans-test-1",
            "--json",
        ],
        &project,
    );
    let ask_stdout = String::from_utf8_lossy(&ask.stdout);
    let ask_stderr = String::from_utf8_lossy(&ask.stderr);
    if ask.status.success() {
        let ans: Value = serde_json::from_slice(&ask.stdout).unwrap();
        assert_eq!(ans["answerId"], "ans-test-1");
        assert_eq!(ans["liveEngine"], true);
        let statement = ans["freshness"]["statement"].as_str().unwrap_or("");
        assert!(
            statement.to_ascii_lowercase().contains("fresh")
                || statement.to_ascii_lowercase().contains("stale")
                || statement.to_ascii_lowercase().contains("age"),
            "missing freshness disclosure: {statement}"
        );
        let show = run(
            &["online-proxy", "show", "--id", "ans-test-1", "--json"],
            &project,
        );
        assert!(show.status.success());
    } else {
        // Without Agent Engine API key, live ask must fail closed (no scaffold theater).
        let combined = format!("{ask_stdout}{ask_stderr}");
        assert!(
            combined.contains("API key") || combined.contains("missing_api_key"),
            "expected missing API key error, got stdout={ask_stdout} stderr={ask_stderr}"
        );
    }
}

#[test]
fn online_proxy_help() {
    let out = {
        let mut c = cli();
        c.args(["online-proxy", "--help"]);
        c.output().unwrap()
    };
    let help = String::from_utf8_lossy(&out.stdout).to_ascii_lowercase();
    for cmd in ["init", "status", "ask", "show"] {
        assert!(help.contains(cmd), "missing {cmd}");
    }
}
