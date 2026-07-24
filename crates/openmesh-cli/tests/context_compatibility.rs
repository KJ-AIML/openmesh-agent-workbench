//! Dev Track 0.1.5 Checkpoint G — context CLI compatibility and isolation proofs.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_storage::{
    context_pack_projections_dir, proxy_context_pack_path, PROXY_CONTEXT_PACK_FILENAME,
};
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{ActorRef, ProducerRef, WorkSignal, WorkSignalKind};
use openmesh_core::domain::{EvidenceAttachment, EvidenceRef, ProxyContextPack, WorkEvent};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::work_proxy_profile_path;
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::signals::write_signal;
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
const EVENT_TS: &str = "2026-07-17T01:00:00Z";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-context-compat-{label}-{}-{n}",
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

fn project_id(project: &Path) -> String {
    let raw = fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    serde_json::from_str::<Value>(&raw)
        .unwrap()
        .get("id")
        .and_then(|id| id.as_str())
        .unwrap()
        .to_string()
}

fn init_profile(project: &Path) {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Compat Owner",
            "--role-label",
            "Builder",
            "--json",
        ],
        project,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn seed_continuity(project: &Path) {
    let project_path = project.to_string_lossy().to_string();
    let workspace_id = project_id(project);
    append_event(
        &project_path,
        &WorkEvent::new(
            "evt-compat",
            &workspace_id,
            "work.completed",
            "Compatibility event",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
                observed_at: None,
            }],
            EVENT_TS,
        ),
    )
    .unwrap();
    write_signal(
        &project_path,
        &WorkSignal {
            signal_id: "sig-compat".into(),
            workspace_id: workspace_id.clone(),
            producer: ProducerRef::Reporter("compat-cli".into()),
            actor: ActorRef::Unknown,
            kind: WorkSignalKind::Progress,
            summary: "compat pending signal".into(),
            timestamp: EVENT_TS.into(),
            evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
            correlation_hint: None,
            sensitivity: Sensitivity::Private,
            protocol_version: "1.0".into(),
        },
    )
    .unwrap();
}

fn build_args(write: bool, json: bool) -> Vec<&'static str> {
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
    if json {
        args.push("--json");
    }
    args
}

fn file_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}

fn persist_valid_pack(project: &Path) -> ProxyContextPack {
    init_profile(project);
    seed_continuity(project);
    assert!(run(&build_args(true, false), project).status.success());
    let path = proxy_context_pack_path(&project.to_string_lossy());
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn state_builds_when_context_pack_is_missing() {
    let project = temp_project("state-no-pack");
    init_profile(&project);
    seed_continuity(&project);
    assert!(!proxy_context_pack_path(&project.to_string_lossy()).exists());
    assert!(run(&["state", "--rebuild", "--json"], &project)
        .status
        .success());
}

#[test]
fn catch_up_builds_when_context_pack_is_missing() {
    let project = temp_project("catchup-no-pack");
    init_profile(&project);
    seed_continuity(&project);
    assert!(
        run(&["catch-up", "--since", WINDOW_SINCE, "--json"], &project)
            .status
            .success()
    );
}

#[test]
fn state_builds_when_valid_context_pack_is_persisted() {
    let project = temp_project("state-with-pack");
    persist_valid_pack(&project);
    assert!(run(&["state", "--json"], &project).status.success());
}

#[test]
fn catch_up_builds_when_valid_context_pack_is_persisted() {
    let project = temp_project("catchup-with-pack");
    persist_valid_pack(&project);
    assert!(
        run(&["catch-up", "--since", WINDOW_SINCE, "--json"], &project)
            .status
            .success()
    );
}

#[test]
fn profile_commands_work_when_context_pack_is_persisted() {
    let project = temp_project("profile-with-pack");
    persist_valid_pack(&project);
    assert!(run(&["profile", "show", "--json"], &project)
        .status
        .success());
    assert!(run(&["profile", "validate", "--json"], &project)
        .status
        .success());
}

#[test]
fn malformed_persisted_context_pack_does_not_block_state() {
    let project = temp_project("malformed-state");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, "{broken-pack-json").unwrap();
    assert!(run(&["state", "--rebuild", "--json"], &project)
        .status
        .success());
}

#[test]
fn malformed_persisted_context_pack_does_not_block_catch_up() {
    let project = temp_project("malformed-catchup");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, "{broken-pack-json").unwrap();
    assert!(
        run(&["catch-up", "--since", WINDOW_SINCE, "--json"], &project)
            .status
            .success()
    );
}

#[test]
fn malformed_persisted_context_pack_does_not_block_profile_commands() {
    let project = temp_project("malformed-profile");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, "{broken-pack-json").unwrap();
    assert!(run(&["profile", "show", "--json"], &project)
        .status
        .success());
    assert!(run(&["profile", "validate", "--json"], &project)
        .status
        .success());
}

#[test]
fn state_and_catch_up_do_not_modify_context_pack_bytes() {
    let project = temp_project("state-catchup-pack-bytes");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    assert!(run(&["state", "--json"], &project).status.success());
    assert!(
        run(&["catch-up", "--since", WINDOW_SINCE, "--json"], &project)
            .status
            .success()
    );
    assert_eq!(file_bytes(&path), before);
}

#[test]
fn profile_commands_do_not_modify_context_pack_bytes() {
    let project = temp_project("profile-pack-bytes");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    assert!(run(&["profile", "show", "--json"], &project)
        .status
        .success());
    assert!(run(&["profile", "validate", "--json"], &project)
        .status
        .success());
    assert!(run(
        &["profile", "update", "--working-style", "async-first"],
        &project
    )
    .status
    .success());
    assert_eq!(file_bytes(&path), before);
}

#[test]
fn event_and_signal_commands_do_not_modify_context_pack_bytes() {
    let project = temp_project("event-signal-pack-bytes");
    persist_valid_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    assert!(run(&["event", "inspect", "evt-compat", "--json"], &project)
        .status
        .success());
    assert!(run(
        &[
            "signal",
            "progress",
            "--summary",
            "compat signal write",
            "--json"
        ],
        &project
    )
    .status
    .success());
    assert_eq!(file_bytes(&path), before);
}

#[test]
fn ephemeral_context_build_does_not_modify_continuity_files() {
    let project = temp_project("ephemeral-continuity");
    init_profile(&project);
    seed_continuity(&project);
    let project_path = project.to_string_lossy().to_string();
    let ledger_before = fs::read_dir(ledger_dir(&project_path))
        .map(|e| e.count())
        .unwrap_or(0);
    let pending_before = fs::read_dir(get_project_dir(&project_path).join("signals/pending"))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(false, false), &project).status.success());
    assert_eq!(
        fs::read_dir(ledger_dir(&project_path))
            .map(|e| e.count())
            .unwrap_or(0),
        ledger_before
    );
    assert_eq!(
        fs::read_dir(get_project_dir(&project_path).join("signals/pending"))
            .map(|e| e.count())
            .unwrap_or(0),
        pending_before
    );
    assert!(!current_state_projection_path(&project_path).exists());
}

#[test]
fn persisted_context_build_modifies_only_proxy_context_pack() {
    let project = temp_project("persist-only-pack");
    init_profile(&project);
    seed_continuity(&project);
    assert!(run(&build_args(true, false), &project).status.success());
    let entries = fs::read_dir(context_pack_projections_dir(&project.to_string_lossy()))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(entries, vec![PROXY_CONTEXT_PACK_FILENAME.to_string()]);
}

#[test]
fn context_build_does_not_modify_current_state_projection() {
    let project = temp_project("build-no-state");
    init_profile(&project);
    seed_continuity(&project);
    run(&["state", "--rebuild", "--json"], &project);
    let path = current_state_projection_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    assert!(run(&build_args(true, false), &project).status.success());
    assert_eq!(file_bytes(&path), before);
}

#[test]
fn context_build_does_not_process_pending_signals() {
    let project = temp_project("build-no-signals");
    init_profile(&project);
    seed_continuity(&project);
    let signals_root = get_project_dir(&project.to_string_lossy()).join("signals");
    let before = bucket_snapshot(&signals_root);
    assert!(run(&build_args(true, false), &project).status.success());
    assert_eq!(bucket_snapshot(&signals_root), before);
}

#[test]
fn context_build_does_not_create_work_events() {
    let project = temp_project("build-no-events");
    init_profile(&project);
    seed_continuity(&project);
    let before = fs::read_dir(ledger_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(true, false), &project).status.success());
    assert_eq!(
        fs::read_dir(ledger_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0),
        before
    );
}

#[test]
fn context_build_does_not_mutate_promotion_audit() {
    let project = temp_project("build-no-promotion");
    init_profile(&project);
    seed_continuity(&project);
    let before = fs::read_dir(promotion_decisions_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    assert!(run(&build_args(true, false), &project).status.success());
    assert_eq!(
        fs::read_dir(promotion_decisions_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0),
        before
    );
}

#[test]
fn malformed_pack_validation_does_not_echo_raw_content() {
    let project = temp_project("malformed-echo");
    let file = std::env::temp_dir().join(format!(
        "openmesh-compat-malformed-{}-{}",
        std::process::id(),
        COUNTER.load(Ordering::SeqCst)
    ));
    let secret = "compat-secret-malformed-token";
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
fn context_pack_compatibility_generates_no_proxy_answer() {
    let project = temp_project("no-proxy-answer");
    persist_valid_pack(&project);
    for args in [
        vec!["context", "show"],
        vec!["context", "validate"],
        build_args(false, false),
    ] {
        let output = run(&args, &project);
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_ascii_lowercase();
        assert!(!combined.contains("proxy_response="));
        assert!(!combined.contains("answer_text="));
        assert!(!combined.contains("i am the owner"));
    }
}

#[test]
fn compatibility_tests_start_no_ask_my_proxy() {
    let source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
            .expect("read context.rs")
            .to_ascii_lowercase();
    for forbidden in ["ask-my-proxy", "ask my proxy", "generate_answer"] {
        assert!(!source.contains(forbidden));
    }
}

#[test]
fn compatibility_tests_touch_no_tauri_remote_or_team_mesh() {
    let source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context.rs"))
            .expect("read context.rs")
            .to_ascii_lowercase();
    for forbidden in [
        "tauri",
        "reqwest",
        "team_mesh",
        "teammate",
        "http://",
        "https://",
    ] {
        assert!(!source.contains(forbidden));
    }
}

// Lifecycle amendment (Checkpoint E isolation patch): context module isolation only.
#[test]
fn checkpoint_g_does_not_start_0_1_6_or_0_1_7() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let lowered = fs::read_to_string(root.join("context.rs"))
        .expect("read")
        .to_ascii_lowercase();
    for forbidden in ["0.1.6", "0.1.7", "askmyproxy", "context document annex"] {
        assert!(
            !lowered.contains(forbidden),
            "context.rs must not start {forbidden}"
        );
    }
}

#[test]
fn invalid_profile_context_build_preserves_persisted_pack() {
    let project = temp_project("invalid-profile");
    persist_valid_pack(&project);
    let pack_path = proxy_context_pack_path(&project.to_string_lossy());
    let pack_before = file_bytes(&pack_path);
    let profile_path = work_proxy_profile_path(&project.to_string_lossy());
    let profile_backup = file_bytes(&profile_path);
    fs::write(&profile_path, "{invalid-profile-json").unwrap();
    assert!(!run(&build_args(false, false), &project).status.success());
    assert_eq!(file_bytes(&pack_path), pack_before);
    fs::write(&profile_path, profile_backup).unwrap();
    assert!(run(&["profile", "validate", "--json"], &project)
        .status
        .success());
}

#[test]
fn checkpoint_g_real_projects_remain_isolated() {
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
    for root in [ACTIVE_PROJECT_ROOT, WORKTREE_PROJECT_ROOT] {
        assert!(!PathBuf::from(root)
            .join(".openmesh/projections/proxy-context-pack.json")
            .exists());
    }
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
