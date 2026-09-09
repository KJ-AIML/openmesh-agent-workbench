use super::*;
use crate::context::Sensitivity;

fn signal(
    id: &str,
    producer: ProducerRef,
    actor: ActorRef,
    kind: WorkSignalKind,
    correlation_hint: Option<&str>,
    evidence_refs: Vec<EvidenceRef>,
) -> WorkSignal {
    WorkSignal {
        signal_id: id.to_string(),
        workspace_id: "ws-1".to_string(),
        producer,
        actor,
        kind,
        summary: format!("signal {id}"),
        timestamp: "2026-07-08T00:00:00Z".to_string(),
        evidence_refs,
        correlation_hint: correlation_hint.map(|s| s.to_string()),
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
    }
}

fn attachment(evidence_ref: EvidenceRef) -> EvidenceAttachment {
    EvidenceAttachment {
        evidence_ref,
        observed_at: None,
    }
}

fn minimal_event(
    kind: &str,
    summary: &str,
    evidence_refs: Vec<EvidenceRef>,
    timestamp: &str,
) -> WorkEvent {
    WorkEvent::new(
        "evt-test-1",
        "ws-1",
        kind,
        summary,
        evidence_refs.into_iter().map(attachment).collect(),
        timestamp,
    )
}

// Category A — Positive Representability Tests (execution plan §6/§9.D1).

/// WEC-29: an agent's completion claim, a verification result, and a
/// human's acceptance exist as three separately-attributed records linked
/// by a shared correlation hint — not one record with a boolean flag.
#[test]
fn wec_29_claim_verification_and_human_acceptance_are_distinct_records() {
    let claim = signal(
        "s-claim",
        ProducerRef::Reporter("codex".into()),
        ActorRef::Unknown,
        WorkSignalKind::Progress,
        Some("corr-1"),
        vec![],
    );
    let verification = signal(
        "s-verify",
        ProducerRef::Native,
        ActorRef::Unknown,
        WorkSignalKind::Progress,
        Some("corr-1"),
        vec![EvidenceRef::FilePath("tests/output.log".into())],
    );
    let acceptance = signal(
        "s-accept",
        ProducerRef::Native,
        ActorRef::Person("ter".into()),
        WorkSignalKind::Decision,
        Some("corr-1"),
        vec![EvidenceRef::ProducerSignal("s-verify".into())],
    );

    // All three share the same correlation hint, so they're correlatable...
    assert_eq!(claim.correlation_hint, verification.correlation_hint);
    assert_eq!(verification.correlation_hint, acceptance.correlation_hint);
    // ...but remain three distinct records, not one record with a status flag.
    assert_ne!(claim.signal_id, verification.signal_id);
    assert_ne!(verification.signal_id, acceptance.signal_id);
    // Acceptance is distinctly human-attributed; the claim is not.
    assert!(matches!(acceptance.actor, ActorRef::Person(_)));
    assert!(!matches!(claim.actor, ActorRef::Person(_)));
}

/// CC-1: many signals may support one WorkEvent, and producer count does
/// not determine event count — both a single event with several evidence
/// refs and several single-evidence events must be constructible.
#[test]
fn cc_1_many_signals_can_support_one_event_or_several() {
    let refs: Vec<EvidenceRef> = (0..4)
        .map(|i| EvidenceRef::ProducerSignal(format!("s-{i}")))
        .collect();

    let one_event_many_refs = minimal_event(
        "bugfix",
        "clear_project fixed",
        refs.clone(),
        "2026-07-08T00:00:00Z",
    );
    assert_eq!(one_event_many_refs.evidence.len(), 4);

    let four_events: Vec<WorkEvent> = refs
        .into_iter()
        .map(|r| {
            minimal_event(
                "bugfix",
                "clear_project fixed",
                vec![r],
                "2026-07-08T00:00:00Z",
            )
        })
        .collect();
    assert_eq!(four_events.len(), 4);
    assert!(four_events.iter().all(|e| e.evidence.len() == 1));
}

/// WEC-26 + WEC-32: same-producer duplicate vs. cross-producer
/// corroboration must be distinguishable data, not the same shape.
#[test]
fn wec_26_32_duplicate_vs_corroborating_signals_are_distinguishable() {
    let duplicate_a = signal(
        "s-dup-a",
        ProducerRef::Reporter("codex".into()),
        ActorRef::Unknown,
        WorkSignalKind::Milestone,
        Some("corr-dup"),
        vec![],
    );
    let duplicate_b = signal(
        "s-dup-b",
        ProducerRef::Reporter("codex".into()),
        ActorRef::Unknown,
        WorkSignalKind::Milestone,
        Some("corr-dup"),
        vec![],
    );
    assert_eq!(duplicate_a.producer, duplicate_b.producer);

    let corroborating_a = signal(
        "s-corr-a",
        ProducerRef::Native,
        ActorRef::Unknown,
        WorkSignalKind::Milestone,
        Some("corr-shared"),
        vec![],
    );
    let corroborating_b = signal(
        "s-corr-b",
        ProducerRef::Git,
        ActorRef::Unknown,
        WorkSignalKind::Milestone,
        Some("corr-shared"),
        vec![],
    );
    assert_ne!(corroborating_a.producer, corroborating_b.producer);
    assert_eq!(
        corroborating_a.correlation_hint,
        corroborating_b.correlation_hint
    );
}

/// WEC-15: a claim existing only in conversation, not yet backed by
/// durable evidence, is representable as a WorkSignal with no evidence
/// refs — distinct from (and never auto-promoted to) a WorkEvent.
#[test]
fn wec_15_conversation_only_claim_has_no_evidence_refs() {
    let conversation_only = signal(
        "s-convo",
        ProducerRef::Native,
        ActorRef::Person("ter".into()),
        WorkSignalKind::Decision,
        None,
        vec![],
    );
    assert!(conversation_only.evidence_refs.is_empty());
}

/// WEC-30: a genuinely ambiguous case (one event or six?) must remain
/// representable either way — the contracts must not resolve ambiguity.
#[test]
fn wec_30_ambiguous_one_event_or_six_both_remain_constructible() {
    let refs: Vec<EvidenceRef> = (0..6)
        .map(|i| EvidenceRef::ProducerSignal(format!("round-{i}")))
        .collect();

    let one_event = minimal_event("stabilized", "0.1.2.5 stabilized", refs.clone(), "t");
    assert_eq!(one_event.evidence.len(), 6);

    let six_events: Vec<WorkEvent> = refs
        .into_iter()
        .map(|r| minimal_event("bugfix-round", "one closure round", vec![r], "t"))
        .collect();
    assert_eq!(six_events.len(), 6);
}

// Generic structural test (not a Classification Pack case). WEC-33
// (Git-state evidence representability) is implemented in Dev Track 0.1.3.6
// Checkpoint A — see `producer_contracts.rs`.
#[test]
fn evidence_ref_variants_construct_and_compare() {
    let a = EvidenceRef::FilePath("docs/overview.md".into());
    let b = EvidenceRef::FilePath("docs/overview.md".into());
    let c = EvidenceRef::ProducerSignal("s-1".into());
    let d = EvidenceRef::GitState(sample_git_state());
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_ne!(c, d);
}

fn sample_git_state() -> GitState {
    GitState {
        repo_id: "fnv1a-abc123".into(),
        branch: "main".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: true,
        staged_count: 0,
        unstaged_count: 1,
        untracked_count: 0,
        changed_paths: vec!["crates/openmesh-core/src/domain.rs".into()],
        observed_at: "2026-07-16T04:30:00Z".into(),
        ahead: None,
        behind: None,
        base_ref: None,
        worktree_root: None,
    }
}

// ------------------------------------------------------------------
// Dev Track 0.1.3.2, Checkpoint A — protocol contract stabilization.
// ------------------------------------------------------------------

/// Full WorkSignal round-trips through JSON, preserving every field.
#[test]
fn work_signal_round_trips_through_json() {
    let original = signal(
        "s-roundtrip",
        ProducerRef::Reporter("codex".into()),
        ActorRef::Person("ter".into()),
        WorkSignalKind::Handoff,
        Some("corr-rt"),
        vec![
            EvidenceRef::FilePath("docs/overview.md".into()),
            EvidenceRef::ProducerSignal("s-other".into()),
        ],
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: WorkSignal = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.signal_id, original.signal_id);
    assert_eq!(restored.workspace_id, original.workspace_id);
    assert_eq!(restored.producer, original.producer);
    assert_eq!(restored.actor, original.actor);
    assert_eq!(restored.kind, original.kind);
    assert_eq!(restored.summary, original.summary);
    assert_eq!(restored.timestamp, original.timestamp);
    assert_eq!(restored.evidence_refs, original.evidence_refs);
    assert_eq!(restored.correlation_hint, original.correlation_hint);
    assert_eq!(restored.sensitivity, original.sensitivity);
    assert_eq!(restored.protocol_version, original.protocol_version);
}

/// Every data-carrying enum variant round-trips, including all unit variants.
#[test]
fn producer_ref_variants_round_trip() {
    for p in [
        ProducerRef::Native,
        ProducerRef::Heli,
        ProducerRef::Git,
        ProducerRef::Reporter("codex".into()),
    ] {
        let json = serde_json::to_string(&p).expect("serialize");
        let back: ProducerRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, p);
    }
}

#[test]
fn actor_ref_variants_round_trip() {
    for a in [
        ActorRef::Person("ter".into()),
        ActorRef::Device("laptop-1".into()),
        ActorRef::Proxy("proxy-1".into()),
        ActorRef::Unknown,
    ] {
        let json = serde_json::to_string(&a).expect("serialize");
        let back: ActorRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, a);
    }
}

#[test]
fn evidence_ref_variants_round_trip() {
    for e in [
        EvidenceRef::FilePath("docs/overview.md".into()),
        EvidenceRef::ProducerSignal("s-1".into()),
        EvidenceRef::GitState(sample_git_state()),
    ] {
        let json = serde_json::to_string(&e).expect("serialize");
        let back: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, e);
    }
}

#[test]
fn work_signal_kind_variants_round_trip() {
    for k in [
        WorkSignalKind::Progress,
        WorkSignalKind::Decision,
        WorkSignalKind::Blocker,
        WorkSignalKind::BlockerResolved,
        WorkSignalKind::ScopeChange,
        WorkSignalKind::Milestone,
        WorkSignalKind::ReviewRequired,
        WorkSignalKind::UnresolvedQuestion,
        WorkSignalKind::Handoff,
        WorkSignalKind::SessionEnd,
        WorkSignalKind::AgentSwitch,
    ] {
        let json = serde_json::to_string(&k).expect("serialize");
        let back: WorkSignalKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, k);
    }
}

/// Locks the exact wire shape approved in the 0.1.3.2 execution plan §3.12:
/// camelCase fields, kebab-case kind, lowercase sensitivity, adjacently
/// tagged data-carrying enums. Any accidental future drift in field
/// naming, casing, or enum representation fails this test immediately.
#[test]
fn deserializes_the_canonical_example_json_fixture() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{}/tests/fixtures/signals/valid.json", manifest_dir);
    let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
    let s: WorkSignal = serde_json::from_str(&json).expect("deserialize fixture");

    assert_eq!(s.signal_id, "codex-2026-07-08-001");
    assert_eq!(s.workspace_id, "1720400000000-a1b2c3");
    assert_eq!(s.producer, ProducerRef::Reporter("codex".into()));
    assert_eq!(s.actor, ActorRef::Unknown);
    assert_eq!(s.kind, WorkSignalKind::Progress);
    assert_eq!(
            s.summary,
            "Implemented the shared continuity foundation and migrated context/index/ingestion into openmesh-core."
        );
    assert_eq!(s.timestamp, "2026-07-08T09:15:00Z");
    assert_eq!(
        s.evidence_refs,
        vec![EvidenceRef::FilePath(
            "crates/openmesh-core/src/domain.rs".into()
        )]
    );
    assert_eq!(s.correlation_hint, Some("corr-0.1.3.1".to_string()));
    assert_eq!(s.sensitivity, Sensitivity::Private);
    assert_eq!(s.protocol_version, "1.0");
    assert_eq!(WORK_SIGNAL_PROTOCOL_VERSION, "1.0");
}

/// A missing `sensitivity` field must deserialize as `Private` — proving
/// the `#[serde(default)]` field attribute actually wires in the enum's
/// own `#[default]`, not merely relying on it existing (approved plan §3.9).
#[test]
fn missing_sensitivity_field_defaults_to_private() {
    let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "progress",
            "summary": "no sensitivity field here",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "protocolVersion": "1.0"
        }"#;
    let s: WorkSignal = serde_json::from_str(json).expect("deserialize");
    assert_eq!(s.sensitivity, Sensitivity::Private);
}

/// An unrecognized `kind` string under the *current* protocol version is
/// invalid current-version data, not a future-version compatibility case
/// — it must fail strict deserialization (approved plan §3.4/§10),
/// mirroring `context.rs`'s existing `deserialize_invalid_kind_fails`.
/// This is the direct evidence motivating the one-field (not per-enum)
/// preflight the classifier implements in Checkpoint C.
#[test]
fn unrecognized_kind_fails_strict_deserialize() {
    let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "not-a-real-kind",
            "summary": "bad kind",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "sensitivity": "private",
            "protocolVersion": "1.0"
        }"#;
    let result: Result<WorkSignal, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "unrecognized kind should fail deserialization"
    );
}

// ------------------------------------------------------------------
// Dev Track 0.1.3.4, Checkpoint A — WorkEvent wire contract.
// ------------------------------------------------------------------

fn sample_event() -> WorkEvent {
    WorkEvent {
        event_id: "1783605120049-event001".into(),
        workspace_id: "1783586870822-7352d".into(),
        kind: "work.completed".into(),
        summary: "Canonical WorkEvent for Checkpoint A.".into(),
        timestamp: "2026-07-15T07:00:00Z".into(),
        evidence: vec![
            EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into()),
                observed_at: Some("2026-07-15T07:00:01Z".into()),
            },
            EvidenceAttachment {
                evidence_ref: EvidenceRef::ProducerSignal("s-verify".into()),
                observed_at: None,
            },
        ],
        corrects_event_id: None,
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_EVENT_PROTOCOL_VERSION.to_string(),
        actor: None,
    }
}

#[test]
fn work_event_round_trips_through_json() {
    let original = sample_event();
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, original);
}

#[test]
fn evidence_attachment_round_trips_through_json() {
    let original = EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
        observed_at: Some("2026-07-15T07:00:00Z".into()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let restored: EvidenceAttachment = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, original);
}

#[test]
fn deserializes_the_canonical_event_fixture() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_path = format!("{manifest_dir}/tests/fixtures/events/valid.json");
    let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
    let event: WorkEvent = serde_json::from_str(&json).expect("deserialize fixture");

    assert_eq!(event.event_id, "1783605120049-event001");
    assert_eq!(event.workspace_id, "1783586870822-7352d");
    assert_eq!(event.kind, "work.completed");
    assert_eq!(
        event.summary,
        "Canonical WorkEvent fixture for Dev Track 0.1.3.4 Checkpoint A."
    );
    assert_eq!(event.timestamp, "2026-07-15T07:00:00Z");
    assert_eq!(event.evidence.len(), 2);
    assert_eq!(
        event.evidence[0].evidence_ref,
        EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into())
    );
    assert_eq!(
        event.evidence[0].observed_at.as_deref(),
        Some("2026-07-15T07:00:01Z")
    );
    assert_eq!(
        event.evidence[1].evidence_ref,
        EvidenceRef::ProducerSignal("s-verify".into())
    );
    assert!(event.evidence[1].observed_at.is_none());
    assert_eq!(event.corrects_event_id, None);
    assert_eq!(event.sensitivity, Sensitivity::Private);
    assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
}

#[test]
fn work_event_missing_sensitivity_is_rejected() {
    let json = r#"{
            "eventId": "evt-1",
            "workspaceId": "ws-1",
            "kind": "work.completed",
            "summary": "completed the task",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [
                { "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }
            ],
            "protocolVersion": "1.0"
        }"#;
    let result: Result<WorkEvent, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "missing sensitivity must fail strict WorkEvent deserialization"
    );
}

#[test]
fn work_event_corrects_event_id_round_trips() {
    let mut event = sample_event();
    event.corrects_event_id = Some("evt-original".into());
    let json = serde_json::to_string(&event).expect("serialize");
    let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.corrects_event_id.as_deref(), Some("evt-original"));
}

#[test]
fn validate_event_semantics_accepts_valid_event() {
    validate_event_semantics(&sample_event()).expect("valid event");
}

#[test]
fn validate_event_semantics_rejects_empty_evidence() {
    let mut event = sample_event();
    event.evidence.clear();
    assert_eq!(
        validate_event_semantics(&event),
        Err(EventValidationError::EmptyEvidence)
    );
}

#[test]
fn validate_event_semantics_rejects_invalid_timestamp() {
    let mut event = sample_event();
    event.timestamp = "2026-07-15T07:00:00-05:00".into();
    assert!(matches!(
        validate_event_semantics(&event),
        Err(EventValidationError::InvalidTimestamp(_))
    ));
}

#[test]
fn validate_event_semantics_rejects_wrong_protocol_version() {
    let mut event = sample_event();
    event.protocol_version = "99.0".into();
    assert_eq!(
        validate_event_semantics(&event),
        Err(EventValidationError::UnsupportedProtocolVersion {
            found: "99.0".into(),
        })
    );
}

#[test]
fn validate_event_semantics_rejects_invalid_observed_at() {
    let mut event = sample_event();
    event.evidence[0].observed_at = Some("2026-07-15T07:00:01-05:00".into());
    assert!(matches!(
        validate_event_semantics(&event),
        Err(EventValidationError::InvalidObservedAt(_))
    ));
}

/// Legacy `1.0` WorkEvents omit `actor` on wire; `1.1` promoted events require it.
#[test]
fn work_event_v1_0_serializes_without_actor_on_wire() {
    let json = serde_json::to_string(&sample_event()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    assert!(!obj.contains_key("actor"));
}

/// EvidenceRef includes pointer-only variants — `FilePath`, `ProducerSignal`,
/// and bounded `GitState` metadata (no source/diff bodies).
#[test]
fn evidence_ref_includes_git_state_variant() {
    let variants = [
        EvidenceRef::FilePath("path".into()),
        EvidenceRef::ProducerSignal("s-1".into()),
        EvidenceRef::GitState(sample_git_state()),
    ];
    for variant in variants {
        let json = serde_json::to_string(&variant).expect("serialize");
        let restored: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, variant);
    }
    match EvidenceRef::GitState(sample_git_state()) {
        EvidenceRef::FilePath(_) | EvidenceRef::ProducerSignal(_) | EvidenceRef::GitState(_) => {}
    }
}
