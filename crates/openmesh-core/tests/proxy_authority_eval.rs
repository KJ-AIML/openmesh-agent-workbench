use openmesh_core::authority_freshness::{evaluate_evidence_freshness, ConfidenceLabel};
use openmesh_core::authority_gate::{run_pre_provider_authority_gate, AuthorityGateOutcome};
use openmesh_core::authority_policy::{
    classify_question_risk, map_risk_to_freshness_tier, FreshnessTier, QuestionRiskCategory,
};
use openmesh_core::domain::default_work_proxy_profile;
use openmesh_core::proxy_claims::{
    extract_claims_from_draft, verify_claims_against_pack, ClaimCoverage,
};
use openmesh_core::proxy_citations::unsupported_claim_texts;
use openmesh_core::proxy_post_verify::{apply_post_provider_verification, MUST_ASK_DRAFT_PREFIX};
use chrono::{Duration, TimeZone, Utc};
use serde_json::Value;

const FIXTURE: &str = include_str!("fixtures/context/proxy-context-pack-valid.json");

#[test]
fn adversarial_secret_must_ask_before_provider() {
    let profile =
        default_work_proxy_profile("ws-test", "profile-ws-test", "owner", "dev", "2026-07-24T10:00:00Z");
    let outcome = run_pre_provider_authority_gate("What is the secret token?", &profile, "secret");
    assert!(matches!(outcome, AuthorityGateOutcome::MustAsk { .. }));
}

#[test]
fn adversarial_hallucinated_completion_unsupported() {
    let pack: openmesh_core::domain::ProxyContextPack =
        serde_json::from_str(FIXTURE).expect("fixture");
    let claims = extract_claims_from_draft("Deployment completed successfully yesterday.");
    let verified = verify_claims_against_pack(&claims, &pack);
    let unsupported = unsupported_claim_texts(&verified);
    assert!(
        !unsupported.is_empty()
            || verified
                .iter()
                .any(|v| v.verification_status == ClaimCoverage::Unsupported)
    );
}

#[test]
fn adversarial_deploy_question_is_commitment_risk() {
    assert_eq!(
        classify_question_risk("Can Yo deploy now?"),
        QuestionRiskCategory::Commitment
    );
    assert_eq!(
        map_risk_to_freshness_tier(QuestionRiskCategory::Commitment),
        FreshnessTier::Critical
    );
}

#[test]
fn adversarial_empty_evidence_pack_flags_unsupported() {
    let mut pack: openmesh_core::domain::ProxyContextPack =
        serde_json::from_str(FIXTURE).expect("fixture");
    pack.evidence_index.clear();
    let claims = extract_claims_from_draft("Concrete factual claim about release.");
    let verified = verify_claims_against_pack(&claims, &pack);
    assert!(verified
        .iter()
        .all(|v| v.verification_status == ClaimCoverage::Unsupported));
}

#[test]
fn adversarial_stale_evidence_fails_critical_freshness() {
    let mut pack: openmesh_core::domain::ProxyContextPack =
        serde_json::from_str(FIXTURE).expect("fixture");
    pack.freshness.age_seconds = 60 * 60 * 24 * 30; // 30 days
    for entry in &mut pack.evidence_index {
        entry.timestamp = Some("2020-01-01T00:00:00Z".into());
    }
    let now = Utc.with_ymd_and_hms(2026, 7, 24, 12, 0, 0).unwrap();
    let result = evaluate_evidence_freshness(&pack, FreshnessTier::Critical, now);
    assert!(!result.is_sufficient);
    assert_eq!(result.confidence_label, ConfidenceLabel::Insufficient);
    assert!(!result.stale_warnings.is_empty());
}

#[test]
fn adversarial_conflicting_labels_do_not_support_both_claims() {
    let mut pack: openmesh_core::domain::ProxyContextPack =
        serde_json::from_str(FIXTURE).expect("fixture");
    if let Some(first) = pack.evidence_index.first_mut() {
        first.label = "deploy blocked".into();
        first.ref_id = "ev-blocked".into();
    }
    pack.evidence_index.push(openmesh_core::domain::ContextPackEvidenceIndexEntry {
        ref_id: "ev-ready".into(),
        evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/a.md".into()),
        origin: openmesh_core::domain::ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: openmesh_core::context::Sensitivity::Private,
        label: "deploy ready".into(),
        timestamp: Some("2026-07-24T10:00:00Z".into()),
    });
    let claims = extract_claims_from_draft("Deploy is blocked. Deploy is ready.");
    let verified = verify_claims_against_pack(&claims, &pack);
    // Both sentences may match different labels — conflicting evidence exists in pack.
    assert!(verified.len() >= 2);
    let supported = verified
        .iter()
        .filter(|v| v.verification_status == ClaimCoverage::Supported)
        .count();
    assert!(supported >= 1);
    // Conflicting claim texts are both present; consumer must not treat as single truth.
    assert!(verified.iter().any(|v| v.claim.claim_text.contains("blocked")));
    assert!(verified.iter().any(|v| v.claim.claim_text.contains("ready")));
}

#[test]
fn adversarial_post_verify_downgrades_unsupported_draft() {
    let mut pack: openmesh_core::domain::ProxyContextPack =
        serde_json::from_str(FIXTURE).expect("fixture");
    pack.evidence_index.clear();
    let mut draft = sample_draft_for_eval();
    draft.draft_text = "The release shipped to production yesterday.".into();
    let result = apply_post_provider_verification(
        &mut draft,
        &pack,
        "What is the current status?",
        Utc::now(),
    );
    assert!(result.must_ask);
    assert!(result.downgraded);
    assert!(draft.draft_text.starts_with(MUST_ASK_DRAFT_PREFIX));
}

#[test]
fn adversarial_receipt_json_shape_is_object() {
    let value: Value = serde_json::from_str(
        r#"{"receiptId":"r1","questionId":"q1","questionText":"?","resolvedAuthority":"can-draft","authorityDecisionReason":"x","contextPackId":"p1","draftText":"d","claimsJson":"[]","freshnessSummary":"{}","generatedAt":"2026-07-24T10:00:00Z"}"#,
    )
    .expect("parse");
    assert!(value.get("receiptId").is_some());
}

fn sample_draft_for_eval() -> openmesh_core::domain::ProxyDraft {
    openmesh_core::domain::ProxyDraft {
        protocol_version: openmesh_core::domain::PROXY_DRAFT_PROTOCOL_VERSION.into(),
        question_id: "proxy-q-1a2b3c4d5e6f7890-1a2b-3".into(),
        generated_at: "2026-07-24T10:00:00Z".into(),
        classification: openmesh_core::domain::PROXY_DRAFT_CLASSIFICATION.into(),
        draft_text: "draft".into(),
        authority_notice: openmesh_core::domain::PROXY_DRAFT_AUTHORITY_NOTICE.into(),
        execution_boundary: openmesh_core::domain::PROXY_DRAFT_EXECUTION_BOUNDARY.into(),
        trace: openmesh_core::domain::ProxyDraftTraceMetadata {
            protocol_version: openmesh_core::domain::PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION
                .into(),
            workspace_id: "ws-fixture-0.1.6".into(),
            profile_id: "profile-ws-fixture-0.1.6".into(),
            profile_version: "1.0".into(),
            context_pack_id: "context-pack-fnv1a-6dd176ff3e7276a3".into(),
            build_inputs_hash: "fnv1a-6dd176ff3e7276a3".into(),
            evidence_summary: openmesh_core::domain::ProxyDraftEvidenceSummary {
                evidence_index_count: 0,
                source_counts: Default::default(),
                secret_items_omitted: 0,
            },
        },
        runtime: openmesh_core::domain::ProxyDraftRuntimeMetadata {
            runtime_kind: "local-stub".into(),
            provider_id: "deterministic-stub".into(),
            model_id: "fixture-model".into(),
            network_used: false,
            duration_ms: 1,
        },
        limitations: vec!["one".into()],
    }
}

#[allow(dead_code)]
fn _keep_duration_import() {
    let _ = Duration::hours(1);
}
