//! Dev Track 0.1.7.2 — Pre-provider authority gate.
//!
//! Runs before context pack build or runtime invocation. Denied questions must not
//! send sensitive context to the provider.

use crate::authority_policy::{
    classify_question_risk, evaluate_authority_policy, AuthorityPolicyInput, AuthorityPolicyDecision,
    QuestionRiskCategory, RequesterRelation,
};
use crate::domain::{ProxyAuthorityLevel, WorkProxyProfile};

/// Visible outcome label for CLI/UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityOutcomeLabel {
    Proceed,
    MustAskHuman,
    CannotAnswer,
    DeniedBeforeProvider,
}

/// Pre-provider gate result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityGateOutcome {
    Proceed {
        decision: AuthorityPolicyDecision,
        label: AuthorityOutcomeLabel,
    },
    /// High-risk / policy Must Ask — do not call provider; create a pending question.
    MustAsk {
        decision: AuthorityPolicyDecision,
        label: AuthorityOutcomeLabel,
        message: String,
    },
    Denied {
        decision: AuthorityPolicyDecision,
        label: AuthorityOutcomeLabel,
        message: String,
    },
}

/// Build policy input from a raw question string.
pub fn build_authority_policy_input(question: &str, scope: &str) -> AuthorityPolicyInput {
    let risk = classify_question_risk(question);
    AuthorityPolicyInput {
        question: question.to_string(),
        risk,
        requester: RequesterRelation::LocalOwner,
        scope: scope.to_string(),
        involves_secret_topic: matches!(risk, QuestionRiskCategory::Secret),
        is_irreversible: matches!(risk, QuestionRiskCategory::Commitment),
    }
}

/// Run the pre-provider authority gate. Fail closed.
pub fn run_pre_provider_authority_gate(
    question: &str,
    profile: &WorkProxyProfile,
    scope: &str,
) -> AuthorityGateOutcome {
    let input = build_authority_policy_input(question, scope);
    let decision = evaluate_authority_policy(&input, profile);

    if decision.deny_before_provider {
        let message = decision
            .deny_reason
            .clone()
            .unwrap_or_else(|| "authority gate denied before provider".to_string());
        // High-risk Must Ask / secret paths create pending human attention.
        if matches!(
            decision.resolved_authority,
            ProxyAuthorityLevel::MustAskHuman
        ) || input.involves_secret_topic
            || matches!(
                input.risk,
                QuestionRiskCategory::Secret | QuestionRiskCategory::Commitment
            )
        {
            return AuthorityGateOutcome::MustAsk {
                decision,
                label: AuthorityOutcomeLabel::MustAskHuman,
                message,
            };
        }
        return AuthorityGateOutcome::Denied {
            decision,
            label: AuthorityOutcomeLabel::DeniedBeforeProvider,
            message,
        };
    }

    let label = match decision.resolved_authority {
        ProxyAuthorityLevel::CannotAnswer => AuthorityOutcomeLabel::CannotAnswer,
        ProxyAuthorityLevel::MustAskHuman => AuthorityOutcomeLabel::MustAskHuman,
        _ => AuthorityOutcomeLabel::Proceed,
    };

    if decision.resolved_authority == ProxyAuthorityLevel::CannotAnswer {
        return AuthorityGateOutcome::Denied {
            decision,
            label,
            message: "proxy cannot answer under current authority policy".to_string(),
        };
    }

    // Local-owner draft path: MustAskHuman profile defaults may still produce a
    // draft-only answer (0.1.6 compatibility). High-risk cases already returned above.
    AuthorityGateOutcome::Proceed { decision, label }
}
