//! Dev Track 0.1.6 Checkpoint B — opaque question identity and construction (pure).

use crate::domain::{
    normalize_proxy_question_text, validate_proxy_question, validate_proxy_question_id,
    ProxyQuestion, ProxyQuestionValidationError, PROXY_QUESTION_PROTOCOL_VERSION,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Process-global sequence shared by every `ProcessLocalRequestIdentityProvider` instance.
static PROCESS_LOCAL_QUESTION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Produces opaque `questionId` values for Ask My Proxy requests.
pub trait ProxyRequestIdentityProvider {
    fn next_question_id(&self) -> Result<String, ProxyQuestionIdentityError>;
}

/// Opaque process-local question identity for production CLI use.
///
/// This is not a security token and is not claimed to be cryptographically secure.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProcessLocalRequestIdentityProvider;

impl ProcessLocalRequestIdentityProvider {
    pub fn new() -> Self {
        Self
    }
}

impl ProxyRequestIdentityProvider for ProcessLocalRequestIdentityProvider {
    fn next_question_id(&self) -> Result<String, ProxyQuestionIdentityError> {
        let elapsed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ProxyQuestionIdentityError::ClockBeforeUnixEpoch)?;
        let sequence = PROCESS_LOCAL_QUESTION_ID_COUNTER.fetch_add(1, Ordering::Relaxed) + 1;
        let pid = std::process::id();
        let question_id = format!("proxy-q-{:x}-{:x}-{:x}", elapsed.as_nanos(), pid, sequence);
        validate_proxy_question_id(&question_id)
            .map_err(|err| ProxyQuestionIdentityError::GeneratedIdInvalid(err.to_string()))?;
        Ok(question_id)
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyQuestionIdentityError {
    #[error("system clock is before the Unix epoch")]
    ClockBeforeUnixEpoch,
    #[error("generated question id failed validation")]
    GeneratedIdInvalid(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyQuestionConstructionError {
    #[error("question text is invalid")]
    InvalidText(#[from] ProxyQuestionValidationError),
    #[error("question identity generation failed")]
    IdentityGenerationFailed(#[from] ProxyQuestionIdentityError),
}

/// Construct a validated `ProxyQuestion` using an injected identity provider.
pub fn create_proxy_question(
    text: &str,
    identity_provider: &dyn ProxyRequestIdentityProvider,
) -> Result<ProxyQuestion, ProxyQuestionConstructionError> {
    let normalized = normalize_proxy_question_text(text);
    if normalized.is_empty() {
        return Err(ProxyQuestionValidationError::EmptyText.into());
    }
    if normalized.len() > crate::domain::MAX_PROXY_QUESTION_TEXT_BYTES {
        return Err(ProxyQuestionValidationError::TextTooLong {
            max: crate::domain::MAX_PROXY_QUESTION_TEXT_BYTES,
        }
        .into());
    }
    let question_id = identity_provider.next_question_id()?;
    let question = ProxyQuestion {
        protocol_version: PROXY_QUESTION_PROTOCOL_VERSION.to_string(),
        question_id,
        text: text.to_string(),
    };
    validate_proxy_question(&question)?;
    Ok(question)
}
