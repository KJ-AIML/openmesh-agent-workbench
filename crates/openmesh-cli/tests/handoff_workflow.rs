//! Dev Track 0.1.8 Checkpoint F — handoff CLI workflow tests.

use openmesh_core::domain::{EvidenceAttachment, EvidenceRef, WorkEvent};
use openmesh_core::events::ledger_dir;
use openmesh_core::handoff::{handoff_dir, HANDOFF_DIR};
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::storage::{get_project_dir, init_project};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const ACTIVE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab";
const WORKTREE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab\repos\openmesh-agent-workbench";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-handoff-workflow-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
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
    cmd.output().expect("spawn cli")
}

fn run_raw(args: &[&str]) -> std::process::Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.output().expect("spawn cli")
}

fn workspace_id(project: &Path) -> String {
    fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id")
}

fn seed_event(project: &Path) {
    let project_path = project.to_string_lossy();
    let workspace_id = workspace_id(project);
    let event = WorkEvent::new(
        "evt-handoff-cli-seed",
        &workspace_id,
        "work.completed",
        "handoff cli seed",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-29T12:00:00Z",
    );
    openmesh_core::events::append_event(&project_path, &event).expect("append");
}

fn handoff_help() -> String {
    String::from_utf8_lossy(&run_raw(&["handoff", "--help"]).stdout).into_owned()
}

fn top_level_help() -> String {
    String::from_utf8_lossy(&run_raw(&["--help"]).stdout).into_owned()
}

#[test]
fn handoff_cli_exposes_create_show_approve_and_export() {
    let help = handoff_help().to_ascii_lowercase();
    for sub in ["create", "show", "approve", "export"] {
        assert!(help.contains(sub), "handoff --help should list `{sub}`");
    }
}

#[test]
fn top_level_handoff_is_distinct_from_signal_handoff_kind() {
    let top = top_level_help().to_ascii_lowercase();
    assert!(top.contains("handoff"));
    let signal_help =
        String::from_utf8_lossy(&run_raw(&["signal", "--help"]).stdout).to_ascii_lowercase();
    assert!(signal_help.contains("handoff"));
    assert!(top.contains("  handoff"));
    assert!(top.contains("signal"));
}

#[test]
fn create_rejects_empty_recipient() {
    let project = temp_project("empty-recipient");
    let output = run(&["handoff", "create", "--recipient", "   "], &project);
    assert_eq!(output.status.code(), Some(3));
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    assert!(combined.contains("recipient"));
}

#[test]
fn handoff_create_show_approve_export_workflow() {
    let project = temp_project("workflow");
    seed_event(&project);

    let create = run(
        &[
            "handoff",
            "create",
            "--recipient",
            "Alex",
            "--role",
            "teammate",
            "--json",
        ],
        &project,
    );
    assert!(
        create.status.success(),
        "create failed: stderr={}",
        String::from_utf8_lossy(&create.stderr)
    );
    let created: Value = serde_json::from_slice(&create.stdout).expect("create json");
    assert_eq!(created["status"], "ok");
    let handoff_id = created["handoffId"]
        .as_str()
        .expect("handoff id")
        .to_string();
    assert_eq!(created["handoffStatus"], "draft");

    let note_path = handoff_dir(&project.to_string_lossy()).join(format!("{handoff_id}.json"));
    assert!(note_path.exists());

    let show = run(
        &["handoff", "show", "--id", &handoff_id, "--json"],
        &project,
    );
    assert!(show.status.success());
    let shown: Value = serde_json::from_slice(&show.stdout).expect("show json");
    assert_eq!(shown["handoffId"], handoff_id);
    assert_eq!(shown["handoffStatus"], "draft");

    let note = &shown["note"];
    let section_keys = [
        "whatChanged",
        "whatIsComplete",
        "whatIsBlocked",
        "whatNeedsReview",
        "openQuestions",
        "safeToAnswerContext",
        "nextSuggestedStep",
    ];
    let has_section_items = section_keys.iter().any(|key| {
        note[*key]["items"]
            .as_array()
            .map(|items| !items.is_empty())
            .unwrap_or(false)
    });
    let has_limitations = note["limitations"]
        .as_array()
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    assert!(
        has_section_items || has_limitations,
        "handoff must include evidence-backed items or explicit limitations"
    );

    let approve = run(
        &[
            "handoff",
            "approve",
            "--id",
            &handoff_id,
            "--json",
            "--link-event",
        ],
        &project,
    );
    assert!(
        approve.status.success(),
        "approve failed: stderr={}",
        String::from_utf8_lossy(&approve.stderr)
    );
    let approved: Value = serde_json::from_slice(&approve.stdout).expect("approve json");
    assert_eq!(approved["handoffStatus"], "approved");
    assert!(approved["workEventId"].is_string());

    let export = run(&["handoff", "export", "--id", &handoff_id], &project);
    assert!(export.status.success());
    let markdown = String::from_utf8_lossy(&export.stdout);
    assert!(markdown.contains("# Handoff Note"));
    assert!(markdown.contains(&handoff_id));
    assert!(markdown.contains("Alex"));
}

#[test]
fn handoff_storage_does_not_touch_signal_pending_or_proxy_pending() {
    let project = temp_project("storage-boundary");
    let project_path = project.to_string_lossy();
    let signals_root = get_project_dir(&project_path).join("signals");
    let proxy_pending = get_project_dir(&project_path).join("proxy/pending");

    let before_signals = signals_root.exists();
    let before_proxy = proxy_pending.exists();

    let output = run(
        &["handoff", "create", "--recipient", "Sam", "--json"],
        &project,
    );
    assert!(output.status.success());

    assert_eq!(signals_root.exists(), before_signals);
    assert_eq!(proxy_pending.exists(), before_proxy);
    assert!(handoff_dir(&project_path).exists());
}

#[test]
fn handoff_workflow_does_not_mutate_promotion_or_unrelated_roots() {
    let before_signals =
        bucket_snapshot(&PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/signals"));
    let before_worktree =
        bucket_snapshot(&PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/signals"));

    let project = temp_project("no-side-effects");
    seed_event(&project);
    let create = run(
        &["handoff", "create", "--recipient", "Pat", "--json"],
        &project,
    );
    assert!(create.status.success());

    assert!(
        !promotion_decisions_dir(&project.to_string_lossy()).exists()
            || fs::read_dir(promotion_decisions_dir(&project.to_string_lossy()))
                .map(|entries| entries.count())
                .unwrap_or(0)
                == 0
    );

    let after_signals =
        bucket_snapshot(&PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/signals"));
    let after_worktree =
        bucket_snapshot(&PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/signals"));
    assert_eq!(before_signals, after_signals);
    assert_eq!(before_worktree, after_worktree);
}

#[test]
fn handoff_show_missing_id_fails_closed() {
    let project = temp_project("missing");
    let output = run(&["handoff", "show", "--id", "missing-handoff-id"], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BucketSnapshot {
    pending: usize,
    processed: usize,
    quarantine: usize,
    duplicate: usize,
}

fn bucket_snapshot(signals_root: &Path) -> BucketSnapshot {
    let count = |bucket: &str| -> usize {
        let dir = signals_root.join(bucket);
        if dir.exists() {
            fs::read_dir(dir)
                .map(|entries| entries.count())
                .unwrap_or(0)
        } else {
            0
        }
    };
    BucketSnapshot {
        pending: count("pending"),
        processed: count("processed"),
        quarantine: count("quarantine"),
        duplicate: count("duplicate"),
    }
}

#[test]
fn handoff_persists_under_openmesh_handoff_only() {
    let project = temp_project("path");
    let output = run(
        &["handoff", "create", "--recipient", "Riley", "--json"],
        &project,
    );
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    let handoff_id = payload["handoffId"].as_str().unwrap();
    let expected = project
        .join(".openmesh")
        .join(HANDOFF_DIR)
        .join(format!("{handoff_id}.json"));
    assert!(expected.exists());
    assert!(
        !project.join(".openmesh/signals/pending").exists()
            || fs::read_dir(project.join(".openmesh/signals/pending"))
                .map(|entries| entries.count())
                .unwrap_or(0)
                == 0
    );
}

#[test]
fn link_event_appends_work_event_to_ledger() {
    let project = temp_project("link-event");
    let create = run(
        &[
            "handoff",
            "create",
            "--recipient",
            "Jordan",
            "--json",
            "--link-event",
        ],
        &project,
    );
    assert!(create.status.success());
    let payload: Value = serde_json::from_slice(&create.stdout).expect("json");
    let handoff_id = payload["handoffId"].as_str().unwrap();
    let work_event_id = payload["workEventId"].as_str().expect("linked event");
    assert!(work_event_id.contains(handoff_id));

    let ledger = ledger_dir(&project.to_string_lossy());
    assert!(ledger.join(format!("{work_event_id}.json")).exists());
}
