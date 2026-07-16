//! Dev Track 0.1.3.6 Checkpoint F — cross-producer correlation proof (read-only).

use openmesh_core::domain::{ActorRef, EvidenceRef, GitState, ProducerRef, WorkSignalKind};
use openmesh_core::promotion::{correlate_and_evaluate, PromotionOutcome, SignalRef};

fn git_state_dirty() -> GitState {
    GitState {
        repo_id: "fnv1a-2ad3a48b04b15c64".into(),
        branch: "main".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: true,
        staged_count: 0,
        unstaged_count: 1,
        untracked_count: 0,
        changed_paths: vec!["crates/openmesh-core/src/domain.rs".into()],
        observed_at: "2026-07-16T06:00:00Z".into(),
        ahead: Some(0),
        behind: Some(0),
        base_ref: None,
        worktree_root: None,
    }
}

fn signal(
    id: &str,
    producer: ProducerRef,
    kind: WorkSignalKind,
    summary: &str,
    hint: &str,
    evidence_refs: Vec<EvidenceRef>,
) -> SignalRef {
    SignalRef {
        signal_id: id.into(),
        kind,
        summary: summary.into(),
        producer,
        actor: ActorRef::Unknown,
        timestamp: "2026-07-16T06:00:00Z".into(),
        correlation_hint: Some(hint.into()),
        evidence_refs,
    }
}

#[test]
fn cross_producer_corroboration_fixture_groups_three_producers() {
    let hint = "proof-0.1.3.6-cross";
    let shared_summary = "evidence producers active on domain contracts";
    let signals = vec![
        signal(
            "sig-reporter",
            ProducerRef::Reporter("test".into()),
            WorkSignalKind::Progress,
            shared_summary,
            hint,
            vec![EvidenceRef::FilePath(
                "crates/openmesh-core/src/domain.rs".into(),
            )],
        ),
        signal(
            "sig-git",
            ProducerRef::Git,
            WorkSignalKind::Progress,
            shared_summary,
            hint,
            vec![EvidenceRef::GitState(git_state_dirty())],
        ),
        signal(
            "sig-heli",
            ProducerRef::Heli,
            WorkSignalKind::Progress,
            shared_summary,
            hint,
            vec![EvidenceRef::FilePath(
                ".heli-harness/state/current-task.md".into(),
            )],
        ),
    ];

    let result = correlate_and_evaluate("ws-proof", &signals).expect("correlate");
    assert_eq!(result.correlation.groups.len(), 1);
    assert_eq!(result.correlation.groups[0].signals.len(), 3);

    let producers: Vec<_> = result.correlation.groups[0]
        .signals
        .iter()
        .map(|s| s.producer.clone())
        .collect();
    assert!(producers.contains(&ProducerRef::Reporter("test".into())));
    assert!(producers.contains(&ProducerRef::Git));
    assert!(producers.contains(&ProducerRef::Heli));

    let decision = result
        .decisions
        .iter()
        .find(|d| d.group.correlation_hint.as_deref() == Some(hint))
        .expect("decision for corroboration group");
    assert!(decision.corroborating_signal_ids.len() >= 2);
    assert_eq!(
        result.correlation.corroboration_refs.len(),
        1,
        "cross-producer corroboration must be visible"
    );
}

#[test]
fn cross_producer_contradiction_fixture_does_not_blindly_promote() {
    let hint = "proof-0.1.3.6-contradiction";
    let signals = vec![
        signal(
            "sig-reporter-claim",
            ProducerRef::Reporter("test".into()),
            WorkSignalKind::Milestone,
            "Migration complete and ready to ship",
            hint,
            vec![EvidenceRef::FilePath("docs/release.md".into())],
        ),
        signal(
            "sig-git-dirty",
            ProducerRef::Git,
            WorkSignalKind::ScopeChange,
            "Git shows unpushed scope-impacting work remains",
            hint,
            vec![EvidenceRef::GitState(git_state_dirty())],
        ),
        signal(
            "sig-heli-active",
            ProducerRef::Heli,
            WorkSignalKind::Progress,
            "Heli: task=Dev Track 0.1.3.6 active checkpoint",
            hint,
            vec![EvidenceRef::FilePath(
                ".heli-harness/state/current-task.md".into(),
            )],
        ),
    ];

    let result = correlate_and_evaluate("ws-proof", &signals).expect("correlate");
    let decision = result
        .decisions
        .iter()
        .find(|d| d.group.correlation_hint.as_deref() == Some(hint))
        .expect("contradiction decision");
    assert_eq!(decision.decision.outcome, PromotionOutcome::Ambiguous);
}

#[test]
fn cross_producer_evidence_has_no_full_source_or_diff_bodies() {
    let hint = "proof-0.1.3.6-bounds";
    let signals = vec![signal(
        "sig-git-bounds",
        ProducerRef::Git,
        WorkSignalKind::Progress,
        "Git snapshot",
        hint,
        vec![EvidenceRef::GitState(git_state_dirty())],
    )];
    let json = serde_json::to_string(&signals[0]).expect("serialize");
    assert!(!json.contains("diffBody"));
    assert!(!json.contains("SECRET"));
    assert!(json.contains("changedPaths"));
}
