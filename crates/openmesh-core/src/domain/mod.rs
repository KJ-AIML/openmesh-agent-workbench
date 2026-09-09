// ============================================================================
// OpenMesh Work Continuity Domain Contracts — Dev Track 0.1.3.1
// ============================================================================
// Minimum ownership-boundary contracts only. No serialization schema frozen,
// no persistence, no promotion logic. See:
//   .heli-harness/state/reports/openmesh-0.1.3.1-execution-plan.md, section 5.
//
// What this module deliberately did NOT introduce in 0.1.3.1 (Category B):
//   CurrentStateProjection and PendingAttention were deferred to 0.1.3.7.
// Dev Track 0.1.3.6 Checkpoint A — `EvidenceRef::GitState` (WEC-33) and WorkSignal
// protocol `1.1` compatibility for Git evidence producers.
// Dev Track 0.1.3.7 Checkpoint A — `CurrentStateProjection`, `PendingAttentionItem`,
// and `CatchUpView` wire contracts (pure types + validation; no I/O).
//
// Dev Track 0.1.3.4 Checkpoint A hardens the serializable WorkEvent wire shape
// and EvidenceAttachment model. Ledger persistence is Checkpoint B.
// ============================================================================

pub mod context_pack;
pub mod corrections;
pub mod events;
pub mod profile;
pub mod projections;
pub mod signals;

pub use context_pack::*;
pub use corrections::*;
pub use events::*;
pub use profile::*;
pub use projections::*;
pub use signals::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
// ============================================================================
// Dev Track 0.1.6 Checkpoint A — proxy draft domain and wire contracts (pure)
// ============================================================================
// Structural validators only. No prompt composition, runtime adapters, CLI,
// persistence, or authority execution in this checkpoint.

/// Wire-schema version for `ProxyQuestion`.
pub const PROXY_QUESTION_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyDraft`.
pub const PROXY_DRAFT_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyDraftTraceMetadata`.
pub const PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION: &str = "1.0";

/// Wire-schema version for `ProxyPromptBundle`.
pub const PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION: &str = "1.0";

/// Frozen `ProxyDraft.classification` wire value.
pub const PROXY_DRAFT_CLASSIFICATION: &str = "local-proxy-draft";

/// Frozen `ProxyDraft.authorityNotice` wire value.
pub const PROXY_DRAFT_AUTHORITY_NOTICE: &str =
    "Policy metadata only — no authority decision was executed.";

/// Frozen `ProxyDraft.executionBoundary` wire value.
pub const PROXY_DRAFT_EXECUTION_BOUNDARY: &str = "draft-only; no authority execution in 0.1.6";

/// Frozen bound: `ProxyQuestion.text` maximum UTF-8 byte length.
pub const MAX_PROXY_QUESTION_TEXT_BYTES: usize = 2048;

/// Frozen bound: `ProxyDraft.draftText` maximum UTF-8 byte length.
pub const MAX_PROXY_DRAFT_TEXT_BYTES: usize = 8192;

/// Frozen bound: `ProxyDraft.limitations` entry count.
pub const MAX_PROXY_DRAFT_LIMITATIONS: usize = 32;

pub const MAX_PROXY_QUESTION_ID_BYTES: usize = 256;
pub const MAX_PROXY_PROMPT_FIELD_BYTES: usize = 65_536;
pub const MAX_PROXY_RUNTIME_KIND_BYTES: usize = 64;
pub const MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES: usize = 64;
pub const MAX_PROXY_RUNTIME_MODEL_ID_BYTES: usize = 128;
pub const MAX_PROXY_TRACE_WORKSPACE_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_PROFILE_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_PROFILE_VERSION_BYTES: usize = 64;
pub const MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES: usize = MAX_CONTEXT_PACK_ID_BYTES;
pub const MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES: usize = MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES;
pub const MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES: usize = 64;
pub const MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES: usize = 32;
pub const MAX_PROXY_DRAFT_LIMITATION_BYTES: usize = MAX_LIMITATION_BYTES;

/// Returns true when `version` is a supported Proxy Question protocol.
pub fn is_supported_proxy_question_protocol(version: &str) -> bool {
    version == PROXY_QUESTION_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Draft protocol.
pub fn is_supported_proxy_draft_protocol(version: &str) -> bool {
    version == PROXY_DRAFT_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Draft trace metadata protocol.
pub fn is_supported_proxy_draft_trace_metadata_protocol(version: &str) -> bool {
    version == PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION
}

/// Returns true when `version` is a supported Proxy Prompt Bundle protocol.
pub fn is_supported_proxy_prompt_bundle_protocol(version: &str) -> bool {
    version == PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION
}

/// Ask-my-proxy question wire contract (identity generation deferred to Checkpoint B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyQuestion {
    pub protocol_version: String,
    pub question_id: String,
    pub text: String,
}

/// Immutable prompt bundle referenced by runtime requests (composition deferred to Checkpoint B).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptBundle {
    pub protocol_version: String,
    pub system_message: String,
    pub context_json: String,
    pub user_message: String,
}

/// Provider-neutral runtime request contract (invocation deferred to Checkpoint C).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyRuntimeRequest {
    pub prompt: ProxyPromptBundle,
    pub timeout_ms: u64,
    pub max_output_bytes: u32,
}

/// Runtime-owned draft output (safety validation deferred to Checkpoint D).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyRuntimeOutput {
    pub draft_text: String,
    pub provider_id: String,
    pub model_id: String,
    pub network_used: bool,
    pub duration_ms: u64,
}

/// Aggregate-only evidence summary under `ProxyDraft.trace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftEvidenceSummary {
    pub evidence_index_count: u32,
    pub source_counts: BTreeMap<String, u32>,
    pub secret_items_omitted: u32,
}

/// OpenMesh-owned trace metadata nested under `ProxyDraft.trace`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftTraceMetadata {
    pub protocol_version: String,
    pub workspace_id: String,
    pub profile_id: String,
    pub profile_version: String,
    pub context_pack_id: String,
    pub build_inputs_hash: String,
    pub evidence_summary: ProxyDraftEvidenceSummary,
}

/// Runtime metadata shell included in `ProxyDraft`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraftRuntimeMetadata {
    pub runtime_kind: String,
    pub provider_id: String,
    pub model_id: String,
    pub network_used: bool,
    pub duration_ms: u64,
}

/// Canonical proxy draft response wire contract v1.0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyDraft {
    pub protocol_version: String,
    pub question_id: String,
    pub generated_at: String,
    pub classification: String,
    pub draft_text: String,
    pub authority_notice: String,
    pub execution_boundary: String,
    pub trace: ProxyDraftTraceMetadata,
    pub runtime: ProxyDraftRuntimeMetadata,
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyQuestionValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("question_id is empty after trim")]
    EmptyQuestionId,
    #[error("question_id exceeds the {max}-byte bound")]
    QuestionIdTooLong { max: usize },
    #[error("question_id format is invalid: {0}")]
    InvalidQuestionId(String),
    #[error("text is empty after normalization")]
    EmptyText,
    #[error("text exceeds the {max}-byte bound after normalization")]
    TextTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyPromptBundleValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("system_message is empty after trim")]
    EmptySystemMessage,
    #[error("user_message is empty after trim")]
    EmptyUserMessage,
    #[error("context_json is empty after trim")]
    EmptyContextJson,
    #[error("context_json is not valid JSON: {0}")]
    InvalidContextJson(String),
    #[error("prompt field exceeds the {max}-byte bound")]
    FieldTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyRuntimeRequestValidationError {
    #[error("timeout_ms must be greater than zero")]
    ZeroTimeout,
    #[error("max_output_bytes must be greater than zero")]
    ZeroMaxOutputBytes,
    #[error("max_output_bytes exceeds the {max}-byte bound")]
    MaxOutputBytesTooLarge { max: usize },
    #[error("prompt bundle is invalid: {0}")]
    InvalidPromptBundle(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyRuntimeOutputValidationError {
    #[error("draft_text is empty after trim")]
    EmptyDraftText,
    #[error("draft_text exceeds the {max}-byte bound")]
    DraftTextTooLong { max: usize },
    #[error("provider_id is empty after trim")]
    EmptyProviderId,
    #[error("model_id is empty after trim")]
    EmptyModelId,
    #[error("provider_id exceeds the {max}-byte bound")]
    ProviderIdTooLong { max: usize },
    #[error("model_id exceeds the {max}-byte bound")]
    ModelIdTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftEvidenceSummaryValidationError {
    #[error("source_counts exceed the {max}-entry bound")]
    TooManySourceCategories { max: usize },
    #[error("source_counts key is empty after trim")]
    EmptySourceCategory,
    #[error("source_counts key exceeds the {max}-byte bound")]
    SourceCategoryTooLong { max: usize },
    #[error("source_counts key is path-like or canonical-ref-like: {0}")]
    InvalidSourceCategory(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftTraceMetadataValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("profile_id is empty after trim")]
    EmptyProfileId,
    #[error("profile_version is empty after trim")]
    EmptyProfileVersion,
    #[error("context_pack_id is empty after trim")]
    EmptyContextPackId,
    #[error("build_inputs_hash is empty after trim")]
    EmptyBuildInputsHash,
    #[error("workspace_id exceeds the {max}-byte bound")]
    WorkspaceIdTooLong { max: usize },
    #[error("profile_id exceeds the {max}-byte bound")]
    ProfileIdTooLong { max: usize },
    #[error("profile_version exceeds the {max}-byte bound")]
    ProfileVersionTooLong { max: usize },
    #[error("context_pack_id exceeds the {max}-byte bound")]
    ContextPackIdTooLong { max: usize },
    #[error("build_inputs_hash exceeds the {max}-byte bound")]
    BuildInputsHashTooLong { max: usize },
    #[error("context_pack_id must start with context-pack-")]
    InvalidContextPackIdPrefix,
    #[error("evidence_summary is invalid: {0}")]
    InvalidEvidenceSummary(String),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftRuntimeMetadataValidationError {
    #[error("runtime_kind is empty after trim")]
    EmptyRuntimeKind,
    #[error("provider_id is empty after trim")]
    EmptyProviderId,
    #[error("model_id is empty after trim")]
    EmptyModelId,
    #[error("runtime_kind exceeds the {max}-byte bound")]
    RuntimeKindTooLong { max: usize },
    #[error("provider_id exceeds the {max}-byte bound")]
    ProviderIdTooLong { max: usize },
    #[error("model_id exceeds the {max}-byte bound")]
    ModelIdTooLong { max: usize },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyDraftValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("question_id is invalid: {0}")]
    InvalidQuestionId(String),
    #[error("generated_at is invalid: {0}")]
    InvalidGeneratedAt(String),
    #[error("classification must be the frozen local-proxy-draft value")]
    InvalidClassification,
    #[error("authority_notice must be the frozen policy metadata notice")]
    InvalidAuthorityNotice,
    #[error("execution_boundary must be the frozen 0.1.6 draft-only boundary")]
    InvalidExecutionBoundary,
    #[error("draft_text is empty after trim")]
    EmptyDraftText,
    #[error("draft_text exceeds the {max}-byte bound")]
    DraftTextTooLong { max: usize },
    #[error("trace metadata is invalid: {0}")]
    InvalidTraceMetadata(String),
    #[error("runtime metadata is invalid: {0}")]
    InvalidRuntimeMetadata(String),
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation is empty after trim")]
    EmptyLimitation,
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
}

/// Normalize question text for validation (trim only; not serialized separately).
pub fn normalize_proxy_question_text(text: &str) -> String {
    text.trim().to_string()
}

/// Validate frozen `proxy-q-<nanos_hex>-<pid_hex>-<counter_hex>` question identity format.
pub fn validate_proxy_question_id(question_id: &str) -> Result<(), ProxyQuestionValidationError> {
    if question_id.trim().is_empty() {
        return Err(ProxyQuestionValidationError::EmptyQuestionId);
    }
    if question_id.len() > MAX_PROXY_QUESTION_ID_BYTES {
        return Err(ProxyQuestionValidationError::QuestionIdTooLong {
            max: MAX_PROXY_QUESTION_ID_BYTES,
        });
    }
    if question_id.chars().any(char::is_whitespace) {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must not contain whitespace".into(),
        ));
    }
    for forbidden in ['/', '\\', ':', '%'] {
        if question_id.contains(forbidden) {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(format!(
                "question_id must not contain '{forbidden}'"
            )));
        }
    }
    const PREFIX: &str = "proxy-q-";
    if !question_id.starts_with(PREFIX) {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must start with proxy-q-".into(),
        ));
    }
    let remainder = &question_id[PREFIX.len()..];
    let segments: Vec<&str> = remainder.split('-').collect();
    if segments.len() != 3 {
        return Err(ProxyQuestionValidationError::InvalidQuestionId(
            "question_id must contain exactly three lowercase hexadecimal segments after proxy-q-"
                .into(),
        ));
    }
    for segment in segments {
        if segment.is_empty() {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(
                "question_id segments must be non-empty".into(),
            ));
        }
        if !segment
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, 'a'..='f'))
        {
            return Err(ProxyQuestionValidationError::InvalidQuestionId(
                "question_id segments must be lowercase hexadecimal".into(),
            ));
        }
    }
    Ok(())
}

/// Structural validation for `ProxyQuestion` (pure, no I/O).
pub fn validate_proxy_question(
    question: &ProxyQuestion,
) -> Result<(), ProxyQuestionValidationError> {
    if !is_supported_proxy_question_protocol(&question.protocol_version) {
        return Err(ProxyQuestionValidationError::UnsupportedProtocolVersion {
            found: question.protocol_version.clone(),
            expected: PROXY_QUESTION_PROTOCOL_VERSION,
        });
    }
    validate_proxy_question_id(&question.question_id)?;
    let normalized = normalize_proxy_question_text(&question.text);
    if normalized.is_empty() {
        return Err(ProxyQuestionValidationError::EmptyText);
    }
    if normalized.len() > MAX_PROXY_QUESTION_TEXT_BYTES {
        return Err(ProxyQuestionValidationError::TextTooLong {
            max: MAX_PROXY_QUESTION_TEXT_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyPromptBundle` (pure, no I/O).
pub fn validate_proxy_prompt_bundle(
    bundle: &ProxyPromptBundle,
) -> Result<(), ProxyPromptBundleValidationError> {
    if !is_supported_proxy_prompt_bundle_protocol(&bundle.protocol_version) {
        return Err(
            ProxyPromptBundleValidationError::UnsupportedProtocolVersion {
                found: bundle.protocol_version.clone(),
                expected: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
            },
        );
    }
    validate_proxy_prompt_field(
        &bundle.system_message,
        ProxyPromptBundleValidationError::EmptySystemMessage,
    )?;
    validate_proxy_prompt_field(
        &bundle.user_message,
        ProxyPromptBundleValidationError::EmptyUserMessage,
    )?;
    if bundle.context_json.trim().is_empty() {
        return Err(ProxyPromptBundleValidationError::EmptyContextJson);
    }
    if bundle.context_json.len() > MAX_PROXY_PROMPT_FIELD_BYTES {
        return Err(ProxyPromptBundleValidationError::FieldTooLong {
            max: MAX_PROXY_PROMPT_FIELD_BYTES,
        });
    }
    serde_json::from_str::<serde_json::Value>(&bundle.context_json)
        .map_err(|err| ProxyPromptBundleValidationError::InvalidContextJson(err.to_string()))?;
    Ok(())
}

/// Structural validation for `ProxyRuntimeRequest` (pure, no I/O).
pub fn validate_proxy_runtime_request(
    request: &ProxyRuntimeRequest,
) -> Result<(), ProxyRuntimeRequestValidationError> {
    validate_proxy_prompt_bundle(&request.prompt)
        .map_err(|err| ProxyRuntimeRequestValidationError::InvalidPromptBundle(err.to_string()))?;
    if request.timeout_ms == 0 {
        return Err(ProxyRuntimeRequestValidationError::ZeroTimeout);
    }
    if request.max_output_bytes == 0 {
        return Err(ProxyRuntimeRequestValidationError::ZeroMaxOutputBytes);
    }
    if request.max_output_bytes as usize > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyRuntimeRequestValidationError::MaxOutputBytesTooLarge {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyRuntimeOutput` (pure, no I/O).
pub fn validate_proxy_runtime_output(
    output: &ProxyRuntimeOutput,
) -> Result<(), ProxyRuntimeOutputValidationError> {
    if output.draft_text.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyDraftText);
    }
    if output.draft_text.len() > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyRuntimeOutputValidationError::DraftTextTooLong {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    if output.provider_id.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyProviderId);
    }
    if output.model_id.trim().is_empty() {
        return Err(ProxyRuntimeOutputValidationError::EmptyModelId);
    }
    if output.provider_id.len() > MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES {
        return Err(ProxyRuntimeOutputValidationError::ProviderIdTooLong {
            max: MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES,
        });
    }
    if output.model_id.len() > MAX_PROXY_RUNTIME_MODEL_ID_BYTES {
        return Err(ProxyRuntimeOutputValidationError::ModelIdTooLong {
            max: MAX_PROXY_RUNTIME_MODEL_ID_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for aggregate-only `ProxyDraftEvidenceSummary` (pure, no I/O).
pub fn validate_proxy_draft_evidence_summary(
    summary: &ProxyDraftEvidenceSummary,
) -> Result<(), ProxyDraftEvidenceSummaryValidationError> {
    if summary.source_counts.len() > MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::TooManySourceCategories {
                max: MAX_PROXY_EVIDENCE_SOURCE_CATEGORIES,
            },
        );
    }
    for key in summary.source_counts.keys() {
        validate_proxy_evidence_source_category_key(key)?;
    }
    let _ = (summary.evidence_index_count, summary.secret_items_omitted);
    Ok(())
}

/// Structural validation for `ProxyDraftTraceMetadata` (pure, no I/O).
pub fn validate_proxy_draft_trace_metadata(
    trace: &ProxyDraftTraceMetadata,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if !is_supported_proxy_draft_trace_metadata_protocol(&trace.protocol_version) {
        return Err(
            ProxyDraftTraceMetadataValidationError::UnsupportedProtocolVersion {
                found: trace.protocol_version.clone(),
                expected: PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION,
            },
        );
    }
    validate_proxy_trace_workspace_id(&trace.workspace_id)?;
    validate_proxy_trace_profile_id(&trace.profile_id)?;
    validate_proxy_trace_profile_version(&trace.profile_version)?;
    validate_proxy_trace_context_pack_id(&trace.context_pack_id)?;
    validate_proxy_trace_build_inputs_hash(&trace.build_inputs_hash)?;
    validate_proxy_draft_evidence_summary(&trace.evidence_summary).map_err(|err| {
        ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(err.to_string())
    })?;
    Ok(())
}

/// Structural validation for `ProxyDraftRuntimeMetadata` (pure, no I/O).
pub fn validate_proxy_draft_runtime_metadata(
    runtime: &ProxyDraftRuntimeMetadata,
) -> Result<(), ProxyDraftRuntimeMetadataValidationError> {
    if runtime.runtime_kind.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyRuntimeKind);
    }
    if runtime.provider_id.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyProviderId);
    }
    if runtime.model_id.trim().is_empty() {
        return Err(ProxyDraftRuntimeMetadataValidationError::EmptyModelId);
    }
    if runtime.runtime_kind.len() > MAX_PROXY_RUNTIME_KIND_BYTES {
        return Err(
            ProxyDraftRuntimeMetadataValidationError::RuntimeKindTooLong {
                max: MAX_PROXY_RUNTIME_KIND_BYTES,
            },
        );
    }
    if runtime.provider_id.len() > MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES {
        return Err(
            ProxyDraftRuntimeMetadataValidationError::ProviderIdTooLong {
                max: MAX_PROXY_RUNTIME_PROVIDER_ID_BYTES,
            },
        );
    }
    if runtime.model_id.len() > MAX_PROXY_RUNTIME_MODEL_ID_BYTES {
        return Err(ProxyDraftRuntimeMetadataValidationError::ModelIdTooLong {
            max: MAX_PROXY_RUNTIME_MODEL_ID_BYTES,
        });
    }
    Ok(())
}

/// Structural validation for `ProxyDraft` v1.0 (pure, no I/O).
pub fn validate_proxy_draft(draft: &ProxyDraft) -> Result<(), ProxyDraftValidationError> {
    if !is_supported_proxy_draft_protocol(&draft.protocol_version) {
        return Err(ProxyDraftValidationError::UnsupportedProtocolVersion {
            found: draft.protocol_version.clone(),
            expected: PROXY_DRAFT_PROTOCOL_VERSION,
        });
    }
    validate_proxy_question_id(&draft.question_id)
        .map_err(|err| ProxyDraftValidationError::InvalidQuestionId(err.to_string()))?;
    validate_utc_timestamp(&draft.generated_at)
        .map_err(ProxyDraftValidationError::InvalidGeneratedAt)?;
    if draft.classification != PROXY_DRAFT_CLASSIFICATION {
        return Err(ProxyDraftValidationError::InvalidClassification);
    }
    if draft.authority_notice != PROXY_DRAFT_AUTHORITY_NOTICE {
        return Err(ProxyDraftValidationError::InvalidAuthorityNotice);
    }
    if draft.execution_boundary != PROXY_DRAFT_EXECUTION_BOUNDARY {
        return Err(ProxyDraftValidationError::InvalidExecutionBoundary);
    }
    if draft.draft_text.trim().is_empty() {
        return Err(ProxyDraftValidationError::EmptyDraftText);
    }
    if draft.draft_text.len() > MAX_PROXY_DRAFT_TEXT_BYTES {
        return Err(ProxyDraftValidationError::DraftTextTooLong {
            max: MAX_PROXY_DRAFT_TEXT_BYTES,
        });
    }
    validate_proxy_draft_trace_metadata(&draft.trace)
        .map_err(|err| ProxyDraftValidationError::InvalidTraceMetadata(err.to_string()))?;
    validate_proxy_draft_runtime_metadata(&draft.runtime)
        .map_err(|err| ProxyDraftValidationError::InvalidRuntimeMetadata(err.to_string()))?;
    if draft.limitations.len() > MAX_PROXY_DRAFT_LIMITATIONS {
        return Err(ProxyDraftValidationError::TooManyLimitations {
            max: MAX_PROXY_DRAFT_LIMITATIONS,
        });
    }
    for limitation in &draft.limitations {
        if limitation.trim().is_empty() {
            return Err(ProxyDraftValidationError::EmptyLimitation);
        }
        if limitation.len() > MAX_PROXY_DRAFT_LIMITATION_BYTES {
            return Err(ProxyDraftValidationError::LimitationTooLong {
                max: MAX_PROXY_DRAFT_LIMITATION_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_proxy_prompt_field(
    value: &str,
    empty_error: ProxyPromptBundleValidationError,
) -> Result<(), ProxyPromptBundleValidationError> {
    if value.trim().is_empty() {
        return Err(empty_error);
    }
    if value.len() > MAX_PROXY_PROMPT_FIELD_BYTES {
        return Err(ProxyPromptBundleValidationError::FieldTooLong {
            max: MAX_PROXY_PROMPT_FIELD_BYTES,
        });
    }
    Ok(())
}

fn validate_proxy_evidence_source_category_key(
    key: &str,
) -> Result<(), ProxyDraftEvidenceSummaryValidationError> {
    if key.trim().is_empty() {
        return Err(ProxyDraftEvidenceSummaryValidationError::EmptySourceCategory);
    }
    if key.len() > MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::SourceCategoryTooLong {
                max: MAX_PROXY_EVIDENCE_SOURCE_CATEGORY_BYTES,
            },
        );
    }
    if key.contains('/') || key.contains('\\') {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::InvalidSourceCategory(
                "source category must not contain path separators".into(),
            ),
        );
    }
    if looks_like_canonical_ref_key(key) {
        return Err(
            ProxyDraftEvidenceSummaryValidationError::InvalidSourceCategory(
                "source category must not look like a canonical ref".into(),
            ),
        );
    }
    Ok(())
}

fn looks_like_canonical_ref_key(key: &str) -> bool {
    let lowered = key.to_ascii_lowercase();
    lowered.starts_with("openmesh://")
        || lowered.contains("://")
        || lowered.starts_with("context-pack-")
        || lowered.starts_with("proxy-q-")
        || lowered.starts_with("evt-")
        || lowered.starts_with("ref-")
}

fn validate_proxy_trace_workspace_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyWorkspaceId);
    }
    if value.len() > MAX_PROXY_TRACE_WORKSPACE_ID_BYTES {
        return Err(ProxyDraftTraceMetadataValidationError::WorkspaceIdTooLong {
            max: MAX_PROXY_TRACE_WORKSPACE_ID_BYTES,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "workspace_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_profile_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyProfileId);
    }
    if value.len() > MAX_PROXY_TRACE_PROFILE_ID_BYTES {
        return Err(ProxyDraftTraceMetadataValidationError::ProfileIdTooLong {
            max: MAX_PROXY_TRACE_PROFILE_ID_BYTES,
        });
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "profile_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_profile_version(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyProfileVersion);
    }
    if value.len() > MAX_PROXY_TRACE_PROFILE_VERSION_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::ProfileVersionTooLong {
                max: MAX_PROXY_TRACE_PROFILE_VERSION_BYTES,
            },
        );
    }
    Ok(())
}

fn validate_proxy_trace_context_pack_id(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyContextPackId);
    }
    if value.len() > MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::ContextPackIdTooLong {
                max: MAX_PROXY_TRACE_CONTEXT_PACK_ID_BYTES,
            },
        );
    }
    if !value.starts_with("context-pack-") {
        return Err(ProxyDraftTraceMetadataValidationError::InvalidContextPackIdPrefix);
    }
    if value.contains('/') || value.contains('\\') {
        return Err(
            ProxyDraftTraceMetadataValidationError::InvalidEvidenceSummary(
                "context_pack_id must not contain path separators".into(),
            ),
        );
    }
    Ok(())
}

fn validate_proxy_trace_build_inputs_hash(
    value: &str,
) -> Result<(), ProxyDraftTraceMetadataValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyDraftTraceMetadataValidationError::EmptyBuildInputsHash);
    }
    if value.len() > MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES {
        return Err(
            ProxyDraftTraceMetadataValidationError::BuildInputsHashTooLong {
                max: MAX_PROXY_TRACE_BUILD_INPUTS_HASH_BYTES,
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(
        id: &str,
        producer: ProducerRef,
        actor: ActorRef,
        kind: WorkSignalKind,
        correlation_hint: Option<&str>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> WorkSignal {
        WorkSignal {
            signal_id: id.to_string(),
            workspace_id: "ws-1".to_string(),
            producer,
            actor,
            kind,
            summary: format!("signal {id}"),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            evidence_refs,
            correlation_hint: correlation_hint.map(|s| s.to_string()),
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
        }
    }

    fn attachment(evidence_ref: EvidenceRef) -> EvidenceAttachment {
        EvidenceAttachment {
            evidence_ref,
            observed_at: None,
        }
    }

    fn minimal_event(
        kind: &str,
        summary: &str,
        evidence_refs: Vec<EvidenceRef>,
        timestamp: &str,
    ) -> WorkEvent {
        WorkEvent::new(
            "evt-test-1",
            "ws-1",
            kind,
            summary,
            evidence_refs.into_iter().map(attachment).collect(),
            timestamp,
        )
    }

    // Category A — Positive Representability Tests (execution plan §6/§9.D1).

    /// WEC-29: an agent's completion claim, a verification result, and a
    /// human's acceptance exist as three separately-attributed records linked
    /// by a shared correlation hint — not one record with a boolean flag.
    #[test]
    fn wec_29_claim_verification_and_human_acceptance_are_distinct_records() {
        let claim = signal(
            "s-claim",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![],
        );
        let verification = signal(
            "s-verify",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![EvidenceRef::FilePath("tests/output.log".into())],
        );
        let acceptance = signal(
            "s-accept",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            Some("corr-1"),
            vec![EvidenceRef::ProducerSignal("s-verify".into())],
        );

        // All three share the same correlation hint, so they're correlatable...
        assert_eq!(claim.correlation_hint, verification.correlation_hint);
        assert_eq!(verification.correlation_hint, acceptance.correlation_hint);
        // ...but remain three distinct records, not one record with a status flag.
        assert_ne!(claim.signal_id, verification.signal_id);
        assert_ne!(verification.signal_id, acceptance.signal_id);
        // Acceptance is distinctly human-attributed; the claim is not.
        assert!(matches!(acceptance.actor, ActorRef::Person(_)));
        assert!(!matches!(claim.actor, ActorRef::Person(_)));
    }

    /// CC-1: many signals may support one WorkEvent, and producer count does
    /// not determine event count — both a single event with several evidence
    /// refs and several single-evidence events must be constructible.
    #[test]
    fn cc_1_many_signals_can_support_one_event_or_several() {
        let refs: Vec<EvidenceRef> = (0..4)
            .map(|i| EvidenceRef::ProducerSignal(format!("s-{i}")))
            .collect();

        let one_event_many_refs = minimal_event(
            "bugfix",
            "clear_project fixed",
            refs.clone(),
            "2026-07-08T00:00:00Z",
        );
        assert_eq!(one_event_many_refs.evidence.len(), 4);

        let four_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| {
                minimal_event(
                    "bugfix",
                    "clear_project fixed",
                    vec![r],
                    "2026-07-08T00:00:00Z",
                )
            })
            .collect();
        assert_eq!(four_events.len(), 4);
        assert!(four_events.iter().all(|e| e.evidence.len() == 1));
    }

    /// WEC-26 + WEC-32: same-producer duplicate vs. cross-producer
    /// corroboration must be distinguishable data, not the same shape.
    #[test]
    fn wec_26_32_duplicate_vs_corroborating_signals_are_distinguishable() {
        let duplicate_a = signal(
            "s-dup-a",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        let duplicate_b = signal(
            "s-dup-b",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        assert_eq!(duplicate_a.producer, duplicate_b.producer);

        let corroborating_a = signal(
            "s-corr-a",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        let corroborating_b = signal(
            "s-corr-b",
            ProducerRef::Git,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        assert_ne!(corroborating_a.producer, corroborating_b.producer);
        assert_eq!(
            corroborating_a.correlation_hint,
            corroborating_b.correlation_hint
        );
    }

    /// WEC-15: a claim existing only in conversation, not yet backed by
    /// durable evidence, is representable as a WorkSignal with no evidence
    /// refs — distinct from (and never auto-promoted to) a WorkEvent.
    #[test]
    fn wec_15_conversation_only_claim_has_no_evidence_refs() {
        let conversation_only = signal(
            "s-convo",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            None,
            vec![],
        );
        assert!(conversation_only.evidence_refs.is_empty());
    }

    /// WEC-30: a genuinely ambiguous case (one event or six?) must remain
    /// representable either way — the contracts must not resolve ambiguity.
    #[test]
    fn wec_30_ambiguous_one_event_or_six_both_remain_constructible() {
        let refs: Vec<EvidenceRef> = (0..6)
            .map(|i| EvidenceRef::ProducerSignal(format!("round-{i}")))
            .collect();

        let one_event = minimal_event("stabilized", "0.1.2.5 stabilized", refs.clone(), "t");
        assert_eq!(one_event.evidence.len(), 6);

        let six_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| minimal_event("bugfix-round", "one closure round", vec![r], "t"))
            .collect();
        assert_eq!(six_events.len(), 6);
    }

    // Generic structural test (not a Classification Pack case). WEC-33
    // (Git-state evidence representability) is implemented in Dev Track 0.1.3.6
    // Checkpoint A — see `producer_contracts.rs`.
    #[test]
    fn evidence_ref_variants_construct_and_compare() {
        let a = EvidenceRef::FilePath("docs/overview.md".into());
        let b = EvidenceRef::FilePath("docs/overview.md".into());
        let c = EvidenceRef::ProducerSignal("s-1".into());
        let d = EvidenceRef::GitState(sample_git_state());
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(c, d);
    }

    fn sample_git_state() -> GitState {
        GitState {
            repo_id: "fnv1a-abc123".into(),
            branch: "main".into(),
            head: "2ad3a48b04b15c64b82e2bc7c1db36b41503c571".into(),
            dirty: true,
            staged_count: 0,
            unstaged_count: 1,
            untracked_count: 0,
            changed_paths: vec!["crates/openmesh-core/src/domain.rs".into()],
            observed_at: "2026-07-16T04:30:00Z".into(),
            ahead: None,
            behind: None,
            base_ref: None,
            worktree_root: None,
        }
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.2, Checkpoint A — protocol contract stabilization.
    // ------------------------------------------------------------------

    /// Full WorkSignal round-trips through JSON, preserving every field.
    #[test]
    fn work_signal_round_trips_through_json() {
        let original = signal(
            "s-roundtrip",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Person("ter".into()),
            WorkSignalKind::Handoff,
            Some("corr-rt"),
            vec![
                EvidenceRef::FilePath("docs/overview.md".into()),
                EvidenceRef::ProducerSignal("s-other".into()),
            ],
        );
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: WorkSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.signal_id, original.signal_id);
        assert_eq!(restored.workspace_id, original.workspace_id);
        assert_eq!(restored.producer, original.producer);
        assert_eq!(restored.actor, original.actor);
        assert_eq!(restored.kind, original.kind);
        assert_eq!(restored.summary, original.summary);
        assert_eq!(restored.timestamp, original.timestamp);
        assert_eq!(restored.evidence_refs, original.evidence_refs);
        assert_eq!(restored.correlation_hint, original.correlation_hint);
        assert_eq!(restored.sensitivity, original.sensitivity);
        assert_eq!(restored.protocol_version, original.protocol_version);
    }

    /// Every data-carrying enum variant round-trips, including all unit variants.
    #[test]
    fn producer_ref_variants_round_trip() {
        for p in [
            ProducerRef::Native,
            ProducerRef::Heli,
            ProducerRef::Git,
            ProducerRef::Reporter("codex".into()),
        ] {
            let json = serde_json::to_string(&p).expect("serialize");
            let back: ProducerRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, p);
        }
    }

    #[test]
    fn actor_ref_variants_round_trip() {
        for a in [
            ActorRef::Person("ter".into()),
            ActorRef::Device("laptop-1".into()),
            ActorRef::Proxy("proxy-1".into()),
            ActorRef::Unknown,
        ] {
            let json = serde_json::to_string(&a).expect("serialize");
            let back: ActorRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, a);
        }
    }

    #[test]
    fn evidence_ref_variants_round_trip() {
        for e in [
            EvidenceRef::FilePath("docs/overview.md".into()),
            EvidenceRef::ProducerSignal("s-1".into()),
            EvidenceRef::GitState(sample_git_state()),
        ] {
            let json = serde_json::to_string(&e).expect("serialize");
            let back: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, e);
        }
    }

    #[test]
    fn work_signal_kind_variants_round_trip() {
        for k in [
            WorkSignalKind::Progress,
            WorkSignalKind::Decision,
            WorkSignalKind::Blocker,
            WorkSignalKind::BlockerResolved,
            WorkSignalKind::ScopeChange,
            WorkSignalKind::Milestone,
            WorkSignalKind::ReviewRequired,
            WorkSignalKind::UnresolvedQuestion,
            WorkSignalKind::Handoff,
            WorkSignalKind::SessionEnd,
            WorkSignalKind::AgentSwitch,
        ] {
            let json = serde_json::to_string(&k).expect("serialize");
            let back: WorkSignalKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, k);
        }
    }

    /// Locks the exact wire shape approved in the 0.1.3.2 execution plan §3.12:
    /// camelCase fields, kebab-case kind, lowercase sensitivity, adjacently
    /// tagged data-carrying enums. Any accidental future drift in field
    /// naming, casing, or enum representation fails this test immediately.
    #[test]
    fn deserializes_the_canonical_example_json_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!("{}/tests/fixtures/signals/valid.json", manifest_dir);
        let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
        let s: WorkSignal = serde_json::from_str(&json).expect("deserialize fixture");

        assert_eq!(s.signal_id, "codex-2026-07-08-001");
        assert_eq!(s.workspace_id, "1720400000000-a1b2c3");
        assert_eq!(s.producer, ProducerRef::Reporter("codex".into()));
        assert_eq!(s.actor, ActorRef::Unknown);
        assert_eq!(s.kind, WorkSignalKind::Progress);
        assert_eq!(
            s.summary,
            "Implemented the shared continuity foundation and migrated context/index/ingestion into openmesh-core."
        );
        assert_eq!(s.timestamp, "2026-07-08T09:15:00Z");
        assert_eq!(
            s.evidence_refs,
            vec![EvidenceRef::FilePath(
                "crates/openmesh-core/src/domain.rs".into()
            )]
        );
        assert_eq!(s.correlation_hint, Some("corr-0.1.3.1".to_string()));
        assert_eq!(s.sensitivity, Sensitivity::Private);
        assert_eq!(s.protocol_version, "1.0");
        assert_eq!(WORK_SIGNAL_PROTOCOL_VERSION, "1.0");
    }

    /// A missing `sensitivity` field must deserialize as `Private` — proving
    /// the `#[serde(default)]` field attribute actually wires in the enum's
    /// own `#[default]`, not merely relying on it existing (approved plan §3.9).
    #[test]
    fn missing_sensitivity_field_defaults_to_private() {
        let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "progress",
            "summary": "no sensitivity field here",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "protocolVersion": "1.0"
        }"#;
        let s: WorkSignal = serde_json::from_str(json).expect("deserialize");
        assert_eq!(s.sensitivity, Sensitivity::Private);
    }

    /// An unrecognized `kind` string under the *current* protocol version is
    /// invalid current-version data, not a future-version compatibility case
    /// — it must fail strict deserialization (approved plan §3.4/§10),
    /// mirroring `context.rs`'s existing `deserialize_invalid_kind_fails`.
    /// This is the direct evidence motivating the one-field (not per-enum)
    /// preflight the classifier implements in Checkpoint C.
    #[test]
    fn unrecognized_kind_fails_strict_deserialize() {
        let json = r#"{
            "signalId": "s-1",
            "workspaceId": "ws-1",
            "producer": { "type": "native" },
            "actor": { "type": "unknown" },
            "kind": "not-a-real-kind",
            "summary": "bad kind",
            "timestamp": "2026-07-08T00:00:00Z",
            "evidenceRefs": [],
            "sensitivity": "private",
            "protocolVersion": "1.0"
        }"#;
        let result: Result<WorkSignal, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unrecognized kind should fail deserialization"
        );
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.4, Checkpoint A — WorkEvent wire contract.
    // ------------------------------------------------------------------

    fn sample_event() -> WorkEvent {
        WorkEvent {
            event_id: "1783605120049-event001".into(),
            workspace_id: "1783586870822-7352d".into(),
            kind: "work.completed".into(),
            summary: "Canonical WorkEvent for Checkpoint A.".into(),
            timestamp: "2026-07-15T07:00:00Z".into(),
            evidence: vec![
                EvidenceAttachment {
                    evidence_ref: EvidenceRef::FilePath(
                        "crates/openmesh-core/src/domain.rs".into(),
                    ),
                    observed_at: Some("2026-07-15T07:00:01Z".into()),
                },
                EvidenceAttachment {
                    evidence_ref: EvidenceRef::ProducerSignal("s-verify".into()),
                    observed_at: None,
                },
            ],
            corrects_event_id: None,
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_EVENT_PROTOCOL_VERSION.to_string(),
            actor: None,
        }
    }

    #[test]
    fn work_event_round_trips_through_json() {
        let original = sample_event();
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, original);
    }

    #[test]
    fn evidence_attachment_round_trips_through_json() {
        let original = EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: Some("2026-07-15T07:00:00Z".into()),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let restored: EvidenceAttachment = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, original);
    }

    #[test]
    fn deserializes_the_canonical_event_fixture() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!("{manifest_dir}/tests/fixtures/events/valid.json");
        let json = std::fs::read_to_string(&fixture_path).expect("read fixture");
        let event: WorkEvent = serde_json::from_str(&json).expect("deserialize fixture");

        assert_eq!(event.event_id, "1783605120049-event001");
        assert_eq!(event.workspace_id, "1783586870822-7352d");
        assert_eq!(event.kind, "work.completed");
        assert_eq!(
            event.summary,
            "Canonical WorkEvent fixture for Dev Track 0.1.3.4 Checkpoint A."
        );
        assert_eq!(event.timestamp, "2026-07-15T07:00:00Z");
        assert_eq!(event.evidence.len(), 2);
        assert_eq!(
            event.evidence[0].evidence_ref,
            EvidenceRef::FilePath("crates/openmesh-core/src/domain.rs".into())
        );
        assert_eq!(
            event.evidence[0].observed_at.as_deref(),
            Some("2026-07-15T07:00:01Z")
        );
        assert_eq!(
            event.evidence[1].evidence_ref,
            EvidenceRef::ProducerSignal("s-verify".into())
        );
        assert!(event.evidence[1].observed_at.is_none());
        assert_eq!(event.corrects_event_id, None);
        assert_eq!(event.sensitivity, Sensitivity::Private);
        assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
    }

    #[test]
    fn work_event_missing_sensitivity_is_rejected() {
        let json = r#"{
            "eventId": "evt-1",
            "workspaceId": "ws-1",
            "kind": "work.completed",
            "summary": "completed the task",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [
                { "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }
            ],
            "protocolVersion": "1.0"
        }"#;
        let result: Result<WorkEvent, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "missing sensitivity must fail strict WorkEvent deserialization"
        );
    }

    #[test]
    fn work_event_corrects_event_id_round_trips() {
        let mut event = sample_event();
        event.corrects_event_id = Some("evt-original".into());
        let json = serde_json::to_string(&event).expect("serialize");
        let restored: WorkEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.corrects_event_id.as_deref(), Some("evt-original"));
    }

    #[test]
    fn validate_event_semantics_accepts_valid_event() {
        validate_event_semantics(&sample_event()).expect("valid event");
    }

    #[test]
    fn validate_event_semantics_rejects_empty_evidence() {
        let mut event = sample_event();
        event.evidence.clear();
        assert_eq!(
            validate_event_semantics(&event),
            Err(EventValidationError::EmptyEvidence)
        );
    }

    #[test]
    fn validate_event_semantics_rejects_invalid_timestamp() {
        let mut event = sample_event();
        event.timestamp = "2026-07-15T07:00:00-05:00".into();
        assert!(matches!(
            validate_event_semantics(&event),
            Err(EventValidationError::InvalidTimestamp(_))
        ));
    }

    #[test]
    fn validate_event_semantics_rejects_wrong_protocol_version() {
        let mut event = sample_event();
        event.protocol_version = "99.0".into();
        assert_eq!(
            validate_event_semantics(&event),
            Err(EventValidationError::UnsupportedProtocolVersion {
                found: "99.0".into(),
            })
        );
    }

    #[test]
    fn validate_event_semantics_rejects_invalid_observed_at() {
        let mut event = sample_event();
        event.evidence[0].observed_at = Some("2026-07-15T07:00:01-05:00".into());
        assert!(matches!(
            validate_event_semantics(&event),
            Err(EventValidationError::InvalidObservedAt(_))
        ));
    }

    /// Legacy `1.0` WorkEvents omit `actor` on wire; `1.1` promoted events require it.
    #[test]
    fn work_event_v1_0_serializes_without_actor_on_wire() {
        let json = serde_json::to_string(&sample_event()).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        let obj = value.as_object().expect("object");
        assert!(!obj.contains_key("actor"));
    }

    /// EvidenceRef includes pointer-only variants — `FilePath`, `ProducerSignal`,
    /// and bounded `GitState` metadata (no source/diff bodies).
    #[test]
    fn evidence_ref_includes_git_state_variant() {
        let variants = [
            EvidenceRef::FilePath("path".into()),
            EvidenceRef::ProducerSignal("s-1".into()),
            EvidenceRef::GitState(sample_git_state()),
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).expect("serialize");
            let restored: EvidenceRef = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(restored, variant);
        }
        match EvidenceRef::GitState(sample_git_state()) {
            EvidenceRef::FilePath(_)
            | EvidenceRef::ProducerSignal(_)
            | EvidenceRef::GitState(_) => {}
        }
    }
}
