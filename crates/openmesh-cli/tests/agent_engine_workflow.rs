//! CLI workflow tests for Agent Engine (0.1.23) — no live network.

use openmesh_core::storage::init_project;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-agent-{label}-{}-{n}",
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

fn run(args: &[&str], project: Option<&Path>, env: &[(&str, &str)], clear: &[&str]) -> std::process::Output {
    let mut cmd = cli();
    for a in args {
        cmd.arg(a);
    }
    if let Some(p) = project {
        cmd.arg("--project").arg(p);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    for k in clear {
        cmd.env_remove(k);
    }
    cmd.output().expect("run cli")
}

#[test]
fn agent_help_lists_ask() {
    let out = cli().args(["agent", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("ask"));
    assert!(stdout.contains("secret-status"));
}

#[test]
fn agent_secret_status_json() {
    let out = cli()
        .args(["agent", "secret-status", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success());
    assert!(stdout.contains("configured"));
}

#[test]
fn agent_ask_without_key_fails_closed() {
    let proj = temp_project("nokey");
    let fake_home = proj.join("_home");
    std::fs::create_dir_all(&fake_home).unwrap();
    let out = run(
        &["agent", "ask", "--question", "hello"],
        Some(&proj),
        &[
            ("HOME", fake_home.to_str().unwrap()),
            (
                "XDG_CONFIG_HOME",
                fake_home.join("config").to_str().unwrap(),
            ),
        ],
        &[
            "OPENMESH_AGENT_API_KEY",
            "OPENAI_API_KEY",
            "DEEPSEEK_API_KEY",
        ],
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "stderr={stderr}");
    assert!(stderr.contains("API key"), "stderr={stderr}");
}
