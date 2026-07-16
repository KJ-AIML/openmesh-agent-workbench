//! Dev Track 0.1.3.6 Checkpoint D — producer compose + inbox write tests (temp only).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use openmesh_core::domain::{
    validate_work_signal_semantics, ActorRef, EvidenceRef, ProducerRef, ProducerSkipReason,
    WORK_SIGNAL_PROTOCOL_VERSION, WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
};
use openmesh_core::events::{ledger_dir, list_events};
use openmesh_core::producers::{
    collect_git_signal, collect_heli_signal, map_git_snapshot_to_kind, CollectSignalOutcome,
};
use openmesh_core::promotion::promotion_decisions_dir;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> (PathBuf, String) {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("openmesh-compose-{label}-{id}"));
    let _ = fs::remove_dir_all(&root);
    let project = root.join("project");
    fs::create_dir_all(&project).expect("project dir");
    let om = project.join(".openmesh");
    fs::create_dir_all(om.join("signals/pending")).expect("pending");
    let project_id = format!("proj-{label}-{id}");
    let now = "2026-07-16T06:00:00Z";
    let project_json = serde_json::json!({
        "id": project_id,
        "name": "Compose Test",
        "folderPath": project.to_str().unwrap(),
        "repoUrl": null,
        "defaultBranch": "main",
        "sprintSource": "none",
        "docsFolder": null,
        "terminalDir": null,
        "defaultAgentCli": null,
        "notes": null,
        "status": "active",
        "createdAt": now,
        "updatedAt": now,
    });
    fs::write(
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .expect("project.json");
    (project, project_id)
}

fn init_git_repo(project: &Path) {
    run_git(project, &["init"]).expect("git init");
    run_git(project, &["config", "user.email", "compose@test.openmesh"]).expect("email");
    run_git(project, &["config", "user.name", "Compose Test"]).expect("name");
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn pending_dir(project: &Path) -> PathBuf {
    project.join(".openmesh/signals/pending")
}

fn pending_count(project: &Path) -> usize {
    fs::read_dir(pending_dir(project))
        .map(|e| e.count())
        .unwrap_or(0)
}

fn read_single_pending_signal(project: &Path) -> openmesh_core::domain::WorkSignal {
    let pending = pending_dir(project);
    let path = fs::read_dir(&pending)
        .expect("pending dir")
        .map(|e| e.expect("entry").path())
        .find(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .expect("one pending json");
    let json = fs::read_to_string(path).expect("read pending");
    serde_json::from_str(&json).expect("deserialize signal")
}

#[test]
fn compose_git_clean_snapshot_writes_protocol_1_1_signal() {
    if !git_available() {
        eprintln!("SKIP: git unavailable");
        return;
    }
    let (project, project_id) = temp_project("git-clean");
    init_git_repo(&project);
    fs::write(project.join("README.md"), "v1\n").expect("write");
    run_git(&project, &["add", "README.md"]).expect("add");
    run_git(&project, &["commit", "-m", "initial"]).expect("commit");

    let outcome = collect_git_signal(&project, &project_id, None).expect("collect");
    let CollectSignalOutcome::Written { signal_id } = outcome else {
        panic!("expected written signal");
    };
    assert!(!signal_id.is_empty());
    assert_eq!(pending_count(&project), 1);

    let signal = read_single_pending_signal(&project);
    assert_eq!(signal.producer, ProducerRef::Git);
    assert_eq!(signal.actor, ActorRef::Unknown);
    assert_eq!(
        signal.protocol_version,
        WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE
    );
    assert!(signal
        .evidence_refs
        .iter()
        .any(|e| matches!(e, EvidenceRef::GitState(_))));
    validate_work_signal_semantics(&signal).expect("valid semantics");
    let json = serde_json::to_string(&signal).expect("serialize");
    assert!(!json.contains("diffBody"));
}

#[test]
fn compose_git_dirty_snapshot_maps_to_progress() {
    if !git_available() {
        return;
    }
    let (project, project_id) = temp_project("git-dirty");
    init_git_repo(&project);
    fs::write(project.join("tracked.txt"), "v1\n").expect("write");
    run_git(&project, &["add", "tracked.txt"]).expect("add");
    run_git(&project, &["commit", "-m", "initial"]).expect("commit");
    fs::write(project.join("dirty.txt"), "new\n").expect("dirty");

    collect_git_signal(&project, &project_id, None).expect("collect");
    let signal = read_single_pending_signal(&project);
    assert_eq!(signal.kind, openmesh_core::domain::WorkSignalKind::Progress);
}

#[test]
fn compose_heli_active_task_writes_file_path_evidence() {
    let (project, project_id) = temp_project("heli-active");
    let state = project.join(".heli-harness/state");
    fs::create_dir_all(&state).expect("state");
    fs::write(
        state.join("current-task.md"),
        "Dev Track 0.1.3.6 — active checkpoint\n",
    )
    .expect("task");

    collect_heli_signal(&project, &project_id, None).expect("collect");
    let signal = read_single_pending_signal(&project);
    assert_eq!(signal.producer, ProducerRef::Heli);
    assert_eq!(signal.protocol_version, WORK_SIGNAL_PROTOCOL_VERSION);
    assert!(signal.evidence_refs.iter().any(|e| matches!(
        e,
        EvidenceRef::FilePath(p) if p == ".heli-harness/state/current-task.md"
    )));
}

#[test]
fn compose_heli_absent_writes_nothing() {
    let (project, project_id) = temp_project("heli-absent");
    let outcome = collect_heli_signal(&project, &project_id, None).expect("collect");
    assert!(matches!(
        outcome,
        CollectSignalOutcome::Skipped {
            reason: ProducerSkipReason::HeliAbsent
        }
    ));
    assert_eq!(pending_count(&project), 0);
}

#[test]
fn compose_preserves_correlation_hint() {
    let (project, project_id) = temp_project("hint");
    let state = project.join(".heli-harness/state");
    fs::create_dir_all(&state).expect("state");
    fs::write(state.join("current-task.md"), "hint test\n").expect("task");
    collect_heli_signal(&project, &project_id, Some("proof-0.1.3.6".into())).expect("collect");
    let signal = read_single_pending_signal(&project);
    assert_eq!(signal.correlation_hint.as_deref(), Some("proof-0.1.3.6"));
}

#[test]
fn compose_workspace_mismatch_is_rejected_by_write_signal() {
    if !git_available() {
        return;
    }
    let (project, _project_id) = temp_project("ws-mismatch");
    init_git_repo(&project);
    fs::write(project.join("README.md"), "v1\n").expect("write");
    run_git(&project, &["add", "README.md"]).expect("add");
    run_git(&project, &["commit", "-m", "initial"]).expect("commit");
    let err = collect_git_signal(&project, "wrong-workspace-id", None).expect_err("mismatch");
    assert!(err.to_string().contains("signal write"));
    assert_eq!(pending_count(&project), 0);
}

#[test]
fn compose_does_not_write_events_or_promotion_audit() {
    let (project, project_id) = temp_project("no-ledger");
    let state = project.join(".heli-harness/state");
    fs::create_dir_all(&state).expect("state");
    fs::write(state.join("current-task.md"), "no ledger\n").expect("task");
    collect_heli_signal(&project, &project_id, None).expect("collect");
    assert!(!ledger_dir(&project.to_string_lossy()).exists());
    assert!(!promotion_decisions_dir(&project.to_string_lossy()).exists());
    assert!(list_events(&project.to_string_lossy()).unwrap().is_empty());
}

#[test]
fn map_git_clean_with_ahead_maps_to_handoff() {
    let snapshot = openmesh_core::domain::GitState {
        repo_id: "fnv1a-abc123".into(),
        branch: "feature/ahead".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: false,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        changed_paths: vec![],
        observed_at: "2026-07-16T06:00:00Z".into(),
        ahead: Some(2),
        behind: Some(0),
        base_ref: Some("main".into()),
        worktree_root: None,
    };
    assert_eq!(
        map_git_snapshot_to_kind(&snapshot),
        openmesh_core::domain::WorkSignalKind::Handoff
    );
}

#[test]
fn map_git_kind_matrix_smoke() {
    let mut snapshot = openmesh_core::domain::GitState {
        repo_id: "fnv1a-abc123".into(),
        branch: "main".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: false,
        staged_count: 0,
        unstaged_count: 0,
        untracked_count: 0,
        changed_paths: vec![],
        observed_at: "2026-07-16T06:00:00Z".into(),
        ahead: Some(0),
        behind: None,
        base_ref: None,
        worktree_root: None,
    };
    assert_eq!(
        map_git_snapshot_to_kind(&snapshot),
        openmesh_core::domain::WorkSignalKind::Milestone
    );
    snapshot.dirty = true;
    assert_eq!(
        map_git_snapshot_to_kind(&snapshot),
        openmesh_core::domain::WorkSignalKind::Progress
    );
}
