//! Dev Track 0.1.5 Checkpoint F — context CLI no-answer / no-authority boundary proofs.

use openmesh_core::context_pack_storage::{context_pack_projections_dir, proxy_context_pack_path};
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::events::ledger_dir;
use openmesh_core::profile::work_proxy_profile_path;
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::storage::{get_project_dir, init_project};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const ACTIVE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab";
const WORKTREE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab\worktrees\openmesh-0.1.3";
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";
const WINDOW_UNTIL: &str = "2026-07-18T00:00:00Z";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-context-boundary-{label}-{}-{n}",
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
    assert!(output.status.success());
}

fn seed_event(project: &Path) {
    let project_path = project.to_string_lossy();
    let workspace_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    let event = openmesh_core::domain::WorkEvent::new(
        "evt-boundary-seed",
        &workspace_id,
        "work.completed",
        "boundary seed",
        vec![openmesh_core::domain::EvidenceAttachment {
            evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    );
    openmesh_core::events::append_event(&project_path, &event).expect("append");
}

fn build_args(write: bool) -> Vec<&'static str> {
    let mut args = vec![
        "context",
        "build",
        "--since",
        WINDOW_SINCE,
        "--until",
        WINDOW_UNTIL,
    ];
    if write {
        args.push("--write");
    }
    args
}

fn context_help() -> String {
    String::from_utf8_lossy(&run_raw(&["context", "--help"]).stdout).into_owned()
}

fn context_source() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
        .expect("read context.rs")
}

#[test]
fn context_cli_exposes_only_build_show_and_validate() {
    let help = context_help().to_ascii_lowercase();
    for required in ["build", "show", "validate"] {
        assert!(help.contains(required), "missing subcommand {required}");
    }
    let lines: Vec<_> = help
        .lines()
        .filter(|line| {
            line.trim_start().starts_with("build")
                || line.trim_start().starts_with("show")
                || line.trim_start().starts_with("validate")
        })
        .collect();
    assert!(lines.len() >= 3);
}

#[test]
fn context_cli_exposes_no_ask_answer_query_chat_or_respond() {
    let help = context_help().to_ascii_lowercase();
    for forbidden in [
        "ask ",
        "answer ",
        "query ",
        "chat ",
        "respond ",
        "execute ",
        "approve ",
        "delegate ",
        "prompt ",
        "model ",
    ] {
        assert!(!help.contains(forbidden), "forbidden surface: {forbidden}");
    }
}

#[test]
fn context_build_generates_context_not_answer() {
    let project = temp_project("build-not-answer");
    init_profile(&project);
    seed_event(&project);
    let output = run(
        &[
            "context",
            "build",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
            "--json",
        ],
        &project,
    );
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert!(payload.get("pack").is_some());
    assert!(payload.get("answer").is_none());
    assert!(payload.get("response").is_none());
    let pack = &payload["pack"];
    assert!(pack.get("contextPackId").is_some());
    assert!(pack.get("answer").is_none());
}

#[test]
fn context_show_generates_summary_not_proxy_response() {
    let project = temp_project("show-not-response");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "show"], &project);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("context_pack_id="));
    assert!(stdout.contains("not an answer"));
    assert!(!stdout.contains("proxy_response="));
    assert!(!stdout.contains("answer_text="));
}

#[test]
fn context_validate_generates_validation_metadata_only() {
    let project = temp_project("validate-metadata");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "validate", "--json"], &project);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["valid"], true);
    assert!(payload.get("contextPackId").is_some());
    assert!(payload.get("answer").is_none());
    assert!(payload.get("response").is_none());
    assert!(payload.get("pack").is_none());
}

#[test]
fn human_output_never_claims_to_be_profile_owner() {
    let project = temp_project("no-impersonation-human");
    init_profile(&project);
    seed_event(&project);
    let output = run(&build_args(false), &project);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_ascii_lowercase();
    for phrase in [
        "i am the owner",
        "i am owner",
        "speaking as the owner",
        "this is the owner",
    ] {
        assert!(!combined.contains(phrase));
    }
    assert!(combined.contains("not an answer"));
}

#[test]
fn context_commands_perform_no_network_or_remote_access() {
    let source = context_source().to_ascii_lowercase();
    for forbidden in [
        "reqwest",
        "http://",
        "https://",
        "hyper::",
        "ureq::",
        "remote_storage",
        "cloud_sync",
        "team_mesh",
    ] {
        assert!(!source.contains(forbidden));
    }
}

#[test]
fn context_commands_expose_no_team_mesh_or_teammate_proxy_surface() {
    let source = context_source().to_ascii_lowercase();
    for forbidden in [
        "team_mesh",
        "teammate",
        "team-mesh",
        "mesh_lookup",
        "proxy_lookup",
    ] {
        assert!(!source.contains(forbidden));
    }
}

#[test]
fn ephemeral_context_build_performs_zero_writes() {
    let project = temp_project("ephemeral-zero-write");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(false), &project).status.success());
    assert!(!context_pack_projections_dir(&project.to_string_lossy()).exists());
}

#[test]
fn persisted_build_writes_only_proxy_context_pack() {
    let project = temp_project("write-only-pack");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(true), &project).status.success());
    let entries = fs::read_dir(context_pack_projections_dir(&project.to_string_lossy()))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["proxy-context-pack.json".to_string()]);
}

#[test]
fn context_show_and_validate_are_read_only() {
    let project = temp_project("show-validate-read-only");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = fs::read_to_string(&path).unwrap();
    assert!(run(&["context", "show"], &project).status.success());
    assert!(run(&["context", "validate"], &project).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn context_commands_do_not_modify_current_state_projection() {
    let project = temp_project("no-current-state");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(true), &project).status.success());
    assert!(!current_state_projection_path(&project.to_string_lossy()).exists());
}

#[test]
fn context_commands_do_not_modify_profile() {
    let project = temp_project("no-profile-write");
    init_profile(&project);
    seed_event(&project);
    let path = work_proxy_profile_path(&project.to_string_lossy());
    let before = fs::read_to_string(&path).unwrap();
    assert!(run(&build_args(true), &project).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn context_commands_do_not_process_or_promote_signals() {
    let project = temp_project("no-signal-processing");
    init_profile(&project);
    seed_event(&project);
    let signals_root = get_project_dir(&project.to_string_lossy()).join("signals");
    let before = bucket_snapshot(&signals_root);
    assert!(run(&build_args(true), &project).status.success());
    assert_eq!(bucket_snapshot(&signals_root), before);
}

#[test]
fn context_commands_do_not_create_work_events() {
    let project = temp_project("no-events");
    init_profile(&project);
    seed_event(&project);
    let before = fs::read_dir(ledger_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(true), &project).status.success());
    assert_eq!(
        fs::read_dir(ledger_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0),
        before
    );
}

#[test]
fn context_commands_do_not_mutate_promotion_audit() {
    let project = temp_project("no-promotion");
    init_profile(&project);
    seed_event(&project);
    let before = fs::read_dir(promotion_decisions_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(true), &project).status.success());
    assert_eq!(
        fs::read_dir(promotion_decisions_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0),
        before
    );
}

#[test]
fn context_errors_echo_no_secret_values() {
    let project = temp_project("safe-errors");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, "{not-json:secret-boundary-token}").unwrap();
    let output = run(&["context", "show"], &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!combined.contains("secret-boundary-token"));
}

#[test]
fn malformed_json_error_does_not_echo_input() {
    let project = temp_project("malformed-json");
    let file = std::env::temp_dir().join(format!(
        "openmesh-boundary-malformed-{}-{}",
        std::process::id(),
        COUNTER.load(Ordering::SeqCst)
    ));
    fs::write(&file, "{broken:super-secret-malformed}").unwrap();
    let output = run(
        &[
            "context",
            "validate",
            "--file",
            file.to_str().unwrap(),
            "--json",
        ],
        &project,
    );
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!combined.contains("super-secret-malformed"));
}

#[test]
fn human_output_prints_no_evidence_paths_by_default() {
    let project = temp_project("no-evidence-paths");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "show"], &project);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("docs/"));
    assert!(!stdout.contains("file-path"));
}

#[test]
fn profile_commands_do_not_build_context_pack() {
    let project = temp_project("profile-no-pack");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&["profile", "show"], &project).status.success());
    assert!(run(&["profile", "validate"], &project).status.success());
    assert!(!proxy_context_pack_path(&project.to_string_lossy()).exists());
}

#[test]
fn state_and_catch_up_do_not_modify_context_pack() {
    let project = temp_project("state-catchup-no-pack");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&["state", "--json"], &project).status.success());
    assert!(
        run(&["catch-up", "--since", WINDOW_SINCE, "--json"], &project)
            .status
            .success()
    );
    assert!(!proxy_context_pack_path(&project.to_string_lossy()).exists());
}

#[test]
fn signal_and_event_commands_do_not_invoke_context_builder() {
    let cli_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in [
        "signal.rs",
        "event.rs",
        "state.rs",
        "catch_up.rs",
        "profile.rs",
    ] {
        let content = fs::read_to_string(cli_src.join(file)).expect("read source");
        assert!(!content.contains("build_proxy_context_pack"));
        assert!(!content.contains("compose_proxy_context_pack"));
    }
}

#[test]
fn checkpoint_f_does_not_start_ask_my_proxy() {
    let lowered = context_source().to_ascii_lowercase();
    for forbidden in [
        "ask-my-proxy",
        "ask my proxy",
        "generate_answer",
        "openmesh_ai_runtime",
    ] {
        assert!(!lowered.contains(forbidden));
    }
}

#[test]
fn checkpoint_f_does_not_start_0_1_6_or_0_1_7() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in ["context.rs", "main.rs"] {
        let lowered = fs::read_to_string(root.join(file))
            .expect("read")
            .to_ascii_lowercase();
        for forbidden in ["0.1.6", "0.1.7", "context document annex", "askmyproxy"] {
            assert!(
                !lowered.contains(forbidden),
                "{file} must not start {forbidden}"
            );
        }
    }
}

#[test]
fn checkpoint_f_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    let content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    assert_eq!(content.matches("#[tauri::command]").count(), 52);
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
fn checkpoint_f_real_projects_remain_isolated() {
    let active = bucket_snapshot(&PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/signals"));
    let worktree = bucket_snapshot(&PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/signals"));
    assert_eq!(
        active,
        BucketSnapshot {
            pending: 0,
            processed: 0,
            quarantine: 0,
            duplicate: 0
        }
    );
    assert_eq!(
        worktree,
        BucketSnapshot {
            pending: 0,
            processed: 5,
            quarantine: 0,
            duplicate: 0
        }
    );
    assert!(!PathBuf::from(ACTIVE_PROJECT_ROOT)
        .join(".openmesh/projections/proxy-context-pack.json")
        .exists());
    assert!(!PathBuf::from(WORKTREE_PROJECT_ROOT)
        .join(".openmesh/projections/proxy-context-pack.json")
        .exists());
}
