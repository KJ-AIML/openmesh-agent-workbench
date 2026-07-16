//! Dev Track 0.1.3.6 Checkpoint A — producer domain contract tests (pure, no I/O).

use openmesh_core::domain::{
    bound_git_changed_paths, validate_evidence_ref, validate_git_state,
    validate_work_signal_semantics, ActorRef, EvidenceRef, GitProducerError, GitProducerResult,
    GitSnapshot, GitState, HeliProducerError, HeliProducerResult, HeliSnapshot, ProducerRef,
    ProducerSkipReason, SignalValidationError, WorkSignal, WorkSignalKind,
    MAX_GIT_STATE_CHANGED_PATHS, WORK_SIGNAL_PROTOCOL_VERSION,
    WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
};

fn sample_git_state() -> GitState {
    GitState {
        repo_id: "fnv1a-2ad3a48b04b15c64b82e2bc".into(),
        branch: "feat/openmesh-0.1.3".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: true,
        staged_count: 0,
        unstaged_count: 1,
        untracked_count: 0,
        changed_paths: vec!["crates/openmesh-core/src/domain.rs".into()],
        observed_at: "2026-07-16T04:30:00Z".into(),
        ahead: Some(0),
        behind: Some(0),
        base_ref: Some("origin/feat/openmesh-0.1.3".into()),
        worktree_root: None,
    }
}

fn base_signal(protocol_version: &str, evidence_refs: Vec<EvidenceRef>) -> WorkSignal {
    WorkSignal {
        signal_id: "git-producer-001".into(),
        workspace_id: "1783586870822-7352d".into(),
        producer: ProducerRef::Git,
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: "Git: branch=feat/openmesh-0.1.3 head=2ad3a48 dirty=true changed=1".into(),
        timestamp: "2026-07-16T04:30:00Z".into(),
        evidence_refs,
        correlation_hint: None,
        sensitivity: openmesh_core::context::Sensitivity::Private,
        protocol_version: protocol_version.to_string(),
    }
}

#[test]
fn git_state_evidence_round_trips_json() {
    let evidence = EvidenceRef::GitState(sample_git_state());
    let json = serde_json::to_string(&evidence).expect("serialize");
    let restored: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, evidence);
    assert!(json.contains("\"type\":\"git-state\""));
    assert!(json.contains("\"repoId\""));
    assert!(json.contains("\"changedPaths\""));

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture = format!("{manifest_dir}/tests/fixtures/producers/git-state-valid.json");
    let fixture_json = std::fs::read_to_string(&fixture).expect("read fixture");
    let from_fixture: EvidenceRef = serde_json::from_str(&fixture_json).expect("fixture");
    validate_evidence_ref(&from_fixture).expect("fixture valid");
}

#[test]
fn git_state_evidence_requires_protocol_1_1() {
    let signal = base_signal(
        WORK_SIGNAL_PROTOCOL_VERSION,
        vec![EvidenceRef::GitState(sample_git_state())],
    );
    assert_eq!(
        validate_work_signal_semantics(&signal),
        Err(SignalValidationError::Protocol10WithGitState)
    );
}

#[test]
fn work_signal_protocol_1_0_without_git_state_remains_valid() {
    let signal = base_signal(
        WORK_SIGNAL_PROTOCOL_VERSION,
        vec![EvidenceRef::FilePath("docs/overview.md".into())],
    );
    validate_work_signal_semantics(&signal).expect("1.0 without git-state");
}

#[test]
fn work_signal_protocol_1_1_with_git_state_is_valid() {
    let signal = base_signal(
        WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
        vec![EvidenceRef::GitState(sample_git_state())],
    );
    validate_work_signal_semantics(&signal).expect("1.1 with git-state");
}

#[test]
fn work_signal_unknown_protocol_is_rejected() {
    let signal = base_signal("99.0", vec![]);
    assert_eq!(
        validate_work_signal_semantics(&signal),
        Err(SignalValidationError::UnsupportedProtocolVersion {
            found: "99.0".into(),
        })
    );
}

#[test]
fn git_state_changed_paths_are_bounded_to_64() {
    let paths: Vec<String> = (0..80).map(|i| format!("path/{i}/file.rs")).collect();
    let bounded = bound_git_changed_paths(paths.clone());
    assert_eq!(bounded.len(), MAX_GIT_STATE_CHANGED_PATHS);

    let mut state = sample_git_state();
    state.changed_paths = paths;
    assert!(validate_git_state(&state).is_err());

    state.changed_paths = bounded;
    validate_git_state(&state).expect("bounded paths valid");
}

#[test]
fn git_state_rejects_full_diff_or_source_content_fields() {
    let json = r#"{
        "type": "git-state",
        "value": {
            "repoId": "fnv1a-abc123",
            "branch": "main",
            "head": "2ad3a48b04b15c64b82e2bc7c1db36b41503c571",
            "dirty": true,
            "stagedCount": 0,
            "unstagedCount": 1,
            "untrackedCount": 0,
            "changedPaths": ["a.rs"],
            "observedAt": "2026-07-16T04:30:00Z",
            "diffBody": "forbidden patch text"
        }
    }"#;
    let result: Result<EvidenceRef, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "unknown diff/source fields must be rejected at deserialize time"
    );
}

#[test]
fn git_state_observed_at_must_be_utc_rfc3339() {
    let mut state = sample_git_state();
    state.observed_at = "2026-07-16T04:30:00-05:00".into();
    assert!(validate_git_state(&state).is_err());

    state.observed_at = "2026-07-16T04:30:00Z".into();
    validate_git_state(&state).expect("utc Z accepted");
}

#[test]
fn existing_file_path_evidence_still_valid() {
    let evidence = EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into());
    validate_evidence_ref(&evidence).expect("file-path still valid");
    let signal = base_signal(WORK_SIGNAL_PROTOCOL_VERSION, vec![evidence]);
    validate_work_signal_semantics(&signal).expect("1.0 file-path signal");
}

#[test]
fn existing_producer_signal_evidence_still_valid() {
    let evidence = EvidenceRef::ProducerSignal("s-verify".into());
    validate_evidence_ref(&evidence).expect("producer-signal still valid");
    let signal = base_signal(WORK_SIGNAL_PROTOCOL_VERSION, vec![evidence]);
    validate_work_signal_semantics(&signal).expect("1.0 producer-signal signal");
}

#[test]
fn producer_contract_types_are_pure_no_io() {
    let git_snapshot: GitSnapshot = sample_git_state();
    let git_result = GitProducerResult::Snapshot(git_snapshot);
    let git_skip = GitProducerResult::Skip(ProducerSkipReason::GitUnavailable);
    let git_err = GitProducerResult::Err(GitProducerError::NotARepository);

    let heli_snapshot = HeliSnapshot {
        current_task_excerpt: Some("Dev Track 0.1.3.6".into()),
        decisions_tail_excerpt: None,
        latest_report_path: None,
        observed_at: "2026-07-16T04:30:00Z".into(),
    };
    let heli_result = HeliProducerResult::Snapshot(heli_snapshot);
    let heli_skip = HeliProducerResult::Skip(ProducerSkipReason::HeliAbsent);
    let heli_err = HeliProducerResult::Err(HeliProducerError::ReadFailed("bounded".into()));

    assert!(matches!(git_result, GitProducerResult::Snapshot(_)));
    assert!(matches!(git_skip, GitProducerResult::Skip(_)));
    assert!(matches!(git_err, GitProducerResult::Err(_)));
    assert!(matches!(heli_result, HeliProducerResult::Snapshot(_)));
    assert!(matches!(heli_skip, HeliProducerResult::Skip(_)));
    assert!(matches!(heli_err, HeliProducerResult::Err(_)));
}

#[test]
fn no_inbox_files_created_by_checkpoint_a() {
    let signal = base_signal(
        WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
        vec![EvidenceRef::GitState(sample_git_state())],
    );
    validate_work_signal_semantics(&signal).expect("valid");
    let _json = serde_json::to_string(&signal).expect("serialize");
}
