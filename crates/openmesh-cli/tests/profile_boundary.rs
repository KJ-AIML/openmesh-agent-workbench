//! Dev Track 0.1.4 Checkpoint E — CLI profile boundary proofs.

use openmesh_core::domain::WorkProxyProfile;
use openmesh_core::events::ledger_dir;
use openmesh_core::profile::{profile_dir, profile_exists, work_proxy_profile_path};
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
        "openmesh-cli-profile-boundary-{label}-{}-{n}",
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

fn init_profile(project: &Path, owner: &str, role: &str) {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            owner,
            "--role-label",
            role,
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

fn top_level_help() -> String {
    let output = cli().arg("--help").output().expect("help");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn profile_help() -> String {
    let output = cli()
        .args(["profile", "--help"])
        .output()
        .expect("profile help");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn profile_human_output_never_claims_to_be_owner() {
    let project = temp_project("human-output");
    init_profile(&project, "Alex Owner", "Lead");
    let show = run(&["profile", "show"], &project);
    assert!(show.status.success());
    let stdout = String::from_utf8_lossy(&show.stdout).to_ascii_lowercase();
    assert!(!stdout.contains("i am alex owner"));
    assert!(!stdout.contains("i am the owner"));
    assert!(stdout.contains("not the human owner"));
}

#[test]
fn profile_json_contains_no_runtime_identity_claim() {
    let project = temp_project("json-identity");
    init_profile(&project, "Alex Owner", "Lead");
    let output = run(&["profile", "show", "--json"], &project);
    assert!(output.status.success());
    let profile: WorkProxyProfile = serde_json::from_slice(&output.stdout).expect("profile json");
    assert!(!profile.owner_label.to_ascii_lowercase().contains("i am "));
    let raw = String::from_utf8_lossy(&output.stdout).to_ascii_lowercase();
    assert!(!raw.contains("runtime_identity"));
}

#[test]
fn no_impersonation_refusal_survives_profile_update() {
    let project = temp_project("refusal-survives");
    init_profile(&project, "Owner", "Role");
    let output = run(
        &["profile", "update", "--role-label", "Updated Role"],
        &project,
    );
    assert!(output.status.success());
    let profile =
        openmesh_core::profile::read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert!(profile
        .default_refusal_rules
        .iter()
        .any(|rule| rule.statement.contains("cannot impersonate owner")));
}

#[test]
fn profile_show_missing_does_not_create_default_profile() {
    let project = temp_project("show-missing");
    let output = run(&["profile", "show"], &project);
    assert_eq!(output.status.code(), Some(3));
    assert!(!profile_dir(&project.to_string_lossy()).exists());
    assert!(!profile_exists(&project.to_string_lossy()).unwrap());
}

#[test]
fn profile_validate_missing_does_not_create_default_profile() {
    let project = temp_project("validate-missing");
    let output = run(&["profile", "validate"], &project);
    assert_eq!(output.status.code(), Some(3));
    assert!(!work_proxy_profile_path(&project.to_string_lossy()).exists());
}

#[test]
fn only_profile_init_creates_profile() {
    let project = temp_project("only-init");
    assert!(!profile_exists(&project.to_string_lossy()).unwrap());
    let _ = run(&["profile", "show"], &project);
    let _ = run(&["profile", "validate"], &project);
    assert!(!profile_exists(&project.to_string_lossy()).unwrap());
    init_profile(&project, "Owner", "Role");
    assert!(profile_exists(&project.to_string_lossy()).unwrap());
}

#[test]
fn profile_commands_do_not_read_continuity_records() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/profile.rs");
    let content = fs::read_to_string(root).expect("read profile.rs");
    for forbidden in [
        "run_state",
        "run_catch_up",
        "read_current_state_projection",
        "build_catch_up_view",
        "rebuild_current_state_projection",
    ] {
        assert!(
            !content.contains(forbidden),
            "profile CLI must not read continuity via {forbidden}"
        );
    }
}

#[test]
fn profile_commands_do_not_mutate_continuity_records() {
    let project = temp_project("no-continuity-mutation");
    init_profile(&project, "Owner", "Role");
    let _ = run(&["profile", "show"], &project);
    let _ = run(&["profile", "validate"], &project);
    let _ = run(
        &["profile", "update", "--working-style", "async-first"],
        &project,
    );
    assert!(!project.join(".openmesh/projections").exists());
}

#[test]
fn default_refusals_cannot_be_weakened_by_basic_update() {
    let project = temp_project("refusal-update");
    init_profile(&project, "Owner", "Role");
    let before =
        openmesh_core::profile::read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    let output = run(
        &[
            "profile",
            "update",
            "--communication-style",
            "verbose",
            "--decision-preferences",
            "fast-decisions",
        ],
        &project,
    );
    assert!(output.status.success());
    let after =
        openmesh_core::profile::read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(after.default_refusal_rules, before.default_refusal_rules);
    assert!(after
        .default_refusal_rules
        .iter()
        .any(|rule| rule.statement.contains("cannot invent evidence")));
}

#[test]
fn profile_commands_expose_no_context_pack_command() {
    let top = top_level_help();
    let profile = profile_help();
    let combined = format!("{top}\n{profile}").to_ascii_lowercase();
    assert!(!combined.contains("context-pack"));
    assert!(!combined.contains("context pack"));
    assert!(!combined.contains("proxy-context"));
}

#[test]
fn profile_commands_expose_no_ask_or_answer_command() {
    let top = top_level_help();
    let profile = profile_help();
    for line in top.lines().chain(profile.lines()) {
        let trimmed = line.trim();
        if trimmed.starts_with("ask ") || trimmed.starts_with("answer ") {
            panic!("unexpected ask/answer command surface: {trimmed}");
        }
    }
    let combined = format!("{top}\n{profile}").to_ascii_lowercase();
    for forbidden in [
        "ask-my-proxy",
        "proxy query",
        "proxy chat",
        "proxy response",
    ] {
        assert!(!combined.contains(forbidden));
    }
}

#[test]
fn profile_modules_invoke_no_llm_axga_or_model_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/profile.rs");
    let content = fs::read_to_string(root).expect("read profile.rs");
    let lowered = content.to_ascii_lowercase();
    for forbidden in [
        "openai",
        "anthropic",
        "axga",
        "langchain",
        "llm::",
        "invoke_model",
        "chat_completion",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "profile CLI must not invoke {forbidden}"
        );
    }
}

#[test]
fn profile_boundary_tests_create_no_projection_files() {
    let before = real_inbox_snapshots();
    let project = temp_project("no-projection");
    init_profile(&project, "Owner", "Role");
    let _ = run(&["profile", "validate", "--json"], &project);
    assert!(!project.join(".openmesh/projections").exists());
    let after = real_inbox_snapshots();
    assert_eq!(before, after);
}

#[test]
fn profile_boundary_tests_mutate_no_signal_event_or_promotion_data() {
    let before = real_inbox_snapshots();
    let project = temp_project("no-ledger-mutation");
    let project_path = project.to_string_lossy().to_string();
    init_profile(&project, "Owner", "Role");
    let _ = run(&["profile", "show"], &project);
    let _ = run(&["profile", "validate"], &project);
    assert!(!ledger_dir(&project_path).exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
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
fn checkpoint_e_does_not_touch_tauri_remote_or_team_mesh() {
    let cli_profile =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/profile.rs"))
            .expect("read profile.rs");
    let lowered = cli_profile.to_ascii_lowercase();
    for forbidden in [
        "tauri::",
        "#[tauri::command]",
        "reqwest",
        "team mesh",
        "remote sync",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "profile CLI must not touch {forbidden}"
        );
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tauri_lib = workspace_root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        52,
        "Tauri command count must remain 52"
    );
}

#[test]
fn checkpoint_e_does_not_start_0_1_5_or_0_1_6() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in ["profile.rs", "main.rs"] {
        let content = fs::read_to_string(root.join(file)).expect("read source");
        let lowered = content.to_ascii_lowercase();
        for forbidden in ["proxycontextpack", "askmyproxy", "0.1.5", "0.1.6"] {
            assert!(
                !lowered.contains(forbidden),
                "{file} must not start {forbidden}"
            );
        }
    }
}

#[test]
fn profile_validate_json_reports_metadata_not_answers() {
    let project = temp_project("validate-json");
    init_profile(&project, "Owner", "Role");
    let output = run(&["profile", "validate", "--json"], &project);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["valid"], true);
    assert!(payload.get("resolvedSafeDefaultAuthority").is_some());
    assert!(payload.get("answer").is_none());
    assert!(payload.get("response").is_none());
}
