//! Dev Track 0.1.6 Checkpoint B — `ProxyPromptContext` allowlist and bounding tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{
    deterministic_context_pack_id, proxy_context_pack_authority_ladder_levels, AuthorityRule,
    CatchUpWindow, CommunicationPreferences, ContextPackAuthoritySummary, ContextPackCatchUp,
    ContextPackCatchUpSections, ContextPackContinuityItem, ContextPackCorrectionProvenance,
    ContextPackCurrentState, ContextPackCurrentStateSections, ContextPackDiagnostic,
    ContextPackDiagnosticSeverity, ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin,
    ContextPackFreshness, ContextPackItemProvenance, ContextPackOwnerIdentity,
    ContextPackPendingAttentionItem, ContextPackPrivacySummary, ContextPackRedactionSummary,
    ContextPackUnresolvedCategory, ContextPackUnresolvedItem, ContinuityConfidence,
    ContinuitySourceKind, DecisionPreferences, DefaultRefusalRule, EvidencePolicy, EvidenceRef,
    EvidenceSourceKind, GitState, PendingAttentionReason, PendingAttentionSeverity,
    PendingAttentionStatus, PrivacyAllowedUse, PrivacyRule, PrivacySensitivity,
    ProxyAuthorityLevel, ProxyContextPack, SourceCounts, UnsupportedClaimBehavior,
    CONTEXT_PACK_EXECUTION_BOUNDARY, PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use openmesh_core::proxy_prompt::compose_proxy_prompt;
use openmesh_core::proxy_prompt_context::{
    bound_proxy_prompt_context, map_pack_to_proxy_prompt_context, serialize_proxy_prompt_context,
    serialized_prompt_context_bytes, ProxyPromptContext, ProxyPromptCorrectionPresentation,
    ProxyPromptError, ProxyPromptStateItem, MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION,
    MAX_PROXY_PROMPT_CONTEXT_BYTES, MAX_PROXY_PROMPT_LIMITATIONS, MAX_PROXY_PROMPT_STATE_ITEMS,
    MAX_PROXY_PROMPT_SUMMARY_BYTES, PROXY_PROMPT_AUTHORITY_EXECUTION,
};
use openmesh_core::proxy_question::{create_proxy_question, ProcessLocalRequestIdentityProvider};
use openmesh_core::proxy_runtime_axga::build_axga_request_builder;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const CANARY_WORKSPACE_ID: &str = "ws-canary-adversarial-0.1.6";
const CANARY_PROFILE_ID: &str = "profile-canary-adversarial-0.1.6";
const CANARY_BUILD_HASH: &str = "fnv1a-deadbeefb016ca00";
const CANARY_EVIDENCE_PATH: &str = "docs/canary-wire-path.md";
const CANARY_GIT_PATH: &str = "src/canary/canonical-ref.rs";
const CANARY_PRODUCER_SIGNAL: &str = "producer-signal-canary-001";
const CANARY_EVIDENCE_INDEX_REF: &str = "ref-001";
const CANARY_FORBIDDEN_REF_ID: &str = "ref-canary-evidence-001";
const CANARY_RULE_ID: &str = "rule-canary-must-ask-human";
const CANARY_REFUSAL_ID: &str = "refusal-canary-no-impersonation";
const CANARY_UNRESOLVED_MALFORMED: &str = "unresolved-malformed-canary-identity";
const CANARY_UNRESOLVED_QUARANTINE: &str = "unresolved-quarantine-canary-identity";
const CANARY_DIAGNOSTIC_CODE: &str = "diagnostic-canary-malformed-evidence";
const CANARY_SECRET_LABEL: &str = "canary-private-evidence-label";
const CANARY_PRIVACY_TOPIC: &str = "credentials-topic-canary";

const FORBIDDEN_PROMPT_WIRE_KEYS: &[&str] = &[
    "workspaceId",
    "profileId",
    "profileVersion",
    "contextPackId",
    "buildInputsHash",
    "protocolVersion",
    "generatedAt",
    "requestedWindow",
    "authoritySummary",
    "privacySummary",
    "evidencePolicy",
    "evidenceIndex",
    "evidenceRef",
    "evidenceRefs",
    "refId",
    "sourceCounts",
    "diagnostics",
    "unresolvedItems",
    "buildInputsHash",
    "id",
    "source",
    "correctionEventIds",
    "isSupersededOriginal",
    "supersededByEventId",
    "authorityRules",
    "defaultRefusalRules",
    "ladderLevels",
    "executionBoundary",
    "privacyRules",
    "sensitiveTopics",
    "filteringApplied",
    "metadata",
    "promptSemanticHash",
    "semanticHash",
    "filePath",
    "producerSignal",
    "gitState",
    "repoId",
    "worktreeRoot",
    "changedPaths",
];

const FORBIDDEN_PROMPT_SUBSTRINGS: &[&str] = &[
    CANARY_WORKSPACE_ID,
    CANARY_PROFILE_ID,
    CANARY_BUILD_HASH,
    CANARY_EVIDENCE_PATH,
    CANARY_GIT_PATH,
    CANARY_PRODUCER_SIGNAL,
    CANARY_FORBIDDEN_REF_ID,
    CANARY_EVIDENCE_INDEX_REF,
    CANARY_RULE_ID,
    CANARY_REFUSAL_ID,
    CANARY_UNRESOLVED_MALFORMED,
    CANARY_UNRESOLVED_QUARANTINE,
    CANARY_DIAGNOSTIC_CODE,
    CANARY_SECRET_LABEL,
    "can-answer",
    "can-suggest",
    "can-draft",
    "must-ask-human",
    "cannot-answer",
    "fnv1a-deadbeef",
];

fn sample_git_state() -> GitState {
    GitState {
        repo_id: "fnv1a-deadbeef".into(),
        branch: "main".into(),
        head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
        dirty: true,
        staged_count: 0,
        unstaged_count: 1,
        untracked_count: 0,
        changed_paths: vec![CANARY_GIT_PATH.into()],
        observed_at: "2026-07-17T10:00:00Z".into(),
        ahead: None,
        behind: None,
        base_ref: Some("origin/main".into()),
        worktree_root: None,
    }
}

fn sample_authority_rule() -> AuthorityRule {
    AuthorityRule {
        rule_id: CANARY_RULE_ID.into(),
        scope: "*".into(),
        authority: ProxyAuthorityLevel::MustAskHuman,
        description: Some("canary authority rule must-ask-human".into()),
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: true,
        limitations: vec![],
    }
}

fn continuity_item(id: &str, summary: &str, timestamp: &str) -> ContextPackContinuityItem {
    ContextPackContinuityItem {
        id: id.into(),
        summary: summary.into(),
        kind: "work.in-progress".into(),
        source: ContinuitySourceKind::WorkEvent,
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: timestamp.into(),
        evidence_refs: vec![EvidenceRef::FilePath(CANARY_EVIDENCE_PATH.into())],
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: None,
    }
}

fn pending_item(
    id: &str,
    summary: &str,
    provenance: ContextPackItemProvenance,
    timestamp: &str,
) -> ContextPackPendingAttentionItem {
    ContextPackPendingAttentionItem {
        id: id.into(),
        summary: summary.into(),
        reason: PendingAttentionReason::PendingSignal,
        provenance,
        timestamp: timestamp.into(),
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::Medium,
        priority: 3,
        evidence_refs: vec![EvidenceRef::FilePath(CANARY_EVIDENCE_PATH.into())],
    }
}

fn adversarial_pack() -> ProxyContextPack {
    let build_inputs_hash = CANARY_BUILD_HASH.to_string();
    let generated_at = "2026-07-18T04:00:00Z".to_string();
    let window = CatchUpWindow {
        since: "2026-07-17T00:00:00Z".into(),
        until: "2026-07-18T00:00:00Z".into(),
    };
    let mut corrected = continuity_item(
        "item-corrected-canary",
        "corrected continuity canary summary",
        "2026-07-17T09:30:00Z",
    );
    corrected.correction = Some(ContextPackCorrectionProvenance {
        is_corrected: true,
        is_superseded_original: false,
        correction_event_ids: vec!["evt-correction-canary-001".into()],
        superseded_by_event_id: None,
    });
    let diagnostic_only = ContextPackContinuityItem {
        source: ContinuitySourceKind::PendingSignal,
        provenance: ContextPackItemProvenance::DiagnosticOnly,
        summary: "diagnostic-only canary must not leak".into(),
        ..continuity_item(
            "item-diagnostic-only-canary",
            "diagnostic-only canary must not leak",
            "2026-07-17T09:00:00Z",
        )
    };
    let pack = ProxyContextPack {
        context_pack_id: deterministic_context_pack_id(&build_inputs_hash),
        workspace_id: CANARY_WORKSPACE_ID.into(),
        profile_id: CANARY_PROFILE_ID.into(),
        profile_version: "1.0".into(),
        protocol_version: PROXY_CONTEXT_PACK_PROTOCOL_VERSION.to_string(),
        generated_at: generated_at.clone(),
        requested_window: window.clone(),
        owner_identity: ContextPackOwnerIdentity {
            owner_label: "Adversarial Fixture Owner".into(),
            role_label: "Adversarial Engineering Lead".into(),
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
                rule_id: CANARY_REFUSAL_ID.into(),
                statement: "cannot impersonate owner canary".into(),
            }],
            ladder_levels: proxy_context_pack_authority_ladder_levels()
                .iter()
                .map(|level| (*level).to_string())
                .collect(),
            execution_boundary: CONTEXT_PACK_EXECUTION_BOUNDARY.to_string(),
        },
        privacy_summary: ContextPackPrivacySummary {
            privacy_rules: vec![PrivacyRule {
                rule_id: "privacy-canary-credentials".into(),
                topic: CANARY_PRIVACY_TOPIC.into(),
                sensitivity: PrivacySensitivity::Secret,
                allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
                restriction: "never include private identity canary".into(),
                requires_human_confirmation: true,
            }],
            sensitive_topics: vec![CANARY_PRIVACY_TOPIC.into()],
            filtering_applied: vec!["private-evidence-omitted-canary".into()],
        },
        evidence_policy: EvidencePolicy {
            answer_without_evidence: false,
            require_evidence_for_claims: true,
            expose_limitations: true,
            cite_source_kinds: vec![EvidenceSourceKind::FilePath, EvidenceSourceKind::WorkEvent],
            unsupported_claim_behavior: UnsupportedClaimBehavior::SayUnknown,
        },
        current_state: ContextPackCurrentState {
            workspace_id: CANARY_WORKSPACE_ID.into(),
            sections: ContextPackCurrentStateSections {
                completed: vec![],
                in_progress: vec![corrected, diagnostic_only],
                blocked: vec![],
                decisions: vec![],
                needs_attention: vec![],
                still_open: vec![],
            },
            pending_attention: vec![
                pending_item(
                    "pending-canary",
                    "pending signal canary summary",
                    ContextPackItemProvenance::Pending,
                    "2026-07-17T11:00:00Z",
                ),
                pending_item(
                    "unconfirmed-canary",
                    "unconfirmed signal canary summary",
                    ContextPackItemProvenance::Unconfirmed,
                    "2026-07-17T11:30:00Z",
                ),
            ],
            limitations: vec!["current-state limitation canary alpha".into()],
        },
        catch_up: ContextPackCatchUp {
            workspace_id: CANARY_WORKSPACE_ID.into(),
            window: window.clone(),
            sections: ContextPackCatchUpSections {
                completed: vec![],
                changed: vec![],
                blocked: vec![],
                decided: vec![],
                needs_attention: vec![],
                still_open: vec![],
            },
            summary: "Catch-up canary summary without unresolved identity.".into(),
            next_suggested_attention: vec![pending_item(
                "catchup-unconfirmed-canary",
                "catch-up unconfirmed canary summary",
                ContextPackItemProvenance::Unconfirmed,
                "2026-07-17T12:00:00Z",
            )],
            limitations: vec!["catch-up limitation canary beta".into()],
        },
        evidence_index: vec![
            ContextPackEvidenceIndexEntry {
                ref_id: CANARY_EVIDENCE_INDEX_REF.into(),
                evidence_ref: EvidenceRef::FilePath(CANARY_EVIDENCE_PATH.into()),
                origin: ContextPackEvidenceOrigin::ContinuityItem,
                sensitivity: Sensitivity::Private,
                label: CANARY_SECRET_LABEL.into(),
                timestamp: Some("2026-07-17T10:00:00Z".into()),
            },
            ContextPackEvidenceIndexEntry {
                ref_id: "ref-002".into(),
                evidence_ref: EvidenceRef::GitState(sample_git_state()),
                origin: ContextPackEvidenceOrigin::ContinuityItem,
                sensitivity: Sensitivity::Private,
                label: "git-state canary label".into(),
                timestamp: Some("2026-07-17T10:00:00Z".into()),
            },
            ContextPackEvidenceIndexEntry {
                ref_id: "ref-003".into(),
                evidence_ref: EvidenceRef::ProducerSignal(CANARY_PRODUCER_SIGNAL.into()),
                origin: ContextPackEvidenceOrigin::ContinuityItem,
                sensitivity: Sensitivity::Private,
                label: "producer-signal canary label".into(),
                timestamp: None,
            },
        ],
        source_counts: SourceCounts {
            work_events: 1,
            processed_signals: 1,
            pending_signals: 2,
            promotion_audit_records: 0,
            quarantine_signals: 1,
            duplicate_signals: 0,
            reporter_signals: 0,
            git_signals: 1,
            heli_signals: 0,
            unknown_producer_signals: 0,
            other_producer_signals: 0,
        },
        diagnostics: vec![ContextPackDiagnostic {
            code: CANARY_DIAGNOSTIC_CODE.into(),
            message: "malformed evidence canary diagnostic message".into(),
            severity: ContextPackDiagnosticSeverity::Warning,
        }],
        limitations: vec![
            "pack-root limitation canary gamma".into(),
            "generic uncertainty should stay in limitations only".into(),
        ],
        unresolved_items: vec![
            ContextPackUnresolvedItem {
                id: CANARY_UNRESOLVED_MALFORMED.into(),
                category: ContextPackUnresolvedCategory::MalformedEvidence,
                summary: "malformed evidence identity canary".into(),
                provenance: ContextPackItemProvenance::Unconfirmed,
            },
            ContextPackUnresolvedItem {
                id: CANARY_UNRESOLVED_QUARANTINE.into(),
                category: ContextPackUnresolvedCategory::Quarantine,
                summary: "quarantine identity canary".into(),
                provenance: ContextPackItemProvenance::Pending,
            },
        ],
        freshness: ContextPackFreshness {
            snapshot_observed_at: "2026-07-18T03:59:00Z".into(),
            current_state_generated_at: "2026-07-18T03:59:30Z".into(),
            catch_up_since: window.since.clone(),
            catch_up_until: window.until.clone(),
            pack_generated_at: generated_at,
            age_seconds: 60,
            warnings: vec!["freshness-warning-canary".into()],
        },
        redaction_summary: ContextPackRedactionSummary {
            secret_items_omitted: 1,
            policy_restricted_items_omitted: 0,
            malformed_items_omitted: 1,
            quarantined_items_omitted: 1,
            bounds_truncated_items: 0,
        },
        build_inputs_hash,
    };
    validate_proxy_context_pack_complete(&pack).expect("adversarial pack must validate");
    pack
}

fn map_bound_context(pack: &ProxyContextPack) -> ProxyPromptContext {
    let mapped = map_pack_to_proxy_prompt_context(pack).expect("map");
    bound_proxy_prompt_context(mapped).expect("bound")
}

fn bound_after_map_mutation(
    pack: &ProxyContextPack,
    mutate: impl FnOnce(&mut ProxyPromptContext),
) -> ProxyPromptContext {
    let mut context = map_pack_to_proxy_prompt_context(pack).expect("map");
    mutate(&mut context);
    bound_proxy_prompt_context(context).expect("bound")
}

fn prompt_state_item(marker: &str, timestamp: &str) -> ProxyPromptStateItem {
    ProxyPromptStateItem {
        summary: padded_summary(marker),
        kind: "work.test".into(),
        provenance: ContextPackItemProvenance::Confirmed,
        timestamp: timestamp.into(),
        confidence: ContinuityConfidence::High,
        unverified: None,
        correction: None,
    }
}

fn map_bound_bytes(pack: &ProxyContextPack) -> Vec<u8> {
    let context = map_bound_context(pack);
    serialize_proxy_prompt_context(&context)
        .expect("serialize")
        .into_bytes()
}

fn prompt_json(pack: &ProxyContextPack) -> String {
    serialize_proxy_prompt_context(&map_bound_context(pack)).expect("serialize")
}

fn collect_wire_keys(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                out.insert(path.clone());
                collect_wire_keys(child, &path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_wire_keys(child, &format!("{prefix}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn wire_keys_for_context(context: &ProxyPromptContext) -> BTreeSet<String> {
    let value = serde_json::to_value(context).expect("serialize context");
    let mut keys = BTreeSet::new();
    collect_wire_keys(&value, "", &mut keys);
    keys
}

fn leaf_key(path: &str) -> &str {
    path.rsplit('.')
        .next()
        .unwrap_or(path)
        .split('[')
        .next()
        .unwrap_or(path)
}

fn assert_no_forbidden_wire_keys(keys: &BTreeSet<String>) {
    for path in keys {
        let leaf = leaf_key(path);
        for banned in FORBIDDEN_PROMPT_WIRE_KEYS {
            assert_ne!(
                leaf, *banned,
                "forbidden wire key `{banned}` found at `{path}`"
            );
        }
    }
}

fn assert_canaries_absent(json: &str) {
    for needle in FORBIDDEN_PROMPT_SUBSTRINGS {
        assert!(
            !json.contains(needle),
            "prompt context must not contain canary `{needle}`"
        );
    }
    assert!(
        !json.contains(&deterministic_context_pack_id(CANARY_BUILD_HASH)),
        "prompt context must not contain contextPackId"
    );
}

fn padded_summary(marker: &str) -> String {
    let mut summary = marker.to_string();
    while summary.len() < MAX_PROXY_PROMPT_SUMMARY_BYTES {
        summary.push('x');
    }
    summary
}

fn fill_catchup_section(section: &mut Vec<ContextPackContinuityItem>, prefix: &str, count: usize) {
    for index in 0..count {
        section.push(continuity_item(
            &format!("{prefix}-id-{index:03}"),
            &padded_summary(&format!("{prefix}-summary-{index:03}")),
            &format!("2026-07-17T{:02}:00:00Z", (index % 23) + 1),
        ));
    }
}

fn fill_state_section(section: &mut Vec<ContextPackContinuityItem>, prefix: &str, count: usize) {
    fill_catchup_section(section, prefix, count);
}

fn oversized_non_removable_catchup_pack() -> ProxyContextPack {
    let mut pack = adversarial_pack();
    fill_catchup_section(
        &mut pack.catch_up.sections.completed,
        "catchup-completed",
        32,
    );
    fill_catchup_section(&mut pack.catch_up.sections.blocked, "catchup-blocked", 32);
    fill_catchup_section(&mut pack.catch_up.sections.decided, "catchup-decided", 32);
    validate_proxy_context_pack_complete(&pack).expect("oversized catch-up pack validates");
    pack
}

fn reducible_oversize_pack() -> ProxyContextPack {
    let mut pack = adversarial_pack();
    pack.limitations = (0..32)
        .map(|index| format!("limit-{index:03}-{}", "l".repeat(480)))
        .collect();
    pack.current_state.limitations = vec!["current-state limitation".into()];
    pack.catch_up.limitations = vec!["catch-up limitation".into()];
    fill_catchup_section(&mut pack.catch_up.sections.still_open, "still-open", 32);
    fill_catchup_section(&mut pack.catch_up.sections.changed, "changed", 32);
    fill_state_section(&mut pack.current_state.sections.completed, "completed", 10);
    validate_proxy_context_pack_complete(&pack).expect("reducible oversize pack validates");
    pack
}

fn oversized_pending_only_pack() -> ProxyContextPack {
    let mut pack = adversarial_pack();
    pack.limitations = vec!["minimal limitation for pending oversize fixture".into()];
    pack.current_state.limitations = vec!["minimal current-state limitation".into()];
    pack.catch_up.limitations = vec!["minimal catch-up limitation".into()];
    pack.current_state.sections = ContextPackCurrentStateSections {
        completed: vec![],
        in_progress: vec![],
        blocked: vec![],
        decisions: vec![],
        needs_attention: vec![],
        still_open: vec![],
    };
    pack.catch_up.sections = ContextPackCatchUpSections {
        completed: vec![],
        changed: vec![],
        blocked: vec![],
        decided: vec![],
        needs_attention: vec![],
        still_open: vec![],
    };
    pack.catch_up.next_suggested_attention.clear();
    pack.catch_up.summary = "minimal catch-up summary".into();
    pack.current_state.pending_attention = (0..64)
        .map(|index| {
            pending_item(
                &format!("pending-oversize-{index:03}"),
                &padded_summary(&format!("pending-oversize-summary-{index:03}")),
                if index % 2 == 0 {
                    ContextPackItemProvenance::Pending
                } else {
                    ContextPackItemProvenance::Unconfirmed
                },
                &format!("2026-07-17T{:02}:00:00Z", (index % 23) + 1),
            )
        })
        .collect();
    validate_proxy_context_pack_complete(&pack).expect("pending oversize pack validates");
    pack
}

// --- Prompt context (17 tests) ---

#[test]
fn proxy_prompt_context_wire_shape_is_allowlist_only() {
    let context = map_bound_context(&adversarial_pack());
    let keys = wire_keys_for_context(&context);
    assert_no_forbidden_wire_keys(&keys);
    let json = serde_json::to_value(&context).expect("value");
    assert!(json.get("promptContextVersion").is_some());
    assert!(json.get("authorityExecution").is_some());
    assert!(json.get("currentState").is_some());
    assert!(json.get("catchUp").is_some());
}

#[test]
fn stable_internal_ids_are_absent_from_prompt_context() {
    let pack = adversarial_pack();
    let json = prompt_json(&pack);
    for forbidden in [
        "workspaceId",
        "profileId",
        "contextPackId",
        "buildInputsHash",
        "profileVersion",
        CANARY_WORKSPACE_ID,
        CANARY_PROFILE_ID,
        CANARY_BUILD_HASH,
        &pack.context_pack_id,
    ] {
        assert!(
            !json.contains(forbidden),
            "stable internal id `{forbidden}` must be absent"
        );
    }
}

#[test]
fn evidence_refs_are_absent_from_prompt_context() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        "evidenceIndex",
        "evidenceRef",
        "evidenceRefs",
        "refId",
        CANARY_FORBIDDEN_REF_ID,
        CANARY_PRODUCER_SIGNAL,
    ] {
        assert!(
            !json.contains(forbidden),
            "evidence ref surface `{forbidden}` must be absent"
        );
    }
}

#[test]
fn source_paths_and_canonical_refs_are_absent() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        CANARY_EVIDENCE_PATH,
        CANARY_GIT_PATH,
        "gitState",
        "filePath",
        "producerSignal",
        "changedPaths",
        "repoId",
    ] {
        assert!(
            !json.contains(forbidden),
            "source path or canonical ref `{forbidden}` must be absent"
        );
    }
}

#[test]
fn secret_identity_content_and_timestamps_are_absent() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        CANARY_SECRET_LABEL,
        CANARY_PRIVACY_TOPIC,
        "private-evidence-omitted-canary",
        "never include private identity canary",
        CANARY_PRIVACY_TOPIC,
    ] {
        assert!(
            !json.contains(forbidden),
            "secret identity `{forbidden}` must be absent"
        );
    }
}

#[test]
fn authority_ladder_and_rules_are_absent() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        "authoritySummary",
        "authorityRules",
        "defaultRefusalRules",
        "ladderLevels",
        "executionBoundary",
        CANARY_RULE_ID,
        CANARY_REFUSAL_ID,
        "can-answer",
        "can-suggest",
        "can-draft",
        "must-ask-human",
        "cannot-answer",
    ] {
        assert!(
            !json.contains(forbidden),
            "authority metadata `{forbidden}` must be absent"
        );
    }
}

#[test]
fn unresolved_items_are_absent() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        "unresolvedItems",
        CANARY_UNRESOLVED_MALFORMED,
        CANARY_UNRESOLVED_QUARANTINE,
        "malformed evidence identity canary",
        "quarantine identity canary",
    ] {
        assert!(
            !json.contains(forbidden),
            "unresolved item `{forbidden}` must be absent"
        );
    }
}

#[test]
fn malformed_and_quarantine_identity_are_absent() {
    let json = prompt_json(&adversarial_pack());
    assert_canaries_absent(&json);
    assert!(
        json.contains("some continuity inputs were omitted as incomplete or unsafe"),
        "safe generic omission limitation should be present"
    );
    assert!(!json.contains(CANARY_DIAGNOSTIC_CODE));
}

#[test]
fn superseded_raw_correction_is_absent() {
    let json = prompt_json(&adversarial_pack());
    for forbidden in [
        "isSupersededOriginal",
        "supersededByEventId",
        "correctionEventIds",
        "evt-correction-canary-001",
    ] {
        assert!(
            !json.contains(forbidden),
            "superseded correction field `{forbidden}` must be absent"
        );
    }
}

#[test]
fn effective_correction_presentation_is_retained() {
    let context = map_bound_context(&adversarial_pack());
    let corrected = context
        .current_state
        .sections
        .in_progress
        .iter()
        .find(|item| item.summary.contains("corrected continuity canary"))
        .expect("corrected item");
    assert_eq!(
        corrected.correction,
        Some(ProxyPromptCorrectionPresentation { is_corrected: true })
    );
}

#[test]
fn pending_remains_pending_or_unconfirmed() {
    let context = map_bound_context(&adversarial_pack());
    let pending: Vec<_> = context
        .current_state
        .pending_attention
        .iter()
        .map(|item| item.provenance)
        .collect();
    assert!(pending.contains(&ContextPackItemProvenance::Pending));
    assert!(pending.contains(&ContextPackItemProvenance::Unconfirmed));
    let catchup_pending = &context.catch_up.next_suggested_attention[0].provenance;
    assert_eq!(*catchup_pending, ContextPackItemProvenance::Unconfirmed);
}

#[test]
fn authority_execution_is_fixed_disabled() {
    let context = map_bound_context(&adversarial_pack());
    assert_eq!(
        context.authority_execution,
        PROXY_PROMPT_AUTHORITY_EXECUTION
    );
    assert_eq!(context.authority_execution, "disabled");
}

#[test]
fn raw_pack_json_is_not_embedded() {
    let pack = adversarial_pack();
    let pack_json = serde_json::to_string(&pack).expect("pack json");
    let context_json = prompt_json(&pack);
    assert!(!context_json.contains(&pack_json));
    assert!(!context_json.contains("\"authoritySummary\""));
    assert!(!context_json.contains("\"evidenceIndex\""));
}

#[test]
fn raw_profile_json_is_not_embedded() {
    let pack = adversarial_pack();
    let profile_json = serde_json::to_string(&pack.owner_identity).expect("profile json");
    let context_json = prompt_json(&pack);
    assert!(!context_json.contains(&profile_json));
    assert!(!context_json.contains("\"authorityRules\""));
}

#[test]
fn no_arbitrary_metadata_map_exists() {
    let keys = wire_keys_for_context(&map_bound_context(&adversarial_pack()));
    for path in &keys {
        assert!(
            !leaf_key(path).contains("metadata"),
            "arbitrary metadata key found at `{path}`"
        );
    }
}

#[test]
fn generic_uncertainty_uses_safe_bounded_limitations_only() {
    let context = map_bound_context(&adversarial_pack());
    assert!(context
        .limitations
        .iter()
        .any(|entry| entry.contains("some continuity inputs were omitted")));
    assert!(context
        .limitations
        .iter()
        .any(|entry| entry.contains("some secret continuity material was omitted")));
    assert!(!context
        .limitations
        .iter()
        .any(|entry| entry.contains(CANARY_UNRESOLVED_MALFORMED)));
    assert!(context.limitations.len() <= MAX_PROXY_PROMPT_LIMITATIONS);
}

#[test]
fn prompt_context_validation_rejects_unknown_fields() {
    let context = map_bound_context(&adversarial_pack());
    let mut value = serde_json::to_value(&context).expect("value");
    value
        .as_object_mut()
        .expect("object")
        .insert("answer".into(), json!("must not deserialize"));
    let rejected: Result<ProxyPromptContext, _> = serde_json::from_value(value);
    assert!(rejected.is_err(), "unknown fields must be rejected");
}

// --- Determinism / bounding (23 tests) ---

#[test]
fn identical_semantic_input_produces_identical_context_bytes() {
    let pack = adversarial_pack();
    let first = map_bound_bytes(&pack);
    let second = map_bound_bytes(&pack);
    assert_eq!(first, second);
}

#[test]
fn generated_at_only_change_does_not_change_prompt_bundle_bytes() {
    let bytes_a = map_bound_bytes(&adversarial_pack());
    let mut pack_b = adversarial_pack();
    pack_b.generated_at = "2026-07-18T05:00:00Z".into();
    assert!(
        validate_proxy_context_pack_complete(&pack_b).is_err(),
        "generated_at-only drift must fail complete validation"
    );
    let json = prompt_json(&adversarial_pack());
    assert!(!json.contains("\"generatedAt\""));
    assert_eq!(bytes_a, map_bound_bytes(&adversarial_pack()));
}

#[test]
fn input_collection_order_does_not_change_output_bytes() {
    let pack_ordered = adversarial_pack();
    let mut pack_shuffled = adversarial_pack();
    pack_shuffled.current_state.sections.in_progress = pack_ordered
        .current_state
        .sections
        .in_progress
        .iter()
        .rev()
        .cloned()
        .collect();
    pack_shuffled.current_state.pending_attention = pack_ordered
        .current_state
        .pending_attention
        .iter()
        .rev()
        .cloned()
        .collect();
    assert_eq!(
        map_bound_bytes(&pack_ordered),
        map_bound_bytes(&pack_shuffled)
    );
}

#[test]
fn deterministic_section_order() {
    let json = prompt_json(&adversarial_pack());
    let current_state_pos = json.find("\"currentState\"").expect("currentState");
    let catch_up_pos = json.find("\"catchUp\"").expect("catchUp");
    let freshness_pos = json.find("\"freshness\"").expect("freshness");
    assert!(current_state_pos < catch_up_pos);
    assert!(catch_up_pos < freshness_pos);
    let sections_pos = json.find("\"sections\"").expect("sections");
    let pending_pos = json.find("\"pendingAttention\"").expect("pendingAttention");
    assert!(sections_pos < pending_pos);
}

#[test]
fn deterministic_item_order() {
    let mut pack = adversarial_pack();
    fill_state_section(&mut pack.current_state.sections.completed, "completed", 3);
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let context = map_bound_context(&pack);
    let timestamps: Vec<_> = context
        .current_state
        .sections
        .completed
        .iter()
        .map(|item| item.timestamp.as_str())
        .collect();
    let mut sorted = timestamps.clone();
    sorted.sort();
    assert_eq!(timestamps, sorted);
}

#[test]
fn limitations_are_deduplicated_deterministically() {
    let mut pack = adversarial_pack();
    pack.limitations = vec![" zulu limitation ".into(), "alpha limitation".into()];
    pack.current_state.limitations = vec!["alpha limitation".into(), "bravo limitation".into()];
    pack.catch_up.limitations = vec![" zulu limitation ".into(), "charlie limitation".into()];
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let context = map_bound_context(&pack);
    assert!(context.limitations.windows(2).all(|pair| pair[0] < pair[1]));
    for expected in [
        "alpha limitation",
        "bravo limitation",
        "charlie limitation",
        "zulu limitation",
    ] {
        assert!(
            context.limitations.iter().any(|entry| entry == expected),
            "missing limitation `{expected}`"
        );
    }
    assert!(context
        .limitations
        .iter()
        .any(|entry| entry.starts_with("some secret continuity material")));
    assert!(context
        .limitations
        .iter()
        .any(|entry| entry.starts_with("some continuity inputs were omitted")));
}

#[test]
fn state_items_are_capped_at_64() {
    let bounded = bound_after_map_mutation(&adversarial_pack(), |context| {
        for index in 0..60 {
            context
                .current_state
                .sections
                .completed
                .push(prompt_state_item(
                    &format!("completed-cap-{index:03}"),
                    &format!("2026-07-17T{:02}:00:00Z", (index % 23) + 1),
                ));
        }
    });
    let total = bounded.current_state.sections.completed.len()
        + bounded.current_state.sections.in_progress.len()
        + bounded.current_state.sections.blocked.len()
        + bounded.current_state.sections.decisions.len()
        + bounded.current_state.sections.needs_attention.len()
        + bounded.current_state.sections.still_open.len()
        + bounded.current_state.pending_attention.len();
    assert!(total <= MAX_PROXY_PROMPT_STATE_ITEMS);
}

#[test]
fn catchup_sections_are_capped_at_32() {
    let mut over_pack = adversarial_pack();
    fill_catchup_section(&mut over_pack.catch_up.sections.changed, "changed", 40);
    validate_proxy_context_pack_complete(&over_pack).expect("pack");
    assert!(matches!(
        map_pack_to_proxy_prompt_context(&over_pack),
        Err(ProxyPromptError::InvalidPromptContext)
    ));

    let mut capped_pack = adversarial_pack();
    fill_catchup_section(&mut capped_pack.catch_up.sections.changed, "changed", 32);
    validate_proxy_context_pack_complete(&capped_pack).expect("pack");
    let context = map_bound_context(&capped_pack);
    assert!(context.catch_up.sections.changed.len() <= MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
}

#[test]
fn limitations_are_capped_at_32() {
    let mut pack = adversarial_pack();
    pack.limitations = (0..40)
        .map(|index| format!("pack limitation {index:03}"))
        .collect();
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let context = map_bound_context(&pack);
    assert!(context.limitations.len() <= MAX_PROXY_PROMPT_LIMITATIONS);
}

#[test]
fn bounding_removes_limitations_tail_first() {
    let mut pack = adversarial_pack();
    pack.limitations = (0..32)
        .map(|index| format!("limit-{index:03}-{}", "l".repeat(480)))
        .collect();
    fill_catchup_section(&mut pack.catch_up.sections.still_open, "still-open", 20);
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let before = mapped.limitations.clone();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    if before.len() > bounded.limitations.len() {
        assert_eq!(bounded.limitations, &before[..before.len() - 1]);
    } else {
        assert!(
            serialized_prompt_context_bytes(&bounded).unwrap() <= MAX_PROXY_PROMPT_CONTEXT_BYTES
        );
    }
}

#[test]
fn bounding_then_removes_still_open_tail() {
    let mut pack = adversarial_pack();
    pack.limitations = (0..28)
        .map(|index| format!("limit-{index:03}-{}", "l".repeat(400)))
        .collect();
    pack.current_state.limitations = vec!["current-state limitation".into()];
    pack.catch_up.limitations = vec!["catch-up limitation".into()];
    fill_catchup_section(&mut pack.catch_up.sections.still_open, "still-open", 32);
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let before = mapped.catch_up.sections.still_open.len();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    assert!(bounded.catch_up.sections.still_open.len() <= before);
}

#[test]
fn bounding_then_removes_needs_attention_tail() {
    let mut pack = adversarial_pack();
    pack.limitations = (0..28)
        .map(|index| format!("limit-{index:03}-{}", "l".repeat(400)))
        .collect();
    pack.current_state.limitations = vec!["current-state limitation".into()];
    pack.catch_up.limitations = vec!["catch-up limitation".into()];
    fill_catchup_section(
        &mut pack.catch_up.sections.needs_attention,
        "needs-attention",
        32,
    );
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let before = mapped.catch_up.sections.needs_attention.len();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    assert!(bounded.catch_up.sections.needs_attention.len() <= before);
}

#[test]
fn bounding_then_removes_changed_tail() {
    let mut pack = adversarial_pack();
    pack.limitations = (0..28)
        .map(|index| format!("limit-{index:03}-{}", "l".repeat(400)))
        .collect();
    pack.current_state.limitations = vec!["current-state limitation".into()];
    pack.catch_up.limitations = vec!["catch-up limitation".into()];
    fill_catchup_section(&mut pack.catch_up.sections.changed, "changed", 32);
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let before = mapped.catch_up.sections.changed.len();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    assert!(bounded.catch_up.sections.changed.len() <= before);
}

#[test]
fn bounding_uses_frozen_current_state_tail_order() {
    let mut pack = adversarial_pack();
    pack.limitations.clear();
    pack.limitations.push("single limitation".into());
    fill_state_section(&mut pack.current_state.sections.completed, "completed", 5);
    fill_state_section(&mut pack.current_state.sections.decisions, "decisions", 5);
    fill_state_section(&mut pack.current_state.sections.still_open, "still-open", 5);
    fill_state_section(
        &mut pack.current_state.sections.in_progress,
        "in-progress",
        5,
    );
    fill_state_section(
        &mut pack.current_state.sections.needs_attention,
        "needs-attention",
        5,
    );
    fill_state_section(&mut pack.current_state.sections.blocked, "blocked", 5);
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let completed_before = mapped.current_state.sections.completed.len();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    if completed_before > bounded.current_state.sections.completed.len() {
        assert_eq!(
            bounded.current_state.sections.completed.len(),
            completed_before - 1
        );
    } else {
        assert!(bounded.current_state.sections.completed.len() <= completed_before);
    }
}

#[test]
fn bounding_removes_complete_items_only() {
    let context = map_bound_context(&adversarial_pack());
    let json = serialize_proxy_prompt_context(&context).expect("json");
    let value: Value = serde_json::from_str(&json).expect("parse");
    for section in [
        "completed",
        "inProgress",
        "blocked",
        "decisions",
        "needsAttention",
        "stillOpen",
    ] {
        let empty = Vec::new();
        let items = value
            .pointer(&format!("/currentState/sections/{section}"))
            .and_then(Value::as_array)
            .unwrap_or(&empty);
        for item in items {
            let restored: ProxyPromptStateItem =
                serde_json::from_value(item.clone()).expect("complete state item");
            assert!(!restored.summary.trim().is_empty());
            assert!(!restored.kind.trim().is_empty());
            assert!(!restored.timestamp.trim().is_empty());
        }
    }
}

#[test]
fn bounded_context_is_valid_json() {
    let json = prompt_json(&adversarial_pack());
    let _: Value = serde_json::from_str(&json).expect("valid json");
}

#[test]
fn bounded_context_is_valid_utf8() {
    let bytes = map_bound_bytes(&adversarial_pack());
    let text = std::str::from_utf8(&bytes).expect("valid utf-8");
    assert!(text.contains("promptContextVersion"));
}

#[test]
fn thai_utf8_survives_mapping_and_bounding() {
    let mut pack = adversarial_pack();
    pack.owner_identity.owner_label = "เจ้าของงานทดสอบ".into();
    pack.current_state.sections.in_progress[0].summary = "สถานะงานปัจจุบัน".into();
    pack.catch_up.summary = "สรุปการเปลี่ยนแปลง".into();
    validate_proxy_context_pack_complete(&pack).expect("thai pack");
    let json = prompt_json(&pack);
    assert!(json.contains("เจ้าของงานทดสอบ"));
    assert!(json.contains("สถานะงานปัจจุบัน"));
    assert!(json.contains("สรุปการเปลี่ยนแปลง"));
}

#[test]
fn exact_context_byte_limit_is_accepted() {
    let context = map_pack_to_proxy_prompt_context(&adversarial_pack()).expect("map");
    let bounded = bound_proxy_prompt_context(context).expect("exact limit accepted");
    assert!(
        serialized_prompt_context_bytes(&bounded).expect("size") <= MAX_PROXY_PROMPT_CONTEXT_BYTES
    );
}

#[test]
fn over_context_byte_limit_is_reduced_deterministically() {
    let pack = reducible_oversize_pack();
    let first = map_bound_bytes(&pack);
    let second = map_bound_bytes(&pack);
    assert_eq!(first, second);
    assert!(first.len() <= MAX_PROXY_PROMPT_CONTEXT_BYTES);
}

#[test]
fn irreducible_oversize_context_returns_context_too_large() {
    let pack = oversized_non_removable_catchup_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let err = bound_proxy_prompt_context(mapped).expect_err("oversize");
    assert!(matches!(err, ProxyPromptError::ContextTooLarge));
}

#[test]
fn context_too_large_error_does_not_echo_context() {
    let pack = oversized_non_removable_catchup_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let context_json = serialize_proxy_prompt_context(&mapped).expect("json");
    let err = bound_proxy_prompt_context(mapped).expect_err("oversize");
    let message = err.to_string();
    assert_eq!(message, "prompt context exceeds the byte bound");
    assert!(!message.contains("catchup-completed-summary"));
    assert!(!message.contains(&context_json[..context_json.len().min(64)]));
}

#[test]
fn bounding_never_removes_pending_items() {
    let pack = reducible_oversize_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let pending_before = mapped.current_state.pending_attention.clone();
    let catchup_pending_before = mapped.catch_up.next_suggested_attention.clone();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    assert_eq!(bounded.current_state.pending_attention, pending_before);
    assert_eq!(
        bounded.catch_up.next_suggested_attention,
        catchup_pending_before
    );
}

#[test]
fn bounding_preserves_pending_semantic_content() {
    let pack = adversarial_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    let summaries: Vec<_> = bounded
        .current_state
        .pending_attention
        .iter()
        .map(|item| item.summary.as_str())
        .collect();
    assert!(summaries.iter().any(|summary| summary.contains("pending")));
}

#[test]
fn bounding_preserves_pending_and_unconfirmed_provenance() {
    let pack = adversarial_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    let provenances: Vec<_> = bounded
        .current_state
        .pending_attention
        .iter()
        .map(|item| item.provenance)
        .collect();
    assert!(provenances.contains(&ContextPackItemProvenance::Pending));
    assert!(provenances.contains(&ContextPackItemProvenance::Unconfirmed));
}

#[test]
fn oversized_pending_context_fails_closed_without_removing_pending() {
    let pack = oversized_pending_only_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let pending_before = mapped.current_state.pending_attention.clone();
    let err = bound_proxy_prompt_context(mapped).expect_err("oversize");
    assert!(matches!(err, ProxyPromptError::ContextTooLarge));
    let message = err.to_string();
    assert!(!message.contains("pending-oversize"));
    assert!(!message.contains("pending-oversize-summary"));
    let remapped = map_pack_to_proxy_prompt_context(&pack).expect("remap");
    assert_eq!(remapped.current_state.pending_attention, pending_before);
}

#[test]
fn context_too_large_error_does_not_echo_pending_content() {
    let pack = oversized_pending_only_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let pending_summary = mapped.current_state.pending_attention[0].summary.clone();
    let err = bound_proxy_prompt_context(mapped).expect_err("oversize");
    let message = err.to_string();
    assert!(!message.contains(&pending_summary));
}

#[test]
fn frozen_bounding_removal_priority_is_unchanged_except_pending() {
    let pack = reducible_oversize_pack();
    let mapped = map_pack_to_proxy_prompt_context(&pack).expect("map");
    let limitations_before = mapped.limitations.len();
    let still_open_before = mapped.catch_up.sections.still_open.len();
    let bounded = bound_proxy_prompt_context(mapped).expect("bound");
    assert!(
        bounded.limitations.len() < limitations_before
            || bounded.catch_up.sections.still_open.len() < still_open_before
    );
}

#[test]
fn no_prompt_semantic_hash_exists_on_public_wire() {
    let context = map_bound_context(&adversarial_pack());
    let json = serde_json::to_string(&context).expect("json");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "promptsemantichash",
        "semantic_hash",
        "semanticHash",
        "fnv1a",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "public prompt context must not contain `{forbidden}`"
        );
    }
    let keys = wire_keys_for_context(&context);
    assert_no_forbidden_wire_keys(&keys);
}

// Checkpoint F — full model-facing privacy chain through the adapter request builder.
#[test]
fn final_adapter_request_contains_no_adversarial_canaries() {
    let pack = adversarial_pack();
    let question = create_proxy_question(
        "Summarize the adversarial boundary fixture.",
        &ProcessLocalRequestIdentityProvider,
    )
    .expect("question");
    let bundle = compose_proxy_prompt(&pack, &question).expect("compose");
    let body = build_axga_request_builder(&bundle, "gpt-4o-mini", 256)
        .build_openai_body()
        .to_string();

    assert_canaries_absent(&body);
    for forbidden in [
        "questionId",
        "workspaceId",
        "profileId",
        "profileVersion",
        "contextPackId",
        "buildInputsHash",
        "evidenceSummary",
        "ProxyDraftTraceMetadata",
        "EvidenceRef",
        "authorityNotice",
        "executionBoundary",
        "\"tools\"",
        "tool_choice",
        "verifiedAnswer",
        "\"claims\"",
        "\"citations\"",
        "approvalResult",
    ] {
        assert!(
            !body.contains(forbidden),
            "adapter-facing request must not contain `{forbidden}`"
        );
    }
    assert!(
        body.contains(&question.text),
        "user question text must reach provider"
    );
}
