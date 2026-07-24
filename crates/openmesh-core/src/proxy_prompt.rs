//! Dev Track 0.1.6 Checkpoint B — deterministic prompt composition (pure).

use crate::context_pack_validation::validate_proxy_context_pack_complete;
use crate::domain::{
    normalize_proxy_question_text, validate_proxy_prompt_bundle, validate_proxy_question,
    ProxyContextPack, ProxyPromptBundle, ProxyQuestion, PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
};
use crate::proxy_prompt_context::{
    bound_proxy_prompt_context, map_pack_to_proxy_prompt_context, serialize_proxy_prompt_context,
    ProxyPromptError,
};

/// Frozen system message constraints for local Work Proxy draft generation.
pub const PROXY_PROMPT_SYSTEM_MESSAGE: &str = "\
You are a local Work Proxy draft assistant. \
You are not the human owner. \
Represent the owner's work context in third person; do not speak as the owner. \
Distinguish known context, inferred draft content, and unknown information. \
Do not claim owner approval, authority, or that any action was performed. \
A configured OpenMesh answering runtime is processing this request; do not claim that no answering runtime exists or is available. \
Do not create tool calls. \
Do not reveal secrets or credentials. \
Authority execution is disabled. \
Answer in the question language when supported. \
Output draft text only.";

/// Compose a validated `ProxyPromptBundle` from a pack and question.
pub fn compose_proxy_prompt(
    pack: &ProxyContextPack,
    question: &ProxyQuestion,
) -> Result<ProxyPromptBundle, ProxyPromptError> {
    validate_proxy_question(question).map_err(|_| ProxyPromptError::InvalidQuestion)?;
    validate_proxy_context_pack_complete(pack).map_err(|_| ProxyPromptError::InvalidContextPack)?;
    let mapped = map_pack_to_proxy_prompt_context(pack)?;
    let bounded = bound_proxy_prompt_context(mapped)?;
    let context_json = serialize_proxy_prompt_context(&bounded)?;
    let bundle = ProxyPromptBundle {
        protocol_version: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION.to_string(),
        system_message: PROXY_PROMPT_SYSTEM_MESSAGE.to_string(),
        context_json,
        user_message: normalize_proxy_question_text(&question.text),
    };
    validate_proxy_prompt_bundle(&bundle).map_err(|_| ProxyPromptError::InvalidPromptBundle)?;
    Ok(bundle)
}
