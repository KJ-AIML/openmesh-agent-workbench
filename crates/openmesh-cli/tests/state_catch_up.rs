//! Dev Track 0.1.3.7 Checkpoint E — `state` and `catch-up` CLI e2e tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{
    validate_catch_up_view, validate_current_state_projection, ActorRef, EvidenceRef, ProducerRef,
    WorkSignal, WorkSignalKind, CATCH_UP_VIEW_PROTOCOL_VERSION,
    CURRENT_STATE_PROJECTION_PROTOCOL_VERSION,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-state-catchup-{label}-{}-{n}",
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

fn bucket_count(project: &Path, bucket: &str) -> usize {
    let dir = project.join(format!(".openmesh/signals/{bucket}"));
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| entries.count())
        .unwrap_or(0)
}

fn bucket_counts(project: &Path) -> (usize, usize, usize, usize) {
    (
        bucket_count(project, "pending"),
        bucket_count(project, "processed"),
        bucket_count(project, "quarantine"),
        bucket_count(project, "duplicate"),
    )
}

fn projection_path(project: &Path) -> PathBuf {
    current_state_projection_path(&project.to_string_lossy())
}

fn catch_up_checkpoint_path(project: &Path) -> PathBuf {
    project.join(".openmesh/projections/catch-up-checkpoint.json")
}

fn sample_signal(id: &str, workspace_id: &str, timestamp: &str) -> WorkSignal {
    WorkSignal {
        signal_id: id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("cli-test".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: format!("summary for {id}"),
        timestamp: timestamp.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
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

#[test]
fn state_rebuild_creates_projection_in_temp_project_only() {
    let project = temp_project("rebuild-creates");
    assert!(!projection_path(&project).exists());
    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(projection_path(&project).exists());
}

#[test]
fn state_default_reads_existing_projection() {
    let project = temp_project("reads-existing");
    let first = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .output()
        .unwrap();
    assert!(first.status.success());
    let before = fs::read_to_string(projection_path(&project)).unwrap();

    let second = cli()
        .args(["state", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(second.status.success());
    let after = fs::read_to_string(projection_path(&project)).unwrap();
    assert_eq!(before, after);
}

#[test]
fn state_default_rebuilds_when_projection_missing() {
    let project = temp_project("rebuild-missing");
    assert!(!projection_path(&project).exists());
    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(projection_path(&project).exists());
}

#[test]
fn state_json_outputs_valid_current_state_projection() {
    let project = temp_project("state-json");
    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: openmesh_core::domain::CurrentStateProjection =
        serde_json::from_str(stdout.trim()).expect("valid projection json");
    assert_eq!(
        parsed.protocol_version,
        CURRENT_STATE_PROJECTION_PROTOCOL_VERSION
    );
    validate_current_state_projection(&parsed).expect("valid projection");
}

#[test]
fn state_human_output_contains_section_counts() {
    let project = temp_project("state-human");
    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("sections: completed="));
    assert!(stdout.contains("in_progress="));
    assert!(stdout.contains("needs_attention="));
    assert!(stdout.contains("still_open="));
    assert!(stdout.contains("pending_attention="));
    assert!(stdout.contains("projection_path="));
}

#[test]
fn state_rebuild_is_idempotent() {
    let project = temp_project("state-idempotent");
    let first = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .arg("--json")
        .output()
        .unwrap();
    assert!(first.status.success());
    let first_json: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();

    let second = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .arg("--json")
        .output()
        .unwrap();
    assert!(second.status.success());
    let second_json: serde_json::Value = serde_json::from_slice(&second.stdout).unwrap();

    assert_eq!(
        first_json["rebuildInputsHash"], second_json["rebuildInputsHash"],
        "rebuild hash must be stable across identical inputs"
    );
    assert_eq!(first_json["sections"], second_json["sections"]);
}

#[test]
fn state_command_does_not_mutate_signal_buckets() {
    let project = temp_project("state-buckets");
    let project_path = project.to_string_lossy().to_string();
    let project_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .unwrap_or_else(|| "ws-test".into());

    let signal = sample_signal("sig-state-bucket", &project_id, "2026-07-16T10:00:00Z");
    write_signal(&project_path, &signal).expect("write signal");
    let before = bucket_counts(&project);

    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(bucket_counts(&project), before);
}

#[test]
fn catch_up_json_outputs_valid_catch_up_view() {
    let project = temp_project("catchup-json");
    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: openmesh_core::domain::CatchUpView =
        serde_json::from_str(stdout.trim()).expect("valid catch-up json");
    assert_eq!(parsed.protocol_version, CATCH_UP_VIEW_PROTOCOL_VERSION);
    validate_catch_up_view(&parsed).expect("valid catch-up view");
}

#[test]
fn catch_up_human_output_contains_six_sections() {
    let project = temp_project("catchup-human");
    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("window: since="));
    assert!(stdout.contains("summary:"));
    assert!(stdout.contains("sections: completed="));
    assert!(stdout.contains("changed="));
    assert!(stdout.contains("blocked="));
    assert!(stdout.contains("decided="));
    assert!(stdout.contains("needs_attention="));
    assert!(stdout.contains("still_open="));
    assert!(stdout.contains("next_suggested_attention="));
}

#[test]
fn catch_up_since_filters_window() {
    let project = temp_project("catchup-since");
    let project_path = project.to_string_lossy().to_string();
    let project_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .unwrap_or_else(|| "ws-test".into());

    write_signal(
        &project_path,
        &sample_signal("sig-old", &project_id, "2020-01-01T00:00:00Z"),
    )
    .expect("write old");

    let recent_ts = (chrono::Utc::now() - chrono::Duration::minutes(30))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_signal(
        &project_path,
        &sample_signal("sig-new", &project_id, &recent_ts),
    )
    .expect("write new");

    let narrow_since = (chrono::Utc::now() - chrono::Duration::hours(2))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let narrow = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--since")
        .arg(&narrow_since)
        .arg("--json")
        .output()
        .unwrap();
    assert!(
        narrow.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&narrow.stderr)
    );
    let narrow_json: serde_json::Value = serde_json::from_slice(&narrow.stdout).unwrap();
    let changed = narrow_json["sections"]["changed"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert!(changed >= 1, "recent signal should appear in window");

    let old_only = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--since")
        .arg("2019-01-01T00:00:00Z")
        .arg("--json")
        .output()
        .unwrap();
    assert!(old_only.status.success());
    let old_json: serde_json::Value = serde_json::from_slice(&old_only.stdout).unwrap();
    let old_changed = old_json["sections"]["changed"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(0);
    assert!(
        old_changed >= changed,
        "wider window should include at least as many changed items"
    );
}

#[test]
fn catch_up_rejects_invalid_since() {
    let project = temp_project("catchup-invalid-since");
    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--since")
        .arg("not-a-timestamp")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("invalid --since"));
}

#[test]
fn catch_up_default_window_is_24h_without_checkpoint_write() {
    let project = temp_project("catchup-default-window");
    assert!(!catch_up_checkpoint_path(&project).exists());
    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(!catch_up_checkpoint_path(&project).exists());

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let since = parsed["window"]["since"].as_str().expect("since");
    let until = parsed["window"]["until"].as_str().expect("until");
    let since_dt = chrono::DateTime::parse_from_rfc3339(since).expect("parse since");
    let until_dt = chrono::DateTime::parse_from_rfc3339(until).expect("parse until");
    let delta = until_dt - since_dt;
    let hours = delta.num_hours();
    assert!(
        (23..=25).contains(&hours),
        "default window should be ~24h, got {hours}h (since={since}, until={until})"
    );
}

#[test]
fn catch_up_does_not_write_catch_up_files() {
    let project = temp_project("catchup-no-write");
    let projections = project.join(".openmesh/projections");
    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    if projections.exists() {
        let names: Vec<_> = fs::read_dir(&projections)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !names.iter().any(|n| n.contains("catch-up")),
            "catch-up must not persist view files: {names:?}"
        );
    }
    assert!(!catch_up_checkpoint_path(&project).exists());
}

#[test]
fn catch_up_does_not_mutate_signal_buckets() {
    let project = temp_project("catchup-buckets");
    let project_path = project.to_string_lossy().to_string();
    let project_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .unwrap_or_else(|| "ws-test".into());
    write_signal(
        &project_path,
        &sample_signal("sig-catchup-bucket", &project_id, "2026-07-16T10:00:00Z"),
    )
    .expect("write signal");
    let before = bucket_counts(&project);

    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(bucket_counts(&project), before);
}

#[test]
fn cli_state_and_catch_up_do_not_touch_tauri_or_desktop_surface() {
    let root = workspace_root();
    let forbidden = [
        "#[tauri::command]",
        "tauri::",
        "invoke(",
        "ContinuityIntelligence",
        "resolve_ambiguous_with_intelligence",
        "collect_git_signal",
        "collect_heli_signal",
    ];
    let cli_src = root.join("crates/openmesh-cli/src");
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    for path in files {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches!(file_name, "state.rs" | "catch_up.rs" | "main.rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read cli source");
        for term in forbidden {
            assert!(
                !content.contains(term),
                "CLI state/catch-up must not reference `{term}`: {}",
                path.display()
            );
        }
    }

    let tauri_lib = root.join("src-tauri/src/lib.rs");
    let tauri_content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
    assert_eq!(
        tauri_content.matches("#[tauri::command]").count(),
        53,
        "Tauri command count must remain 53 (get_host_os)"
    );
    for term in [
        "run_state",
        "run_catch_up",
        "CurrentStateProjection",
        "CatchUpView",
    ] {
        assert!(
            !tauri_content.contains(term),
            "Tauri must not expose continuity CLI surface `{term}`"
        );
    }
}

#[test]
fn cli_state_and_catch_up_do_not_start_0_1_3_8() {
    let root = workspace_root();
    let forbidden = [
        "0.1.3.8",
        "catch-up-checkpoint",
        "append_correction",
        "dogfood_gate",
        "evidence_correction",
        "process_pending_promotions",
        "ContinuityIntelligence",
        "AXGA",
    ];
    let cli_src = root.join("crates/openmesh-cli/src");
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    for path in files {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if !matches!(file_name, "state.rs" | "catch_up.rs" | "main.rs") {
            continue;
        }
        let content = fs::read_to_string(&path).expect("read cli source");
        for term in forbidden {
            assert!(
                !content.contains(term),
                "Checkpoint E must not start 0.1.3.8 (`{term}`): {}",
                path.display()
            );
        }
    }
}
