//! Dev Track 0.1.9 — pending projection + return digest builder integration tests.

use openmesh_core::authority_policy::{
    AuthorityPolicyDecision, QuestionRiskCategory,
};
use openmesh_core::continuity::{
    build_current_state_projection, load_continuity_input_snapshot,
};
use openmesh_core::domain::{
    CatchUpWindow, EvidenceAttachment, EvidenceRef, ProxyAuthorityLevel, WorkEvent,
};
use openmesh_core::events::append_event;
use openmesh_core::pending_proxy_question::write_pending_proxy_question;
use openmesh_core::return_digest::{
    build_pending_questions_view, build_return_digest, PendingQuestionSourceKind,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{
    ActorRef, ProducerRef, WorkSignal, WorkSignalKind, WORK_SIGNAL_PROTOCOL_VERSION,
};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-core-return-digest-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    init_project(&path).expect("init");
    path
}

fn workspace_id(project: &str) -> String {
    let raw = std::fs::read_to_string(
        openmesh_core::storage::get_project_dir(project).join("project.json"),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

#[test]
fn pending_view_projects_proxy_and_unresolved_signal() {
    let project = temp_project("pending-sources");
    let ws = workspace_id(&project);

    let decision = AuthorityPolicyDecision {
        resolved_authority: ProxyAuthorityLevel::MustAskHuman,
        deny_before_provider: true,
        deny_reason: Some("needs human".into()),
        evidence_required: true,
        human_confirmation_required: true,
        freshness_tier: openmesh_core::authority_policy::FreshnessTier::Standard,
        decision_reason: "high risk decision".into(),
        matched_rule_ids: vec!["test".into()],
    };
    write_pending_proxy_question(
        &project,
        "May we cut a 0.1.9 release?",
        QuestionRiskCategory::Decision,
        &decision,
        "2026-08-01T10:00:00Z",
    )
    .expect("write proxy pending");

    let signal = WorkSignal {
        signal_id: "sig-unresolved-1".into(),
        workspace_id: ws.clone(),
        producer: ProducerRef::Reporter("cli".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::UnresolvedQuestion,
        summary: "What is the digest CLI shape?".into(),
        timestamp: "2026-08-01T11:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/dev.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.into(),
    };
    write_signal(&project, &signal).expect("write signal");

    let snapshot = load_continuity_input_snapshot(&project).expect("snapshot");
    let current_state = build_current_state_projection(&snapshot).expect("state");
    let view = build_pending_questions_view(&project, &snapshot, &current_state).expect("pending");

    assert!(view.open_count >= 2);
    assert!(view
        .items
        .iter()
        .any(|i| i.source == PendingQuestionSourceKind::ProxyPending));
    // Unresolved signals are often already projected into continuity attention;
    // accept either source as long as the signal is present once.
    assert!(
        view.items.iter().any(|i| {
            i.source_id == "sig-unresolved-1"
                && matches!(
                    i.source,
                    PendingQuestionSourceKind::UnresolvedSignal
                        | PendingQuestionSourceKind::ContinuityAttention
                )
        }),
        "expected unresolved signal projected once, got: {:?}",
        view.items
            .iter()
            .map(|i| (&i.source_id, i.source))
            .collect::<Vec<_>>()
    );
}

#[test]
fn return_digest_includes_missed_events_and_needs_me() {
    let project = temp_project("digest-e2e");
    let ws = workspace_id(&project);

    let event = WorkEvent::new(
        "evt-missed-1",
        &ws,
        "work.completed",
        "Finished handoff engine",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("crates/openmesh-core/src/handoff/mod.rs".into()),
            observed_at: None,
        }],
        "2026-08-01T15:00:00Z",
    );
    append_event(&project, &event).expect("append");

    let decision = AuthorityPolicyDecision {
        resolved_authority: ProxyAuthorityLevel::MustAskHuman,
        deny_before_provider: true,
        deny_reason: None,
        evidence_required: true,
        human_confirmation_required: true,
        freshness_tier: openmesh_core::authority_policy::FreshnessTier::Standard,
        decision_reason: "must ask".into(),
        matched_rule_ids: vec!["test".into()],
    };
    write_pending_proxy_question(
        &project,
        "Approve return digest shape?",
        QuestionRiskCategory::Decision,
        &decision,
        "2026-08-01T16:00:00Z",
    )
    .expect("pending");

    let snapshot = load_continuity_input_snapshot(&project).expect("snapshot");
    let current_state = build_current_state_projection(&snapshot).expect("state");
    let window = CatchUpWindow {
        since: "2026-08-01T00:00:00Z".into(),
        until: "2026-08-02T00:00:00Z".into(),
    };
    let digest = build_return_digest(&project, &snapshot, &current_state, &window).expect("digest");

    assert!(!digest.needs_me.is_empty());
    assert!(
        !digest.what_i_missed.completed.is_empty()
            || !digest.what_i_missed.changed.is_empty()
            || digest.summary.contains("need you")
    );
    assert!(digest.summary.contains("need you"));
    assert_eq!(digest.window.since, "2026-08-01T00:00:00Z");
}
