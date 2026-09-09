use super::context_pack::{MAX_CONTEXT_PACK_BUILD_INPUTS_HASH_BYTES, MAX_CONTEXT_PACK_ID_BYTES};
use super::events::validate_utc_timestamp;
use super::projections::MAX_LIMITATION_BYTES;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
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
