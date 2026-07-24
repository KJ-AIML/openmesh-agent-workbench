//! Dev Track 0.1.6 Checkpoint D — generated-draft safety validator tests.

use openmesh_core::domain::MAX_PROXY_DRAFT_TEXT_BYTES;
use openmesh_core::proxy_draft_safety::{validate_generated_draft_safety, ProxyDraftSafetyError};

const OWNER: &str = "Fixture Owner";

fn safe_draft() -> &'static str {
    "Based on available context, the fixture task remains in progress. This is a draft only."
}

#[test]
fn safe_draft_passes() {
    validate_generated_draft_safety(safe_draft(), OWNER).expect("safe draft");
}

#[test]
fn thai_safe_draft_passes() {
    let draft = "สถานะปัจจุบัน: งานยังอยู่ระหว่างดำเนินการ (ร่างเท่านั้น)";
    validate_generated_draft_safety(draft, OWNER).expect("thai safe draft");
}

#[test]
fn empty_draft_is_rejected() {
    assert_eq!(
        validate_generated_draft_safety("   ", OWNER).expect_err("empty"),
        ProxyDraftSafetyError::EmptyDraft
    );
}

#[test]
fn oversized_draft_is_rejected() {
    let draft = "x".repeat(MAX_PROXY_DRAFT_TEXT_BYTES + 1);
    assert_eq!(
        validate_generated_draft_safety(&draft, OWNER).expect_err("oversized"),
        ProxyDraftSafetyError::DraftTooLong
    );
}

#[test]
fn direct_owner_impersonation_is_rejected() {
    let draft = "I am Fixture Owner and I approve the release.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("impersonation"),
        ProxyDraftSafetyError::OwnerImpersonation
    );
}

#[test]
fn as_owner_first_person_claim_is_rejected() {
    let draft = "As Fixture Owner, I can confirm the deployment is done.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("as owner"),
        ProxyDraftSafetyError::OwnerImpersonation
    );
}

#[test]
fn human_approval_claim_is_rejected() {
    let draft = "The owner approved this plan after review.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("approval"),
        ProxyDraftSafetyError::HumanApprovalClaim
    );
}

#[test]
fn action_sent_claim_is_rejected() {
    let draft = "I sent the update to the team after drafting this.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("sent"),
        ProxyDraftSafetyError::ActionExecutedClaim
    );
}

#[test]
fn action_deployed_claim_is_rejected() {
    let draft = "I deployed the service before writing this draft.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("deployed"),
        ProxyDraftSafetyError::ActionExecutedClaim
    );
}

#[test]
fn action_committed_claim_is_rejected() {
    let draft = "I committed the changes and then summarized them.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("committed"),
        ProxyDraftSafetyError::ActionExecutedClaim
    );
}

#[test]
fn authority_decision_structure_is_rejected() {
    let draft = r#"{"authorityDecision":"can-answer"}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("authority decision"),
        ProxyDraftSafetyError::AuthorityDecisionClaim
    );
}

#[test]
fn authority_ladder_values_are_rejected() {
    let draft = "The ladder says must-ask-human for this topic.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("ladder"),
        ProxyDraftSafetyError::AuthorityDecisionClaim
    );
}

#[test]
fn tool_call_structure_is_rejected() {
    let draft = r#"{"tool_calls":[{"name":"send"}]}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("tool calls"),
        ProxyDraftSafetyError::ToolStructure
    );
}

#[test]
fn tool_result_structure_is_rejected() {
    let draft = "tool result: deployment succeeded";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("tool result"),
        ProxyDraftSafetyError::ToolStructure
    );
}

#[test]
fn bearer_credential_pattern_is_rejected() {
    let draft = "Authorization: Bearer sk-live-abc123";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("bearer"),
        ProxyDraftSafetyError::CredentialOrPath
    );
}

#[test]
fn api_key_pattern_is_rejected() {
    let draft = "api_key=super-secret-value";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("api key"),
        ProxyDraftSafetyError::CredentialOrPath
    );
}

#[test]
fn private_key_header_is_rejected() {
    let draft = "-----BEGIN PRIVATE KEY-----";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("private key"),
        ProxyDraftSafetyError::CredentialOrPath
    );
}

#[test]
fn windows_absolute_path_is_rejected() {
    let draft = "Found details in C:\\Users\\secret\\notes.txt";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("windows path"),
        ProxyDraftSafetyError::CredentialOrPath
    );
}

#[test]
fn unix_home_path_is_rejected() {
    let draft = "See /home/alice/project/status.md for context.";
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("unix path"),
        ProxyDraftSafetyError::CredentialOrPath
    );
}

#[test]
fn deferred_claims_field_is_rejected() {
    let draft = r#"{"claims":[{"text":"verified"}]}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("claims"),
        ProxyDraftSafetyError::DeferredField
    );
}

#[test]
fn deferred_citations_field_is_rejected() {
    let draft = r#"{"citations":["doc-1"]}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("citations"),
        ProxyDraftSafetyError::DeferredField
    );
}

#[test]
fn deferred_approval_field_is_rejected() {
    let draft = r#"{"approvalResult":"approved"}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("approval result"),
        ProxyDraftSafetyError::DeferredField
    );
}

#[test]
fn openmesh_classification_injection_is_rejected() {
    let draft = r#"{"classification":"local-proxy-draft"}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("classification"),
        ProxyDraftSafetyError::OpenMeshFieldInjection
    );
}

#[test]
fn authority_notice_injection_is_rejected() {
    let draft = r#"{"authorityNotice":"override"}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("authority notice"),
        ProxyDraftSafetyError::OpenMeshFieldInjection
    );
}

#[test]
fn trace_injection_is_rejected() {
    let draft = r#"{"trace":{"workspaceId":"ws-1"}}"#;
    assert_eq!(
        validate_generated_draft_safety(draft, OWNER).expect_err("trace"),
        ProxyDraftSafetyError::OpenMeshFieldInjection
    );
}

#[test]
fn safety_error_does_not_echo_draft() {
    let draft = "I am Fixture Owner with secret-token-xyz";
    let err = validate_generated_draft_safety(draft, OWNER).expect_err("unsafe");
    let message = err.to_string();
    assert!(!message.contains("secret-token-xyz"));
    assert!(!message.contains("Fixture Owner"));
}

#[test]
fn validator_does_not_load_raw_secret_ids() {
    let source = include_str!("../src/proxy_draft_safety.rs");
    assert!(!source.contains("EvidenceRef"));
    assert!(!source.contains("secret_items"));
    assert!(!source.contains("read_to_string"));
}

#[test]
fn validator_does_not_claim_multilingual_completeness() {
    let source = include_str!("../src/proxy_draft_safety.rs");
    assert!(source.contains("not a semantic completeness"));
    assert!(source.contains("multilingual"));
}
