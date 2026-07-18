//! Dev Track 0.1.5 Checkpoint A — Proxy Context Pack domain contracts (pure).

use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{
    deterministic_context_pack_id, is_supported_proxy_context_pack_protocol,
    proxy_context_pack_authority_ladder_levels, validate_proxy_context_pack, AuthorityRule,
    CatchUpWindow, CommunicationPreferences, ContextPackAuthoritySummary, ContextPackCatchUp,
    ContextPackCatchUpSections, ContextPackContinuityItem, ContextPackCorrectionProvenance,
    ContextPackCurrentState, ContextPackCurrentStateSections, ContextPackDiagnostic,
    ContextPackDiagnosticSeverity, ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin,
    ContextPackFreshness, ContextPackItemProvenance, ContextPackOwnerIdentity,
    ContextPackPendingAttentionItem, ContextPackPrivacySummary, ContextPackRedactionSummary,
    ContextPackUnresolvedCategory, ContextPackUnresolvedItem, ContextPackValidationError,
    ContinuityConfidence, ContinuitySourceKind, DecisionPreferences, DefaultRefusalRule,
    EvidencePolicy, EvidenceRef, EvidenceSourceKind, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, PrivacyAllowedUse, PrivacyRule,
    PrivacySensitivity, ProxyAuthorityLevel, ProxyContextPack, SourceCounts,
    UnsupportedClaimBehavior, CONTEXT_PACK_EXECUTION_BOUNDARY, PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use std::fs;
use std::path::PathBuf;

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
            ladder_levels: proxy_context_pack_authority_ladder_levels()
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
            age_seconds: 3600,
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

#[test]
fn proxy_context_pack_round_trips_json() {
    let pack = sample_pack();
    let json = serde_json::to_string(&pack).expect("serialize");
    let restored: ProxyContextPack = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, pack);
}

#[test]
fn proxy_context_pack_uses_protocol_version_1_0() {
    let pack = sample_pack();
    assert_eq!(pack.protocol_version, "1.0");
    assert!(is_supported_proxy_context_pack_protocol(
        &pack.protocol_version
    ));
}

#[test]
fn proxy_context_pack_rejects_unknown_fields() {
    let json = r#"{
        "contextPackId": "context-pack-fnv1a-abc",
        "workspaceId": "ws-1",
        "profileId": "profile-1",
        "profileVersion": "1.0",
        "protocolVersion": "1.0",
        "generatedAt": "2026-07-18T04:00:00Z",
        "requestedWindow": { "since": "2026-07-17T00:00:00Z", "until": "2026-07-18T00:00:00Z" },
        "ownerIdentity": { "ownerLabel": "Owner", "roleLabel": "Role" },
        "communicationPreferences": {
            "tone": "direct", "detailLevel": "medium",
            "asyncPreference": "prefer-async", "correctionPreference": "surface-limitations"
        },
        "decisionPreferences": {
            "decisionStyle": "evidence-first", "escalationPreference": "ask-human-on-ambiguity"
        },
        "authoritySummary": {
            "authorityRules": [{
                "ruleId": "rule-global", "scope": "*", "authority": "must-ask-human",
                "evidenceRequired": true, "humanConfirmationRequired": true, "conditions": [], "limitations": []
            }],
            "defaultRefusalRules": [{ "ruleId": "refusal-no-impersonation", "statement": "cannot impersonate owner" }],
            "ladderLevels": ["can-answer","can-suggest","can-draft","must-ask-human","cannot-answer"],
            "executionBoundary": "policy-metadata-only; no runtime authority execution in 0.1.5"
        },
        "privacySummary": {
            "privacyRules": [],
            "sensitiveTopics": [],
            "filteringApplied": []
        },
        "evidencePolicy": {
            "answerWithoutEvidence": false,
            "requireEvidenceForClaims": true,
            "exposeLimitations": true,
            "citeSourceKinds": ["file-path"],
            "unsupportedClaimBehavior": "say-unknown"
        },
        "currentState": {
            "workspaceId": "ws-1",
            "sections": {
                "completed": [], "inProgress": [], "blocked": [], "decisions": [],
                "needsAttention": [], "stillOpen": []
            },
            "pendingAttention": [],
            "limitations": ["one"]
        },
        "catchUp": {
            "workspaceId": "ws-1",
            "window": { "since": "2026-07-17T00:00:00Z", "until": "2026-07-18T00:00:00Z" },
            "sections": {
                "completed": [], "changed": [], "blocked": [], "decided": [],
                "needsAttention": [], "stillOpen": []
            },
            "summary": "summary",
            "nextSuggestedAttention": [],
            "limitations": ["one"]
        },
        "evidenceIndex": [],
        "sourceCounts": {
            "workEvents": 0, "processedSignals": 0, "pendingSignals": 0,
            "promotionAuditRecords": 0, "quarantineSignals": 0, "duplicateSignals": 0,
            "reporterSignals": 0, "gitSignals": 0, "heliSignals": 0,
            "unknownProducerSignals": 0, "otherProducerSignals": 0
        },
        "diagnostics": [],
        "limitations": ["one"],
        "unresolvedItems": [],
        "freshness": {
            "snapshotObservedAt": "2026-07-18T03:59:00Z",
            "currentStateGeneratedAt": "2026-07-18T03:59:30Z",
            "catchUpSince": "2026-07-17T00:00:00Z",
            "catchUpUntil": "2026-07-18T00:00:00Z",
            "packGeneratedAt": "2026-07-18T04:00:00Z",
            "ageSeconds": 0,
            "warnings": []
        },
        "redactionSummary": {
            "secretItemsOmitted": 0,
            "policyRestrictedItemsOmitted": 0,
            "malformedItemsOmitted": 0,
            "quarantinedItemsOmitted": 0,
            "boundsTruncatedItems": 0
        },
        "buildInputsHash": "fnv1a-abc",
        "answer": "must not deserialize"
    }"#;
    let result: Result<ProxyContextPack, _> = serde_json::from_str(json);
    assert!(result.is_err(), "unknown top-level fields must be rejected");
}

#[test]
fn proxy_context_pack_requires_non_empty_identity_fields() {
    let mut pack = sample_pack();
    pack.workspace_id = "   ".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::EmptyWorkspaceId)
    ));

    let mut pack = sample_pack();
    pack.profile_id = "".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::EmptyProfileId)
    ));
}

#[test]
fn proxy_context_pack_requires_valid_utc_timestamps() {
    let mut pack = sample_pack();
    pack.generated_at = "2026-07-18T04:00:00-05:00".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::InvalidTimestamp(_))
    ));
}

#[test]
fn requested_window_rejects_since_after_until() {
    let mut pack = sample_pack();
    pack.requested_window.since = "2026-07-19T00:00:00Z".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::CatchUpWindowInverted)
    ));
}

#[test]
fn context_pack_id_and_build_inputs_hash_are_required() {
    let mut pack = sample_pack();
    pack.build_inputs_hash = "".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::EmptyBuildInputsHash)
    ));

    let mut pack = sample_pack();
    pack.context_pack_id = "context-pack-wrong".into();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::InvalidOwnerIdentity(_))
    ));
}

#[test]
fn authority_ladder_uses_exact_wire_values() {
    let levels = proxy_context_pack_authority_ladder_levels();
    assert_eq!(
        levels,
        [
            "can-answer",
            "can-suggest",
            "can-draft",
            "must-ask-human",
            "cannot-answer"
        ]
    );
    let pack = sample_pack();
    assert_eq!(
        pack.authority_summary.ladder_levels,
        levels.iter().map(|v| (*v).to_string()).collect::<Vec<_>>()
    );
}

#[test]
fn authority_summary_is_policy_metadata_only() {
    let pack = sample_pack();
    assert_eq!(
        pack.authority_summary.execution_boundary,
        CONTEXT_PACK_EXECUTION_BOUNDARY
    );
    assert!(!pack.authority_summary.authority_rules.is_empty());
    assert!(!pack.authority_summary.default_refusal_rules.is_empty());
}

#[test]
fn authority_summary_contains_no_executed_decision() {
    let json = serde_json::to_string(&sample_pack().authority_summary).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "resolvedauthority",
        "approvalresult",
        "answerbody",
        "draftbody",
        "suggestionbody",
        "executeddecision",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "authority summary must not contain {forbidden}"
        );
    }
}

#[test]
fn owner_identity_is_metadata_not_impersonation() {
    let identity = sample_pack().owner_identity;
    assert_eq!(identity.owner_label, "Fixture Owner");
    assert!(!identity
        .owner_label
        .to_ascii_lowercase()
        .contains("i am the owner"));
}

#[test]
fn evidence_index_contains_references_only() {
    let entry = &sample_pack().evidence_index[0];
    assert!(matches!(entry.evidence_ref, EvidenceRef::FilePath(_)));
    assert_eq!(entry.origin, ContextPackEvidenceOrigin::ContinuityItem);
}

#[test]
fn evidence_index_rejects_secret_entries() {
    let mut pack = sample_pack();
    pack.evidence_index[0].sensitivity = Sensitivity::Secret;
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::SecretEvidenceIndexEntry)
    ));
}

#[test]
fn evidence_index_requires_unique_ref_ids() {
    let mut pack = sample_pack();
    pack.evidence_index.push(pack.evidence_index[0].clone());
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::DuplicateEvidenceIndexRefId { .. })
    ));
}

#[test]
fn redaction_summary_contains_aggregate_counts_only() {
    let summary = sample_pack().redaction_summary;
    assert_eq!(summary.secret_items_omitted, 1);
    let json = serde_json::to_string(&summary).expect("serialize");
    assert!(!json.contains("evidenceRef"));
    assert!(!json.contains("label"));
}

#[test]
fn redaction_summary_contains_no_source_identity() {
    let json = serde_json::to_string(&sample_pack().redaction_summary).expect("serialize");
    for forbidden in ["path", "title", "timestamp", "refId", "summary"] {
        assert!(
            !json.contains(forbidden),
            "redaction summary must not expose {forbidden}"
        );
    }
}

#[test]
fn context_pack_contains_no_answer_response_or_query_contract() {
    let json = serde_json::to_string(&sample_pack()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "\"answer\"",
        "\"response\"",
        "\"query\"",
        "\"prompt\"",
        "\"generatedresponse\"",
        "\"draftcontent\"",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "pack contract must not contain {forbidden}"
        );
    }
}

#[test]
fn context_pack_contains_no_llm_axga_or_model_contract() {
    let json = serde_json::to_string(&sample_pack()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "\"model\"",
        "\"provider\"",
        "\"temperature\"",
        "\"axga\"",
        "\"llm\"",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "pack contract must not contain {forbidden}"
        );
    }
}

#[test]
fn context_pack_contains_no_context_document_annex() {
    let json = serde_json::to_string(&sample_pack()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "contextdocument",
        "documentannex",
        "contextindex",
        "agentcontextenabled",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "pack v1.0 must not contain annex field {forbidden}"
        );
    }
    for entry in &sample_pack().evidence_index {
        assert_eq!(entry.origin, ContextPackEvidenceOrigin::ContinuityItem);
    }
}

#[test]
fn current_state_contract_preserves_pending_provenance() {
    let pending = &sample_pack().current_state.pending_attention[0];
    assert_eq!(pending.reason, PendingAttentionReason::PendingSignal);
    assert_eq!(pending.provenance, ContextPackItemProvenance::Pending);
}

#[test]
fn catch_up_contract_preserves_unconfirmed_provenance() {
    let mut pack = sample_pack();
    pack.catch_up
        .next_suggested_attention
        .push(ContextPackPendingAttentionItem {
            provenance: ContextPackItemProvenance::Unconfirmed,
            ..sample_pending_item()
        });
    validate_proxy_context_pack(&pack).expect("unconfirmed pending attention is representable");
}

#[test]
fn correction_provenance_is_representable() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].correction = Some(ContextPackCorrectionProvenance {
        is_corrected: true,
        is_superseded_original: false,
        correction_event_ids: vec!["evt-correction-1".into()],
        superseded_by_event_id: None,
    });
    validate_proxy_context_pack(&pack).expect("corrected item with provenance validates");
}

#[test]
fn raw_superseded_presentation_is_not_required_by_pack_contract() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].correction = Some(ContextPackCorrectionProvenance {
        is_corrected: false,
        is_superseded_original: true,
        correction_event_ids: vec![],
        superseded_by_event_id: Some("evt-new".into()),
    });
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::InvalidContinuityItem(_))
    ));
}

#[test]
fn freshness_is_objective_metadata_only() {
    let freshness = sample_pack().freshness;
    assert_eq!(freshness.catch_up_since, "2026-07-17T00:00:00Z");
    assert_eq!(freshness.age_seconds, 3600);
}

#[test]
fn freshness_has_no_strict_enforcement_field() {
    let json = serde_json::to_string(&sample_pack().freshness).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "strictfreshness",
        "stalethreshold",
        "isfresh",
        "enforcefreshness",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "freshness must not contain enforcement field {forbidden}"
        );
    }
}

#[test]
fn diagnostics_are_bounded() {
    let mut pack = sample_pack();
    pack.diagnostics = (0..33)
        .map(|i| ContextPackDiagnostic {
            code: format!("code-{i}"),
            message: "safe diagnostic".into(),
            severity: ContextPackDiagnosticSeverity::Info,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::TooManyDiagnostics { .. })
    ));
}

#[test]
fn limitations_are_bounded_and_required() {
    let mut pack = sample_pack();
    pack.limitations.clear();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::EmptyLimitations)
    ));
}

#[test]
fn unresolved_items_are_bounded() {
    let mut pack = sample_pack();
    pack.unresolved_items = (0..33)
        .map(|i| ContextPackUnresolvedItem {
            id: format!("unresolved-{i}"),
            category: ContextPackUnresolvedCategory::Pending,
            summary: "pending item".into(),
            provenance: ContextPackItemProvenance::Pending,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::TooManyUnresolvedItems { .. })
    ));
}

#[test]
fn evidence_index_is_bounded() {
    let mut pack = sample_pack();
    pack.evidence_index = (0..129)
        .map(|i| ContextPackEvidenceIndexEntry {
            ref_id: format!("ref-{i}"),
            evidence_ref: EvidenceRef::FilePath(format!("docs/file-{i}.md")),
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity: Sensitivity::Private,
            label: format!("label-{i}"),
            timestamp: None,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::TooManyEvidenceIndexEntries { .. })
    ));
}

#[test]
fn fixture_proxy_context_pack_is_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let raw = fs::read_to_string(root.join("tests/fixtures/context/proxy-context-pack-valid.json"))
        .expect("read fixture");
    let pack: ProxyContextPack = serde_json::from_str(&raw).expect("parse fixture");
    validate_proxy_context_pack(&pack).expect("fixture pack must validate");
}

#[test]
fn checkpoint_a_contracts_are_pure_no_io() {
    let pack = sample_pack();
    let _ = validate_proxy_context_pack(&pack);
    let _ = deterministic_context_pack_id("fnv1a-test");
    let _ = is_supported_proxy_context_pack_protocol(PROXY_CONTEXT_PACK_PROTOCOL_VERSION);
}

#[test]
fn checkpoint_a_does_not_touch_profile_or_continuity_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "src/profile.rs",
        "src/profile_validation.rs",
        "src/continuity/current_state.rs",
        "src/continuity/catch_up.rs",
        "src/continuity/readers.rs",
    ] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        assert!(!content.contains("validate_proxy_context_pack"));
        assert!(!content.contains("ContextPackCurrentState"));
    }
}

#[test]
fn checkpoint_a_context_storage_and_cli_exist_after_checkpoint_e() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(root.join("context_pack_storage.rs").exists());
    assert!(root.join("context_pack_selection.rs").exists());
    assert!(root.join("context_pack.rs").exists());
    let cli_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openmesh-cli/src");
    assert!(cli_root.join("context.rs").exists());
    let main_rs = fs::read_to_string(cli_root.join("main.rs")).expect("read main.rs");
    assert!(main_rs.contains("mod context;"));
}

#[test]
fn checkpoint_a_does_not_start_ask_my_proxy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/domain.rs")).expect("read domain");
    let lowered = domain.to_ascii_lowercase();
    for forbidden in [
        "askmyproxy",
        "ask my proxy",
        "generate_answer",
        "openmesh_ai_runtime",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "domain.rs must not start {forbidden}"
        );
    }
}

#[test]
fn checkpoint_a_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    let content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    let count = content.matches("#[tauri::command]").count();
    assert_eq!(count, 52, "Tauri command count must remain 52");
}

#[test]
fn pending_signal_cannot_be_represented_as_confirmed() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].source = ContinuitySourceKind::PendingSignal;
    pack.current_state.sections.in_progress[0].provenance = ContextPackItemProvenance::Confirmed;
    assert!(matches!(
        validate_proxy_context_pack(&pack),
        Err(ContextPackValidationError::PendingSignalRepresentedAsConfirmed)
    ));
}
