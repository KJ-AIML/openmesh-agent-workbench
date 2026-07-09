// Pre-dogfood reconciliation, Phase 2 — JSON output safety.
// Proves the emitted --json output remains valid JSON, via serde_json's own
// structural serialization, for adversarial content the Reporter Skill's
// --json consumer must be able to rely on: quotes, backslashes, embedded
// newlines, and real Windows path separators.

use openmesh_core::storage::init_project;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-json-safety-{label}-{}-{n}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init_project should succeed");
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

#[test]
fn summary_with_quote_and_backslash_round_trips_through_valid_json() {
    let project = temp_project("quote-backslash");
    let summary = r#"she said "ship it" then C:\Users\ter\notes.txt was mentioned"#;
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            summary,
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Must parse as valid JSON despite the quote/backslash content — proves
    // serde_json's own escaping, not manual string concatenation, is in use.
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("output must be valid JSON");
    assert_eq!(value["status"], "ok");
    assert!(value["signal_id"].is_string());
}

#[test]
fn summary_with_embedded_newline_round_trips_through_valid_json() {
    let project = temp_project("newline");
    let summary = "line one\nline two";
    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            summary,
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("output must be valid JSON despite embedded newline in summary");
    assert_eq!(value["status"], "ok");
}

#[test]
fn windows_path_with_backslashes_in_project_field_is_valid_json_and_round_trips_exactly() {
    let project = temp_project("winpath");
    // A real Windows temp path already contains backslashes (e.g. C:\Users\...\Temp\...).
    let project_str = project.to_string_lossy().to_string();
    assert!(
        project_str.contains('\\'),
        "test precondition: expected a real Windows path with backslashes, got {project_str}"
    );

    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "path safety check",
            "--json",
            "--project",
        ])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("output must be valid JSON despite backslashes in the project path");
    // The parsed value must equal the real path exactly — proving the
    // backslashes were escaped for the wire and correctly unescaped back,
    // not corrupted or double-escaped.
    assert_eq!(value["project"].as_str().unwrap(), project_str);
}

#[test]
fn failure_message_containing_a_windows_path_is_valid_json() {
    // Project-resolution failures embed the (backslash-containing) path
    // directly into the JSON "message" field — the one failure path where
    // path content reliably appears in an error message.
    let not_a_project = temp_project("failure-path-parent").join("never-initialized");
    fs::create_dir_all(&not_a_project).unwrap();
    let project_str = not_a_project.to_string_lossy().to_string();
    assert!(project_str.contains('\\'));

    let output = cli()
        .args([
            "signal",
            "progress",
            "--summary",
            "x",
            "--json",
            "--project",
        ])
        .arg(&not_a_project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("failure output must be valid JSON despite a Windows path in the message");
    assert_eq!(value["status"], "error");
    assert_eq!(value["category"], "project-resolution");
    assert!(value["message"].as_str().unwrap().contains(&project_str));
}
