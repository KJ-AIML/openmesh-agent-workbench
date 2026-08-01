use openmesh_core::authority_policy::{
    classify_question_risk, evaluate_authority_policy, map_risk_to_freshness_tier,
    AuthorityPolicyInput, FreshnessTier, QuestionRiskCategory, RequesterRelation,
};
use openmesh_core::domain::{default_work_proxy_profile, ProxyAuthorityLevel};

#[test]
fn classifies_commitment_questions() {
    assert_eq!(
        classify_question_risk("Can we deploy to production now?"),
        QuestionRiskCategory::Commitment
    );
}

#[test]
fn classifies_secret_questions() {
    assert_eq!(
        classify_question_risk("What is the API secret?"),
        QuestionRiskCategory::Secret
    );
}

#[test]
fn secret_questions_deny_before_provider() {
    let profile = default_work_proxy_profile(
        "ws-test",
        "profile-ws-test",
        "owner",
        "dev",
        "2026-07-24T10:00:00Z",
    );
    let input = AuthorityPolicyInput {
        question: "share the password".into(),
        risk: QuestionRiskCategory::Secret,
        requester: RequesterRelation::LocalOwner,
        scope: "local".into(),
        involves_secret_topic: true,
        is_irreversible: false,
    };
    let decision = evaluate_authority_policy(&input, &profile);
    assert!(decision.deny_before_provider);
}

#[test]
fn critical_risk_maps_to_critical_freshness() {
    assert_eq!(
        map_risk_to_freshness_tier(QuestionRiskCategory::Commitment),
        FreshnessTier::Critical
    );
}

#[test]
fn progress_maps_to_low_impact_freshness() {
    assert_eq!(
        map_risk_to_freshness_tier(QuestionRiskCategory::Progress),
        FreshnessTier::LowImpact
    );
}

#[test]
fn local_owner_can_proceed_for_status() {
    let profile = default_work_proxy_profile(
        "ws-test",
        "profile-ws-test",
        "owner",
        "dev",
        "2026-07-24T10:00:00Z",
    );
    let input = AuthorityPolicyInput {
        question: "What is the current status?".into(),
        risk: QuestionRiskCategory::Status,
        requester: RequesterRelation::LocalOwner,
        scope: "status".into(),
        involves_secret_topic: false,
        is_irreversible: false,
    };
    let decision = evaluate_authority_policy(&input, &profile);
    assert!(!decision.deny_before_provider);
    assert_ne!(
        decision.resolved_authority,
        ProxyAuthorityLevel::CannotAnswer
    );
}
