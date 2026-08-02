//! Dev Track 0.1.3.8 Checkpoint B — `event inspect` and `event correct` CLI tests.

use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{EvidenceAttachment, EvidenceRef, WorkEvent};
use openmesh_core::events::append_event;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-event-correction-{label}-{}-{n}",
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

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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
        "Original completed summary",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    )
}

fn seed_target_event(project: &Path, event_id: &str) {
    let project_path = project.to_string_lossy().to_string();
    let workspace_id = project_id(project);
    append_event(&project_path, &sample_event(event_id, &workspace_id)).unwrap();
}

fn catch_up_checkpoint_path(project: &Path) -> PathBuf {
    project.join(".openmesh/projections/catch-up-checkpoint.json")
}

#[test]
fn event_inspect_json_shows_original_and_effective_presentation() {
    let project = temp_project("inspect-json");
    seed_target_event(&project, "evt-target");

    let output = cli()
        .args(["event", "inspect", "evt-target", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["eventId"], "evt-target");
    assert_eq!(payload["originalKind"], "work.completed");
    assert_eq!(payload["effectiveKind"], "work.completed");
    assert_eq!(payload["effectiveSummary"], "Original completed summary");
    assert_eq!(payload["confidence"], "high");
}

#[test]
fn event_inspect_human_shows_correction_chain() {
    let project = temp_project("inspect-human");
    let project_path = project.to_string_lossy().to_string();
    seed_target_event(&project, "evt-target");
    openmesh_core::events::append_event_correction(
        &project_path,
        "evt-target",
        &openmesh_core::events::EventCorrectionRequest {
            corrected_kind: "work.blocked".into(),
            corrected_summary: "Blocked instead".into(),
            actor_label: Some("cli-operator".into()),
            timestamp: Some("2026-07-17T02:00:00Z".into()),
        },
    )
    .unwrap();

    let output = cli()
        .args(["event", "inspect", "evt-target", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("effective_kind=work.blocked"));
    assert!(stdout.contains("correction_chain="));
    assert!(stdout.contains("confidence=medium"));
}

#[test]
fn event_correct_appends_correction_event() {
    let project = temp_project("correct-append");
    seed_target_event(&project, "evt-target");

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "work.blocked",
            "--summary",
            "Corrected via CLI",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("correction_event_id=evt-correction-"));
    assert!(stdout.contains("target_event_id=evt-target"));
}

#[test]
fn event_correct_json_returns_correction_and_effective_presentation() {
    let project = temp_project("correct-json");
    seed_target_event(&project, "evt-target");

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "work.blocked",
            "--summary",
            "Corrected via CLI",
            "--project",
        ])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());

    let payload: Value = serde_json::from_slice(&output.stdout).expect("json");
    assert_eq!(payload["status"], "ok");
    assert_eq!(payload["targetEventId"], "evt-target");
    assert_eq!(payload["effectiveKind"], "work.blocked");
    assert_eq!(payload["effectiveSummary"], "Corrected via CLI");
    assert_eq!(payload["confidence"], "medium");
    assert!(payload["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .starts_with("evt-correction-"));
}

#[test]
fn event_correct_rejects_unknown_event() {
    let project = temp_project("unknown-event");
    let output = cli()
        .args([
            "event",
            "correct",
            "evt-missing",
            "--kind",
            "work.blocked",
            "--summary",
            "Nope",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn event_correct_rejects_invalid_kind() {
    let project = temp_project("invalid-kind");
    seed_target_event(&project, "evt-target");

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "   ",
            "--summary",
            "Valid summary",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn event_correct_rejects_empty_summary() {
    let project = temp_project("empty-summary");
    seed_target_event(&project, "evt-target");

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "work.blocked",
            "--summary",
            "   ",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn event_correct_does_not_rebuild_current_state() {
    let project = temp_project("no-state-rebuild");
    seed_target_event(&project, "evt-target");
    assert!(!current_state_projection_path(&project.to_string_lossy()).exists());

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "work.blocked",
            "--summary",
            "Corrected via CLI",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!current_state_projection_path(&project.to_string_lossy()).exists());
}

#[test]
fn event_correct_does_not_write_catch_up_files() {
    let project = temp_project("no-catchup-file");
    seed_target_event(&project, "evt-target");
    assert!(!catch_up_checkpoint_path(&project).exists());

    let output = cli()
        .args([
            "event",
            "correct",
            "evt-target",
            "--kind",
            "work.blocked",
            "--summary",
            "Corrected via CLI",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!catch_up_checkpoint_path(&project).exists());
}

#[test]
fn event_commands_do_not_touch_tauri_or_0_1_4() {
    let root = workspace_root();
    let event_rs = root.join("crates/openmesh-cli/src/event.rs");
    let content = fs::read_to_string(&event_rs).expect("read event.rs");
    for term in [
        "tauri::",
        "#[tauri::command]",
        "0.1.4",
        "ContinuityIntelligence",
    ] {
        assert!(
            !content.contains(term),
            "event CLI must not reference `{term}`"
        );
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        53,
        "Tauri command count must remain 53 (get_host_os)"
    );
    for term in [
        "run_event_inspect",
        "run_event_correct",
        "append_event_correction",
    ] {
        assert!(
            !tauri_content.contains(term),
            "Tauri must not expose event CLI `{term}`"
        );
    }
}
