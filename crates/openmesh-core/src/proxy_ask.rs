//! Dev Track 0.1.6 Checkpoint D — provider-neutral Ask My Proxy composition service (pure).

use crate::context_pack_validation::validate_proxy_context_pack_complete;
use crate::domain::{
    validate_proxy_draft, validate_proxy_draft_runtime_metadata,
    validate_proxy_draft_trace_metadata, validate_proxy_question, validate_proxy_runtime_output,
    validate_proxy_runtime_request, ProxyContextPack, ProxyDraft, ProxyDraftEvidenceSummary,
    ProxyDraftRuntimeMetadata, ProxyDraftTraceMetadata, ProxyQuestion, ProxyRuntimeRequest,
    MAX_PROXY_DRAFT_LIMITATIONS, MAX_PROXY_DRAFT_TEXT_BYTES, PROXY_DRAFT_AUTHORITY_NOTICE,
    PROXY_DRAFT_CLASSIFICATION, PROXY_DRAFT_EXECUTION_BOUNDARY, PROXY_DRAFT_PROTOCOL_VERSION,
    PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION,
};
use crate::proxy_draft_safety::{
    filter_stale_runtime_limitations, validate_generated_draft_safety,
    validate_networked_runtime_consistency, PROXY_DRAFT_FIXED_LIMITATION,
};
use crate::proxy_prompt::compose_proxy_prompt;
use crate::proxy_prompt_context::{bound_proxy_prompt_context, map_pack_to_proxy_prompt_context};
use crate::proxy_runtime::{ProxyDraftRuntime, ProxyDraftRuntimeError};
use std::collections::{BTreeMap, BTreeSet};

/// Provider-neutral options for Ask My Proxy composition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyAskOptions {
    pub timeout_ms: u64,
    pub max_output_bytes: u32,
}

impl ProxyAskOptions {
    pub fn new(timeout_ms: u64, max_output_bytes: u32) -> Self {
        Self {
            timeout_ms,
            max_output_bytes,
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            timeout_ms: 60_000,
            max_output_bytes: MAX_PROXY_DRAFT_TEXT_BYTES as u32,
        }
    }
}

/// Injected UTC timestamp source for `ProxyDraft.generatedAt`.
pub trait ProxyDraftClock: Send + Sync {
    fn now_utc(&self) -> Result<String, ProxyDraftClockError>;
}

/// Production clock using repository UTC RFC3339 conventions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SystemProxyDraftClock;

impl ProxyDraftClock for SystemProxyDraftClock {
    fn now_utc(&self) -> Result<String, ProxyDraftClockError> {
        Ok(chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
    }
}

/// Fixed UTC timestamp for deterministic tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixedProxyDraftClock {
    timestamp: String,
}

impl FixedProxyDraftClock {
    pub fn new(timestamp: impl Into<String>) -> Self {
        Self {
            timestamp: timestamp.into(),
        }
    }
}

impl ProxyDraftClock for FixedProxyDraftClock {
    fn now_utc(&self) -> Result<String, ProxyDraftClockError> {
        Ok(self.timestamp.clone())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("proxy draft clock is unavailable")]
pub struct ProxyDraftClockError;

/// Typed, secret-safe Ask My Proxy composition errors.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyAskError {
    #[error("proxy ask options are invalid")]
    InvalidOptions,
    #[error("proxy question is invalid")]
    InvalidQuestion,
    #[error("proxy context pack is invalid")]
    InvalidContextPack,
    #[error("proxy prompt composition failed")]
    PromptCompositionFailed,
    #[error("proxy draft trace construction failed")]
    TraceConstructionFailed,
    #[error("proxy runtime request is invalid")]
    InvalidRuntimeRequest,
    #[error("proxy draft runtime is not configured")]
    RuntimeNotConfigured,
    #[error("proxy draft runtime timed out")]
    RuntimeTimeout,
    #[error("proxy draft runtime is unavailable")]
    RuntimeUnavailable,
    #[error("proxy draft runtime provider failed")]
    ProviderFailure,
    #[error("proxy draft runtime produced invalid output")]
    InvalidRuntimeOutput,
    #[error("generated draft failed safety validation")]
    UnsafeDraft,
    #[error("proxy draft clock is unavailable")]
    ClockFailure,
    #[error("assembled proxy draft is invalid")]
    InvalidProxyDraft,
}

/// Build aggregate-only trace metadata from a validated context pack.
pub fn build_proxy_draft_trace_metadata(
    pack: &ProxyContextPack,
) -> Result<ProxyDraftTraceMetadata, ProxyAskError> {
    validate_proxy_context_pack_complete(pack).map_err(|_| ProxyAskError::InvalidContextPack)?;
    let evidence_summary = build_proxy_draft_evidence_summary(pack);
    let trace = ProxyDraftTraceMetadata {
        protocol_version: PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION.to_string(),
        workspace_id: pack.workspace_id.clone(),
        profile_id: pack.profile_id.clone(),
        profile_version: pack.profile_version.clone(),
        context_pack_id: pack.context_pack_id.clone(),
        build_inputs_hash: pack.build_inputs_hash.clone(),
        evidence_summary,
    };
    validate_proxy_draft_trace_metadata(&trace)
        .map_err(|_| ProxyAskError::TraceConstructionFailed)?;
    Ok(trace)
}

/// Compose a complete local proxy draft from supplied in-memory inputs.
pub fn ask_my_proxy_local(
    pack: &ProxyContextPack,
    question: &ProxyQuestion,
    options: &ProxyAskOptions,
    runtime: &dyn ProxyDraftRuntime,
    clock: &dyn ProxyDraftClock,
) -> Result<ProxyDraft, ProxyAskError> {
    validate_proxy_ask_options(options)?;
    validate_proxy_question(question).map_err(|_| ProxyAskError::InvalidQuestion)?;
    validate_proxy_context_pack_complete(pack).map_err(|_| ProxyAskError::InvalidContextPack)?;

    let trace = build_proxy_draft_trace_metadata(pack)?;
    let prompt_bundle =
        compose_proxy_prompt(pack, question).map_err(map_prompt_composition_error)?;
    let prompt_context = bound_proxy_prompt_context(
        map_pack_to_proxy_prompt_context(pack).map_err(map_prompt_composition_error)?,
    )
    .map_err(map_prompt_composition_error)?;

    let runtime_request = ProxyRuntimeRequest {
        prompt: prompt_bundle,
        timeout_ms: options.timeout_ms,
        max_output_bytes: options.max_output_bytes,
    };
    validate_proxy_runtime_request(&runtime_request)
        .map_err(|_| ProxyAskError::InvalidRuntimeRequest)?;

    let runtime_output = runtime
        .generate_draft(&runtime_request)
        .map_err(map_runtime_error)?;

    validate_proxy_runtime_output(&runtime_output)
        .map_err(|_| ProxyAskError::InvalidRuntimeOutput)?;

    if runtime_output.draft_text.len() > options.max_output_bytes as usize {
        return Err(ProxyAskError::InvalidRuntimeOutput);
    }

    validate_generated_draft_safety(&runtime_output.draft_text, &prompt_context.owner_label)
        .map_err(|_| ProxyAskError::UnsafeDraft)?;

    let limitations = build_proxy_draft_limitations(&prompt_context.limitations);

    if runtime_output.network_used {
        validate_networked_runtime_consistency(&runtime_output.draft_text, &limitations)
            .map_err(|_| ProxyAskError::UnsafeDraft)?;
    }

    let runtime_metadata = ProxyDraftRuntimeMetadata {
        runtime_kind: runtime.runtime_kind().to_string(),
        provider_id: runtime_output.provider_id,
        model_id: runtime_output.model_id,
        network_used: runtime_output.network_used,
        duration_ms: runtime_output.duration_ms,
    };
    validate_proxy_draft_runtime_metadata(&runtime_metadata)
        .map_err(|_| ProxyAskError::InvalidProxyDraft)?;

    let generated_at = clock.now_utc().map_err(|_| ProxyAskError::ClockFailure)?;

    let draft = ProxyDraft {
        protocol_version: PROXY_DRAFT_PROTOCOL_VERSION.to_string(),
        question_id: question.question_id.clone(),
        generated_at,
        classification: PROXY_DRAFT_CLASSIFICATION.to_string(),
        draft_text: runtime_output.draft_text,
        authority_notice: PROXY_DRAFT_AUTHORITY_NOTICE.to_string(),
        execution_boundary: PROXY_DRAFT_EXECUTION_BOUNDARY.to_string(),
        trace,
        runtime: runtime_metadata,
        limitations,
    };

    validate_proxy_draft(&draft).map_err(|_| ProxyAskError::InvalidProxyDraft)?;
    Ok(draft)
}

fn validate_proxy_ask_options(options: &ProxyAskOptions) -> Result<(), ProxyAskError> {
    if options.timeout_ms == 0 || options.max_output_bytes == 0 {
        return Err(ProxyAskError::InvalidOptions);
    }
    if options.max_output_bytes as usize > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyAskError::InvalidOptions);
    }
    Ok(())
}

fn map_prompt_composition_error(
    err: crate::proxy_prompt_context::ProxyPromptError,
) -> ProxyAskError {
    match err {
        crate::proxy_prompt_context::ProxyPromptError::InvalidQuestion => {
            ProxyAskError::InvalidQuestion
        }
        crate::proxy_prompt_context::ProxyPromptError::InvalidContextPack => {
            ProxyAskError::InvalidContextPack
        }
        _ => ProxyAskError::PromptCompositionFailed,
    }
}

fn map_runtime_error(err: ProxyDraftRuntimeError) -> ProxyAskError {
    match err {
        ProxyDraftRuntimeError::InvalidRequest => ProxyAskError::InvalidRuntimeRequest,
        ProxyDraftRuntimeError::RuntimeNotConfigured => ProxyAskError::RuntimeNotConfigured,
        ProxyDraftRuntimeError::Timeout => ProxyAskError::RuntimeTimeout,
        ProxyDraftRuntimeError::RuntimeUnavailable => ProxyAskError::RuntimeUnavailable,
        ProxyDraftRuntimeError::ProviderFailure => ProxyAskError::ProviderFailure,
        ProxyDraftRuntimeError::InvalidOutput => ProxyAskError::InvalidRuntimeOutput,
    }
}

fn build_proxy_draft_evidence_summary(pack: &ProxyContextPack) -> ProxyDraftEvidenceSummary {
    let mut source_counts = BTreeMap::new();
    let continuity_count = count_pack_continuity_items(pack);
    if continuity_count > 0 {
        source_counts.insert("continuityItem".into(), continuity_count);
    }
    let pending_count = pack.current_state.pending_attention.len() as u32
        + pack.catch_up.next_suggested_attention.len() as u32;
    if pending_count > 0 {
        source_counts.insert("pendingAttention".into(), pending_count);
    }

    ProxyDraftEvidenceSummary {
        evidence_index_count: pack.evidence_index.len() as u32,
        source_counts,
        secret_items_omitted: pack.redaction_summary.secret_items_omitted,
    }
}

fn count_pack_continuity_items(pack: &ProxyContextPack) -> u32 {
    let mut count = 0u32;
    count += pack.current_state.sections.completed.len() as u32;
    count += pack.current_state.sections.in_progress.len() as u32;
    count += pack.current_state.sections.blocked.len() as u32;
    count += pack.current_state.sections.decisions.len() as u32;
    count += pack.current_state.sections.needs_attention.len() as u32;
    count += pack.current_state.sections.still_open.len() as u32;
    count += pack.catch_up.sections.completed.len() as u32;
    count += pack.catch_up.sections.changed.len() as u32;
    count += pack.catch_up.sections.blocked.len() as u32;
    count += pack.catch_up.sections.decided.len() as u32;
    count += pack.catch_up.sections.needs_attention.len() as u32;
    count += pack.catch_up.sections.still_open.len() as u32;
    count
}

fn build_proxy_draft_limitations(prompt_limitations: &[String]) -> Vec<String> {
    let mut limitations = vec![PROXY_DRAFT_FIXED_LIMITATION.to_string()];
    let mut seen = BTreeSet::from([PROXY_DRAFT_FIXED_LIMITATION.to_string()]);
    for limitation in filter_stale_runtime_limitations(prompt_limitations) {
        if limitations.len() >= MAX_PROXY_DRAFT_LIMITATIONS {
            break;
        }
        if seen.insert(limitation.clone()) {
            limitations.push(limitation.clone());
        }
    }
    limitations
}
