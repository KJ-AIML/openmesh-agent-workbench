//! Dev Track 0.1.6 Checkpoint C — provider-neutral proxy draft runtime seam (pure).

use crate::domain::{
    validate_proxy_runtime_output, validate_proxy_runtime_request, ProxyPromptBundle,
    ProxyRuntimeOutput, ProxyRuntimeRequest, MAX_PROXY_DRAFT_TEXT_BYTES,
};

/// Adapter-declared runtime kind for the unconfigured production placeholder.
pub const PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED: &str = "unconfigured";

/// Adapter-declared runtime kind for the deterministic test stub.
pub const PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB: &str = "deterministic-stub";

/// Stable test-only provider identifier returned by the deterministic stub.
pub const DETERMINISTIC_STUB_PROVIDER_ID: &str = "openmesh-test";

/// Stable test-only model identifier returned by the deterministic stub.
pub const DETERMINISTIC_STUB_MODEL_ID: &str = "deterministic-stub";

/// Fixed deterministic duration reported by the deterministic stub.
pub const DETERMINISTIC_STUB_DURATION_MS: u64 = 0;

/// Synchronous, provider-neutral runtime seam for Ask My Proxy draft generation.
///
/// Implementations return runtime-owned `ProxyRuntimeOutput` only. They must not
/// construct `ProxyDraft`, populate trace metadata, or perform authority actions.
pub trait ProxyDraftRuntime: Send + Sync {
    /// Adapter-declared OpenMesh runtime kind (not derived from model text).
    fn runtime_kind(&self) -> &'static str;

    /// Generate draft text for a validated runtime request.
    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError>;
}

/// Typed, secret-safe runtime errors for the proxy draft seam.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftRuntimeError {
    #[error("proxy runtime request is invalid")]
    InvalidRequest,
    #[error("proxy draft runtime is not configured")]
    RuntimeNotConfigured,
    #[error("proxy draft runtime timed out")]
    Timeout,
    #[error("proxy draft runtime is unavailable")]
    RuntimeUnavailable,
    #[error("proxy draft runtime provider failed")]
    ProviderFailure,
    #[error("proxy draft runtime produced invalid output")]
    InvalidOutput,
}

/// Claim A runtime — validates requests but never fabricates draft text.
///
/// Performs no I/O, network access, environment reads, threading, or sleeping.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnconfiguredProxyDraftRuntime;

impl UnconfiguredProxyDraftRuntime {
    pub fn new() -> Self {
        Self
    }
}

impl ProxyDraftRuntime for UnconfiguredProxyDraftRuntime {
    fn runtime_kind(&self) -> &'static str {
        PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_runtime_request(request)?;
        Err(ProxyDraftRuntimeError::RuntimeNotConfigured)
    }
}

/// Deterministic test/CI runtime — not a model and not a production fallback.
///
/// Derives draft text only from the validated `ProxyPromptBundle` embedded in the
/// request. Never reads project files, context packs, credentials, or the network.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeterministicStubProxyDraftRuntime;

impl DeterministicStubProxyDraftRuntime {
    /// Construct the deterministic stub for automated tests and CI injection only.
    ///
    /// This runtime is not a model, does not perform network I/O, and must not be
    /// selected by any production runtime factory.
    pub fn new_for_tests() -> Self {
        Self
    }
}

impl ProxyDraftRuntime for DeterministicStubProxyDraftRuntime {
    fn runtime_kind(&self) -> &'static str {
        PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_runtime_request(request)?;
        let draft_text =
            deterministic_stub_draft_text(&request.prompt, request.max_output_bytes as usize);
        let output = ProxyRuntimeOutput {
            draft_text,
            provider_id: DETERMINISTIC_STUB_PROVIDER_ID.to_string(),
            model_id: DETERMINISTIC_STUB_MODEL_ID.to_string(),
            network_used: false,
            duration_ms: DETERMINISTIC_STUB_DURATION_MS,
        };
        validate_proxy_runtime_output(&output)
            .map_err(|_| ProxyDraftRuntimeError::InvalidOutput)?;
        Ok(output)
    }
}

fn validate_runtime_request(request: &ProxyRuntimeRequest) -> Result<(), ProxyDraftRuntimeError> {
    validate_proxy_runtime_request(request).map_err(|_| ProxyDraftRuntimeError::InvalidRequest)
}

fn deterministic_stub_draft_text(bundle: &ProxyPromptBundle, max_output_bytes: usize) -> String {
    let bound = max_output_bytes.min(MAX_PROXY_DRAFT_TEXT_BYTES);
    let fingerprint = bundle_fingerprint(bundle);
    let draft = format!(
        "Local proxy draft (deterministic test stub).\n\nQuestion: {}\n\nPrompt fingerprint: fnv1a-{}\n\nDraft-only response with no authority execution.",
        bundle.user_message, fingerprint
    );
    truncate_to_byte_bound_utf8_nonempty(&draft, bound)
}

fn bundle_fingerprint(bundle: &ProxyPromptBundle) -> String {
    let material = format!(
        "{}\0{}\0{}\0{}",
        bundle.protocol_version, bundle.system_message, bundle.context_json, bundle.user_message
    );
    fnv1a_hex(&material)
}

fn fnv1a_hex(input: &str) -> String {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;
    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("{:016x}", hash)
}

fn truncate_to_byte_bound_utf8_nonempty(text: &str, max_bytes: usize) -> String {
    let max_bytes = max_bytes.max(1);
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        return text.chars().next().expect("non-empty draft").to_string();
    }
    let truncated = text[..end].to_string();
    if truncated.trim().is_empty() {
        text.chars().next().expect("non-empty draft").to_string()
    } else {
        truncated
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn truncate_preserves_utf8_char_boundaries() {
        let text = "กขค";
        let truncated = truncate_to_byte_bound_utf8_nonempty(text, 4);
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    }
}
