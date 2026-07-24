//! Dev Track 0.1.6 Checkpoint B — deterministic prompt composition boundary tests (pure).

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{
    deterministic_context_pack_id, validate_proxy_prompt_bundle, validate_proxy_question,
    AuthorityRule, CatchUpWindow, CommunicationPreferences, ContextPackAuthoritySummary,
    ContextPackCatchUp, ContextPackCatchUpSections, ContextPackContinuityItem,
    ContextPackCurrentState, ContextPackCurrentStateSections, ContextPackEvidenceIndexEntry,
    ContextPackEvidenceOrigin, ContextPackFreshness, ContextPackItemProvenance,
    ContextPackOwnerIdentity, ContextPackPendingAttentionItem, ContextPackPrivacySummary,
    ContextPackRedactionSummary, ContinuityConfidence, ContinuitySourceKind, DecisionPreferences,
    DefaultRefusalRule, EvidencePolicy, EvidenceRef, EvidenceSourceKind, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, PrivacyAllowedUse, PrivacyRule,
    PrivacySensitivity, ProxyAuthorityLevel, ProxyContextPack, ProxyQuestion, SourceCounts,
    UnsupportedClaimBehavior, CONTEXT_PACK_EXECUTION_BOUNDARY, PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
    PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
};
use openmesh_core::proxy_prompt::{compose_proxy_prompt, PROXY_PROMPT_SYSTEM_MESSAGE};
use openmesh_core::proxy_prompt_context::ProxyPromptError;
use openmesh_core::proxy_question::{
    create_proxy_question, ProxyQuestionIdentityError, ProxyRequestIdentityProvider,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

struct TestSequenceIdentityProvider {
    sequence: AtomicU64,
}

impl TestSequenceIdentityProvider {
    fn new() -> Self {
        Self {
            sequence: AtomicU64::new(1),
        }
    }
}

impl ProxyRequestIdentityProvider for TestSequenceIdentityProvider {
    fn next_question_id(&self) -> Result<String, ProxyQuestionIdentityError> {
        let value = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(format!("proxy-q-deadbeef-{value:04x}-00"))
    }
}

fn sample_authority_rule() -> AuthorityRule {
    AuthorityRule {
        rule_id: "rule-global".into(),
        scope: "*".into(),
        authority: ProxyAuthorityLevel::MustAskHuman,
        description: Some("default safe baseline".into()),
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: true,
        limitations: vec![],
    }
}

fn sample_continuity_item() -> ContextPackContinuityItem {
    ContextPackContinuityItem {
        id: "item-in-progress-1".into(),
        summary: "bootstrap continuity fixture".into(),
        kind: "work.in-progress".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: "2026-07-17T10:00:00Z".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/readme.md".into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: None,
    }
}

fn sample_pending_item() -> ContextPackPendingAttentionItem {
    ContextPackPendingAttentionItem {
        id: "pending-1".into(),
        summary: "pending signal awaiting promotion".into(),
        reason: PendingAttentionReason::PendingSignal,
        provenance: ContextPackItemProvenance::Pending,
        timestamp: "2026-07-17T11:00:00Z".into(),
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::Medium,
        priority: 3,
        evidence_refs: vec![],
    }
}

fn sample_pack() -> ProxyContextPack {
    let build_inputs_hash = "fnv1a-6dd176ff3e7276a3".to_string();
    let generated_at = "2026-07-18T04:00:00Z".to_string();
    let window = CatchUpWindow {
        since: "2026-07-17T00:00:00Z".into(),
        until: "2026-07-18T00:00:00Z".into(),
    };
    ProxyContextPack {
        context_pack_id: deterministic_context_pack_id(&build_inputs_hash),
        workspace_id: "ws-fixture-0.1.5".into(),
        profile_id: "profile-ws-fixture-0.1.5".into(),
        profile_version: "1.0".into(),
        protocol_version: PROXY_CONTEXT_PACK_PROTOCOL_VERSION.to_string(),
        generated_at: generated_at.clone(),
        requested_window: window.clone(),
        owner_identity: ContextPackOwnerIdentity {
            owner_label: "Fixture Owner".into(),
            role_label: "Engineering lead".into(),
        },
        communication_preferences: CommunicationPreferences {
            tone: "direct".into(),
            detail_level: "medium".into(),
            async_preference: "prefer-async".into(),
            correction_preference: "surface-limitations".into(),
        },
        decision_preferences: DecisionPreferences {
            decision_style: "evidence-first".into(),
            escalation_preference: "ask-human-on-ambiguity".into(),
        },
        authority_summary: ContextPackAuthoritySummary {
            authority_rules: vec![sample_authority_rule()],
            default_refusal_rules: vec![DefaultRefusalRule {
                rule_id: "refusal-no-impersonation".into(),
                statement: "cannot impersonate owner".into(),
            }],
            ladder_levels: openmesh_core::domain::proxy_context_pack_authority_ladder_levels()
                .iter()
                .map(|level| (*level).to_string())
                .collect(),
            execution_boundary: CONTEXT_PACK_EXECUTION_BOUNDARY.to_string(),
        },
        privacy_summary: ContextPackPrivacySummary {
            privacy_rules: vec![PrivacyRule {
                rule_id: "privacy-credentials".into(),
                topic: "credentials".into(),
                sensitivity: PrivacySensitivity::Secret,
                allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
                restriction: "never include in proxy output".into(),
                requires_human_confirmation: true,
            }],
            sensitive_topics: vec!["credentials".into()],
            filtering_applied: vec!["secret-evidence-omitted".into()],
        },
        evidence_policy: EvidencePolicy {
            answer_without_evidence: false,
            require_evidence_for_claims: true,
            expose_limitations: true,
            cite_source_kinds: vec![EvidenceSourceKind::FilePath, EvidenceSourceKind::WorkEvent],
            unsupported_claim_behavior: UnsupportedClaimBehavior::SayUnknown,
        },
        current_state: ContextPackCurrentState {
            workspace_id: "ws-fixture-0.1.5".into(),
            sections: ContextPackCurrentStateSections {
                completed: vec![],
                in_progress: vec![sample_continuity_item()],
                blocked: vec![],
                decisions: vec![],
                needs_attention: vec![],
                still_open: vec![],
            },
            pending_attention: vec![sample_pending_item()],
            limitations: vec!["context pack metadata only; no answering runtime in 0.1.5".into()],
        },
        catch_up: ContextPackCatchUp {
            workspace_id: "ws-fixture-0.1.5".into(),
            window: window.clone(),
            sections: ContextPackCatchUpSections {
                completed: vec![],
                changed: vec![],
                blocked: vec![],
                decided: vec![],
                needs_attention: vec![],
                still_open: vec![],
            },
            summary: "No material changes in fixture window.".into(),
            next_suggested_attention: vec![],
            limitations: vec!["context pack metadata only; no answering runtime in 0.1.5".into()],
        },
        evidence_index: vec![ContextPackEvidenceIndexEntry {
            ref_id: "ref-001".into(),
            evidence_ref: EvidenceRef::FilePath("docs/readme.md".into()),
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity: Sensitivity::Private,
            label: "readme evidence".into(),
            timestamp: Some("2026-07-17T10:00:00Z".into()),
        }],
        source_counts: SourceCounts {
            work_events: 1,
            processed_signals: 1,
            pending_signals: 1,
            promotion_audit_records: 0,
            quarantine_signals: 0,
            duplicate_signals: 0,
            reporter_signals: 0,
            git_signals: 0,
            heli_signals: 0,
            unknown_producer_signals: 0,
            other_producer_signals: 0,
        },
        diagnostics: vec![],
        limitations: vec!["context pack metadata only; no answering runtime in 0.1.5".into()],
        unresolved_items: vec![],
        freshness: ContextPackFreshness {
            snapshot_observed_at: "2026-07-18T03:59:00Z".into(),
            current_state_generated_at: "2026-07-18T03:59:30Z".into(),
            catch_up_since: window.since.clone(),
            catch_up_until: window.until.clone(),
            pack_generated_at: generated_at,
            age_seconds: 60,
            warnings: vec![],
        },
        redaction_summary: ContextPackRedactionSummary {
            secret_items_omitted: 1,
            policy_restricted_items_omitted: 0,
            malformed_items_omitted: 0,
            quarantined_items_omitted: 0,
            bounds_truncated_items: 0,
        },
        build_inputs_hash,
    }
}

fn compose_fixture(text: &str) -> openmesh_core::domain::ProxyPromptBundle {
    let pack = sample_pack();
    validate_proxy_context_pack_complete(&pack).expect("fixture pack must validate completely");
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question(text, &provider).expect("valid question");
    compose_proxy_prompt(&pack, &question).expect("valid composition")
}

fn bundle_json_bytes(bundle: &openmesh_core::domain::ProxyPromptBundle) -> String {
    serde_json::to_string(bundle).expect("serialize bundle")
}

#[test]
fn compose_proxy_prompt_validates_question_first() {
    let mut pack = sample_pack();
    pack.workspace_id = "   ".into();
    let invalid_question = ProxyQuestion {
        protocol_version: "99.0".into(),
        question_id: "proxy-q-deadbeef-0001-00".into(),
        text: "status?".into(),
    };
    let err = compose_proxy_prompt(&pack, &invalid_question).expect_err("question first");
    assert_eq!(err, ProxyPromptError::InvalidQuestion);
}

#[test]
fn compose_proxy_prompt_validates_pack_completely() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("status?", &provider).expect("valid question");
    let mut pack = sample_pack();
    pack.workspace_id = "   ".into();
    let err = compose_proxy_prompt(&pack, &question).expect_err("invalid pack");
    assert_eq!(err, ProxyPromptError::InvalidContextPack);
}

#[test]
fn user_message_contains_normalized_question_only() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("  What changed?  ", &provider).expect("valid question");
    let bundle = compose_proxy_prompt(&sample_pack(), &question).expect("compose");
    assert_eq!(bundle.user_message, "What changed?");
    assert_eq!(question.text, "  What changed?  ");
}

#[test]
fn user_message_contains_no_question_id() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("What changed?", &provider).expect("valid question");
    let bundle = compose_proxy_prompt(&sample_pack(), &question).expect("compose");
    assert!(!bundle.user_message.contains(&question.question_id));
    assert!(!bundle.user_message.contains("proxy-q-"));
}

#[test]
fn context_json_contains_no_trace_metadata() {
    let pack = sample_pack();
    let bundle = compose_fixture("status?");
    let lowered = bundle.context_json.to_ascii_lowercase();
    for forbidden in [
        "\"workspaceid\"",
        "\"profileid\"",
        "\"profileversion\"",
        "\"contextpackid\"",
        "\"buildinputshash\"",
        "\"evidencesummary\"",
        "\"questionid\"",
        "\"trace\"",
        "\"unresolveditems\"",
        "\"evidenceindex\"",
        "\"evidenceref\"",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "context_json must not contain {forbidden}"
        );
    }
    for leaked in [
        &pack.workspace_id,
        &pack.profile_id,
        &pack.context_pack_id,
        &pack.build_inputs_hash,
        "item-in-progress-1",
        "pending-1",
        "ref-001",
        "docs/readme.md",
    ] {
        assert!(
            !bundle.context_json.contains(leaked),
            "context_json must not leak stable identity {leaked}"
        );
    }
}

#[test]
fn system_message_contains_no_stable_ids() {
    let pack = sample_pack();
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("status?", &provider).expect("valid question");
    let bundle = compose_proxy_prompt(&pack, &question).expect("compose");
    for leaked in [
        &pack.workspace_id,
        &pack.profile_id,
        &pack.context_pack_id,
        &pack.build_inputs_hash,
        &question.question_id,
        "item-in-progress-1",
        "pending-1",
        "ref-001",
    ] {
        assert!(
            !bundle.system_message.contains(leaked),
            "system_message must not contain stable id {leaked}"
        );
    }
}

#[test]
fn system_message_contains_fixed_no_impersonation_constraint() {
    let bundle = compose_fixture("status?");
    assert_eq!(bundle.system_message, PROXY_PROMPT_SYSTEM_MESSAGE);
    assert!(bundle.system_message.contains("not the human owner"));
    assert!(bundle.system_message.contains("do not speak as the owner"));
}

#[test]
fn system_message_contains_fixed_no_authority_constraint() {
    let bundle = compose_fixture("status?");
    assert!(bundle
        .system_message
        .contains("Do not claim owner approval, authority"));
    assert!(bundle
        .system_message
        .contains("Authority execution is disabled"));
}

#[test]
fn system_message_contains_fixed_no_action_constraint() {
    let bundle = compose_fixture("status?");
    assert!(bundle
        .system_message
        .contains("that any action was performed"));
}

#[test]
fn system_message_contains_fixed_no_tools_constraint() {
    let bundle = compose_fixture("status?");
    assert!(bundle.system_message.contains("Do not create tool calls"));
}

#[test]
fn system_message_contains_fixed_secret_constraint() {
    let bundle = compose_fixture("status?");
    assert!(bundle
        .system_message
        .contains("Do not reveal secrets or credentials"));
}

#[test]
fn runtime_receives_no_proxy_draft_trace_metadata() {
    let pack = sample_pack();
    let bundle = compose_fixture("status?");
    let json = bundle_json_bytes(&bundle);
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "\"workspaceid\"",
        "\"profileid\"",
        "\"profileversion\"",
        "\"contextpackid\"",
        "\"buildinputshash\"",
        "\"evidencesummary\"",
        "\"trace\"",
        "\"questionid\"",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "runtime-facing bundle must not contain trace field {forbidden}"
        );
    }
    for leaked in [
        &pack.workspace_id,
        &pack.profile_id,
        &pack.context_pack_id,
        &pack.build_inputs_hash,
    ] {
        assert!(
            !json.contains(leaked),
            "runtime-facing bundle must not leak {leaked}"
        );
    }
}

#[test]
fn prompt_bundle_passes_checkpoint_a_validation() {
    let bundle = compose_fixture("What is the current status?");
    validate_proxy_prompt_bundle(&bundle).expect("checkpoint A bundle validation");
    assert_eq!(
        bundle.protocol_version,
        PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION
    );
}

#[test]
fn prompt_composition_performs_zero_io() {
    let bundle = compose_fixture("status?");
    validate_proxy_prompt_bundle(&bundle).expect("valid bundle");
    for source in [
        std::include_str!("../src/proxy_prompt.rs"),
        std::include_str!("../src/proxy_prompt_context.rs"),
    ] {
        assert!(!source.contains("fs::"));
        assert!(!source.contains("read_to_string"));
        assert!(!source.contains("write("));
        assert!(!source.contains("OpenOptions"));
    }
}

#[test]
fn prompt_composition_performs_no_runtime_invocation() {
    let _ = compose_fixture("status?");
    for source in [
        std::include_str!("../src/proxy_prompt.rs"),
        std::include_str!("../src/proxy_prompt_context.rs"),
        std::include_str!("../src/proxy_question.rs"),
    ] {
        for forbidden in [
            "trait ProxyDraftRuntime",
            "UnconfiguredProxyDraftRuntime",
            "DeterministicStubProxyDraftRuntime",
            "ask_my_proxy",
            "generate_draft",
            "resolve_production_proxy_draft_runtime",
            "validate_proxy_runtime_request",
            "validate_proxy_runtime_output",
        ] {
            assert!(
                !source.contains(forbidden),
                "prompt composition must not reference runtime symbol {forbidden}"
            );
        }
    }
}

#[test]
fn prompt_composition_performs_no_network_access() {
    for source in [
        std::include_str!("../src/proxy_prompt.rs"),
        std::include_str!("../src/proxy_prompt_context.rs"),
        std::include_str!("../src/proxy_question.rs"),
    ] {
        let lowered = source.to_ascii_lowercase();
        for forbidden in [
            "reqwest",
            "hyper::",
            "tcpstream",
            "udpsocket",
            "network_used",
            "http::",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "prompt composition must not reference network symbol {forbidden}"
            );
        }
    }
}

#[test]
fn prompt_composition_does_not_process_signals() {
    for source in [
        std::include_str!("../src/proxy_prompt.rs"),
        std::include_str!("../src/proxy_prompt_context.rs"),
    ] {
        for forbidden in [
            "WorkSignal",
            "processed_signals",
            "pending_signals",
            "promotion_audit",
            "collect_git_signal",
            "collect_heli_signal",
        ] {
            assert!(
                !source.contains(forbidden),
                "prompt composition must not process signals via {forbidden}"
            );
        }
    }
}

#[test]
fn prompt_composition_does_not_mutate_pack() {
    let pack = sample_pack();
    let before = pack.clone();
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("status?", &provider).expect("valid question");
    compose_proxy_prompt(&pack, &question).expect("compose");
    assert_eq!(pack, before);
}

#[test]
fn checkpoint_b_does_not_start_checkpoint_c() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["src/proxy_prompt.rs", "src/proxy_question.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        for forbidden in [
            "trait ProxyDraftRuntime",
            "UnconfiguredProxyDraftRuntime",
            "DeterministicStubProxyDraftRuntime",
            "proxy_runtime.rs",
            "ask_my_proxy",
            "build_proxy_draft_trace_metadata",
            "proxy_ask.rs",
        ] {
            assert!(
                !content.contains(forbidden),
                "{rel} must not start checkpoint C symbol {forbidden}"
            );
        }
    }
}

#[test]
fn checkpoint_b_does_not_start_dg() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["src/proxy_prompt.rs", "src/proxy_question.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        let lowered = content.to_ascii_lowercase();
        for forbidden in [
            "proxy_runtime_axga",
            "dg-0.1.6",
            "resolve_production_proxy_draft_runtime",
            "openmesh_ai_runtime",
            "runtimenotconfigured",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "{rel} must not start DG symbol {forbidden}"
            );
        }
    }
}

#[test]
fn checkpoint_b_does_not_start_0_1_7() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["src/proxy_prompt.rs", "src/proxy_question.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        for forbidden in [
            "pub struct ProxyClaim",
            "pub struct ProxyCitation",
            "verifiedAnswer",
        ] {
            assert!(
                !content.contains(forbidden),
                "{rel} must not start 0.1.7 type {forbidden}"
            );
        }
    }
}

#[test]
fn identical_semantic_input_produces_identical_prompt_bundle_bytes() {
    let pack = sample_pack();
    let provider = TestSequenceIdentityProvider::new();
    let first_question = create_proxy_question("What changed?", &provider).expect("first");
    let second_question = create_proxy_question("What changed?", &provider).expect("second");
    assert_ne!(first_question.question_id, second_question.question_id);

    let first_bundle = compose_proxy_prompt(&pack, &first_question).expect("first compose");
    let second_bundle = compose_proxy_prompt(&pack, &second_question).expect("second compose");
    let third_bundle = compose_proxy_prompt(&pack, &first_question).expect("third compose");

    let first_bytes = bundle_json_bytes(&first_bundle);
    let second_bytes = bundle_json_bytes(&second_bundle);
    let third_bytes = bundle_json_bytes(&third_bundle);

    assert_eq!(first_bytes, second_bytes);
    assert_eq!(first_bytes, third_bytes);
    validate_proxy_question(&first_question).expect("question validates");
    validate_proxy_question(&second_question).expect("question validates");
}

#[test]
fn compose_proxy_prompt_accepts_thai_utf8_question() {
    let provider = TestSequenceIdentityProvider::new();
    let question = create_proxy_question("สถานะปัจจุบันคืออะไร", &provider).expect("Thai question");
    let bundle = compose_proxy_prompt(&sample_pack(), &question).expect("compose");
    assert_eq!(bundle.user_message, "สถานะปัจจุบันคืออะไร");
    validate_proxy_prompt_bundle(&bundle).expect("Thai bundle validates");
}
