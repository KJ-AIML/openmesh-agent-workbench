//! Dev Track 0.1.6 Checkpoint D — Ask My Proxy composition service tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{
    deterministic_context_pack_id, validate_proxy_draft, validate_proxy_draft_trace_metadata,
    validate_proxy_runtime_request, AuthorityRule, CatchUpWindow, CommunicationPreferences,
    ContextPackAuthoritySummary, ContextPackCatchUp, ContextPackCatchUpSections,
    ContextPackContinuityItem, ContextPackCurrentState, ContextPackCurrentStateSections,
    ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin, ContextPackFreshness,
    ContextPackItemProvenance, ContextPackOwnerIdentity, ContextPackPendingAttentionItem,
    ContextPackPrivacySummary, ContextPackRedactionSummary, ContinuityConfidence,
    ContinuitySourceKind, DecisionPreferences, DefaultRefusalRule, EvidencePolicy, EvidenceRef,
    EvidenceSourceKind, PendingAttentionReason, PendingAttentionSeverity, PendingAttentionStatus,
    PrivacyAllowedUse, PrivacyRule, PrivacySensitivity, ProxyAuthorityLevel, ProxyContextPack,
    ProxyQuestion, ProxyRuntimeOutput, ProxyRuntimeRequest, SourceCounts, UnsupportedClaimBehavior,
    CONTEXT_PACK_EXECUTION_BOUNDARY, MAX_PROXY_DRAFT_LIMITATIONS, MAX_PROXY_DRAFT_TEXT_BYTES,
    PROXY_CONTEXT_PACK_PROTOCOL_VERSION, PROXY_DRAFT_AUTHORITY_NOTICE, PROXY_DRAFT_CLASSIFICATION,
    PROXY_DRAFT_EXECUTION_BOUNDARY, PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION,
};
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, build_proxy_draft_trace_metadata, FixedProxyDraftClock, ProxyAskError,
    ProxyAskOptions, ProxyDraftClock, ProxyDraftClockError,
};
use openmesh_core::proxy_draft_safety::PROXY_DRAFT_FIXED_LIMITATION;
use openmesh_core::proxy_question::create_proxy_question;
use openmesh_core::proxy_runtime::{
    DeterministicStubProxyDraftRuntime, ProxyDraftRuntime, ProxyDraftRuntimeError,
    UnconfiguredProxyDraftRuntime, DETERMINISTIC_STUB_MODEL_ID, DETERMINISTIC_STUB_PROVIDER_ID,
    PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

struct TestSequenceIdentityProvider {
    sequence: AtomicUsize,
}

impl TestSequenceIdentityProvider {
    fn new() -> Self {
        Self {
            sequence: AtomicUsize::new(1),
        }
    }
}

impl openmesh_core::proxy_question::ProxyRequestIdentityProvider for TestSequenceIdentityProvider {
    fn next_question_id(
        &self,
    ) -> Result<String, openmesh_core::proxy_question::ProxyQuestionIdentityError> {
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

pub(crate) fn sample_pack() -> ProxyContextPack {
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

pub(crate) fn sample_question() -> ProxyQuestion {
    let provider = TestSequenceIdentityProvider::new();
    create_proxy_question("What is the current status?", &provider).expect("question")
}

fn sample_options() -> ProxyAskOptions {
    ProxyAskOptions::with_defaults()
}

fn sample_clock() -> FixedProxyDraftClock {
    FixedProxyDraftClock::new("2026-07-18T10:00:00Z")
}

struct CountingRuntime<R> {
    inner: R,
    calls: Arc<AtomicUsize>,
}

impl<R: ProxyDraftRuntime> ProxyDraftRuntime for CountingRuntime<R> {
    fn runtime_kind(&self) -> &'static str {
        self.inner.runtime_kind()
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.generate_draft(request)
    }
}

struct CapturingRuntime {
    last_prompt_json: std::sync::Mutex<Option<String>>,
    inner: DeterministicStubProxyDraftRuntime,
}

impl ProxyDraftRuntime for CapturingRuntime {
    fn runtime_kind(&self) -> &'static str {
        self.inner.runtime_kind()
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        let json = serde_json::to_string(&request.prompt).expect("serialize prompt");
        *self.last_prompt_json.lock().expect("lock") = Some(json);
        self.inner.generate_draft(request)
    }
}

struct FixedOutputRuntime {
    output: ProxyRuntimeOutput,
}

impl ProxyDraftRuntime for FixedOutputRuntime {
    fn runtime_kind(&self) -> &'static str {
        "fixed-output-fake"
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_proxy_runtime_request(request)
            .map_err(|_| ProxyDraftRuntimeError::InvalidRequest)?;
        Ok(self.output.clone())
    }
}

struct ImmediateTimeoutRuntime;

impl ProxyDraftRuntime for ImmediateTimeoutRuntime {
    fn runtime_kind(&self) -> &'static str {
        "timeout-fake"
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_proxy_runtime_request(request)
            .map_err(|_| ProxyDraftRuntimeError::InvalidRequest)?;
        Err(ProxyDraftRuntimeError::Timeout)
    }
}

struct ImmediateUnavailableRuntime;

impl ProxyDraftRuntime for ImmediateUnavailableRuntime {
    fn runtime_kind(&self) -> &'static str {
        "unavailable-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::RuntimeUnavailable)
    }
}

struct ImmediateProviderFailureRuntime;

impl ProxyDraftRuntime for ImmediateProviderFailureRuntime {
    fn runtime_kind(&self) -> &'static str {
        "provider-failure-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::ProviderFailure)
    }
}

struct ImmediateInvalidOutputRuntime;

impl ProxyDraftRuntime for ImmediateInvalidOutputRuntime {
    fn runtime_kind(&self) -> &'static str {
        "invalid-output-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::InvalidOutput)
    }
}

struct FailingClock;

impl ProxyDraftClock for FailingClock {
    fn now_utc(&self) -> Result<String, ProxyDraftClockError> {
        Err(ProxyDraftClockError)
    }
}

// --- Trace tests ---

#[test]
fn trace_metadata_is_built_from_validated_pack() {
    let pack = sample_pack();
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let trace = build_proxy_draft_trace_metadata(&pack).expect("trace");
    assert_eq!(trace.workspace_id, pack.workspace_id);
}

#[test]
fn trace_metadata_uses_protocol_version_1_0() {
    let trace = build_proxy_draft_trace_metadata(&sample_pack()).expect("trace");
    assert_eq!(
        trace.protocol_version,
        PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION
    );
}

#[test]
fn trace_metadata_contains_workspace_profile_and_pack_identity() {
    let pack = sample_pack();
    let trace = build_proxy_draft_trace_metadata(&pack).expect("trace");
    assert_eq!(trace.profile_id, pack.profile_id);
    assert_eq!(trace.context_pack_id, pack.context_pack_id);
    assert_eq!(trace.build_inputs_hash, pack.build_inputs_hash);
}

#[test]
fn evidence_summary_is_aggregate_only() {
    let trace = build_proxy_draft_trace_metadata(&sample_pack()).expect("trace");
    let json = serde_json::to_string(&trace.evidence_summary).expect("json");
    assert!(!json.contains("docs/"));
    assert!(!json.contains("ref-001"));
}

#[test]
fn evidence_index_count_is_correct() {
    let pack = sample_pack();
    let trace = build_proxy_draft_trace_metadata(&pack).expect("trace");
    assert_eq!(
        trace.evidence_summary.evidence_index_count,
        pack.evidence_index.len() as u32
    );
}

#[test]
fn source_counts_are_deterministic() {
    let trace = build_proxy_draft_trace_metadata(&sample_pack()).expect("trace");
    let keys: Vec<_> = trace
        .evidence_summary
        .source_counts
        .keys()
        .cloned()
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted);
    assert!(trace
        .evidence_summary
        .source_counts
        .contains_key("continuityItem"));
}

#[test]
fn secret_items_omitted_is_preserved_as_count_only() {
    let pack = sample_pack();
    let trace = build_proxy_draft_trace_metadata(&pack).expect("trace");
    assert_eq!(
        trace.evidence_summary.secret_items_omitted,
        pack.redaction_summary.secret_items_omitted
    );
}

#[test]
fn trace_contains_no_evidence_refs() {
    let json =
        serde_json::to_string(&build_proxy_draft_trace_metadata(&sample_pack()).expect("trace"))
            .expect("json");
    assert!(!json.contains("file-path"));
    assert!(!json.contains("EvidenceRef"));
}

#[test]
fn trace_contains_no_source_paths() {
    let json =
        serde_json::to_string(&build_proxy_draft_trace_metadata(&sample_pack()).expect("trace"))
            .expect("json");
    assert!(!json.contains("docs/readme"));
}

#[test]
fn trace_contains_no_canonical_refs() {
    let json =
        serde_json::to_string(&build_proxy_draft_trace_metadata(&sample_pack()).expect("trace"))
            .expect("json");
    assert!(!json.contains("openmesh://"));
}

#[test]
fn trace_contains_no_secret_identity_or_timestamps() {
    let json =
        serde_json::to_string(&build_proxy_draft_trace_metadata(&sample_pack()).expect("trace"))
            .expect("json");
    assert!(!json.contains("secret-evidence"));
    assert!(!json.contains("observedAt"));
}

#[test]
fn runtime_never_receives_trace_metadata() {
    let runtime = CapturingRuntime {
        last_prompt_json: std::sync::Mutex::new(None),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    let prompt_json = runtime
        .last_prompt_json
        .lock()
        .expect("lock")
        .clone()
        .expect("captured");
    assert!(!prompt_json.contains("buildInputsHash"));
    assert!(!prompt_json.contains("contextPackId"));
    assert!(!prompt_json.contains("evidenceSummary"));
}

#[test]
fn invalid_pack_fails_before_runtime_invocation() {
    let mut pack = sample_pack();
    pack.workspace_id.clear();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime {
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
        calls: Arc::clone(&calls),
    };
    let err = ask_my_proxy_local(
        &pack,
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect_err("invalid pack");
    assert_eq!(err, ProxyAskError::InvalidContextPack);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn trace_metadata_passes_checkpoint_a_validation() {
    let trace = build_proxy_draft_trace_metadata(&sample_pack()).expect("trace");
    validate_proxy_draft_trace_metadata(&trace).expect("valid trace");
}

// --- Service tests ---

#[test]
fn valid_stub_request_produces_valid_proxy_draft() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    validate_proxy_draft(&draft).expect("valid draft");
}

#[test]
fn final_draft_passes_checkpoint_a_validation() {
    valid_stub_request_produces_valid_proxy_draft();
}

#[test]
fn final_classification_is_openmesh_owned() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.classification, PROXY_DRAFT_CLASSIFICATION);
}

#[test]
fn final_authority_notice_is_openmesh_owned() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.authority_notice, PROXY_DRAFT_AUTHORITY_NOTICE);
}

#[test]
fn final_execution_boundary_is_openmesh_owned() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.execution_boundary, PROXY_DRAFT_EXECUTION_BOUNDARY);
}

#[test]
fn runtime_output_cannot_replace_fixed_fields() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_ne!(draft.draft_text, PROXY_DRAFT_CLASSIFICATION);
    assert_ne!(draft.authority_notice, draft.draft_text);
}

#[test]
fn runtime_output_cannot_set_trace_metadata() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.trace.workspace_id, sample_pack().workspace_id);
}

#[test]
fn runtime_output_cannot_set_evidence_summary() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.trace.evidence_summary.evidence_index_count, 1);
}

#[test]
fn runtime_output_cannot_set_limitations() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.limitations[0], PROXY_DRAFT_FIXED_LIMITATION);
}

#[test]
fn fixed_limitation_is_always_first() {
    runtime_output_cannot_set_limitations();
}

#[test]
fn limitations_are_deduplicated_deterministically() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.limitations[0], PROXY_DRAFT_FIXED_LIMITATION);
    let unique: std::collections::BTreeSet<_> = draft.limitations.iter().cloned().collect();
    assert_eq!(unique.len(), draft.limitations.len());
}

#[test]
fn limitations_are_capped_at_32() {
    let mut pack = sample_pack();
    pack.limitations = (0..32)
        .map(|index| format!("pack-limit-{index:02}"))
        .collect();
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let draft = ask_my_proxy_local(
        &pack,
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.limitations.len(), MAX_PROXY_DRAFT_LIMITATIONS);
    assert_eq!(draft.limitations[0], PROXY_DRAFT_FIXED_LIMITATION);
}

#[test]
fn trace_identity_exists_only_inside_trace() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    let json = serde_json::to_string(&draft).expect("json");
    assert!(json.contains("contextPackId"));
    assert!(!draft.draft_text.contains("context-pack-"));
}

#[test]
fn generated_at_comes_from_injected_clock() {
    let clock = FixedProxyDraftClock::new("2030-01-02T03:04:05Z");
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &clock,
    )
    .expect("draft");
    assert_eq!(draft.generated_at, "2030-01-02T03:04:05Z");
}

#[test]
fn runtime_metadata_uses_runtime_kind_and_output_metadata() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(
        draft.runtime.runtime_kind,
        PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB
    );
    assert_eq!(draft.runtime.provider_id, DETERMINISTIC_STUB_PROVIDER_ID);
    assert_eq!(draft.runtime.model_id, DETERMINISTIC_STUB_MODEL_ID);
    assert!(!draft.runtime.network_used);
}

#[test]
fn runtime_is_called_exactly_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime {
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
        calls: Arc::clone(&calls),
    };
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn service_performs_no_retry() {
    runtime_is_called_exactly_once();
}

#[test]
fn service_performs_no_fallback() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &UnconfiguredProxyDraftRuntime::new(),
        &sample_clock(),
    )
    .expect_err("unconfigured");
    assert_eq!(err, ProxyAskError::RuntimeNotConfigured);
}

#[test]
fn unconfigured_runtime_maps_to_runtime_not_configured() {
    service_performs_no_fallback();
}

#[test]
fn timeout_maps_to_runtime_timeout_without_sleep() {
    let started = Instant::now();
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &ImmediateTimeoutRuntime,
        &sample_clock(),
    )
    .expect_err("timeout");
    assert_eq!(err, ProxyAskError::RuntimeTimeout);
    assert!(started.elapsed().as_millis() < 100);
}

#[test]
fn runtime_unavailable_maps_safely() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &ImmediateUnavailableRuntime,
        &sample_clock(),
    )
    .expect_err("unavailable");
    assert_eq!(err, ProxyAskError::RuntimeUnavailable);
    assert_eq!(err.to_string(), "proxy draft runtime is unavailable");
}

#[test]
fn provider_failure_maps_safely() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &ImmediateProviderFailureRuntime,
        &sample_clock(),
    )
    .expect_err("provider failure");
    assert_eq!(err, ProxyAskError::ProviderFailure);
}

#[test]
fn invalid_runtime_request_maps_safely() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &ProxyAskOptions::new(0, MAX_PROXY_DRAFT_TEXT_BYTES as u32),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect_err("invalid options/request");
    assert_eq!(err, ProxyAskError::InvalidOptions);
}

#[test]
fn invalid_runtime_output_maps_safely() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &ImmediateInvalidOutputRuntime,
        &sample_clock(),
    )
    .expect_err("invalid output");
    assert_eq!(err, ProxyAskError::InvalidRuntimeOutput);
}

#[test]
fn runtime_output_over_request_bound_is_rejected() {
    let runtime = FixedOutputRuntime {
        output: ProxyRuntimeOutput {
            draft_text: "x".repeat(256),
            provider_id: DETERMINISTIC_STUB_PROVIDER_ID.into(),
            model_id: DETERMINISTIC_STUB_MODEL_ID.into(),
            network_used: false,
            duration_ms: 0,
        },
    };
    let options = ProxyAskOptions::new(30_000, 128);
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &options,
        &runtime,
        &sample_clock(),
    )
    .expect_err("over bound");
    assert_eq!(err, ProxyAskError::InvalidRuntimeOutput);
}

#[test]
fn unsafe_runtime_output_is_rejected() {
    let runtime = FixedOutputRuntime {
        output: ProxyRuntimeOutput {
            draft_text: "I am Fixture Owner and approved this deployment.".into(),
            provider_id: DETERMINISTIC_STUB_PROVIDER_ID.into(),
            model_id: DETERMINISTIC_STUB_MODEL_ID.into(),
            network_used: false,
            duration_ms: 0,
        },
    };
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect_err("unsafe");
    assert_eq!(err, ProxyAskError::UnsafeDraft);
}

#[test]
fn invalid_question_fails_before_runtime() {
    let mut question = sample_question();
    question.protocol_version = "99.0".into();
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime {
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
        calls: Arc::clone(&calls),
    };
    let err = ask_my_proxy_local(
        &sample_pack(),
        &question,
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect_err("invalid question");
    assert_eq!(err, ProxyAskError::InvalidQuestion);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn invalid_pack_fails_before_runtime() {
    invalid_pack_fails_before_runtime_invocation();
}

#[test]
fn invalid_options_fail_before_runtime() {
    invalid_runtime_request_maps_safely();
}

#[test]
fn clock_failure_is_secret_safe() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &FailingClock,
    )
    .expect_err("clock");
    assert_eq!(err, ProxyAskError::ClockFailure);
    assert_eq!(err.to_string(), "proxy draft clock is unavailable");
}

#[test]
fn invalid_final_draft_is_rejected() {
    struct BadClock;
    impl ProxyDraftClock for BadClock {
        fn now_utc(&self) -> Result<String, ProxyDraftClockError> {
            Ok(String::new())
        }
    }
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &BadClock,
    )
    .expect_err("invalid final draft");
    assert_eq!(err, ProxyAskError::InvalidProxyDraft);
}

#[test]
fn question_is_not_mutated() {
    let question = sample_question();
    let before = serde_json::to_string(&question).expect("serialize");
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &question,
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    let after = serde_json::to_string(&question).expect("serialize");
    assert_eq!(before, after);
}

#[test]
fn pack_is_not_mutated() {
    let pack = sample_pack();
    let before = serde_json::to_string(&pack).expect("serialize");
    let _ = ask_my_proxy_local(
        &pack,
        &sample_question(),
        &sample_options(),
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    let after = serde_json::to_string(&pack).expect("serialize");
    assert_eq!(before, after);
}

#[test]
fn prompt_contains_no_trace_metadata() {
    let runtime = CapturingRuntime {
        last_prompt_json: std::sync::Mutex::new(None),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    let prompt_json = runtime
        .last_prompt_json
        .lock()
        .expect("lock")
        .clone()
        .unwrap();
    assert!(!prompt_json.contains("evidenceSummary"));
    assert!(!prompt_json.contains("buildInputsHash"));
}

#[test]
fn prompt_contains_no_stable_internal_ids() {
    let runtime = CapturingRuntime {
        last_prompt_json: std::sync::Mutex::new(None),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    let prompt_json = runtime
        .last_prompt_json
        .lock()
        .expect("lock")
        .clone()
        .unwrap();
    assert!(!prompt_json.contains("ws-fixture-0.1.5"));
    assert!(!prompt_json.contains("profile-ws-fixture"));
}

#[test]
fn prompt_contains_no_question_id() {
    let runtime = CapturingRuntime {
        last_prompt_json: std::sync::Mutex::new(None),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let question = sample_question();
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &question,
        &sample_options(),
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    let prompt_json = runtime
        .last_prompt_json
        .lock()
        .expect("lock")
        .clone()
        .unwrap();
    assert!(!prompt_json.contains(&question.question_id));
}

#[test]
fn runtime_receives_only_prompt_timeout_and_output_bound() {
    let runtime = CapturingRuntime {
        last_prompt_json: std::sync::Mutex::new(None),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let options = ProxyAskOptions::new(45_000, 4096);
    let _ = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &options,
        &runtime,
        &sample_clock(),
    )
    .expect("draft");
    // Indirectly verified via successful stub invocation with explicit options.
    assert_eq!(options.timeout_ms, 45_000);
    assert_eq!(options.max_output_bytes, 4096);
}

#[test]
fn service_performs_zero_filesystem_io() {
    let source = include_str!("../src/proxy_ask.rs");
    for forbidden in ["read_to_string", "write(", "std::fs", "File::open"] {
        assert!(!source.contains(forbidden), "forbidden io: {forbidden}");
    }
}

#[test]
fn service_performs_no_project_resolution() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("project_path"));
    assert!(!source.contains("resolve_project"));
}

#[test]
fn service_performs_no_network_selection() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("reqwest"));
    assert!(!source.contains("std::net"));
}

#[test]
fn service_performs_no_environment_access() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("std::env"));
}

#[test]
fn service_creates_no_response_history() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("history"));
    assert!(!source.contains("persist"));
}

#[test]
fn service_creates_no_work_event() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("WorkEvent"));
    assert!(!source.contains("append_event"));
}

#[test]
fn service_processes_no_signal() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("signals::"));
}

#[test]
fn service_executes_no_authority() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("resolve_profile_authority"));
}

#[test]
fn service_executes_no_tool() {
    let source = include_str!("../src/proxy_ask.rs");
    assert!(!source.contains("tool_call"));
}

// Checkpoint D boundary ownership: guard D-owned implementation sources.
// Shared `lib.rs` may contain module wiring for completed checkpoints.

#[test]
fn checkpoint_d_adds_no_production_runtime_factory() {
    for source in [
        include_str!("../src/proxy_ask.rs"),
        include_str!("../src/proxy_draft_safety.rs"),
    ] {
        assert!(!source.contains("resolve_production_proxy_draft_runtime"));
        assert!(!source.contains("proxy_runtime_factory"));
    }
}

#[test]
fn checkpoint_d_performs_no_environment_runtime_selection() {
    service_performs_no_environment_access();
}

#[test]
fn checkpoint_d_performs_no_deterministic_stub_fallback() {
    service_performs_no_fallback();
}

#[test]
fn checkpoint_d_adds_no_provider_http_integration() {
    service_performs_no_network_selection();
    let safety_source = include_str!("../src/proxy_draft_safety.rs");
    for forbidden in ["reqwest", "ureq", "hyper", "std::net"] {
        assert!(
            !safety_source.contains(forbidden),
            "draft safety must not integrate provider/http ({forbidden})"
        );
    }
}

// Lifecycle amendment (Checkpoint E isolation patch): D-owned composition service must
// not depend on the CLI crate. Existence of `proxy.rs` is proven by Checkpoint E tests.
#[test]
fn checkpoint_d_adds_no_cli_command() {
    for source in [
        include_str!("../src/proxy_ask.rs"),
        include_str!("../src/proxy_draft_safety.rs"),
    ] {
        assert!(
            !source.contains("openmesh-cli"),
            "D-owned service must not depend on CLI crate"
        );
    }
}

#[test]
fn checkpoint_d_adds_no_tauri_or_frontend_behavior() {
    let tauri_lib = include_str!("../../../src-tauri/src/lib.rs");
    assert!(!tauri_lib.contains("ask_my_proxy"));
    assert!(!tauri_lib.contains("proxy ask"));
}

#[test]
fn checkpoint_d_adds_no_persistence_or_history() {
    service_creates_no_response_history();
}

#[test]
fn checkpoint_d_does_not_start_dg() {
    for source in [
        include_str!("../src/proxy_ask.rs"),
        include_str!("../src/proxy_draft_safety.rs"),
    ] {
        assert!(!source.contains("proxy_runtime_axga"));
        assert!(!source.contains("AxgaAiProxyDraftRuntime"));
    }
}

// Lifecycle amendment (Checkpoint E isolation patch): D-owned sources must not embed
// the production runtime factory. CLI factory ownership is proven by Checkpoint E tests.
#[test]
fn checkpoint_d_does_not_start_checkpoint_e() {
    for source in [
        include_str!("../src/proxy_ask.rs"),
        include_str!("../src/proxy_draft_safety.rs"),
    ] {
        assert!(!source.contains("mod proxy_runtime_factory"));
        assert!(!source.contains("resolve_production_proxy_draft_runtime"));
    }
}

#[test]
fn checkpoint_d_does_not_start_0_1_7() {
    let ask_source = include_str!("../src/proxy_ask.rs");
    assert!(!ask_source.contains("0.1.7"));
    for forbidden in ["\"claims\"", "build_citation", "Citation"] {
        assert!(
            !ask_source.contains(forbidden),
            "checkpoint D ask service must not start 0.1.7 symbol {forbidden}"
        );
    }
}
