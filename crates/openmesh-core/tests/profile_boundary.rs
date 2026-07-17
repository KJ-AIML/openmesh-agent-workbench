//! Dev Track 0.1.4 Checkpoint E — core profile boundary proofs.

use openmesh_core::domain::{
    default_work_proxy_profile, validate_work_proxy_profile, AuthorityRule, ProfileValidationError,
    ProxyAuthorityLevel, WorkProxyProfile,
};
use openmesh_core::profile::{profile_exists, read_work_proxy_profile, ProfileError};
use openmesh_core::profile_validation::{
    proxy_behavior_allowed, resolve_profile_authority, ProfileEvaluationContext,
    ProfilePolicyResult,
};
use std::fs;
use std::path::PathBuf;

fn sample_profile(workspace_id: &str) -> WorkProxyProfile {
    default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Fixture Owner",
        "Engineering lead",
        "2026-07-17T08:00:00Z",
    )
}

fn authority_rule(rule_id: &str, scope: &str, authority: ProxyAuthorityLevel) -> AuthorityRule {
    AuthorityRule {
        rule_id: rule_id.into(),
        scope: scope.into(),
        authority,
        description: None,
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: matches!(
            authority,
            ProxyAuthorityLevel::MustAskHuman | ProxyAuthorityLevel::CannotAnswer
        ),
        limitations: vec![],
    }
}

fn assert_policy_metadata_only(result: &ProfilePolicyResult) {
    let serialized = format!("{result:?}").to_ascii_lowercase();
    for forbidden in [
        "answer_text",
        "response_body",
        "draft_text",
        "suggestion_text",
        "proxy_response",
        "generated_answer",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "policy result must not contain {forbidden}"
        );
    }
}

#[test]
fn profile_owner_label_is_metadata_not_impersonation() {
    let profile = sample_profile("ws-boundary");
    assert_eq!(profile.owner_label, "Fixture Owner");
    assert!(!profile.owner_label.to_ascii_lowercase().contains("i am "));
    validate_work_proxy_profile(&profile).expect("descriptive owner label validates");
}

#[test]
fn permissive_authority_cannot_override_no_impersonation_refusal() {
    let mut profile = sample_profile("ws-impersonation");
    profile.authority_rules = vec![authority_rule(
        "rule-answer",
        "identity",
        ProxyAuthorityLevel::CanAnswer,
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
}

#[test]
fn impersonation_conflicting_profile_fails_validation() {
    let mut profile = sample_profile("ws-bad-owner");
    profile.owner_label = "I am the human owner".into();
    assert_eq!(
        validate_work_proxy_profile(&profile),
        Err(ProfileValidationError::ImpersonationClaim)
    );
}

#[test]
fn authority_resolution_returns_policy_metadata_only() {
    let profile = sample_profile("ws-policy-meta");
    let result = resolve_profile_authority(
        &profile,
        "work.progress",
        &ProfileEvaluationContext::default(),
    );
    assert_policy_metadata_only(&result);
    assert!(!result.decision_reason.is_empty());
}

#[test]
fn can_answer_rule_does_not_generate_answer() {
    let mut profile = sample_profile("ws-can-answer");
    profile.authority_rules = vec![authority_rule(
        "rule-answer",
        "work",
        ProxyAuthorityLevel::CanAnswer,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.status",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CanAnswer);
    assert_policy_metadata_only(&result);
}

#[test]
fn can_draft_rule_does_not_generate_draft() {
    let mut profile = sample_profile("ws-can-draft");
    profile.authority_rules = vec![authority_rule(
        "rule-draft",
        "work",
        ProxyAuthorityLevel::CanDraft,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.status",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CanDraft);
    assert_policy_metadata_only(&result);
}

#[test]
fn can_suggest_rule_does_not_generate_suggestion() {
    let mut profile = sample_profile("ws-can-suggest");
    profile.authority_rules = vec![authority_rule(
        "rule-suggest",
        "work",
        ProxyAuthorityLevel::CanSuggest,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.status",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::CanSuggest);
    assert_policy_metadata_only(&result);
}

#[test]
fn must_ask_human_does_not_invent_human_confirmation() {
    let mut profile = sample_profile("ws-must-ask");
    profile.authority_rules = vec![authority_rule(
        "rule-ask",
        "work",
        ProxyAuthorityLevel::MustAskHuman,
    )];
    let result = resolve_profile_authority(
        &profile,
        "work.status",
        &ProfileEvaluationContext::default(),
    );
    assert_eq!(result.resolved_authority, ProxyAuthorityLevel::MustAskHuman);
    assert!(result.human_confirmation_required);
    let reason = result.decision_reason.to_ascii_lowercase();
    assert!(!reason.contains("human approved"));
    assert!(!reason.contains("owner approved"));
    assert_policy_metadata_only(&result);
}

#[test]
fn cannot_answer_remains_terminal() {
    let mut profile = sample_profile("ws-cannot-answer");
    profile.authority_rules = vec![
        authority_rule("rule-answer", "work", ProxyAuthorityLevel::CanAnswer),
        authority_rule(
            "rule-deny",
            "work.secrets",
            ProxyAuthorityLevel::CannotAnswer,
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
fn missing_profile_is_fail_closed() {
    assert!(!proxy_behavior_allowed(None));
}

#[test]
fn privacy_restriction_dominates_permissive_authority() {
    let profile = sample_profile("ws-privacy");
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
fn profile_modules_invoke_no_llm_axga_or_model_runtime() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for rel in ["profile.rs", "profile_validation.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        let lowered = content.to_ascii_lowercase();
        for forbidden in [
            "openai",
            "anthropic",
            "axga",
            "langchain",
            "llm::",
            "invoke_model",
            "chat_completion",
            "continuityintelligence",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "{rel} must not reference {forbidden}"
            );
        }
    }
}

#[test]
fn checkpoint_e_does_not_start_0_1_5_or_0_1_6() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for rel in ["profile.rs", "profile_validation.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        let lowered = content.to_ascii_lowercase();
        for forbidden in [
            "proxycontextpack",
            "contextpack",
            "askmyproxy",
            "0.1.5",
            "0.1.6",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "{rel} must not start {forbidden}"
            );
        }
    }
}

#[test]
fn read_missing_profile_returns_explicit_error_without_synthesis() {
    let dir = std::env::temp_dir().join(format!(
        "openmesh-profile-boundary-missing-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".openmesh")).unwrap();
    fs::write(
        dir.join(".openmesh/project.json"),
        r#"{"id":"ws-missing","name":"x","folderPath":"","defaultBranch":"main","sprintSource":"none","status":"active","createdAt":"2026-07-17T08:00:00Z","updatedAt":"2026-07-17T08:00:00Z"}"#,
    )
    .unwrap();
    let path = dir.to_string_lossy().to_string();
    assert!(matches!(
        read_work_proxy_profile(&path),
        Err(ProfileError::ProfileMissing)
    ));
    assert!(!profile_exists(&path).unwrap());
}
