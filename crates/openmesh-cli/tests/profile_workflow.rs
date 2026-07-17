//! Dev Track 0.1.4 Checkpoint D — profile CLI workflow and boundary tests.

use openmesh_core::events::ledger_dir;
use openmesh_core::profile::work_proxy_profile_path;
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const ACTIVE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab";
const WORKTREE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab\worktrees\openmesh-0.1.3";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-profile-workflow-{label}-{}-{n}",
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

fn init_profile(project: &Path) {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Owner",
            "--role-label",
            "Role",
        ],
        project,
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
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

fn real_inbox_snapshots() -> (BucketSnapshot, BucketSnapshot) {
    let active = bucket_snapshot(&PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/signals"));
    let worktree = bucket_snapshot(&PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/signals"));
    (active, worktree)
}

#[test]
fn profile_commands_do_not_generate_answer_content() {
    let project = temp_project("no-answer");
    init_profile(&project);
    for args in [vec!["profile", "show"], vec!["profile", "validate"]] {
        let output = run(&args, &project);
        assert!(output.status.success());
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_ascii_lowercase();
        assert!(!combined.contains("answer_text"));
        assert!(!combined.contains("response_body"));
        assert!(!combined.contains("i am the human"));
    }
}

#[test]
fn profile_commands_do_not_read_current_state_or_catch_up() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let content = fs::read_to_string(root.join("profile.rs")).expect("read profile.rs");
    assert!(!content.contains("run_state"));
    assert!(!content.contains("run_catch_up"));
    assert!(!content.contains("build_catch_up_view"));
    assert!(!content.contains("rebuild_current_state_projection"));
}

#[test]
fn profile_commands_do_not_mutate_signal_inboxes() {
    let before = real_inbox_snapshots();
    let project = temp_project("signals");
    init_profile(&project);
    let _ = run(&["profile", "show"], &project);
    let _ = run(&["profile", "validate"], &project);
    let _ = run(&["profile", "update", "--role-label", "Updated"], &project);
    let after = real_inbox_snapshots();
    assert_eq!(before, after);
    assert_eq!(
        before.0,
        BucketSnapshot {
            pending: 0,
            processed: 0,
            quarantine: 0,
            duplicate: 0,
        }
    );
    assert_eq!(
        before.1,
        BucketSnapshot {
            pending: 0,
            processed: 5,
            quarantine: 0,
            duplicate: 0,
        }
    );
}

#[test]
fn profile_commands_do_not_mutate_event_or_promotion_ledgers() {
    let project = temp_project("ledgers");
    init_profile(&project);
    let project_path = project.to_string_lossy().to_string();
    let _ = run(&["profile", "validate"], &project);
    assert!(!ledger_dir(&project_path).exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn profile_commands_do_not_create_projection_files() {
    let project = temp_project("no-projection");
    init_profile(&project);
    let _ = run(&["profile", "show", "--json"], &project);
    assert!(!project.join(".openmesh/projections").exists());
}

#[test]
fn profile_commands_do_not_touch_tauri_or_remote_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let content = fs::read_to_string(root.join("profile.rs")).expect("read profile.rs");
    assert!(!content.contains("tauri"));
    assert!(!content.contains("http://"));
    assert!(!content.contains("https://"));
    assert!(!content.contains("reqwest"));
}

#[test]
fn profile_commands_do_not_start_context_pack_or_ask_my_proxy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let content = fs::read_to_string(root.join("profile.rs")).expect("read profile.rs");
    let lowered = content.to_ascii_lowercase();
    for forbidden in [
        "ask-my-proxy",
        "ask my proxy",
        "context-pack",
        "context pack",
        "proxycontextpack",
        "generate_answer",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "profile.rs must not reference {forbidden}"
        );
    }
}

#[test]
fn full_profile_cli_workflow_init_show_update_validate() {
    let project = temp_project("workflow");
    let init = run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Workflow Owner",
            "--role-label",
            "Workflow Role",
            "--json",
        ],
        &project,
    );
    assert!(init.status.success());
    let init_json: Value = serde_json::from_slice(&init.stdout).expect("init json");
    assert_eq!(init_json["status"], "ok");

    let show = run(&["profile", "show", "--json"], &project);
    assert!(show.status.success());
    let shown: openmesh_core::domain::WorkProxyProfile =
        serde_json::from_slice(&show.stdout).expect("show json");
    assert_eq!(shown.owner_label, "Workflow Owner");

    let update = run(
        &[
            "profile",
            "update",
            "--role-label",
            "Updated Role",
            "--json",
        ],
        &project,
    );
    assert!(update.status.success());

    let validate = run(&["profile", "validate", "--json"], &project);
    assert!(validate.status.success());
    let validate_json: Value = serde_json::from_slice(&validate.stdout).expect("validate json");
    assert_eq!(validate_json["valid"], true);

    let final_profile =
        openmesh_core::profile::read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(final_profile.role_label, "Updated Role");
    assert!(work_proxy_profile_path(&project.to_string_lossy()).exists());
}
