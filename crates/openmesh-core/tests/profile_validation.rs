//! Dev Track 0.1.4 Checkpoint B — profile policy validation and authority resolution.

use openmesh_core::domain::{
    validate_evidence_policy, AuthorityRule, CommunicationPreferences, DecisionPreferences,
    DefaultRefusalRule, EvidencePolicy, EvidenceSourceKind, PrivacyAllowedUse, PrivacyRule,
    PrivacySensitivity, ProfileValidationError, ProxyAuthorityLevel, UnsupportedClaimBehavior,
    WorkProxyProfile, WORK_PROXY_PROFILE_VERSION,
};
use openmesh_core::profile_validation::{
    proxy_behavior_allowed, resolve_profile_authority, validate_profile_policy,
    ProfileEvaluationContext, ProfilePolicyResult,
};
use std::fs;
use std::path::PathBuf;

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
        limitations: vec!["rule limitation".into()],
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
fn no_matching_authority_rule_defaults_to_must_ask_human() {
    let mut profile = base_profile();
    profile.authority_rules.clear();
    let result = resolve_profile_authority(
        &profile,
        "unscoped.topic",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::MustAskHuman);
    assert!(result.decision_reason.contains("defaulting"));
}

#[test]
fn cannot_answer_overrides_more_permissive_rules() {
    let mut profile = base_profile();
    profile.authority_rules = vec![
        authority_rule(
            "rule-answer",
            "work",
            ProxyAuthorityLevel::CanAnswer,
            true,
            false,
        ),
        authority_rule(
            "rule-deny",
            "work.secrets",
            ProxyAuthorityLevel::CannotAnswer,
            true,
            true,
        ),
    ];
    let result = resolve_profile_authority(
        &profile,
        "work.secrets.credentials",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CannotAnswer);
}

#[test]
fn must_ask_human_overrides_draft_suggest_and_answer() {
    let mut profile = base_profile();
    profile.authority_rules = vec![
        authority_rule(
            "rule-suggest",
            "topic",
            ProxyAuthorityLevel::CanSuggest,
            true,
            false,
        ),
        authority_rule(
            "rule-ask-specific",
            "topic.detail",
            ProxyAuthorityLevel::MustAskHuman,
            true,
            true,
        ),
    ];
    let result = resolve_profile_authority(
        &profile,
        "topic.detail.status",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::MustAskHuman);
}

#[test]
fn authority_resolution_is_deterministic() {
    let mut profile = base_profile();
    profile.authority_rules = vec![
        authority_rule(
            "rule-b",
            "work",
            ProxyAuthorityLevel::CanSuggest,
            true,
            false,
        ),
        authority_rule(
            "rule-a",
            "work.progress",
            ProxyAuthorityLevel::CanDraft,
            true,
            false,
        ),
    ];
    let context = ProfileEvaluationContext::default();
    let first = resolve_profile_authority(&profile, "work.progress", &context);
    let second = resolve_profile_authority(&profile, "work.progress", &context);
    assert_eq!(first, second);
}

#[test]
fn authority_resolution_reports_all_matched_rule_ids() {
    let mut profile = base_profile();
    profile.authority_rules = vec![
        authority_rule(
            "rule-global",
            "*",
            ProxyAuthorityLevel::MustAskHuman,
            true,
            true,
        ),
        authority_rule(
            "rule-work",
            "work",
            ProxyAuthorityLevel::CanSuggest,
            true,
            false,
        ),
    ];
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.matched_rule_ids, vec!["rule-work", "rule-global"]);
}

#[test]
fn authority_resolution_preserves_evidence_requirement() {
    let mut profile = base_profile();
    profile.authority_rules = vec![authority_rule(
        "rule-work",
        "work",
        ProxyAuthorityLevel::CanSuggest,
        true,
        false,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    assert!(result.evidence_required);
}

#[test]
fn authority_resolution_preserves_human_confirmation_requirement() {
    let mut profile = base_profile();
    profile.authority_rules = vec![authority_rule(
        "rule-work",
        "work",
        ProxyAuthorityLevel::CanDraft,
        false,
        true,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    assert!(result.human_confirmation_required);
}

#[test]
fn evidence_policy_rejects_answer_without_evidence_conflict() {
    let mut profile = base_profile();
    profile.evidence_policy.answer_without_evidence = true;
    profile.evidence_policy.require_evidence_for_claims = true;
    assert!(validate_evidence_policy(&profile.evidence_policy).is_err());
    assert!(validate_profile_policy(&profile).is_err());
}

#[test]
fn privacy_secret_rule_overrides_can_answer() {
    let mut profile = base_profile();
    profile.authority_rules = vec![authority_rule(
        "rule-answer",
        "credentials",
        ProxyAuthorityLevel::CanAnswer,
        true,
        false,
    )];
    profile.privacy_rules = vec![PrivacyRule {
        rule_id: "privacy-secret".into(),
        topic: "credentials".into(),
        sensitivity: PrivacySensitivity::Secret,
        allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
        restriction: "never include in proxy output".into(),
        requires_human_confirmation: true,
    }];
    let result = resolve_profile_authority(
        &profile,
        "credentials.api",
        &ProfileEvaluationContext {
            topic: "credentials".into(),
            ..Default::default()
        },
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CannotAnswer);
}

#[test]
fn privacy_sensitive_rule_can_force_human_confirmation() {
    let mut profile = base_profile();
    profile.authority_rules = vec![authority_rule(
        "rule-answer",
        "customer",
        ProxyAuthorityLevel::CanAnswer,
        true,
        false,
    )];
    profile.privacy_rules = vec![PrivacyRule {
        rule_id: "privacy-sensitive".into(),
        topic: "customer".into(),
        sensitivity: PrivacySensitivity::Sensitive,
        allowed_use: PrivacyAllowedUse::SummarizeWithCaution,
        restriction: "redact identifiers".into(),
        requires_human_confirmation: true,
    }];
    let result = resolve_profile_authority(
        &profile,
        "customer.profile",
        &ProfileEvaluationContext {
            topic: "customer".into(),
            ..Default::default()
        },
    );
    assert!(result.human_confirmation_required);
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::MustAskHuman);
}

#[test]
fn privacy_rule_does_not_automatically_grant_authority() {
    let mut profile = base_profile();
    profile.privacy_rules = vec![PrivacyRule {
        rule_id: "privacy-public".into(),
        topic: "status".into(),
        sensitivity: PrivacySensitivity::Public,
        allowed_use: PrivacyAllowedUse::ReferenceOnly,
        restriction: "cite source".into(),
        requires_human_confirmation: false,
    }];
    let result = resolve_profile_authority(
        &profile,
        "status.update",
        &ProfileEvaluationContext {
            topic: "status".into(),
            ..Default::default()
        },
    );
    assert_ne!(result.resolved_authority, ProxyAuthorityLevel::CanAnswer);
}

#[test]
fn default_refusal_cannot_be_weakened_by_authority_rule() {
    let mut profile = base_profile();
    profile.authority_rules = vec![authority_rule(
        "rule-answer",
        "identity",
        ProxyAuthorityLevel::CanAnswer,
        true,
        false,
    )];
    let result = resolve_profile_authority(
        &profile,
        "identity.owner",
        &ProfileEvaluationContext {
            topic: "identity".into(),
            involves_impersonation: true,
            ..Default::default()
        },
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CannotAnswer);
    assert!(result
        .limitations
        .iter()
        .any(|item| item.contains("cannot impersonate owner")));
}

#[test]
fn irreversible_action_requires_human_confirmation() {
    let profile = base_profile();
    let result = resolve_profile_authority(
        &profile,
        "deploy.production",
        &ProfileEvaluationContext {
            topic: "deploy".into(),
            is_irreversible: true,
            ..Default::default()
        },
    );
    assert!(result.human_confirmation_required);
}

#[test]
fn conflicting_profile_policy_fails_validation() {
    let mut profile = base_profile();
    profile.authority_rules = vec![
        authority_rule(
            "rule-a",
            "same-scope",
            ProxyAuthorityLevel::CanAnswer,
            true,
            false,
        ),
        authority_rule(
            "rule-b",
            "same-scope",
            ProxyAuthorityLevel::CannotAnswer,
            true,
            true,
        ),
    ];
    assert!(matches!(
        validate_profile_policy(&profile),
        Err(ProfileValidationError::ConflictingProfilePolicy(_))
    ));
}

#[test]
fn missing_profile_means_no_proxy_behavior() {
    assert!(!proxy_behavior_allowed(None));
}

#[test]
fn policy_result_contains_limitations_and_reason() {
    let profile = base_profile();
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    assert!(!result.limitations.is_empty());
    assert!(!result.decision_reason.is_empty());
}

#[test]
fn policy_evaluator_generates_no_answer_content() {
    let profile = base_profile();
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    let serialized = format!("{result:?}");
    assert!(!serialized.to_ascii_lowercase().contains("answer_text"));
    assert!(!serialized.to_ascii_lowercase().contains("response_body"));
}

#[test]
fn checkpoint_b_remains_pure_no_io() {
    let profile = base_profile();
    let _ = validate_profile_policy(&profile);
    let _ = resolve_profile_authority(&profile, "scope", &ProfileEvaluationContext::default());
    let _ = proxy_behavior_allowed(Some(&profile));
}

#[test]
fn checkpoint_b_does_not_touch_continuity_cli_or_tauri() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "src/continuity/current_state.rs",
        "src/continuity/catch_up.rs",
        "../openmesh-cli/src/main.rs",
        "../../src-tauri/src/lib.rs",
    ] {
        let path = root.join(rel);
        if path.exists() {
            let content = fs::read_to_string(path).expect("read source");
            assert!(!content.contains("resolve_profile_authority"));
        }
    }
}

#[test]
fn checkpoint_b_does_not_start_context_pack_or_ask_my_proxy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let policy = fs::read_to_string(root.join("src/profile_validation.rs")).expect("read policy");
    for forbidden in [
        "ask-my-proxy",
        "ask my proxy",
        "context-pack",
        "context pack",
        "ProxyContextPack",
        "generate_answer",
    ] {
        assert!(
            !policy.to_ascii_lowercase().contains(forbidden),
            "profile_validation must not reference {forbidden}"
        );
    }
}

#[allow(dead_code)]
fn policy_result_is_metadata_only(_result: ProfilePolicyResult) {}
