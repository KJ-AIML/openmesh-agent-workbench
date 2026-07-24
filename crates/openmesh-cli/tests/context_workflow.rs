//! Dev Track 0.1.5 Checkpoint E — context CLI workflow and integration tests.

use openmesh_core::context_pack_storage::{context_pack_projections_dir, proxy_context_pack_path};
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::ProxyContextPack;
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
        "openmesh-cli-context-workflow-{label}-{}-{n}",
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
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_event(project: &Path) {
    let project_path = project.to_string_lossy();
    let workspace_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    let event = openmesh_core::domain::WorkEvent::new(
        "evt-cli-seed",
        &workspace_id,
        "work.completed",
        "cli seed",
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

fn signals_root_for(root: &str) -> PathBuf {
    PathBuf::from(root).join(".openmesh/signals")
}

fn top_level_help() -> String {
    String::from_utf8_lossy(&run_raw(&["--help"]).stdout).into_owned()
}

fn context_help() -> String {
    String::from_utf8_lossy(&run_raw(&["context", "--help"]).stdout).into_owned()
}

#[test]
fn context_build_requires_since_and_until() {
    let project = temp_project("requires-window");
    init_profile(&project);
    seed_event(&project);
    assert!(
        !run(&["context", "build", "--until", WINDOW_UNTIL], &project)
            .status
            .success()
    );
    assert!(
        !run(&["context", "build", "--since", WINDOW_SINCE], &project)
            .status
            .success()
    );
}

#[test]
fn context_build_rejects_invalid_fixed_window() {
    let project = temp_project("invalid-window");
    init_profile(&project);
    seed_event(&project);
    let output = run(
        &[
            "context",
            "build",
            "--since",
            WINDOW_UNTIL,
            "--until",
            WINDOW_SINCE,
        ],
        &project,
    );
    assert!(!output.status.success());
}

#[test]
fn context_build_ephemeral_creates_no_projection_directory() {
    let project = temp_project("ephemeral-no-dir");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(false), &project).status.success());
    assert!(!context_pack_projections_dir(&project.to_string_lossy()).exists());
}

#[test]
fn context_build_ephemeral_outputs_valid_pack() {
    let project = temp_project("ephemeral-valid");
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
    assert_eq!(payload["status"], "ok");
    assert!(payload.get("pack").is_some());
    assert!(payload.get("packPath").is_none());
}

#[test]
fn context_build_ephemeral_hash_is_stable_for_fixed_window() {
    let project = temp_project("stable-hash");
    init_profile(&project);
    seed_event(&project);
    let args = [
        "context",
        "build",
        "--since",
        WINDOW_SINCE,
        "--until",
        WINDOW_UNTIL,
        "--json",
    ];
    let a: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();
    let b: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();
    assert_eq!(a["pack"]["buildInputsHash"], b["pack"]["buildInputsHash"]);
    assert_eq!(a["pack"]["contextPackId"], b["pack"]["contextPackId"]);
}

#[test]
fn context_build_write_persists_canonical_pack() {
    let project = temp_project("write-persist");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(true), &project).status.success());
    assert!(proxy_context_pack_path(&project.to_string_lossy()).exists());
}

#[test]
fn context_build_write_modifies_only_context_pack_projection() {
    let project = temp_project("write-only-pack");
    init_profile(&project);
    seed_event(&project);
    let profile_path = work_proxy_profile_path(&project.to_string_lossy());
    let profile_mtime_before = fs::metadata(&profile_path).unwrap().modified().unwrap();
    let events_before = fs::read_dir(ledger_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(true), &project).status.success());
    let entries = fs::read_dir(context_pack_projections_dir(&project.to_string_lossy()))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec!["proxy-context-pack.json".to_string()]);
    assert_eq!(
        fs::metadata(&profile_path).unwrap().modified().unwrap(),
        profile_mtime_before
    );
    assert_eq!(
        fs::read_dir(ledger_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0),
        events_before
    );
}

#[test]
fn context_build_write_does_not_create_current_state_projection() {
    let project = temp_project("no-current-state");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(true), &project).status.success());
    assert!(!current_state_projection_path(&project.to_string_lossy()).exists());
}

#[test]
fn context_show_reads_persisted_pack_only() {
    let project = temp_project("show-persisted");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "show"], &project);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("context_pack_id="));
}

#[test]
fn context_show_does_not_rebuild_missing_pack() {
    let project = temp_project("show-missing");
    init_profile(&project);
    let output = run(&["context", "show"], &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(combined.contains("pack-not-found") || combined.contains("missing"));
}

#[test]
fn context_show_human_displays_safe_context_summary() {
    let project = temp_project("show-human");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let show_output = run(&["context", "show"], &project);
    let stdout = String::from_utf8_lossy(&show_output.stdout);
    assert!(stdout.contains("evidence_index_count="));
    assert!(stdout.contains("not an answer"));
    assert!(!stdout.contains("docs/"));
}

#[test]
fn context_show_json_outputs_complete_sanitized_pack() {
    let project = temp_project("show-json");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "show", "--json"], &project);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    let _: ProxyContextPack = serde_json::from_value(payload["pack"].clone()).expect("pack shape");
}

#[test]
fn context_show_is_read_only() {
    let project = temp_project("show-read-only");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = fs::read_to_string(&path).unwrap();
    assert!(run(&["context", "show"], &project).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn context_validate_accepts_valid_persisted_pack() {
    let project = temp_project("validate-persisted");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let output = run(&["context", "validate", "--json"], &project);
    assert!(output.status.success());
    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["valid"], true);
}

#[test]
fn context_validate_accepts_explicit_file() {
    let project = temp_project("validate-file");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let pack_path = proxy_context_pack_path(&project.to_string_lossy());
    let output = run(
        &[
            "context",
            "validate",
            "--file",
            pack_path.to_str().unwrap(),
            "--json",
        ],
        &project,
    );
    assert!(output.status.success());
}

#[test]
fn context_validate_rejects_malformed_json_safely() {
    let project = temp_project("validate-malformed");
    let file = std::env::temp_dir().join(format!(
        "openmesh-malformed-pack/bad-{}.json",
        COUNTER.fetch_add(1, Ordering::SeqCst)
    ));
    let secret = "secret-malformed-token";
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, format!("{{broken:{secret}}}")).unwrap();
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
    assert!(!combined.contains(secret));
}

#[test]
fn context_validate_rejects_invalid_pack() {
    let project = temp_project("validate-invalid");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let mut pack: ProxyContextPack =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    pack.build_inputs_hash = "invalid-hash".into();
    fs::write(&path, serde_json::to_string_pretty(&pack).unwrap() + "\n").unwrap();
    assert!(!run(&["context", "validate"], &project).status.success());
}

#[test]
fn context_validate_does_not_rewrite_file() {
    let project = temp_project("validate-no-rewrite");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = fs::read_to_string(&path).unwrap();
    assert!(run(&["context", "validate"], &project).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn context_validate_with_project_checks_workspace_match() {
    let project = temp_project("validate-ws");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let pack_path = proxy_context_pack_path(&project.to_string_lossy());
    let other = temp_project("validate-ws-other");
    assert!(!run(
        &["context", "validate", "--file", pack_path.to_str().unwrap(),],
        &other,
    )
    .status
    .success());
}

#[test]
fn context_commands_map_profile_missing_cleanly() {
    let project = temp_project("profile-missing");
    seed_event(&project);
    let output = run(&build_args(false), &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(combined.contains("profile-missing") || combined.contains("profile is missing"));
}

#[test]
fn context_commands_map_workspace_mismatch_cleanly() {
    let project = temp_project("ws-mismatch-cli");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let mut pack: ProxyContextPack =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    pack.workspace_id = "other-workspace".into();
    fs::write(&path, serde_json::to_string_pretty(&pack).unwrap() + "\n").unwrap();
    assert!(!run(&["context", "show"], &project).status.success());
}

#[test]
fn context_commands_map_validation_errors_without_sensitive_echo() {
    let project = temp_project("safe-errors");
    init_profile(&project);
    seed_event(&project);
    run(&build_args(true), &project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, "{not-json:secret-value-XYZ}").unwrap();
    let output = run(&["context", "show"], &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!combined.contains("secret-value-XYZ"));
}

#[test]
fn context_commands_generate_no_answer_content() {
    let project = temp_project("no-answer");
    init_profile(&project);
    seed_event(&project);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run(&build_args(false), &project).stderr),
        String::from_utf8_lossy(&run(&build_args(false), &project).stdout)
    );
    assert!(!combined.to_ascii_lowercase().contains("answer_text"));
    assert!(combined.contains("not an answer"));
}

#[test]
fn context_commands_execute_no_authority() {
    let project = temp_project("no-authority");
    init_profile(&project);
    seed_event(&project);
    let build_output = run(&build_args(false), &project);
    let stdout = String::from_utf8_lossy(&build_output.stdout);
    assert!(stdout.contains("does not execute proxy authority"));
}

// Lifecycle amendment (Checkpoint E isolation patch): context help must not expose
// ask/answer/query subcommands. Top-level `proxy ask` registration is E-owned.
#[test]
fn context_commands_expose_no_ask_answer_chat_or_query_subcommand() {
    let combined = context_help().to_ascii_lowercase();
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
        assert!(!combined.contains(forbidden));
    }
    assert!(combined.contains("build"));
    assert!(combined.contains("show"));
    assert!(combined.contains("validate"));
}

#[test]
fn context_commands_invoke_no_llm_axga_or_model_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for file in ["context.rs", "main.rs"] {
        let lowered = fs::read_to_string(root.join(file))
            .expect("read")
            .to_ascii_lowercase();
        for forbidden in ["llm", "axga", "openai", "anthropic", "model_runtime"] {
            assert!(!lowered.contains(forbidden));
        }
    }
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
fn context_commands_do_not_mutate_signal_inboxes() {
    let active = bucket_snapshot(&signals_root_for(ACTIVE_PROJECT_ROOT));
    let worktree = bucket_snapshot(&signals_root_for(WORKTREE_PROJECT_ROOT));
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
    let project = temp_project("inbox-isolation");
    init_profile(&project);
    seed_event(&project);
    let _ = run(&build_args(true), &project);
    assert_eq!(
        bucket_snapshot(&signals_root_for(ACTIVE_PROJECT_ROOT)),
        active
    );
    assert_eq!(
        bucket_snapshot(&signals_root_for(WORKTREE_PROJECT_ROOT)),
        worktree
    );
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
fn context_commands_do_not_modify_profile() {
    let project = temp_project("no-profile-mutation");
    init_profile(&project);
    seed_event(&project);
    let path = work_proxy_profile_path(&project.to_string_lossy());
    let before = fs::read_to_string(&path).unwrap();
    assert!(run(&build_args(true), &project).status.success());
    assert_eq!(fs::read_to_string(&path).unwrap(), before);
}

#[test]
fn context_commands_create_no_catch_up_persistence() {
    let project = temp_project("no-catch-up");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(true), &project).status.success());
    assert!(!project.join(".openmesh/catch-up").exists());
}

#[test]
fn context_commands_touch_no_tauri_remote_or_team_mesh() {
    let content =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
            .expect("read context.rs");
    for forbidden in [
        "tauri",
        "reqwest",
        "http://",
        "https://",
        "team_mesh",
        "axga",
        "openai",
    ] {
        assert!(!content.to_ascii_lowercase().contains(forbidden));
    }
}

#[test]
fn full_context_cli_workflow_build_write_show_validate() {
    let project = temp_project("full-workflow");
    init_profile(&project);
    seed_event(&project);
    assert!(run(&build_args(false), &project).status.success());
    assert!(run(&build_args(true), &project).status.success());
    assert!(run(&["context", "show"], &project).status.success());
    assert!(run(&["context", "validate"], &project).status.success());
}

#[test]
fn repeated_cli_build_with_fixed_window_preserves_safe_hash() {
    let project = temp_project("repeat-hash");
    init_profile(&project);
    seed_event(&project);
    let args = [
        "context",
        "build",
        "--since",
        WINDOW_SINCE,
        "--until",
        WINDOW_UNTIL,
        "--json",
    ];
    let a: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();
    let b: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();
    assert_eq!(a["pack"]["buildInputsHash"], b["pack"]["buildInputsHash"]);
}

#[test]
fn secret_only_change_does_not_change_cli_safe_hash_when_counts_equal() {
    let project = temp_project("secret-hash");
    init_profile(&project);
    seed_event(&project);
    let workspace_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace");
    let project_path = project.to_string_lossy().to_string();

    let mut secret_event = openmesh_core::domain::WorkEvent::new(
        "evt-secret-cli",
        &workspace_id,
        "work.completed",
        "vault contains super-secret-alpha",
        vec![openmesh_core::domain::EvidenceAttachment {
            evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/secret.md".into()),
            observed_at: None,
        }],
        "2026-07-17T02:00:00Z",
    );
    secret_event.sensitivity = openmesh_core::context::Sensitivity::Secret;
    openmesh_core::events::append_event(&project_path, &secret_event).expect("append secret");

    let args = [
        "context",
        "build",
        "--since",
        WINDOW_SINCE,
        "--until",
        WINDOW_UNTIL,
        "--json",
    ];
    let baseline: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();

    let event_path = ledger_dir(&project_path).join("evt-secret-cli.json");
    let mut event_json: Value =
        serde_json::from_str(&fs::read_to_string(&event_path).unwrap()).expect("parse event");
    event_json["summary"] = Value::String("vault contains super-secret-beta".into());
    fs::write(
        &event_path,
        serde_json::to_string_pretty(&event_json).unwrap() + "\n",
    )
    .unwrap();

    let after: Value = serde_json::from_slice(&run(&args, &project).stdout).unwrap();
    assert_eq!(
        baseline["pack"]["redactionSummary"]["secretItemsOmitted"],
        after["pack"]["redactionSummary"]["secretItemsOmitted"]
    );
    assert_eq!(
        baseline["pack"]["buildInputsHash"], after["pack"]["buildInputsHash"],
        "secret-only content swaps with equal omission counts must not change safe hash"
    );
}

#[test]
fn checkpoint_e_does_not_start_ask_my_proxy() {
    let lowered =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
            .expect("read")
            .to_ascii_lowercase();
    for forbidden in [
        "ask-my-proxy",
        "ask my proxy",
        "generate_answer",
        "openmesh_ai_runtime",
    ] {
        assert!(!lowered.contains(forbidden));
    }
}

// Lifecycle amendment (Checkpoint E isolation patch): context module isolation only.
#[test]
fn checkpoint_e_does_not_start_0_1_6_or_0_1_7() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let lowered = fs::read_to_string(root.join("context.rs"))
        .expect("read")
        .to_ascii_lowercase();
    for forbidden in ["0.1.6", "0.1.7", "context document annex", "askmyproxy"] {
        assert!(
            !lowered.contains(forbidden),
            "context.rs must not start {forbidden}"
        );
    }
}

#[test]
fn real_projection_directories_remain_untouched_in_active_projects() {
    let active_projection =
        PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/projections/proxy-context-pack.json");
    let worktree_projection =
        PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/projections/proxy-context-pack.json");
    assert!(!active_projection.exists());
    assert!(!worktree_projection.exists());
}
