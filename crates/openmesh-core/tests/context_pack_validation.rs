//! Dev Track 0.1.5 Checkpoint D — Proxy Context Pack complete policy validation tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack_validation::{
    validate_proxy_context_pack_complete, validate_proxy_context_pack_policy,
    ContextPackValidationError,
};
use openmesh_core::domain::{
    deterministic_context_pack_id, proxy_context_pack_authority_ladder_levels,
    validate_proxy_context_pack, AuthorityRule, CatchUpWindow, CommunicationPreferences,
    ContextPackAuthoritySummary, ContextPackCatchUp, ContextPackCatchUpSections,
    ContextPackContinuityItem, ContextPackCorrectionProvenance, ContextPackCurrentState,
    ContextPackCurrentStateSections, ContextPackDiagnostic, ContextPackDiagnosticSeverity,
    ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin, ContextPackFreshness,
    ContextPackItemProvenance, ContextPackOwnerIdentity, ContextPackPendingAttentionItem,
    ContextPackPrivacySummary, ContextPackRedactionSummary, ContextPackUnresolvedCategory,
    ContextPackUnresolvedItem, ContextPackValidationError as StructuralValidationError,
    ContinuityConfidence, ContinuitySourceKind, DecisionPreferences, DefaultRefusalRule,
    EvidencePolicy, EvidenceRef, EvidenceSourceKind, PendingAttentionReason,
    PendingAttentionSeverity, PendingAttentionStatus, PrivacyAllowedUse, PrivacyRule,
    PrivacySensitivity, ProxyAuthorityLevel, ProxyContextPack, SourceCounts,
    UnsupportedClaimBehavior, CONTEXT_PACK_EXECUTION_BOUNDARY, MAX_CONTEXT_PACK_DIAGNOSTICS,
    MAX_CONTEXT_PACK_EVIDENCE_INDEX, MAX_CONTEXT_PACK_LIMITATIONS,
    MAX_CONTEXT_PACK_UNRESOLVED_ITEMS, PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
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

fn err_text(err: &ContextPackValidationError) -> String {
    err.to_string().to_ascii_lowercase()
}

fn expect_complete_validation_failure(
    pack: &ProxyContextPack,
    policy_ok: impl Fn(&ContextPackValidationError) -> bool,
    structural_ok: impl Fn(&StructuralValidationError) -> bool,
) {
    match validate_proxy_context_pack_complete(pack) {
        Err(ContextPackValidationError::Structural(inner)) => {
            assert!(
                structural_ok(&inner),
                "unexpected structural validation error: {inner:?}"
            );
        }
        Err(other) => {
            assert!(
                policy_ok(&other),
                "unexpected policy validation error: {other:?}"
            );
        }
        Ok(()) => panic!("expected complete validation failure"),
    }
}

#[test]
fn complete_validation_accepts_valid_pack() {
    let pack = sample_pack();
    validate_proxy_context_pack(&pack).expect("structural validation");
    validate_proxy_context_pack_complete(&pack).expect("complete validation");
}

#[test]
fn complete_validation_rejects_unsupported_protocol() {
    let mut pack = sample_pack();
    pack.protocol_version = "9.9".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::UnsupportedProtocolVersion),
        |err| {
            matches!(
                err,
                StructuralValidationError::UnsupportedProtocolVersion { .. }
            )
        },
    );
}

#[test]
fn complete_validation_rejects_context_pack_id_hash_mismatch() {
    let mut pack = sample_pack();
    pack.context_pack_id = "context-pack-fnv1a-deadbeefcafebabe".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::ContextPackIdMismatch),
        |err| matches!(err, StructuralValidationError::InvalidOwnerIdentity(_)),
    );
}

#[test]
fn complete_validation_rejects_invalid_hash_format() {
    let mut pack = sample_pack();
    pack.build_inputs_hash = "not-a-valid-hash".into();
    pack.context_pack_id = deterministic_context_pack_id(&pack.build_inputs_hash);
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::InvalidHashFormat)
    ));
}

#[test]
fn complete_validation_rejects_empty_identity_fields() {
    let mut pack = sample_pack();
    pack.workspace_id = "   ".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidIdentity),
        |err| matches!(err, StructuralValidationError::EmptyWorkspaceId),
    );

    let mut pack = sample_pack();
    pack.owner_identity.owner_label = "".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidIdentity),
        |err| matches!(err, StructuralValidationError::InvalidOwnerIdentity(_)),
    );
}

#[test]
fn complete_validation_rejects_non_utc_timestamps() {
    let mut pack = sample_pack();
    pack.generated_at = "2026-07-18T04:00:00-05:00".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidTimestamp),
        |err| matches!(err, StructuralValidationError::InvalidTimestamp(_)),
    );
}

#[test]
fn complete_validation_rejects_window_freshness_mismatch() {
    let mut pack = sample_pack();
    pack.freshness.catch_up_since = "2026-07-16T00:00:00Z".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::FreshnessMismatch),
        |err| matches!(err, StructuralValidationError::InvalidFreshness(_)),
    );
}

#[test]
fn complete_validation_rejects_pack_generated_at_mismatch() {
    let mut pack = sample_pack();
    pack.freshness.pack_generated_at = "2026-07-18T05:00:00Z".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::FreshnessMismatch),
        |err| matches!(err, StructuralValidationError::InvalidFreshness(_)),
    );
}

#[test]
fn complete_validation_rejects_invalid_freshness_age() {
    let mut pack = sample_pack();
    pack.freshness.age_seconds = 42;
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::FreshnessMismatch)
    ));
}

#[test]
fn complete_validation_requires_exact_authority_ladder() {
    let mut pack = sample_pack();
    pack.authority_summary.ladder_levels = vec!["can-answer".into(), "cannot-answer".into()];
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidAuthoritySummary),
        |err| matches!(err, StructuralValidationError::InvalidAuthorityLadder),
    );
}

#[test]
fn complete_validation_requires_policy_only_execution_boundary() {
    let mut pack = sample_pack();
    pack.authority_summary.execution_boundary = "runtime authority execution enabled".into();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidAuthoritySummary),
        |err| matches!(err, StructuralValidationError::InvalidExecutionBoundary),
    );
}

#[test]
fn complete_validation_rejects_executed_authority_metadata() {
    let mut pack = sample_pack();
    pack.authority_summary.authority_rules[0].description =
        Some("generated answer for the owner".into());
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::ForbiddenRuntimeSurface)
    ));
}

#[test]
fn complete_validation_requires_default_refusal_rules() {
    let mut pack = sample_pack();
    pack.authority_summary.default_refusal_rules.clear();
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidAuthoritySummary),
        |err| matches!(err, StructuralValidationError::InvalidAuthoritySummary(_)),
    );
}

#[test]
fn complete_validation_rejects_secret_evidence_index_entry() {
    let mut pack = sample_pack();
    pack.evidence_index[0].sensitivity = Sensitivity::Secret;
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::SecretEvidenceIndexEntry
        ))
    ));
}

#[test]
fn complete_validation_rejects_unknown_or_ambiguous_sensitivity() {
    let mut pack = sample_pack();
    pack.evidence_index[0].sensitivity = Sensitivity::Secret;
    pack.evidence_index[0].label = "safe label".into();
    assert!(matches!(
        validate_proxy_context_pack_policy(&pack),
        Err(ContextPackValidationError::SecretContentDetected)
    ));
}

#[test]
fn secret_validation_error_contains_no_secret_identity() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].summary =
        "vault contains api_key=super-secret-token-value".into();
    let err = validate_proxy_context_pack_complete(&pack).expect_err("secret summary");
    let text = err_text(&err);
    assert!(!text.contains("super-secret-token-value"));
    assert!(!text.contains("api_key=super-secret-token-value"));
}

#[test]
fn validation_errors_do_not_echo_sensitive_values() {
    let mut pack = sample_pack();
    pack.evidence_index[0].label = "token=sk-live-abcdef123456".into();
    let err = validate_proxy_context_pack_complete(&pack).expect_err("secret label");
    let text = err_text(&err);
    assert!(!text.contains("sk-live-abcdef123456"));
    assert!(!text.contains("token=sk-live-abcdef123456"));
}

#[test]
fn evidence_index_requires_unique_ref_ids() {
    let mut pack = sample_pack();
    pack.evidence_index.push(pack.evidence_index[0].clone());
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::DuplicateEvidenceIndexRefId { .. }
        ))
    ));
}

#[test]
fn evidence_index_requires_unique_canonical_evidence_refs() {
    let mut pack = sample_pack();
    pack.evidence_index.push(ContextPackEvidenceIndexEntry {
        ref_id: "ref-002".into(),
        evidence_ref: pack.evidence_index[0].evidence_ref.clone(),
        origin: ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: Sensitivity::Private,
        label: "duplicate canonical".into(),
        timestamp: None,
    });
    assert!(matches!(
        validate_proxy_context_pack_policy(&pack),
        Err(ContextPackValidationError::DuplicateEvidenceIdentity)
    ));
}

#[test]
fn evidence_index_rejects_non_continuity_origin() {
    let serialized = serde_json::to_string(&sample_pack()).expect("serialize");
    let patched = serialized.replace("\"continuity-item\"", "\"promotion-audit\"");
    let parsed: Result<ProxyContextPack, _> = serde_json::from_str(&patched);
    assert!(
        parsed.is_err(),
        "non-continuity evidence origin must not deserialize into v1.0 pack"
    );
}

#[test]
fn evidence_index_rejects_invalid_timestamp() {
    let mut pack = sample_pack();
    pack.evidence_index[0].timestamp = Some("not-a-timestamp".into());
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(_))
    ));
}

#[test]
fn evidence_index_rejects_diagnostic_only_included_item() {
    let mut pack = sample_pack();
    pack.evidence_index[0].label = "diagnostic-only".into();
    assert!(matches!(
        validate_proxy_context_pack_policy(&pack),
        Err(ContextPackValidationError::InvalidEvidenceIndex)
    ));
}

#[test]
fn complete_validation_rejects_contradictory_pending_provenance() {
    let mut pack = sample_pack();
    pack.current_state.pending_attention[0].provenance = ContextPackItemProvenance::Confirmed;
    pack.current_state.pending_attention[0].status = PendingAttentionStatus::Open;
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidProvenance),
        |err| matches!(err, StructuralValidationError::InvalidPendingItem(_)),
    );
}

#[test]
fn complete_validation_rejects_unconfirmed_as_confirmed() {
    let mut pack = sample_pack();
    pack.unresolved_items.push(ContextPackUnresolvedItem {
        id: "unresolved-unconfirmed".into(),
        category: ContextPackUnresolvedCategory::Unconfirmed,
        summary: "needs human review".into(),
        provenance: ContextPackItemProvenance::Confirmed,
    });
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::InvalidProvenance)
    ));
}

#[test]
fn complete_validation_rejects_invalid_correction_metadata() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].correction = Some(ContextPackCorrectionProvenance {
        is_corrected: true,
        is_superseded_original: false,
        correction_event_ids: vec![],
        superseded_by_event_id: None,
    });
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::InvalidCorrectionMetadata)
    ));
}

#[test]
fn complete_validation_rejects_reintroduced_superseded_presentation() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].correction = Some(ContextPackCorrectionProvenance {
        is_corrected: false,
        is_superseded_original: true,
        correction_event_ids: vec![],
        superseded_by_event_id: Some("evt-new".into()),
    });
    expect_complete_validation_failure(
        &pack,
        |err| matches!(err, ContextPackValidationError::InvalidCorrectionMetadata),
        |err| matches!(err, StructuralValidationError::InvalidContinuityItem(_)),
    );
}

#[test]
fn complete_validation_accepts_effective_correction_provenance() {
    let mut pack = sample_pack();
    pack.current_state.sections.in_progress[0].correction = Some(ContextPackCorrectionProvenance {
        is_corrected: true,
        is_superseded_original: false,
        correction_event_ids: vec!["evt-correction-1".into()],
        superseded_by_event_id: None,
    });
    validate_proxy_context_pack_complete(&pack).expect("corrected item validates");
}

#[test]
fn complete_validation_rejects_evidence_bound_overflow() {
    let mut pack = sample_pack();
    pack.evidence_index = (0..MAX_CONTEXT_PACK_EVIDENCE_INDEX + 1)
        .map(|i| ContextPackEvidenceIndexEntry {
            ref_id: format!("ref-{i:03}"),
            evidence_ref: EvidenceRef::FilePath(format!("docs/file-{i}.md")),
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity: Sensitivity::Private,
            label: format!("label-{i}"),
            timestamp: None,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::TooManyEvidenceIndexEntries { .. }
        ))
    ));
}

#[test]
fn complete_validation_rejects_diagnostic_bound_overflow() {
    let mut pack = sample_pack();
    pack.diagnostics = (0..MAX_CONTEXT_PACK_DIAGNOSTICS + 1)
        .map(|i| ContextPackDiagnostic {
            code: format!("code-{i}"),
            message: "safe diagnostic".into(),
            severity: ContextPackDiagnosticSeverity::Info,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::TooManyDiagnostics { .. }
        ))
    ));
}

#[test]
fn complete_validation_rejects_limitation_bound_overflow() {
    let mut pack = sample_pack();
    pack.limitations = (0..MAX_CONTEXT_PACK_LIMITATIONS + 1)
        .map(|i| format!("limitation-{i}"))
        .collect();
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::TooManyLimitations { .. }
        ))
    ));
}

#[test]
fn complete_validation_requires_non_empty_limitations() {
    let mut pack = sample_pack();
    pack.limitations.clear();
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::EmptyLimitations
        ))
    ));
}

#[test]
fn complete_validation_rejects_unresolved_item_bound_overflow() {
    let mut pack = sample_pack();
    pack.unresolved_items = (0..MAX_CONTEXT_PACK_UNRESOLVED_ITEMS + 1)
        .map(|i| ContextPackUnresolvedItem {
            id: format!("unresolved-{i}"),
            category: ContextPackUnresolvedCategory::Pending,
            summary: "pending item".into(),
            provenance: ContextPackItemProvenance::Pending,
        })
        .collect();
    assert!(matches!(
        validate_proxy_context_pack_complete(&pack),
        Err(ContextPackValidationError::Structural(
            StructuralValidationError::TooManyUnresolvedItems { .. }
        ))
    ));
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
fn privacy_filtered_empty_evidence_index_can_be_valid() {
    let mut pack = sample_pack();
    pack.evidence_index.clear();
    pack.redaction_summary.secret_items_omitted = 3;
    validate_proxy_context_pack_complete(&pack).expect("empty evidence index is valid");
}

#[test]
fn malformed_individual_evidence_is_non_fatal() {
    let mut pack = sample_pack();
    pack.diagnostics.push(ContextPackDiagnostic {
        code: "evidence-malformed".into(),
        message: "malformed evidence candidate omitted".into(),
        severity: ContextPackDiagnosticSeverity::Warning,
    });
    pack.redaction_summary.malformed_items_omitted = 1;
    validate_proxy_context_pack_complete(&pack)
        .expect("malformed omission diagnostics are non-fatal");
}

#[test]
fn validation_is_pure_no_io() {
    let pack = sample_pack();
    let _ = validate_proxy_context_pack_complete(&pack);
    let _ = validate_proxy_context_pack_policy(&pack);
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_validation.rs"),
    )
    .expect("read validation module");
    for forbidden in [
        "std::fs",
        "read_to_string",
        "OpenOptions",
        "write(",
        "std::net",
    ] {
        assert!(
            !module.contains(forbidden),
            "validation must not use {forbidden}"
        );
    }
}

#[test]
fn validation_does_not_mutate_profile_or_continuity() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "src/profile.rs",
        "src/profile_validation.rs",
        "src/continuity/current_state.rs",
        "src/continuity/catch_up.rs",
        "src/continuity/readers.rs",
    ] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        assert!(!content.contains("validate_proxy_context_pack_complete"));
        assert!(!content.contains("validate_proxy_context_pack_policy"));
    }
}

#[test]
fn validation_does_not_create_projection_files() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_validation.rs"),
    )
    .expect("read validation module");
    for forbidden in [
        "projections_dir",
        "current_state_projection_path",
        "rebuild_current_state_projection",
        "write_current_state_projection",
    ] {
        assert!(
            !module.contains(forbidden),
            "validation must not create projections via {forbidden}"
        );
    }
}

#[test]
fn validation_contains_no_answer_or_authority_execution() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_validation.rs"),
    )
    .expect("read validation module");
    for forbidden in [
        "resolve_profile_authority",
        "generate_answer",
        "ask_my_proxy",
        "ask-my-proxy",
        "ProxyPolicyResult",
        "answer_text",
    ] {
        assert!(!module.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn checkpoint_d_storage_and_cli_are_present_after_checkpoint_e() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    assert!(root.join("context_pack_storage.rs").exists());
    assert!(root.join("context_pack_validation.rs").exists());
    let cli_context =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openmesh-cli/src/context.rs");
    assert!(cli_context.exists());
}

#[test]
fn checkpoint_d_does_not_start_ask_my_proxy() {
    let module = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack_validation.rs"),
    )
    .expect("read validation module");
    let lowered = module.to_ascii_lowercase();
    assert!(!lowered.contains("ask-my-proxy"));
    assert!(!lowered.contains("ask my proxy"));
    assert!(!lowered.contains("askmyproxy"));
}

#[test]
fn checkpoint_d_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    if tauri_lib.exists() {
        let content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
        assert!(!content.contains("context_pack_validation"));
        assert!(!content.contains("validate_proxy_context_pack_complete"));
        assert_eq!(
            content.matches("#[tauri::command]").count(),
            53,
            "Tauri command count must remain 53 (get_host_os)"
        );
    }
}
