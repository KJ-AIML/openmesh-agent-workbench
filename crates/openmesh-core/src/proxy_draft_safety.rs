//! Dev Track 0.1.6 Checkpoint D — high-confidence generated-draft safety checks (pure).

use crate::domain::MAX_PROXY_DRAFT_TEXT_BYTES;

/// Fixed OpenMesh-owned first limitation for proxy drafts.
pub const PROXY_DRAFT_FIXED_LIMITATION: &str =
    "This is a non-authoritative local preview based on sanitized local work context.";

/// High-confidence generated-draft safety rejection (Local Alpha boundary).
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftSafetyError {
    #[error("generated draft text is empty")]
    EmptyDraft,
    #[error("generated draft text exceeds the allowed byte bound")]
    DraftTooLong,
    #[error("generated draft contains a high-confidence owner impersonation pattern")]
    OwnerImpersonation,
    #[error("generated draft contains a high-confidence human approval claim")]
    HumanApprovalClaim,
    #[error("generated draft contains a high-confidence action-executed claim")]
    ActionExecutedClaim,
    #[error("generated draft contains a high-confidence authority-decision pattern")]
    AuthorityDecisionClaim,
    #[error("generated draft contains a high-confidence tool or action structure")]
    ToolStructure,
    #[error("generated draft contains a high-confidence credential or path pattern")]
    CredentialOrPath,
    #[error("generated draft contains a deferred structured field")]
    DeferredField,
    #[error("generated draft attempts to inject OpenMesh-owned fields")]
    OpenMeshFieldInjection,
    #[error("generated draft contradicts an active answering runtime")]
    RuntimeCapabilityContradiction,
}

/// Optional current limitation when continuity evidence may be incomplete.
pub const PROXY_DRAFT_PENDING_EVIDENCE_LIMITATION: &str =
    "evidence may be incomplete or pending confirmation";

/// Returns true when a limitation describes historical 0.1.4/0.1.5 metadata rather than
/// current 0.1.6 runtime capability.
pub fn is_stale_historical_runtime_limitation(limitation: &str) -> bool {
    let normalized = normalize_for_match(limitation);
    [
        "no answering runtime in 0.1.4",
        "no answering runtime in 0.1.5",
        "no answering runtime is available",
        "no answering runtime exists",
        "no answering runtime in the current environment",
        "context pack metadata only; no answering runtime",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

/// Remove historical runtime-capability limitations before prompt or draft assembly.
pub fn filter_stale_runtime_limitations(limitations: &[String]) -> Vec<String> {
    limitations
        .iter()
        .filter(|entry| !is_stale_historical_runtime_limitation(entry))
        .cloned()
        .collect()
}

/// Fail closed when a networked runtime produced output that explicitly claims no answering
/// runtime exists or is available.
pub fn validate_networked_runtime_consistency(
    draft_text: &str,
    limitations: &[String],
) -> Result<(), ProxyDraftSafetyError> {
    if contains_explicit_no_answering_runtime_claim(&normalize_for_match(draft_text)) {
        return Err(ProxyDraftSafetyError::RuntimeCapabilityContradiction);
    }
    for limitation in limitations {
        if contains_explicit_no_answering_runtime_claim(&normalize_for_match(limitation)) {
            return Err(ProxyDraftSafetyError::RuntimeCapabilityContradiction);
        }
    }
    Ok(())
}

fn contains_explicit_no_answering_runtime_claim(normalized: &str) -> bool {
    [
        "no answering runtime is available",
        "no answering runtime exists",
        "no answering runtime in the current environment",
        "no answering runtime in 0.1.4",
        "no answering runtime in 0.1.5",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

/// Validate high-confidence structural and safety patterns for runtime-generated draft text.
///
/// This is not a semantic completeness or multilingual guarantee. Secret detection is
/// pattern-based only and does not compare against raw secret identifiers.
pub fn validate_generated_draft_safety(
    draft_text: &str,
    owner_label: &str,
) -> Result<(), ProxyDraftSafetyError> {
    let trimmed = draft_text.trim();
    if trimmed.is_empty() {
        return Err(ProxyDraftSafetyError::EmptyDraft);
    }
    if draft_text.len() > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyDraftSafetyError::DraftTooLong);
    }

    let normalized = normalize_for_match(draft_text);
    let owner = normalize_for_match(owner_label.trim());

    if contains_owner_impersonation(&normalized, &owner) {
        return Err(ProxyDraftSafetyError::OwnerImpersonation);
    }
    if contains_human_approval_claim(&normalized, &owner) {
        return Err(ProxyDraftSafetyError::HumanApprovalClaim);
    }
    if contains_action_executed_claim(&normalized) {
        return Err(ProxyDraftSafetyError::ActionExecutedClaim);
    }
    if contains_authority_decision_claim(&normalized) {
        return Err(ProxyDraftSafetyError::AuthorityDecisionClaim);
    }
    if contains_tool_structure(&normalized) {
        return Err(ProxyDraftSafetyError::ToolStructure);
    }
    if contains_credential_or_path_pattern(draft_text, &normalized) {
        return Err(ProxyDraftSafetyError::CredentialOrPath);
    }
    if contains_deferred_field(&normalized) {
        return Err(ProxyDraftSafetyError::DeferredField);
    }
    if contains_openmesh_field_injection(&normalized) {
        return Err(ProxyDraftSafetyError::OpenMeshFieldInjection);
    }

    Ok(())
}

fn normalize_for_match(text: &str) -> String {
    text.replace('\u{2019}', "'").to_ascii_lowercase()
}

fn contains_owner_impersonation(normalized: &str, owner: &str) -> bool {
    if owner.is_empty() {
        return false;
    }
    [
        format!("i am {owner}"),
        format!("i'm {owner}"),
        format!("as {owner}, i"),
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn contains_human_approval_claim(normalized: &str, owner: &str) -> bool {
    let mut patterns = vec![
        "i approve".to_string(),
        "approved by the owner".to_string(),
        "the owner approved this".to_string(),
    ];
    if !owner.is_empty() {
        patterns.push(format!("{owner} approved"));
    }
    patterns.iter().any(|pattern| normalized.contains(pattern))
}

fn contains_action_executed_claim(normalized: &str) -> bool {
    [
        "i sent",
        "i deployed",
        "i committed",
        "i executed",
        "action completed",
        "message sent",
        "deployment completed",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn contains_authority_decision_claim(normalized: &str) -> bool {
    [
        "authoritydecision",
        "authority decision:",
        "can-answer",
        "can-suggest",
        "can-draft",
        "must-ask-human",
        "cannot-answer",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn contains_tool_structure(normalized: &str) -> bool {
    [
        "toolcalls",
        "tool_calls",
        "<tool_call>",
        "tool result:",
        "\"executetool\"",
        "\"function_call\"",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn contains_credential_or_path_pattern(raw: &str, normalized: &str) -> bool {
    if [
        "authorization: bearer ",
        "api_key=",
        "apikey:",
        "secret_key=",
        "-----begin",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
    {
        return true;
    }

    if raw.split_whitespace().any(looks_like_windows_absolute_path) {
        return true;
    }

    normalized.contains("/home/") || normalized.contains("/users/")
}

fn looks_like_windows_absolute_path(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() >= 3
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
        && bytes[0].is_ascii_alphabetic()
}

fn contains_deferred_field(normalized: &str) -> bool {
    [
        "\"claims\"",
        "\"citations\"",
        "\"authoritydecision\"",
        "\"approvalresult\"",
        "\"executionpermission\"",
        "\"verifiedanswer\"",
        "\"confirmedbyhuman\"",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}

fn contains_openmesh_field_injection(normalized: &str) -> bool {
    [
        "\"classification\"",
        "\"authoritynotice\"",
        "\"executionboundary\"",
        "\"trace\"",
        "\"evidencesummary\"",
        "\"limitations\"",
        "\"generatedat\"",
        "\"questionid\"",
    ]
    .iter()
    .any(|pattern| normalized.contains(pattern))
}
