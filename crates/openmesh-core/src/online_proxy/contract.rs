//! Pure wire contracts for Always-Online Work Proxy Alpha.

use crate::authority_freshness::{ConfidenceLabel, FreshnessResult};
use crate::authority_policy::FreshnessTier;
use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const ONLINE_PROXY_PROTOCOL_VERSION: &str = "1.0";
pub const MAX_PROXY_ID_BYTES: usize = 128;
pub const MAX_LABEL_BYTES: usize = 128;
/// Raised for live Agent Engine answers (scaffold drafts were 8 KiB).
pub const MAX_ANSWER_TEXT_BYTES: usize = 32768;
pub const MAX_FRESHNESS_STATEMENT_BYTES: usize = 1024;
pub const MAX_WARNING_BYTES: usize = 512;
pub const MAX_WARNINGS: usize = 16;
pub const MAX_SOURCE_IDS: usize = 64;

/// Deployment mode for the always-online scaffold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnlineProxyMode {
    /// Local process simulating always-online using local + relay-received evidence.
    LocalScaffold,
    /// Reserved for true remote cloud runtime (not fully implemented in alpha).
    CloudScaffold,
}

/// Local configuration for the always-online proxy runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnlineProxyConfig {
    pub protocol_version: String,
    pub proxy_id: String,
    pub workspace_id: String,
    pub owner_label: String,
    pub mode: OnlineProxyMode,
    pub default_freshness_tier: FreshnessTier,
    /// When true, include relay-received packages as remote evidence sources.
    pub use_relay_received: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Explicit freshness disclosure required on every online-proxy answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EvidenceFreshnessStatement {
    /// Human-readable mandatory disclosure (must be non-empty).
    pub statement: String,
    pub evaluated_at: String,
    pub tier: FreshnessTier,
    pub is_sufficient: bool,
    pub confidence_label: ConfidenceLabel,
    pub oldest_evidence_age_seconds: u64,
    #[serde(default)]
    pub stale_warnings: Vec<String>,
    /// Source ids that informed the answer (envelope/package/event ids).
    #[serde(default)]
    pub evidence_source_ids: Vec<String>,
}

/// Always-online proxy answer — draft-only, non-executing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnlineProxyAnswer {
    pub protocol_version: String,
    pub answer_id: String,
    pub proxy_id: String,
    pub workspace_id: String,
    pub question: String,
    pub answer_text: String,
    pub generated_at: String,
    pub freshness: EvidenceFreshnessStatement,
    /// True when answer was refused due to freshness/policy (answer_text explains).
    pub refused: bool,
    pub mode: OnlineProxyMode,
    /// True when answer text came from Agent Engine (not LocalScaffold paste).
    #[serde(default)]
    pub live_engine: bool,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OnlineProxyValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("proxy_id invalid")]
    InvalidProxyId,
    #[error("workspace_id empty")]
    EmptyWorkspaceId,
    #[error("owner_label empty or too long")]
    InvalidOwnerLabel,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("freshness statement empty or too long")]
    InvalidFreshnessStatement,
    #[error("answer text empty or too long")]
    InvalidAnswerText,
    #[error("question empty")]
    EmptyQuestion,
    #[error("too many warnings or sources")]
    Bounds,
    #[error("answer missing mandatory freshness disclosure")]
    MissingFreshness,
}

pub fn validate_online_proxy_config(cfg: &OnlineProxyConfig) -> Result<(), OnlineProxyValidationError> {
    if cfg.protocol_version != ONLINE_PROXY_PROTOCOL_VERSION {
        return Err(OnlineProxyValidationError::UnsupportedProtocol {
            found: cfg.protocol_version.clone(),
        });
    }
    if cfg.proxy_id.trim().is_empty()
        || cfg.proxy_id.len() > MAX_PROXY_ID_BYTES
        || cfg.proxy_id.contains("..")
        || cfg.proxy_id.contains('/')
    {
        return Err(OnlineProxyValidationError::InvalidProxyId);
    }
    if cfg.workspace_id.trim().is_empty() {
        return Err(OnlineProxyValidationError::EmptyWorkspaceId);
    }
    if cfg.owner_label.trim().is_empty() || cfg.owner_label.len() > MAX_LABEL_BYTES {
        return Err(OnlineProxyValidationError::InvalidOwnerLabel);
    }
    validate_utc_timestamp(&cfg.created_at).map_err(OnlineProxyValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&cfg.updated_at).map_err(OnlineProxyValidationError::InvalidTimestamp)?;
    Ok(())
}

pub fn validate_evidence_freshness_statement(
    s: &EvidenceFreshnessStatement,
) -> Result<(), OnlineProxyValidationError> {
    if s.statement.trim().is_empty() || s.statement.len() > MAX_FRESHNESS_STATEMENT_BYTES {
        return Err(OnlineProxyValidationError::InvalidFreshnessStatement);
    }
    // Gate: never silently omit disclosure language.
    let lower = s.statement.to_ascii_lowercase();
    if !lower.contains("fresh") && !lower.contains("stale") && !lower.contains("age") {
        return Err(OnlineProxyValidationError::InvalidFreshnessStatement);
    }
    validate_utc_timestamp(&s.evaluated_at).map_err(OnlineProxyValidationError::InvalidTimestamp)?;
    if s.stale_warnings.len() > MAX_WARNINGS || s.evidence_source_ids.len() > MAX_SOURCE_IDS {
        return Err(OnlineProxyValidationError::Bounds);
    }
    for w in &s.stale_warnings {
        if w.trim().is_empty() || w.len() > MAX_WARNING_BYTES {
            return Err(OnlineProxyValidationError::Bounds);
        }
    }
    Ok(())
}

pub fn validate_online_proxy_answer(a: &OnlineProxyAnswer) -> Result<(), OnlineProxyValidationError> {
    if a.protocol_version != ONLINE_PROXY_PROTOCOL_VERSION {
        return Err(OnlineProxyValidationError::UnsupportedProtocol {
            found: a.protocol_version.clone(),
        });
    }
    if a.answer_id.trim().is_empty() || a.answer_id.contains("..") {
        return Err(OnlineProxyValidationError::InvalidProxyId);
    }
    if a.question.trim().is_empty() {
        return Err(OnlineProxyValidationError::EmptyQuestion);
    }
    if a.answer_text.trim().is_empty() || a.answer_text.len() > MAX_ANSWER_TEXT_BYTES {
        return Err(OnlineProxyValidationError::InvalidAnswerText);
    }
    validate_utc_timestamp(&a.generated_at).map_err(OnlineProxyValidationError::InvalidTimestamp)?;
    validate_evidence_freshness_statement(&a.freshness)?;
    // Scaffold gate: never silently stale — insufficient freshness must refuse.
    // Live Agent Engine answers disclose freshness in-prompt; soft-warn only.
    if !a.freshness.is_sufficient && !a.refused && !a.live_engine {
        return Err(OnlineProxyValidationError::MissingFreshness);
    }
    Ok(())
}

/// Build a mandatory human-readable freshness statement from evaluation.
pub fn build_freshness_statement_text(result: &FreshnessResult, tier: FreshnessTier) -> String {
    let status = if result.is_sufficient {
        "fresh enough"
    } else {
        "stale or insufficient"
    };
    format!(
        "Evidence freshness: {} for tier {:?} (oldest age {}s, confidence {:?}). Always-online proxy will not hide staleness.",
        status, tier, result.oldest_evidence_age_seconds, result.confidence_label
    )
}
