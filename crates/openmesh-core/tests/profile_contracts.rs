//! Dev Track 0.1.4 Checkpoint A — Work Proxy Profile domain contracts (pure).

use openmesh_core::domain::{
    is_supported_work_proxy_profile_version, validate_authority_rule, validate_evidence_policy,
    validate_privacy_rule, validate_work_proxy_profile, AuthorityRule, CommunicationPreferences,
    DecisionPreferences, DefaultRefusalRule, EvidencePolicy, EvidenceSourceKind, PrivacyAllowedUse,
    PrivacyRule, PrivacySensitivity, ProfileValidationError, ProxyAuthorityLevel,
    UnsupportedClaimBehavior, WorkProxyProfile, WORK_PROXY_PROFILE_VERSION,
};
use std::fs;
use std::path::PathBuf;

fn sample_authority_rule(
    rule_id: &str,
    scope: &str,
    authority: ProxyAuthorityLevel,
) -> AuthorityRule {
    AuthorityRule {
        rule_id: rule_id.into(),
        scope: scope.into(),
        authority,
        description: Some(format!("rule for {scope}")),
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: authority == ProxyAuthorityLevel::MustAskHuman
            || authority == ProxyAuthorityLevel::CannotAnswer,
        limitations: vec![],
    }
}

fn sample_profile() -> WorkProxyProfile {
    WorkProxyProfile {
        profile_id: "profile-sample-001".into(),
        workspace_id: "ws-fixture-0.1.4".into(),
        owner_label: "Fixture Owner".into(),
        role_label: "Engineering lead".into(),
        working_style: "async-first".into(),
        communication_style: "concise".into(),
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
        authority_rules: vec![
            sample_authority_rule("rule-global", "*", ProxyAuthorityLevel::MustAskHuman),
            sample_authority_rule(
                "rule-factual",
                "work.progress",
                ProxyAuthorityLevel::CanAnswer,
            ),
        ],
        privacy_rules: vec![PrivacyRule {
            rule_id: "privacy-secret".into(),
            topic: "credentials".into(),
            sensitivity: PrivacySensitivity::Secret,
            allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
            restriction: "never include in proxy output".into(),
            requires_human_confirmation: true,
        }],
        sensitive_topics: vec!["credentials".into()],
        default_refusal_rules: vec![
            DefaultRefusalRule {
                rule_id: "refusal-no-impersonation".into(),
                statement: "cannot impersonate owner".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-invented-evidence".into(),
                statement: "cannot invent evidence".into(),
            },
        ],
        evidence_policy: EvidencePolicy {
            answer_without_evidence: false,
            require_evidence_for_claims: true,
            expose_limitations: true,
            cite_source_kinds: vec![EvidenceSourceKind::FilePath, EvidenceSourceKind::WorkEvent],
            unsupported_claim_behavior: UnsupportedClaimBehavior::AskHuman,
        },
        limitations: vec![
            "proxy profile metadata only".into(),
            "no answering behavior in 0.1.4".into(),
        ],
        created_at: "2026-07-17T08:00:00Z".into(),
        last_updated_at: "2026-07-17T08:30:00Z".into(),
        profile_version: WORK_PROXY_PROFILE_VERSION.to_string(),
    }
}

#[test]
fn work_proxy_profile_round_trips_json() {
    let profile = sample_profile();
    let json = serde_json::to_string(&profile).expect("serialize");
    let restored: WorkProxyProfile = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, profile);
}

#[test]
fn work_proxy_profile_requires_supported_version() {
    let mut profile = sample_profile();
    profile.profile_version = "99.0".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::UnsupportedProfileVersion {
            found: "99.0".into(),
        })
    );
    assert!(is_supported_work_proxy_profile_version("1.0"));
    assert!(!is_supported_work_proxy_profile_version("99.0"));
}

#[test]
fn work_proxy_profile_requires_non_empty_identity_fields() {
    let mut profile = sample_profile();
    profile.owner_label = "   ".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::EmptyOwnerLabel)
    );

    profile = sample_profile();
    profile.workspace_id = "".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::EmptyWorkspaceId)
    );
}

#[test]
fn work_proxy_profile_requires_utc_timestamps() {
    let mut profile = sample_profile();
    profile.created_at = "2026-07-17T08:00:00-05:00".into();
    assert!(matches!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::InvalidTimestamp(_))
    ));
}

#[test]
fn work_proxy_profile_rejects_created_after_last_updated() {
    let mut profile = sample_profile();
    profile.created_at = "2026-07-17T09:00:00Z".into();
    profile.last_updated_at = "2026-07-17T08:00:00Z".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::CreatedAfterLastUpdated)
    );
}

#[test]
fn authority_ladder_serializes_exact_wire_values() {
    let cases = [
        (ProxyAuthorityLevel::CanAnswer, "\"can-answer\""),
        (ProxyAuthorityLevel::CanSuggest, "\"can-suggest\""),
        (ProxyAuthorityLevel::CanDraft, "\"can-draft\""),
        (ProxyAuthorityLevel::MustAskHuman, "\"must-ask-human\""),
        (ProxyAuthorityLevel::CannotAnswer, "\"cannot-answer\""),
    ];
    for (level, expected) in cases {
        let json = serde_json::to_string(&level).expect("serialize authority");
        assert_eq!(json, expected);
    }
}

#[test]
fn authority_rule_requires_valid_authority() {
    let rule = sample_authority_rule("rule-1", "topic", ProxyAuthorityLevel::CanSuggest);
    validate_authority_rule(&rule).expect("valid authority rule");
}

#[test]
fn authority_rule_represents_must_ask_human() {
    let rule = sample_authority_rule("rule-ask", "*", ProxyAuthorityLevel::MustAskHuman);
    assert!(rule.human_confirmation_required);
    validate_authority_rule(&rule).expect("must-ask-human rule validates");
}

#[test]
fn authority_rule_represents_cannot_answer() {
    let rule = sample_authority_rule("rule-deny", "secrets", ProxyAuthorityLevel::CannotAnswer);
    assert_eq!(rule.authority, ProxyAuthorityLevel::CannotAnswer);
    validate_authority_rule(&rule).expect("cannot-answer rule validates");
}

#[test]
fn evidence_policy_requires_evidence_for_claims_when_configured() {
    let mut policy = sample_profile().evidence_policy;
    policy.require_evidence_for_claims = true;
    policy.answer_without_evidence = true;
    assert!(matches!(
        validate_evidence_policy(&policy),
        Err(ProfileValidationError::InvalidEvidencePolicy(_))
    ));
}

#[test]
fn evidence_policy_rejects_unsupported_claim_behavior() {
    let json = r#"{"answerWithoutEvidence":false,"requireEvidenceForClaims":true,"exposeLimitations":true,"citeSourceKinds":["file-path"],"unsupportedClaimBehavior":"fabricate"}"#;
    let result: Result<EvidencePolicy, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "unknown unsupportedClaimBehavior must fail deserialize"
    );
}

#[test]
fn privacy_rule_represents_sensitive_and_secret_boundaries() {
    let sensitive = PrivacyRule {
        rule_id: "privacy-sensitive".into(),
        topic: "customer-data".into(),
        sensitivity: PrivacySensitivity::Sensitive,
        allowed_use: PrivacyAllowedUse::SummarizeWithCaution,
        restriction: "redact identifiers".into(),
        requires_human_confirmation: true,
    };
    let secret = PrivacyRule {
        rule_id: "privacy-secret".into(),
        topic: "credentials".into(),
        sensitivity: PrivacySensitivity::Secret,
        allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
        restriction: "exclude completely".into(),
        requires_human_confirmation: true,
    };
    validate_privacy_rule(&sensitive).expect("sensitive rule");
    validate_privacy_rule(&secret).expect("secret rule");
}

#[test]
fn default_refusal_rules_include_no_impersonation() {
    let profile = sample_profile();
    assert!(profile
        .default_refusal_rules
        .iter()
        .any(|rule| rule.statement.contains("cannot impersonate owner")));
}

#[test]
fn profile_validation_rejects_impersonation_claim() {
    let mut profile = sample_profile();
    profile.owner_label = "I am the human owner".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::ImpersonationClaim)
    );
}

#[test]
fn profile_validation_requires_limitations() {
    let mut profile = sample_profile();
    profile.limitations.clear();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::EmptyLimitations)
    );
}

#[test]
fn profile_validation_bounds_lists_and_text() {
    let mut profile = sample_profile();
    profile.owner_label = "x".repeat(openmesh_core::domain::MAX_PROFILE_LABEL_BYTES + 1);
    assert!(matches!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::OwnerLabelTooLong { .. })
    ));

    let mut profile = sample_profile();
    profile.authority_rules = (0..openmesh_core::domain::MAX_PROFILE_RULES + 1)
        .map(|i| {
            sample_authority_rule(
                &format!("rule-{i}"),
                "scope",
                ProxyAuthorityLevel::CanSuggest,
            )
        })
        .collect();
    assert!(matches!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::TooManyListItems { .. })
    ));
}

#[test]
fn fixture_work_proxy_profile_is_valid() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let raw = fs::read_to_string(root.join("tests/fixtures/profile/work-proxy-profile-valid.json"))
        .expect("read fixture");
    let profile: WorkProxyProfile = serde_json::from_str(&raw).expect("parse fixture");
    validate_work_proxy_profile(&profile).expect("fixture profile must validate");
    assert!(profile
        .default_refusal_rules
        .iter()
        .any(|rule| rule.rule_id == "refusal-no-impersonation"));
}

#[test]
fn checkpoint_a_contracts_are_pure_no_io() {
    let profile = sample_profile();
    let _ = validate_work_proxy_profile(&profile);
    let _ = validate_authority_rule(&profile.authority_rules[0]);
    let _ = validate_privacy_rule(&profile.privacy_rules[0]);
    let _ = validate_evidence_policy(&profile.evidence_policy);
    let _ = is_supported_work_proxy_profile_version(WORK_PROXY_PROFILE_VERSION);
}

#[test]
fn checkpoint_a_does_not_touch_continuity_or_cli() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let continuity_files = [
        "src/continuity/current_state.rs",
        "src/continuity/catch_up.rs",
        "src/continuity/readers.rs",
    ];
    for rel in continuity_files {
        let content = fs::read_to_string(root.join(rel)).expect("read continuity source");
        assert!(!content.contains("WorkProxyProfile"));
        assert!(!content.contains("validate_work_proxy_profile"));
    }
    let cli_root = root.join("../openmesh-cli/src");
    if cli_root.exists() {
        for entry in fs::read_dir(&cli_root).expect("read cli src") {
            let path = entry.expect("entry").path();
            if path.file_name().and_then(|name| name.to_str()) == Some("profile.rs") {
                continue;
            }
            if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                let content = fs::read_to_string(&path).expect("read cli source");
                assert!(!content.contains("WorkProxyProfile"));
                assert!(!content.contains("profile init"));
            }
        }
    }
}

#[test]
fn checkpoint_a_does_not_start_ask_my_proxy_or_context_pack() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/domain.rs")).expect("read domain");
    for forbidden in [
        "ask my proxy",
        "ask-my-proxy",
        "context pack",
        "context-pack",
        "ProxyContextPack",
        "generate_answer",
    ] {
        assert!(
            !domain.to_ascii_lowercase().contains(forbidden),
            "domain.rs must not reference {forbidden}"
        );
    }
}
