//! Dev Track 0.1.22 — LAN serve / send / ask workflow (loopback HTTP).

use openmesh_core::domain::{
    default_work_proxy_profile, deterministic_work_proxy_profile_id, EvidenceAttachment,
    EvidenceRef, WorkEvent,
};
use openmesh_core::events::append_event;
use openmesh_core::profile::write_work_proxy_profile;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-lan-{label}-{}-{n}",
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

fn seed_profile(project: &Path, owner: &str) {
    let ws = workspace_id(project);
    let path = project.to_string_lossy().to_string();
    let profile = default_work_proxy_profile(
        &ws,
        deterministic_work_proxy_profile_id(&ws),
        owner,
        "Owner",
        "2026-08-03T10:00:00Z",
    );
    write_work_proxy_profile(&path, &profile).expect("profile");
    append_event(
        &path,
        &WorkEvent::new(
            "evt-lan-1",
            &ws,
            "work.completed",
            "LAN relay alpha ready",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("README.md".into()),
                observed_at: None,
            }],
            "2026-08-03T09:00:00Z",
        ),
    )
    .unwrap();
}

fn spawn_serve(project: &Path, seconds: u64) -> Child {
    cli()
        .args([
            "lan",
            "serve",
            "--host",
            "127.0.0.1",
            "--http-port",
            "0",
            "--seconds",
            &seconds.to_string(),
            "--json",
            "--project",
        ])
        .arg(project)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn lan serve")
}

fn wait_for_serve_port(child: &mut Child) -> u16 {
    let stdout = child.stdout.take().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read serve json");
    let v: Value = serde_json::from_str(line.trim()).unwrap_or_else(|e| {
        panic!("serve json parse failed: {e}; line={line:?}");
    });
    let port = v["httpPort"].as_u64().expect("httpPort") as u16;
    assert!(port > 0, "expected ephemeral port > 0");
    thread::sleep(Duration::from_millis(120));
    port
}

#[test]
fn lan_serve_send_ask_status_loopback() {
    let server = temp_project("server");
    let client = temp_project("client");
    seed_profile(&server, "ServerOwner");
    seed_profile(&client, "ClientOwner");

    let mut child = spawn_serve(&server, 25);
    let port = wait_for_serve_port(&mut child);
    let addr = format!("127.0.0.1:{port}");

    let status = run(&["lan", "status", "--json"], &server);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let st: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(st["running"], true);

    assert!(run(
        &[
            "mesh",
            "peer",
            "add",
            "--label",
            "Server",
            "--id",
            "server",
            "--json"
        ],
        &client
    )
    .status
    .success());
    let export = run(
        &[
            "mesh",
            "export",
            "--peer",
            "server",
            "--envelope-id",
            "env-lan-cli-1",
            "--since",
            "2026-08-01T00:00:00Z",
            "--json",
        ],
        &client,
    );
    assert!(
        export.status.success(),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    let pack = run(
        &[
            "relay",
            "pack",
            "--envelope-id",
            "env-lan-cli-1",
            "--package-id",
            "pkg-lan-cli-1",
            "--json",
        ],
        &client,
    );
    assert!(
        pack.status.success(),
        "{}",
        String::from_utf8_lossy(&pack.stderr)
    );
    let approve = run(
        &["relay", "approve", "--id", "pkg-lan-cli-1", "--json"],
        &client,
    );
    assert!(
        approve.status.success(),
        "{}",
        String::from_utf8_lossy(&approve.stderr)
    );

    let send = run(
        &[
            "lan",
            "send",
            "--id",
            "pkg-lan-cli-1",
            "--to",
            &addr,
            "--json",
        ],
        &client,
    );
    assert!(
        send.status.success(),
        "send failed stdout={} stderr={}",
        String::from_utf8_lossy(&send.stdout),
        String::from_utf8_lossy(&send.stderr)
    );
    assert!(server
        .join(".openmesh/relay/received/pkg-lan-cli-1.json")
        .exists());

    let ask = run(
        &[
            "lan",
            "ask",
            "--to",
            &addr,
            "--question",
            "What is in progress?",
            "--tier",
            "low-impact",
            "--json",
        ],
        &client,
    );
    let ask_stdout = String::from_utf8_lossy(&ask.stdout);
    let ask_stderr = String::from_utf8_lossy(&ask.stderr);
    if ask.status.success() {
        // Host has a configured Agent Engine key — live answer path.
        let answer: Value = serde_json::from_slice(&ask.stdout).unwrap();
        assert_eq!(answer["readOnly"], true);
        assert!(answer["answerText"].as_str().unwrap().len() > 10);
        assert!(
            !answer["answerText"]
                .as_str()
                .unwrap_or("")
                .contains("local-scaffold"),
            "must not return LocalScaffold paste"
        );
    } else {
        // Dogfood default in CI: peer has no API key → structured fail-closed error.
        let combined = format!("{ask_stdout}{ask_stderr}");
        assert!(
            combined.contains("missing_api_key")
                || combined.contains("API key")
                || combined.contains("503"),
            "expected missing_api_key error, got stdout={ask_stdout} stderr={ask_stderr}"
        );
    }

    let discover = run(&["lan", "discover", "--seconds", "1", "--json"], &client);
    assert!(
        discover.status.success(),
        "{}",
        String::from_utf8_lossy(&discover.stderr)
    );

    let _ = child.wait();
}
