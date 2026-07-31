//! Dev Track 0.1.7.1 — AuthorityPolicy contract and question risk classification.
//!
//! Pure policy evaluation only: no I/O, no LLM, no provider calls.

use crate::domain::{ProxyAuthorityLevel, WorkProxyProfile};
use crate::profile_validation::{resolve_profile_authority, ProfileEvaluationContext};
use serde::{Deserialize, Serialize};

/// Deterministic question risk categories (rules-first).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuestionRiskCategory {
    Progress,
    Status,
    Decision,
    Commitment,
    Secret,
    Personal,
    Unknown,
}

/// Who is asking — local-owner focused in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RequesterRelation {
    #[default]
    LocalOwner,
    Teammate,
    Unknown,
}

/// Freshness requirement tier derived from question risk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreshnessTier {
    LowImpact,
    Standard,
    Critical,
}

/// Input to authority policy evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityPolicyInput {
    pub question: String,
    pub risk: QuestionRiskCategory,
    pub requester: RequesterRelation,
    pub scope: String,
    pub involves_secret_topic: bool,
    pub is_irreversible: bool,
}

/// Policy decision — metadata only; never answer text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityPolicyDecision {
    pub resolved_authority: ProxyAuthorityLevel,
    pub deny_before_provider: bool,
    pub deny_reason: Option<String>,
    pub evidence_required: bool,
    pub human_confirmation_required: bool,
    pub freshness_tier: FreshnessTier,
    pub decision_reason: String,
    pub matched_rule_ids: Vec<String>,
}

/// Classify question risk using deterministic keyword/heuristic rules.
pub fn classify_question_risk(question: &str) -> QuestionRiskCategory {
    let normalized = question.to_ascii_lowercase();
    let compact: String = normalized.chars().filter(|c| c.is_ascii_alphanumeric()).collect();

    if contains_any(&normalized, &["secret", "password", "credential", "api key", "token"]) {
        return QuestionRiskCategory::Secret;
    }
    if contains_any(
        &normalized,
        &["personal", "private life", "salary", "health", "medical"],
    ) {
        return QuestionRiskCategory::Personal;
    }
    if contains_any(
        &normalized,
        &["deploy", "release", "merge", "ship", "production", "approve", "commit to"],
    ) || compact.contains("candeploy") || compact.contains("deploynow")
    {
        return QuestionRiskCategory::Commitment;
    }
    if contains_any(
        &normalized,
        &["decide", "decision", "should we", "architecture", "trade-off", "choose"],
    ) {
        return QuestionRiskCategory::Decision;
    }
    if contains_any(
        &normalized,
        &["status", "blocker", "blocked", "progress", "what happened", "summary"],
    ) {
        return QuestionRiskCategory::Status;
    }
    if contains_any(
        &normalized,
        &["worked on", "completed", "milestone", "checkpoint"],
    ) {
        return QuestionRiskCategory::Progress;
    }
    QuestionRiskCategory::Unknown
}

/// Map risk category to freshness tier.
pub fn map_risk_to_freshness_tier(risk: QuestionRiskCategory) -> FreshnessTier {
    match risk {
        QuestionRiskCategory::Progress | QuestionRiskCategory::Status => FreshnessTier::LowImpact,
        QuestionRiskCategory::Commitment | QuestionRiskCategory::Secret => FreshnessTier::Critical,
        QuestionRiskCategory::Decision
        | QuestionRiskCategory::Personal
        | QuestionRiskCategory::Unknown => FreshnessTier::Standard,
    }
}

/// Evaluate authority policy for a question under a profile.
pub fn evaluate_authority_policy(
    input: &AuthorityPolicyInput,
    profile: &WorkProxyProfile,
) -> AuthorityPolicyDecision {
    let context = ProfileEvaluationContext {
        topic: input.scope.clone(),
        is_irreversible: input.is_irreversible
            || matches!(input.risk, QuestionRiskCategory::Commitment),
        involves_impersonation: false,
        lacks_evidence: false,
    };

    let policy = resolve_profile_authority(profile, &input.scope, &context);
    let freshness_tier = map_risk_to_freshness_tier(input.risk);

    let mut deny_before_provider = false;
    let mut deny_reason: Option<String> = None;

    match policy.resolved_authority {
        ProxyAuthorityLevel::CannotAnswer => {
            deny_before_provider = true;
            deny_reason = Some("authority policy denies this question".to_string());
        }
        ProxyAuthorityLevel::MustAskHuman
            if matches!(
                input.risk,
                QuestionRiskCategory::Secret | QuestionRiskCategory::Commitment
            ) =>
        {
            deny_before_provider = true;
            deny_reason = Some("high-risk question requires human confirmation before provider".to_string());
        }
        _ => {}
    }

    if input.involves_secret_topic || matches!(input.risk, QuestionRiskCategory::Secret) {
        deny_before_provider = true;
        deny_reason = Some("secret-topic questions must not reach provider".to_string());
    }

    if matches!(input.requester, RequesterRelation::Unknown) {
        deny_before_provider = true;
        deny_reason = Some("unknown requester relation".to_string());
    }

    AuthorityPolicyDecision {
        resolved_authority: policy.resolved_authority,
        deny_before_provider,
        deny_reason,
        evidence_required: policy.evidence_required,
        human_confirmation_required: policy.human_confirmation_required,
        freshness_tier,
        decision_reason: policy.decision_reason,
        matched_rule_ids: policy.matched_rule_ids,
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}
