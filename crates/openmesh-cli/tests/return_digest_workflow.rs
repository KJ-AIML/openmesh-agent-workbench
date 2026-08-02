//! Dev Track 0.1.9 — pending + digest CLI workflow tests.

use openmesh_core::authority_policy::{AuthorityPolicyDecision, QuestionRiskCategory};
use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{
    ActorRef, EvidenceAttachment, EvidenceRef, ProducerRef, ProxyAuthorityLevel, WorkEvent,
    WorkSignal, WorkSignalKind, WORK_SIGNAL_PROTOCOL_VERSION,
};
use openmesh_core::events::append_event;
use openmesh_core::pending_proxy_question::write_pending_proxy_question;
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-return-digest-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
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

fn workspace_id(project: &Path) -> String {
    let raw = std::fs::read_to_string(project.join(".openmesh/project.json")).unwrap();
    let v: Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[test]
fn top_level_help_lists_pending_and_digest() {
    let help = String::from_utf8_lossy(&run_raw(&["--help"]).stdout).to_ascii_lowercase();
    assert!(help.contains("pending"), "help should list pending");
    assert!(help.contains("digest"), "help should list digest");
}

#[test]
fn pending_json_lists_proxy_question() {
    let project = temp_project("pending-json");
    let decision = AuthorityPolicyDecision {
        resolved_authority: ProxyAuthorityLevel::MustAskHuman,
        deny_before_provider: true,
        deny_reason: Some("human gate".into()),
        evidence_required: true,
        human_confirmation_required: true,
        freshness_tier: openmesh_core::authority_policy::FreshnessTier::Standard,
        decision_reason: "must ask".into(),
        matched_rule_ids: vec!["test".into()],
    };
    write_pending_proxy_question(
        &project.to_string_lossy(),
        "Ship 0.1.9?",
        QuestionRiskCategory::Decision,
        &decision,
        "2026-08-01T12:00:00Z",
    )
    .expect("write");

    let out = run(&["pending", "--json"], &project);
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(payload["protocolVersion"], "1.0");
    assert!(payload["openCount"].as_u64().unwrap_or(0) >= 1);
    assert!(payload["items"].as_array().unwrap().iter().any(|item| {
        item["source"] == "proxy-pending" && item["summary"].as_str() == Some("Ship 0.1.9?")
    }));
}

#[test]
fn digest_json_covers_absence_window() {
    let project = temp_project("digest-json");
    let ws = workspace_id(&project);
    let path = project.to_string_lossy().to_string();

    append_event(
        &path,
        &WorkEvent::new(
            "evt-cli-digest-1",
            &ws,
            "work.completed",
            "Completed pending projection",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("src/return_digest/mod.rs".into()),
                observed_at: None,
            }],
            "2026-08-01T14:00:00Z",
        ),
    )
    .expect("event");

    write_signal(
        &path,
        &WorkSignal {
            signal_id: "sig-cli-unresolved".into(),
            workspace_id: ws,
            producer: ProducerRef::Reporter("cli".into()),
            actor: ActorRef::Unknown,
            kind: WorkSignalKind::UnresolvedQuestion,
            summary: "Confirm digest fields?".into(),
            timestamp: "2026-08-01T15:00:00Z".into(),
            evidence_refs: vec![],
            correlation_hint: None,
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.into(),
        },
    )
    .expect("signal");

    let out = run(
        &[
            "digest",
            "--json",
            "--since",
            "2026-08-01T00:00:00Z",
        ],
        &project,
    );
    assert!(
        out.status.success(),
        "stderr={} stdout={}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let payload: Value = serde_json::from_slice(&out.stdout).expect("json");
    assert_eq!(payload["protocolVersion"], "1.0");
    assert_eq!(payload["window"]["since"], "2026-08-01T00:00:00Z");
    assert!(payload["summary"].as_str().unwrap_or("").contains("need you"));
    assert!(
        payload["needsMe"].as_array().map(|a| !a.is_empty()).unwrap_or(false),
        "expected needsMe items"
    );
}
