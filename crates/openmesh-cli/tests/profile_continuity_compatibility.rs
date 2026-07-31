//! Dev Track 0.1.4 Checkpoint F — CLI profile / continuity compatibility proofs.

use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::WorkProxyProfile;
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::{read_work_proxy_profile, work_proxy_profile_path};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use openmesh_core::{
    context::Sensitivity,
    domain::{
        ActorRef, EvidenceAttachment, EvidenceRef, ProducerRef, WorkEvent, WorkSignal,
        WorkSignalKind,
    },
};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const ACTIVE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab";
const WORKTREE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab\repos\openmesh-agent-workbench";
const EVENT_TS: &str = "2026-07-17T01:00:00Z";
const CATCH_UP_SINCE: &str = "2026-07-15T00:00:00Z";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-profile-continuity-{label}-{}-{n}",
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

fn sample_event(event_id: &str, workspace_id: &str) -> WorkEvent {
    WorkEvent::new(
        event_id,
        workspace_id,
        "work.completed",
        "Compatibility seed event",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        EVENT_TS,
    )
}

fn sample_signal(signal_id: &str, workspace_id: &str) -> WorkSignal {
    WorkSignal {
        signal_id: signal_id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("compat-cli".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: format!("signal summary for {signal_id}"),
        timestamp: EVENT_TS.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn seed_minimal_continuity(project: &Path) {
    let project_path = project.to_string_lossy().to_string();
    let workspace_id = project_id(project);
    append_event(
        &project_path,
        &sample_event("evt-compat-cli", &workspace_id),
    )
    .unwrap();
    write_signal(
        &project_path,
        &sample_signal("sig-compat-cli", &workspace_id),
    )
    .unwrap();
}

fn state_json(project: &Path) -> Value {
    let output = run(&["state", "--rebuild", "--json"], project);
    assert!(
        output.status.success(),
        "state failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("state json")
}

fn catch_up_json(project: &Path) -> Value {
    let output = run(&["catch-up", "--since", CATCH_UP_SINCE, "--json"], project);
    assert!(
        output.status.success(),
        "catch-up failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("catch-up json")
}

fn continuity_semantics(state: &Value) -> Value {
    serde_json::json!({
        "sections": state["sections"],
        "rebuildInputsHash": state["rebuildInputsHash"],
        "sourceCounts": state["sourceCounts"],
    })
}

fn catch_up_semantics(view: &Value) -> Value {
    serde_json::json!({
        "sections": view["sections"],
        "summary": view["summary"],
    })
}

fn init_profile(project: &Path, owner: &str, role: &str) -> WorkProxyProfile {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            owner,
            "--role-label",
            role,
            "--json",
        ],
        project,
    );
    assert!(
        output.status.success(),
        "init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    read_work_proxy_profile(&project.to_string_lossy()).unwrap()
}

fn profile_bytes(project: &Path) -> Vec<u8> {
    fs::read(work_proxy_profile_path(&project.to_string_lossy())).unwrap()
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
fn profile_update_does_not_change_current_state_semantics() {
    let project = temp_project("update-state");
    seed_minimal_continuity(&project);
    let before = continuity_semantics(&state_json(&project));
    init_profile(&project, "Owner", "Role");
    let output = run(
        &[
            "profile",
            "update",
            "--communication-style",
            "Concise and evidence-first",
            "--limitations",
            "Profile does not represent human approval",
            "--json",
        ],
        &project,
    );
    assert!(output.status.success());
    let after = continuity_semantics(&state_json(&project));
    assert_eq!(before, after);
}

#[test]
fn profile_update_does_not_change_catch_up_semantics() {
    let project = temp_project("update-catchup");
    seed_minimal_continuity(&project);
    let before = catch_up_semantics(&catch_up_json(&project));
    init_profile(&project, "Owner", "Role");
    let output = run(
        &[
            "profile",
            "update",
            "--communication-style",
            "Concise and evidence-first",
            "--json",
        ],
        &project,
    );
    assert!(output.status.success());
    let after = catch_up_semantics(&catch_up_json(&project));
    assert_eq!(before, after);
}

#[test]
fn state_command_does_not_create_or_modify_profile() {
    let project = temp_project("state-profile");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    let before = profile_bytes(&project);
    let output = run(&["state", "--rebuild", "--json"], &project);
    assert!(output.status.success());
    let after = profile_bytes(&project);
    assert_eq!(before, after);
}

#[test]
fn catch_up_command_does_not_create_or_modify_profile() {
    let project = temp_project("catchup-profile");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    let before = profile_bytes(&project);
    let output = run(&["catch-up", "--since", CATCH_UP_SINCE, "--json"], &project);
    assert!(output.status.success());
    let after = profile_bytes(&project);
    assert_eq!(before, after);
    assert!(!project
        .join(".openmesh/projections/catch-up-checkpoint.json")
        .exists());
}

#[test]
fn profile_commands_do_not_read_signal_event_or_projection_content() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/profile.rs");
    let content = fs::read_to_string(root).expect("read profile.rs");
    for forbidden in [
        "load_continuity_input_snapshot",
        "rebuild_current_state_projection",
        "build_catch_up_view",
        "read_current_state_projection",
        "current_state_projection_path",
        "ledger_dir(",
        "append_event",
        "write_signal",
    ] {
        assert!(
            !content.contains(forbidden),
            "profile CLI must not read continuity via {forbidden}"
        );
    }
}

#[test]
fn profile_commands_do_not_mutate_signal_event_or_projection_content() {
    let project = temp_project("profile-no-mutate");
    seed_minimal_continuity(&project);
    let project_path = project.to_string_lossy().to_string();
    let projection_before = fs::read_to_string(current_state_projection_path(&project_path)).ok();
    let ledger_before = ledger_file_count(&project);
    let buckets_before = bucket_counts(&project);

    init_profile(&project, "Owner", "Role");
    let _ = run(&["profile", "show", "--json"], &project);
    let _ = run(&["profile", "validate", "--json"], &project);
    let _ = run(
        &["profile", "update", "--working-style", "async-first"],
        &project,
    );

    assert_eq!(ledger_file_count(&project), ledger_before);
    assert_eq!(bucket_counts(&project), buckets_before);
    if let Some(before) = projection_before {
        let after = fs::read_to_string(current_state_projection_path(&project_path)).unwrap();
        assert_eq!(before, after);
    }
}

fn ledger_file_count(project: &Path) -> usize {
    let dir = ledger_dir(&project.to_string_lossy());
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0)
}

fn bucket_counts(project: &Path) -> (usize, usize, usize, usize) {
    fn count(project: &Path, bucket: &str) -> usize {
        let dir = project.join(format!(".openmesh/signals/{bucket}"));
        if !dir.exists() {
            return 0;
        }
        fs::read_dir(dir)
            .map(|entries| entries.count())
            .unwrap_or(0)
    }
    (
        count(project, "pending"),
        count(project, "processed"),
        count(project, "quarantine"),
        count(project, "duplicate"),
    )
}

#[test]
fn malformed_continuity_record_does_not_modify_valid_profile() {
    let project = temp_project("bad-continuity");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    let before = profile_bytes(&project);

    let pending = project.join(".openmesh/signals/pending");
    fs::create_dir_all(&pending).unwrap();
    fs::write(pending.join("corrupt-signal.json"), "{not-json").unwrap();

    let state = run(&["state", "--rebuild", "--json"], &project);
    assert!(state.status.success());
    let catch_up = run(&["catch-up", "--since", CATCH_UP_SINCE, "--json"], &project);
    assert!(catch_up.status.success());
    assert_eq!(profile_bytes(&project), before);
}

#[test]
fn failed_profile_update_preserves_profile_and_continuity() {
    let project = temp_project("failed-update");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    let state_before = continuity_semantics(&state_json(&project));
    let profile_before = read_work_proxy_profile(&project.to_string_lossy()).unwrap();

    let output = run(&["profile", "update", "--limitations", "   "], &project);
    assert_eq!(output.status.code(), Some(3));

    let profile_after = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(profile_after, profile_before);
    let state_after = continuity_semantics(&state_json(&project));
    assert_eq!(state_before, state_after);
}

#[test]
fn profile_and_continuity_commands_generate_no_proxy_answer() {
    let project = temp_project("no-answer");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    for args in [
        vec!["profile", "show"],
        vec!["profile", "validate"],
        vec!["state"],
        vec!["catch-up", "--since", CATCH_UP_SINCE],
    ] {
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
        assert!(!combined.contains("context pack"));
        assert!(!combined.contains("ask-my-proxy"));
    }
}

#[test]
fn compatibility_tests_touch_no_tauri_remote_or_team_mesh() {
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
        assert!(!lowered.contains(forbidden));
    }

    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tauri_lib = workspace_root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        52,
        "Tauri command count must remain 52"
    );

    let before = real_inbox_snapshots();
    let project = temp_project("isolation");
    seed_minimal_continuity(&project);
    init_profile(&project, "Owner", "Role");
    let _ = run(&["state", "--rebuild"], &project);
    let _ = run(&["catch-up", "--since", CATCH_UP_SINCE], &project);
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
    assert_eq!(before.1.pending, 0);
    assert_eq!(before.1.quarantine, 0);
    assert_eq!(before.1.duplicate, 0);
    assert!(!PathBuf::from(ACTIVE_PROJECT_ROOT)
        .join(".openmesh/profile")
        .exists());
    assert!(!PathBuf::from(WORKTREE_PROJECT_ROOT)
        .join(".openmesh/profile")
        .exists());
}

#[test]
fn checkpoint_f_profile_module_does_not_start_0_1_5_or_0_1_6() {
    let content =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/profile.rs"))
            .expect("read profile.rs");
    let lowered = content.to_ascii_lowercase();
    for forbidden in ["proxycontextpack", "askmyproxy", "0.1.5", "0.1.6"] {
        assert!(
            !lowered.contains(forbidden),
            "profile.rs must not start {forbidden}"
        );
    }
}
