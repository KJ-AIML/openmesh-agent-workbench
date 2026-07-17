//! Dev Track 0.1.4 Checkpoint B — cross-field profile policy validation and authority resolution.
//!
//! Pure policy evaluation only: no I/O, no answer generation, no LLM dependency.

use crate::domain::{
    validate_evidence_policy, validate_work_proxy_profile, AuthorityRule, DefaultRefusalRule,
    PrivacyAllowedUse, PrivacyRule, PrivacySensitivity, ProfileValidationError,
    ProxyAuthorityLevel, UnsupportedClaimBehavior, WorkProxyProfile,
};
use std::collections::HashMap;

/// Optional evaluation context for policy resolution (metadata only).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileEvaluationContext {
    pub topic: String,
    pub is_irreversible: bool,
    pub involves_impersonation: bool,
    pub lacks_evidence: bool,
}

/// Policy evaluation output — authority metadata only; never answer text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilePolicyResult {
    pub resolved_authority: ProxyAuthorityLevel,
    pub matched_rule_ids: Vec<String>,
    pub evidence_required: bool,
    pub human_confirmation_required: bool,
    pub limitations: Vec<String>,
    pub decision_reason: String,
}

/// Returns true only when a validated profile is present (missing profile = no proxy behavior).
pub fn proxy_behavior_allowed(profile: Option<&WorkProxyProfile>) -> bool {
    match profile {
        None => false,
        Some(profile) => {
            validate_work_proxy_profile(profile).is_ok() && validate_profile_policy(profile).is_ok()
        }
    }
}

/// Cross-field semantic validation beyond structural `validate_work_proxy_profile`.
pub fn validate_profile_policy(profile: &WorkProxyProfile) -> Result<(), ProfileValidationError> {
    validate_work_proxy_profile(profile)?;
    validate_evidence_policy(&profile.evidence_policy)?;
    validate_no_impersonation_refusal(&profile.default_refusal_rules)?;
    validate_authority_scope_consistency(&profile.authority_rules)?;
    validate_irreversible_authority_rules(&profile.authority_rules)?;
    validate_privacy_restrictions(&profile.privacy_rules)?;
    validate_evidence_authority_consistency(profile)?;
    validate_refusal_authority_consistency(profile)?;
    Ok(())
}

/// Resolve effective authority for `scope` under `profile` (policy metadata only).
pub fn resolve_profile_authority(
    profile: &WorkProxyProfile,
    scope: &str,
    context: &ProfileEvaluationContext,
) -> ProfilePolicyResult {
    let matching = matching_authority_rules(&profile.authority_rules, scope);
    let mut matched_rule_ids: Vec<String> =
        matching.iter().map(|rule| rule.rule_id.clone()).collect();

    let mut resolved_authority = matching
        .iter()
        .map(|rule| rule.authority)
        .min_by_key(|level| authority_restrictiveness(*level))
        .unwrap_or(ProxyAuthorityLevel::MustAskHuman);

    let mut decision_reason = if matching.is_empty() {
        "no matching authority rule; defaulting to must-ask-human".to_string()
    } else {
        format!(
            "most restrictive of {} matching authority rule(s)",
            matching.len()
        )
    };

    let mut evidence_required = profile.evidence_policy.require_evidence_for_claims
        || matching.iter().any(|rule| rule.evidence_required);
    let mut human_confirmation_required =
        matching.iter().any(|rule| rule.human_confirmation_required);
    let mut limitations = profile.limitations.clone();

    for rule in &matching {
        limitations.extend(rule.limitations.iter().cloned());
    }

    for privacy_rule in matching_privacy_rules(&profile.privacy_rules, scope, context) {
        if !matched_rule_ids.contains(&privacy_rule.rule_id) {
            matched_rule_ids.push(privacy_rule.rule_id.clone());
        }
        if !privacy_rule.restriction.trim().is_empty() {
            limitations.push(privacy_rule.restriction.clone());
        }

        match privacy_rule.sensitivity {
            PrivacySensitivity::Secret
                if privacy_rule.allowed_use == PrivacyAllowedUse::ExcludeFromAnswers =>
            {
                resolved_authority = most_restrictive_authority(
                    resolved_authority,
                    ProxyAuthorityLevel::CannotAnswer,
                );
                decision_reason = format!(
                    "{}; secret privacy rule '{}' dominates authority",
                    decision_reason, privacy_rule.rule_id
                );
                human_confirmation_required = true;
            }
            PrivacySensitivity::Secret
            | PrivacySensitivity::Sensitive
            | PrivacySensitivity::Private
                if privacy_rule.requires_human_confirmation =>
            {
                resolved_authority = most_restrictive_authority(
                    resolved_authority,
                    ProxyAuthorityLevel::MustAskHuman,
                );
                human_confirmation_required = true;
            }
            PrivacySensitivity::Public | PrivacySensitivity::Internal => {
                // Public/internal topics do not automatically grant answer authority.
            }
            _ => {}
        }
    }

    for refusal in &profile.default_refusal_rules {
        let statement = refusal.statement.to_ascii_lowercase();
        limitations.push(refusal.statement.clone());

        if statement.contains("cannot impersonate") && context.involves_impersonation {
            resolved_authority =
                most_restrictive_authority(resolved_authority, ProxyAuthorityLevel::CannotAnswer);
            human_confirmation_required = true;
            decision_reason = format!(
                "{}; default refusal '{}' blocks impersonation",
                decision_reason, refusal.rule_id
            );
        }

        if statement.contains("irreversible") && context.is_irreversible {
            resolved_authority =
                most_restrictive_authority(resolved_authority, ProxyAuthorityLevel::MustAskHuman);
            human_confirmation_required = true;
        }

        if statement.contains("cannot answer outside authority")
            && resolved_authority == ProxyAuthorityLevel::CanAnswer
            && matching.is_empty()
        {
            resolved_authority = ProxyAuthorityLevel::MustAskHuman;
        }
    }

    if context.is_irreversible {
        human_confirmation_required = true;
        resolved_authority =
            most_restrictive_authority(resolved_authority, ProxyAuthorityLevel::MustAskHuman);
    }

    if context.lacks_evidence && profile.evidence_policy.require_evidence_for_claims {
        evidence_required = true;
        match profile.evidence_policy.unsupported_claim_behavior {
            UnsupportedClaimBehavior::Refuse => {
                resolved_authority = most_restrictive_authority(
                    resolved_authority,
                    ProxyAuthorityLevel::CannotAnswer,
                );
            }
            UnsupportedClaimBehavior::AskHuman => {
                resolved_authority = most_restrictive_authority(
                    resolved_authority,
                    ProxyAuthorityLevel::MustAskHuman,
                );
                human_confirmation_required = true;
            }
            UnsupportedClaimBehavior::SayUnknown => {
                resolved_authority = most_restrictive_authority(
                    resolved_authority,
                    ProxyAuthorityLevel::MustAskHuman,
                );
            }
        }
    }

    limitations.sort();
    limitations.dedup();

    ProfilePolicyResult {
        resolved_authority,
        matched_rule_ids,
        evidence_required,
        human_confirmation_required,
        limitations,
        decision_reason,
    }
}

pub(crate) fn authority_restrictiveness(level: ProxyAuthorityLevel) -> u8 {
    match level {
        ProxyAuthorityLevel::CannotAnswer => 0,
        ProxyAuthorityLevel::MustAskHuman => 1,
        ProxyAuthorityLevel::CanDraft => 2,
        ProxyAuthorityLevel::CanSuggest => 3,
        ProxyAuthorityLevel::CanAnswer => 4,
    }
}

pub(crate) fn most_restrictive_authority(
    left: ProxyAuthorityLevel,
    right: ProxyAuthorityLevel,
) -> ProxyAuthorityLevel {
    if authority_restrictiveness(left) <= authority_restrictiveness(right) {
        left
    } else {
        right
    }
}

fn authority_rule_matches(rule: &AuthorityRule, scope: &str) -> bool {
    rule.scope == "*" || scope.starts_with(&rule.scope) || scope == rule.scope
}

fn matching_authority_rules<'a>(rules: &'a [AuthorityRule], scope: &str) -> Vec<&'a AuthorityRule> {
    let mut matched: Vec<&AuthorityRule> = rules
        .iter()
        .filter(|rule| authority_rule_matches(rule, scope))
        .collect();
    matched.sort_by(|left, right| {
        right
            .scope
            .len()
            .cmp(&left.scope.len())
            .then_with(|| left.rule_id.cmp(&right.rule_id))
    });
    matched
}

fn privacy_rule_matches(
    rule: &PrivacyRule,
    scope: &str,
    context: &ProfileEvaluationContext,
) -> bool {
    let topic = if context.topic.is_empty() {
        scope
    } else {
        context.topic.as_str()
    };
    topic == rule.topic
        || topic.starts_with(&format!("{}.", rule.topic))
        || topic.starts_with(&rule.topic)
        || scope.contains(&rule.topic)
}

fn matching_privacy_rules<'a>(
    rules: &'a [PrivacyRule],
    scope: &str,
    context: &ProfileEvaluationContext,
) -> Vec<&'a PrivacyRule> {
    let mut matched: Vec<&PrivacyRule> = rules
        .iter()
        .filter(|rule| privacy_rule_matches(rule, scope, context))
        .collect();
    matched.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    matched
}

fn validate_no_impersonation_refusal(
    refusals: &[DefaultRefusalRule],
) -> Result<(), ProfileValidationError> {
    let has_no_impersonation = refusals.iter().any(|rule| {
        rule.statement
            .to_ascii_lowercase()
            .contains("cannot impersonate")
    });
    if !has_no_impersonation {
        return Err(ProfileValidationError::MissingNoImpersonationRefusal);
    }
    Ok(())
}

fn validate_authority_scope_consistency(
    rules: &[AuthorityRule],
) -> Result<(), ProfileValidationError> {
    let mut by_scope: HashMap<&str, ProxyAuthorityLevel> = HashMap::new();
    for rule in rules {
        if let Some(existing) = by_scope.get(rule.scope.as_str()) {
            if *existing != rule.authority {
                return Err(ProfileValidationError::ConflictingProfilePolicy(format!(
                    "ambiguous authority for scope '{}'",
                    rule.scope
                )));
            }
        } else {
            by_scope.insert(rule.scope.as_str(), rule.authority);
        }
    }
    Ok(())
}

fn validate_irreversible_authority_rules(
    rules: &[AuthorityRule],
) -> Result<(), ProfileValidationError> {
    for rule in rules {
        let scope = rule.scope.to_ascii_lowercase();
        let is_irreversible_scope = scope.contains("irreversible")
            || scope.contains("approve")
            || rule
                .conditions
                .iter()
                .any(|condition| condition.to_ascii_lowercase().contains("irreversible"));
        if is_irreversible_scope
            && matches!(
                rule.authority,
                ProxyAuthorityLevel::CanAnswer | ProxyAuthorityLevel::CanSuggest
            )
            && !rule.human_confirmation_required
        {
            return Err(ProfileValidationError::IrreversibleActionWithoutConfirmation);
        }
        if rule.authority == ProxyAuthorityLevel::CannotAnswer
            && rule
                .conditions
                .iter()
                .any(|condition| condition.to_ascii_lowercase().contains("delegate-approval"))
        {
            return Err(ProfileValidationError::ConflictingProfilePolicy(
                "cannot-answer rule cannot delegate approval".into(),
            ));
        }
    }
    Ok(())
}

fn validate_privacy_restrictions(rules: &[PrivacyRule]) -> Result<(), ProfileValidationError> {
    for rule in rules {
        if matches!(
            rule.sensitivity,
            PrivacySensitivity::Secret | PrivacySensitivity::Sensitive
        ) && rule.restriction.trim().is_empty()
        {
            return Err(ProfileValidationError::SecretTopicWithoutRestriction);
        }
        if rule.sensitivity == PrivacySensitivity::Secret
            && rule.allowed_use != PrivacyAllowedUse::ExcludeFromAnswers
            && rule.restriction.trim().is_empty()
        {
            return Err(ProfileValidationError::SecretTopicWithoutRestriction);
        }
    }
    Ok(())
}

fn validate_evidence_authority_consistency(
    profile: &WorkProxyProfile,
) -> Result<(), ProfileValidationError> {
    let policy = &profile.evidence_policy;
    if policy.require_evidence_for_claims && policy.answer_without_evidence {
        return Err(ProfileValidationError::InvalidEvidencePolicy(
            "require_evidence_for_claims cannot be true when answer_without_evidence is true"
                .into(),
        ));
    }

    if policy.unsupported_claim_behavior == UnsupportedClaimBehavior::Refuse {
        for rule in &profile.authority_rules {
            if rule.authority == ProxyAuthorityLevel::CanAnswer && !rule.evidence_required {
                return Err(ProfileValidationError::ConflictingProfilePolicy(
                    "can-answer rule without evidence conflicts with refuse unsupported claims"
                        .into(),
                ));
            }
        }
    }

    Ok(())
}

fn validate_refusal_authority_consistency(
    profile: &WorkProxyProfile,
) -> Result<(), ProfileValidationError> {
    let allows_impersonation = profile.authority_rules.iter().any(|rule| {
        rule.description
            .as_ref()
            .is_some_and(|text| text.to_ascii_lowercase().contains("impersonate"))
            || rule
                .conditions
                .iter()
                .any(|condition| condition.to_ascii_lowercase().contains("impersonate"))
    });
    if allows_impersonation {
        return Err(ProfileValidationError::ConflictingProfilePolicy(
            "impersonation cannot be allowed while no-impersonation refusal is required".into(),
        ));
    }

    for rule in &profile.authority_rules {
        if rule.authority == ProxyAuthorityLevel::CannotAnswer
            && rule
                .description
                .as_ref()
                .is_some_and(|text| text.to_ascii_lowercase().contains("delegate"))
        {
            return Err(ProfileValidationError::ConflictingProfilePolicy(
                "cannot-answer rule combined with delegated approval behavior".into(),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        CommunicationPreferences, DecisionPreferences, EvidencePolicy, EvidenceSourceKind,
        WORK_PROXY_PROFILE_VERSION,
    };

    fn authority_rule(
        rule_id: &str,
        scope: &str,
        authority: ProxyAuthorityLevel,
        evidence_required: bool,
        human_confirmation_required: bool,
    ) -> AuthorityRule {
        AuthorityRule {
            rule_id: rule_id.into(),
            scope: scope.into(),
            authority,
            description: None,
            conditions: vec![],
            evidence_required,
            human_confirmation_required,
            limitations: vec![],
        }
    }

    fn base_profile() -> WorkProxyProfile {
        WorkProxyProfile {
            profile_id: "profile-policy-001".into(),
            workspace_id: "ws-policy".into(),
            owner_label: "Policy Owner".into(),
            role_label: "Lead".into(),
            working_style: String::new(),
            communication_style: String::new(),
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
            authority_rules: vec![authority_rule(
                "rule-global",
                "*",
                ProxyAuthorityLevel::MustAskHuman,
                true,
                true,
            )],
            privacy_rules: vec![],
            sensitive_topics: vec![],
            default_refusal_rules: vec![
                DefaultRefusalRule {
                    rule_id: "refusal-no-impersonation".into(),
                    statement: "cannot impersonate owner".into(),
                },
                DefaultRefusalRule {
                    rule_id: "refusal-irreversible".into(),
                    statement: "cannot approve irreversible actions".into(),
                },
            ],
            evidence_policy: EvidencePolicy {
                answer_without_evidence: false,
                require_evidence_for_claims: true,
                expose_limitations: true,
                cite_source_kinds: vec![EvidenceSourceKind::WorkEvent],
                unsupported_claim_behavior: UnsupportedClaimBehavior::AskHuman,
            },
            limitations: vec!["policy metadata only".into()],
            created_at: "2026-07-17T08:00:00Z".into(),
            last_updated_at: "2026-07-17T08:30:00Z".into(),
            profile_version: WORK_PROXY_PROFILE_VERSION.to_string(),
        }
    }

    #[test]
    fn restrictiveness_order_is_frozen() {
        assert!(
            authority_restrictiveness(ProxyAuthorityLevel::CannotAnswer)
                < authority_restrictiveness(ProxyAuthorityLevel::MustAskHuman)
        );
        assert!(
            authority_restrictiveness(ProxyAuthorityLevel::MustAskHuman)
                < authority_restrictiveness(ProxyAuthorityLevel::CanAnswer)
        );
    }
}
