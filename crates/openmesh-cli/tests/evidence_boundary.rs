//! Dev Track 0.1.3.7 Checkpoint F — CLI evidence / ambiguity boundary proofs.

use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{
    validate_catch_up_view, validate_current_state_projection, ActorRef, EvidenceRef, ProducerRef,
    WorkSignal, WorkSignalKind,
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
        "openmesh-cli-evidence-boundary-{label}-{}-{n}",
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

fn sample_signal(id: &str, workspace_id: &str) -> WorkSignal {
    WorkSignal {
        signal_id: id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("boundary-test".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::ReviewRequired,
        summary: format!("needs review: {id}"),
        timestamp: "2026-07-16T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/boundary.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn project_id(project: &Path) -> String {
    let raw = fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    json["id"].as_str().unwrap().to_string()
}

const AI_OVERCLAIM_TERMS: &[&str] = &[
    "AI summary",
    "LLM",
    "artificial intelligence",
    "generated summary",
    "definitely completed",
    "100% certain",
    "AXGA",
];

#[test]
fn cli_state_json_preserves_evidence_and_limitations() {
    let project = temp_project("state-json-evidence");
    let project_path = project.to_string_lossy().to_string();
    let pid = project_id(&project);
    write_signal(&project_path, &sample_signal("cli-ev-1", &pid)).expect("write");

    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: openmesh_core::domain::CurrentStateProjection =
        serde_json::from_slice(&output.stdout).expect("valid json");
    validate_current_state_projection(&parsed).expect("valid projection");

    assert!(!parsed.evidence_refs.is_empty() || !parsed.limitations.is_empty());
    assert!(parsed.source_counts.pending_signals >= 1 || !parsed.pending_attention.is_empty());
}

#[test]
fn cli_catch_up_json_preserves_evidence_and_limitations() {
    let project = temp_project("catchup-json-evidence");
    let project_path = project.to_string_lossy().to_string();
    let pid = project_id(&project);
    write_signal(&project_path, &sample_signal("cli-cu-1", &pid)).expect("write");

    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .arg("--json")
        .output()
        .unwrap();
    assert!(output.status.success());
    let parsed: openmesh_core::domain::CatchUpView =
        serde_json::from_slice(&output.stdout).expect("valid json");
    validate_catch_up_view(&parsed).expect("valid catch-up");

    assert!(!parsed.summary.is_empty());
    assert!(!parsed.next_suggested_attention.is_empty() || !parsed.limitations.is_empty());
    assert!(parsed.window.since.contains('T'));
    assert!(parsed.window.until.contains('T'));
}

#[test]
fn cli_human_state_does_not_claim_ai_summary_or_certainty() {
    let project = temp_project("state-human-boundary");
    let project_path = project.to_string_lossy().to_string();
    let pid = project_id(&project);
    write_signal(&project_path, &sample_signal("cli-h-1", &pid)).expect("write");

    let output = cli()
        .args(["state", "--project"])
        .arg(&project)
        .arg("--rebuild")
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();

    for term in AI_OVERCLAIM_TERMS {
        assert!(
            !stdout.contains(&term.to_lowercase()),
            "state human output must not overclaim (`{term}`)"
        );
    }
    assert!(stdout.contains("limitations="));
    assert!(stdout.contains("pending_attention="));
    assert!(stdout.contains("source_counts:"));
    assert!(stdout.contains("sections:"));
}

#[test]
fn cli_human_catch_up_does_not_claim_ai_summary_or_certainty() {
    let project = temp_project("catchup-human-boundary");
    let project_path = project.to_string_lossy().to_string();
    let pid = project_id(&project);
    write_signal(&project_path, &sample_signal("cli-ch-1", &pid)).expect("write");

    let output = cli()
        .args(["catch-up", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();

    for term in AI_OVERCLAIM_TERMS {
        assert!(
            !stdout.contains(&term.to_lowercase()),
            "catch-up human output must not overclaim (`{term}`)"
        );
    }
    assert!(stdout.contains("limitations="));
    assert!(stdout.contains("next_suggested_attention="));
    assert!(stdout.contains("evidence_refs="));
    assert!(stdout.contains("window: since="));
    assert!(stdout.contains("summary:"));
}
