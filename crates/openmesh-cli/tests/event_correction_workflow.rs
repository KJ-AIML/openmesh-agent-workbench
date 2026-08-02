//! Dev Track 0.1.3.8 Checkpoint E — full human correction CLI workflow tests.

use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{
    validate_catch_up_view, validate_current_state_projection, EvidenceAttachment, EvidenceRef,
    WorkEvent, CATCH_UP_VIEW_PROTOCOL_VERSION, CURRENT_STATE_PROJECTION_PROTOCOL_VERSION,
};
use openmesh_core::events::{append_event, get_event, ledger_dir};
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const EVENT_ID: &str = "evt-workflow-target";
const EVENT_TS: &str = "2026-07-17T01:00:00Z";
const CORRECTION_TS: &str = "2026-07-17T02:00:00Z";
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-correction-workflow-{label}-{}-{n}",
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
        EVENT_TS,
    )
}

fn seed_target_event_with_timestamp(project: &Path, timestamp: &str) {
    let project_path = project.to_string_lossy().to_string();
    let mut event = sample_event(EVENT_ID, &project_id(project));
    event.timestamp = timestamp.to_string();
    append_event(&project_path, &event).unwrap();
}

fn seed_target_event(project: &Path) {
    seed_target_event_with_timestamp(project, EVENT_TS);
}

fn projection_path(project: &Path) -> PathBuf {
    current_state_projection_path(&project.to_string_lossy())
}

fn catch_up_checkpoint_path(project: &Path) -> PathBuf {
    project.join(".openmesh/projections/catch-up-checkpoint.json")
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

fn run_ok(args: &[&str], project: &Path) -> Output {
    let output = cli()
        .args(args)
        .arg("--project")
        .arg(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "command failed: {args:?}\nstderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn run_json(args: &[&str], project: &Path) -> Value {
    let mut cmd_args: Vec<&str> = args.to_vec();
    cmd_args.push("--json");
    let output = run_ok(&cmd_args, project);
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn run_correct(project: &Path, kind: &str, summary: &str, timestamp: Option<&str>) -> Value {
    let mut cmd = cli();
    cmd.args([
        "event",
        "correct",
        EVENT_ID,
        "--kind",
        kind,
        "--summary",
        summary,
        "--project",
    ])
    .arg(project)
    .arg("--json");
    if let Some(ts) = timestamp {
        cmd.arg("--timestamp").arg(ts);
    }
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "correct failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("correct json")
}

fn find_event_item_in_projection<'a>(projection: &'a Value, event_id: &str) -> Option<&'a Value> {
    let sections = projection.get("sections")?;
    for section in [
        "completed",
        "inProgress",
        "blocked",
        "decisions",
        "needsAttention",
        "stillOpen",
    ] {
        if let Some(items) = sections.get(section).and_then(|v| v.as_array()) {
            for item in items {
                if item.get("sourceId").and_then(|v| v.as_str()) == Some(event_id) {
                    return Some(item);
                }
            }
        }
    }
    None
}

fn evidence_contains_event_id(item: &Value, event_id: &str) -> bool {
    item.get("evidenceRefs")
        .and_then(|v| v.as_array())
        .is_some_and(|refs| {
            refs.iter().any(|evidence| {
                evidence
                    .get("value")
                    .and_then(|v| v.as_str())
                    .is_some_and(|path| path.contains(event_id))
                    || evidence
                        .get("filePath")
                        .and_then(|v| v.as_str())
                        .is_some_and(|path| path.contains(event_id))
            })
        })
}

fn all_section_items(view: &Value) -> Vec<&Value> {
    let mut out = Vec::new();
    let sections = view
        .get("sections")
        .or_else(|| view.get("sections"))
        .and_then(|v| v.as_object());
    let Some(sections) = sections else {
        return out;
    };
    for key in [
        "completed",
        "changed",
        "blocked",
        "decided",
        "needsAttention",
        "stillOpen",
    ] {
        if let Some(items) = sections.get(key).and_then(|v| v.as_array()) {
            out.extend(items.iter());
        }
    }
    out
}

fn setup_corrected_project(label: &str) -> (PathBuf, Value) {
    let project = temp_project(label);
    seed_target_event(&project);
    let correct = run_correct(
        &project,
        "work.blocked",
        "Workflow corrected summary",
        Some(CORRECTION_TS),
    );
    (project, correct)
}

#[test]
fn cli_workflow_inspect_then_correct_then_inspect_shows_effective_update() {
    let project = temp_project("inspect-correct-inspect");
    seed_target_event(&project);

    let before = run_json(&["event", "inspect", EVENT_ID], &project);
    assert_eq!(before["status"], "ok");
    assert_eq!(before["effectiveSummary"], "Original completed summary");
    assert_eq!(before["confidence"], "high");
    assert_eq!(
        before["correctionEventIds"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        0
    );

    run_correct(
        &project,
        "work.blocked",
        "Workflow corrected summary",
        Some(CORRECTION_TS),
    );

    let after = run_json(&["event", "inspect", EVENT_ID], &project);
    assert_eq!(after["effectiveKind"], "work.blocked");
    assert_eq!(after["effectiveSummary"], "Workflow corrected summary");
    assert_eq!(after["confidence"], "medium");
    assert!(after["isCorrected"].as_bool().unwrap_or(false));
}

#[test]
fn cli_workflow_correct_does_not_rewrite_original_event() {
    let project = temp_project("original-unchanged");
    seed_target_event(&project);
    let before = get_event(&project.to_string_lossy(), EVENT_ID)
        .unwrap()
        .expect("original");
    run_correct(
        &project,
        "work.blocked",
        "Workflow corrected summary",
        Some(CORRECTION_TS),
    );
    let after = get_event(&project.to_string_lossy(), EVENT_ID)
        .unwrap()
        .expect("original after");
    assert_eq!(after.kind, before.kind);
    assert_eq!(after.summary, before.summary);
    assert!(after.corrects_event_id.is_none());
}

#[test]
fn cli_workflow_correct_then_state_rebuild_reflects_correction() {
    let (project, correct) = setup_corrected_project("state-rebuild");
    let correction_id = correct["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .to_string();

    let state = run_json(&["state", "--rebuild"], &project);
    assert!(projection_path(&project).exists());
    assert_eq!(
        state["protocolVersion"].as_str().unwrap(),
        CURRENT_STATE_PROJECTION_PROTOCOL_VERSION
    );
    validate_current_state_projection(
        &serde_json::from_value(state.clone()).expect("projection type"),
    )
    .expect("valid projection");

    let item = find_event_item_in_projection(&state, EVENT_ID).expect("event item");
    assert_eq!(item["kind"], "work.blocked");
    assert_eq!(item["summary"], "Workflow corrected summary");
    assert_eq!(item["confidence"], "medium");
    assert!(evidence_contains_event_id(item, &correction_id));
    assert!(state["limitations"]
        .as_array()
        .is_some_and(|items| items.iter().any(|l| {
            l.as_str()
                .is_some_and(|text| text.contains(EVENT_ID) && text.contains("corrected"))
        })));
}

#[test]
fn cli_workflow_correct_then_catch_up_shows_changed_correction() {
    let (project, correct) = setup_corrected_project("catchup-changed");
    let correction_id = correct["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .to_string();
    run_json(&["state", "--rebuild"], &project);

    let catch_up = run_json(&["catch-up", "--since", WINDOW_SINCE], &project);
    assert_eq!(
        catch_up["protocolVersion"].as_str().unwrap(),
        CATCH_UP_VIEW_PROTOCOL_VERSION
    );
    validate_catch_up_view(&serde_json::from_value(catch_up.clone()).expect("catch-up type"))
        .expect("valid catch-up");

    let changed = catch_up["sections"]["changed"]
        .as_array()
        .expect("changed section");
    assert!(
        changed.iter().any(|item| {
            item.get("sourceId").and_then(|v| v.as_str()) == Some(correction_id.as_str())
                && item.get("correlationHint").and_then(|v| v.as_str()) == Some(EVENT_ID)
        }),
        "changed section must include correction item: {changed:?}"
    );

    let target = all_section_items(&catch_up)
        .into_iter()
        .find(|item| item.get("sourceId").and_then(|v| v.as_str()) == Some(EVENT_ID));
    assert!(target.is_some());
    assert_eq!(target.unwrap()["summary"], "Workflow corrected summary");
}

#[test]
fn cli_workflow_catch_up_respects_correction_window() {
    let project = temp_project("catchup-window");
    let now = chrono::Utc::now();
    let original_ts =
        (now - chrono::Duration::minutes(30)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let correction_ts =
        (now + chrono::Duration::hours(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since =
        (now - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    seed_target_event_with_timestamp(&project, &original_ts);
    let correct = run_correct(
        &project,
        "work.blocked",
        "Workflow corrected summary",
        Some(&correction_ts),
    );
    let correction_id = correct["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .to_string();
    run_json(&["state", "--rebuild"], &project);

    let narrow = run_json(&["catch-up", "--since", &since], &project);
    let changed = narrow["sections"]["changed"].as_array().unwrap();
    assert!(
        !changed.iter().any(|item| {
            item.get("sourceId").and_then(|v| v.as_str()) == Some(correction_id.as_str())
        }),
        "future correction must not appear in changed for current window"
    );

    let target = all_section_items(&narrow)
        .into_iter()
        .find(|item| item.get("sourceId").and_then(|v| v.as_str()) == Some(EVENT_ID));
    assert_eq!(
        target.expect("target in window")["summary"],
        "Workflow corrected summary"
    );
}

#[test]
fn cli_workflow_state_suppresses_superseded_original_summary() {
    let (project, _) = setup_corrected_project("state-suppress");
    let state = run_json(&["state", "--rebuild"], &project);
    let summaries: Vec<String> = [
        "completed",
        "inProgress",
        "blocked",
        "decisions",
        "needsAttention",
        "stillOpen",
    ]
    .iter()
    .flat_map(|section| {
        state["sections"][section]
            .as_array()
            .cloned()
            .unwrap_or_default()
    })
    .filter_map(|item| {
        item.get("summary")
            .and_then(|v| v.as_str().map(str::to_string))
    })
    .collect();
    assert!(!summaries.iter().any(|s| s == "Original completed summary"));
}

#[test]
fn cli_workflow_state_preserves_correction_evidence_refs() {
    let (project, correct) = setup_corrected_project("state-evidence");
    let correction_id = correct["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .to_string();
    let state = run_json(&["state", "--rebuild"], &project);
    let item = find_event_item_in_projection(&state, EVENT_ID).expect("event item");
    assert!(evidence_contains_event_id(item, &correction_id));
}

#[test]
fn cli_workflow_catch_up_preserves_correction_evidence_refs() {
    let (project, correct) = setup_corrected_project("catchup-evidence");
    let correction_id = correct["correctionEvent"]["eventId"]
        .as_str()
        .unwrap()
        .to_string();
    run_json(&["state", "--rebuild"], &project);
    let catch_up = run_json(&["catch-up", "--since", WINDOW_SINCE], &project);
    let item = all_section_items(&catch_up)
        .into_iter()
        .find(|item| item.get("sourceId").and_then(|v| v.as_str()) == Some(EVENT_ID))
        .expect("target item");
    assert!(evidence_contains_event_id(item, &correction_id));
}

#[test]
fn cli_workflow_human_outputs_are_readable_and_do_not_overclaim() {
    let (project, _) = setup_corrected_project("human-smoke");
    run_ok(&["state", "--rebuild"], &project);

    let inspect = run_ok(&["event", "inspect", EVENT_ID], &project);
    let correct = run_ok(
        &[
            "event",
            "correct",
            EVENT_ID,
            "--kind",
            "work.blocked",
            "--summary",
            "Second correction",
            "--timestamp",
            "2026-07-17T03:00:00Z",
        ],
        &project,
    );
    let state = run_ok(&["state", "--rebuild"], &project);
    let catch_up = run_ok(&["catch-up", "--since", WINDOW_SINCE], &project);

    for (label, output) in [
        ("inspect", &inspect),
        ("correct", &correct),
        ("state", &state),
        ("catch-up", &catch_up),
    ] {
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.is_empty(), "{label} stdout must not be empty");
        for forbidden in [
            "agent decided",
            "AI ",
            "LLM",
            "definitely",
            "certainly true",
            "guaranteed",
        ] {
            assert!(
                !stdout.to_ascii_lowercase().contains(forbidden),
                "{label} must not overclaim (`{forbidden}`): {stdout}"
            );
        }
    }

    let inspect_stdout = String::from_utf8_lossy(&inspect.stdout);
    assert!(inspect_stdout.contains("effective_kind="));
    assert!(inspect_stdout.contains("confidence="));
    let state_stdout = String::from_utf8_lossy(&state.stdout);
    assert!(state_stdout.contains("sections:"));
    let catch_up_stdout = String::from_utf8_lossy(&catch_up.stdout);
    assert!(catch_up_stdout.contains("changed="));
}

#[test]
fn cli_workflow_json_outputs_validate() {
    let (project, _) = setup_corrected_project("json-validate");
    run_json(&["state", "--rebuild"], &project);

    let inspect = run_json(&["event", "inspect", EVENT_ID], &project);
    assert_eq!(inspect["status"], "ok");
    assert!(inspect.get("effectiveKind").is_some());

    let state: openmesh_core::domain::CurrentStateProjection =
        serde_json::from_value(run_json(&["state"], &project)).expect("state projection");
    validate_current_state_projection(&state).expect("valid state");

    let catch_up: openmesh_core::domain::CatchUpView =
        serde_json::from_value(run_json(&["catch-up", "--since", WINDOW_SINCE], &project))
            .expect("catch-up view");
    validate_catch_up_view(&catch_up).expect("valid catch-up");
}

#[test]
fn cli_workflow_rejects_invalid_correction_without_mutating_ledger() {
    let project = temp_project("reject-invalid");
    seed_target_event(&project);
    let before = ledger_file_count(&project);

    let output = cli()
        .args([
            "event",
            "correct",
            EVENT_ID,
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
    assert_eq!(ledger_file_count(&project), before);
}

#[test]
fn cli_workflow_rejects_unknown_target_without_creating_projection() {
    let project = temp_project("reject-unknown");
    assert!(!projection_path(&project).exists());

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
    assert!(!projection_path(&project).exists());
}

#[test]
fn cli_workflow_does_not_create_catch_up_files() {
    let (project, _) = setup_corrected_project("no-catchup-file");
    run_json(&["state", "--rebuild"], &project);
    run_json(&["catch-up", "--since", WINDOW_SINCE], &project);
    assert!(!catch_up_checkpoint_path(&project).exists());
    let projections_dir = project.join(".openmesh/projections");
    if projections_dir.exists() {
        let names: Vec<_> = fs::read_dir(&projections_dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !names.iter().any(|n| n.contains("catch-up")),
            "must not persist catch-up files: {names:?}"
        );
    }
}

#[test]
fn cli_workflow_does_not_mutate_signal_buckets() {
    let project = temp_project("signal-buckets");
    let project_path = project.to_string_lossy().to_string();
    let workspace_id = project_id(&project);
    write_signal(
        &project_path,
        &openmesh_core::domain::WorkSignal {
            signal_id: "sig-workflow".into(),
            workspace_id,
            producer: openmesh_core::domain::ProducerRef::Reporter("cli-test".into()),
            actor: openmesh_core::domain::ActorRef::Unknown,
            kind: openmesh_core::domain::WorkSignalKind::Progress,
            summary: "workflow signal".into(),
            timestamp: EVENT_TS.into(),
            evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
            correlation_hint: None,
            sensitivity: openmesh_core::context::Sensitivity::Private,
            protocol_version: "1.0".into(),
        },
    )
    .unwrap();
    let before = bucket_counts(&project);
    seed_target_event(&project);
    run_correct(
        &project,
        "work.blocked",
        "Workflow corrected summary",
        Some(CORRECTION_TS),
    );
    run_json(&["state", "--rebuild"], &project);
    run_json(&["catch-up", "--since", WINDOW_SINCE], &project);
    assert_eq!(bucket_counts(&project), before);
}

#[test]
fn cli_workflow_does_not_mutate_promotion_audit() {
    let (project, _) = setup_corrected_project("no-audit");
    let audit_dir = promotion_decisions_dir(&project.to_string_lossy());
    assert!(!audit_dir.exists());
    run_json(&["state", "--rebuild"], &project);
    run_json(&["catch-up", "--since", WINDOW_SINCE], &project);
    assert!(!audit_dir.exists());
}

#[test]
fn cli_workflow_does_not_touch_tauri_or_0_1_4() {
    let root = workspace_root();
    let cli_src = root.join("crates/openmesh-cli/src");
    let forbidden = [
        "tauri::",
        "#[tauri::command]",
        "0.1.4",
        "ContinuityIntelligence",
    ];
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    for path in files {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if matches!(file_name, "event.rs") {
            let content = fs::read_to_string(&path).expect("read source");
            for term in forbidden {
                assert!(
                    !content.contains(term),
                    "workflow CLI must not reference `{term}`: {}",
                    path.display()
                );
            }
        } else if matches!(file_name, "state.rs" | "catch_up.rs" | "main.rs") {
            let content = fs::read_to_string(&path).expect("read source");
            for term in ["tauri::", "#[tauri::command]", "ContinuityIntelligence"] {
                assert!(
                    !content.contains(term),
                    "workflow CLI must not reference `{term}`: {}",
                    path.display()
                );
            }
        }
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        53,
        "Tauri command count must remain 53 (get_host_os)"
    );
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
