//! Dev Track 0.1.9 — Pending Questions & Return Digest wire contracts (pure).
//!
//! Projection types only — no I/O. Sources are bridged in `pending` / `digest`.

use crate::domain::{
    validate_evidence_ref, validate_utc_timestamp, CatchUpSections, CatchUpWindow, EvidenceRef,
    PendingAttentionSeverity, PendingAttentionStatus,
};
use serde::{Deserialize, Serialize};

/// Wire protocol for pending-questions and return-digest views.
pub const PENDING_QUESTIONS_PROTOCOL_VERSION: &str = "1.0";
pub const RETURN_DIGEST_PROTOCOL_VERSION: &str = "1.0";

pub const MAX_PENDING_QUESTION_ITEMS: usize = 64;
pub const MAX_PENDING_QUESTION_SUMMARY_BYTES: usize = 512;
pub const MAX_PENDING_QUESTION_REASON_BYTES: usize = 512;
pub const MAX_DIGEST_LIMITATIONS: usize = 16;
pub const MAX_DIGEST_LIMITATION_BYTES: usize = 512;
pub const MAX_DIGEST_EVIDENCE_REFS: usize = 64;
pub const MAX_DIGEST_HANDOFF_REFS: usize = 32;
pub const MAX_DIGEST_SUMMARY_BYTES: usize = 1024;

/// Where a unified pending-question item was projected from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PendingQuestionSourceKind {
    /// Must-ask / deny records under `.openmesh/proxy/pending/`.
    ProxyPending,
    /// Continuity `PendingAttentionItem` from current-state projection.
    ContinuityAttention,
    /// WorkSignal kind `unresolved-question` still in inbox buckets.
    UnresolvedSignal,
}

/// Unified "what needs me" item projected from multiple local sources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingQuestionItem {
    pub id: String,
    pub summary: String,
    pub source: PendingQuestionSourceKind,
    pub source_id: String,
    pub status: String,
    pub severity: String,
    pub created_at: String,
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_authority: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
}

/// On-demand projection of everything that currently needs a person.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingQuestionsView {
    pub workspace_id: String,
    pub generated_at: String,
    pub protocol_version: String,
    pub items: Vec<PendingQuestionItem>,
    pub open_count: u32,
    pub source_counts: PendingQuestionSourceCounts,
    #[serde(default)]
    pub limitations: Vec<String>,
}

/// Counts of projected pending-question sources (open items only).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingQuestionSourceCounts {
    pub proxy_pending: u32,
    pub continuity_attention: u32,
    pub unresolved_signal: u32,
}

/// Compact handoff pointer for the return digest (not a full note).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffDigestRef {
    pub handoff_id: String,
    pub status: String,
    pub recipient_label: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_until: Option<String>,
}

/// On-demand return digest: what I missed + what needs me after an absence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReturnDigest {
    pub workspace_id: String,
    pub generated_at: String,
    pub protocol_version: String,
    pub window: CatchUpWindow,
    pub summary: String,
    /// "What needs me" — open pending questions (not window-filtered).
    pub needs_me: Vec<PendingQuestionItem>,
    /// "What did I miss" — Catch-up sections for the absence window.
    pub what_i_missed: CatchUpSections,
    pub catch_up_summary: String,
    /// Local handoff notes relevant to the return (draft + approved).
    pub handoffs: Vec<HandoffDigestRef>,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReturnDigestValidationError {
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("invalid pending question item: {0}")]
    InvalidPendingQuestionItem(String),
    #[error("pending question items exceed the {max}-entry bound")]
    TooManyPendingQuestions { max: usize },
    #[error("invalid catch-up window: {0}")]
    InvalidCatchUpWindow(String),
    #[error("catch-up window since must be <= until")]
    CatchUpWindowInverted,
    #[error("summary exceeds the {max}-byte bound")]
    SummaryTooLong { max: usize },
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
    #[error("evidence_refs exceed the {max}-entry bound")]
    TooManyEvidenceRefs { max: usize },
    #[error("handoffs exceed the {max}-entry bound")]
    TooManyHandoffs { max: usize },
    #[error("invalid handoff digest ref: {0}")]
    InvalidHandoffRef(String),
    #[error("timestamp is invalid: {0}")]
    InvalidTimestamp(String),
    #[error("open_count does not match open items")]
    OpenCountMismatch,
    #[error("source_counts do not match open items")]
    SourceCountsMismatch,
}

pub fn severity_wire(severity: PendingAttentionSeverity) -> &'static str {
    match severity {
        PendingAttentionSeverity::Low => "low",
        PendingAttentionSeverity::Medium => "medium",
        PendingAttentionSeverity::High => "high",
        PendingAttentionSeverity::Critical => "critical",
    }
}

pub fn attention_status_wire(status: PendingAttentionStatus) -> &'static str {
    match status {
        PendingAttentionStatus::Open => "open",
        PendingAttentionStatus::Acknowledged => "acknowledged",
        PendingAttentionStatus::Resolved => "resolved",
        PendingAttentionStatus::Deferred => "deferred",
    }
}

pub fn is_open_status(status: &str) -> bool {
    let s = status.trim().to_ascii_lowercase();
    matches!(s.as_str(), "open" | "acknowledged" | "deferred" | "")
}

pub fn validate_pending_question_item(
    item: &PendingQuestionItem,
) -> Result<(), ReturnDigestValidationError> {
    if item.id.trim().is_empty() {
        return Err(ReturnDigestValidationError::InvalidPendingQuestionItem(
            "id is empty".into(),
        ));
    }
    if item.source_id.trim().is_empty() {
        return Err(ReturnDigestValidationError::InvalidPendingQuestionItem(
            "source_id is empty".into(),
        ));
    }
    if item.summary.trim().is_empty() {
        return Err(ReturnDigestValidationError::InvalidPendingQuestionItem(
            "summary is empty".into(),
        ));
    }
    if item.summary.len() > MAX_PENDING_QUESTION_SUMMARY_BYTES {
        return Err(ReturnDigestValidationError::InvalidPendingQuestionItem(
            format!("summary exceeds {MAX_PENDING_QUESTION_SUMMARY_BYTES} bytes"),
        ));
    }
    if item.reason.len() > MAX_PENDING_QUESTION_REASON_BYTES {
        return Err(ReturnDigestValidationError::InvalidPendingQuestionItem(
            format!("reason exceeds {MAX_PENDING_QUESTION_REASON_BYTES} bytes"),
        ));
    }
    validate_utc_timestamp(&item.created_at)
        .map_err(ReturnDigestValidationError::InvalidTimestamp)?;
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence).map_err(|e| {
            ReturnDigestValidationError::InvalidPendingQuestionItem(e.to_string())
        })?;
    }
    Ok(())
}

pub fn validate_pending_questions_view(
    view: &PendingQuestionsView,
) -> Result<(), ReturnDigestValidationError> {
    if view.workspace_id.trim().is_empty() {
        return Err(ReturnDigestValidationError::EmptyWorkspaceId);
    }
    if view.protocol_version != PENDING_QUESTIONS_PROTOCOL_VERSION {
        return Err(ReturnDigestValidationError::UnsupportedProtocolVersion {
            found: view.protocol_version.clone(),
            expected: PENDING_QUESTIONS_PROTOCOL_VERSION,
        });
    }
    validate_utc_timestamp(&view.generated_at)
        .map_err(ReturnDigestValidationError::InvalidTimestamp)?;
    if view.items.len() > MAX_PENDING_QUESTION_ITEMS {
        return Err(ReturnDigestValidationError::TooManyPendingQuestions {
            max: MAX_PENDING_QUESTION_ITEMS,
        });
    }
    for item in &view.items {
        validate_pending_question_item(item)?;
    }
    validate_limitations(&view.limitations)?;

    let open_items: Vec<_> = view
        .items
        .iter()
        .filter(|i| is_open_status(&i.status))
        .collect();
    if view.open_count as usize != open_items.len() {
        return Err(ReturnDigestValidationError::OpenCountMismatch);
    }
    let mut counts = PendingQuestionSourceCounts::default();
    for item in &open_items {
        match item.source {
            PendingQuestionSourceKind::ProxyPending => counts.proxy_pending += 1,
            PendingQuestionSourceKind::ContinuityAttention => counts.continuity_attention += 1,
            PendingQuestionSourceKind::UnresolvedSignal => counts.unresolved_signal += 1,
        }
    }
    if counts != view.source_counts {
        return Err(ReturnDigestValidationError::SourceCountsMismatch);
    }
    Ok(())
}

pub fn validate_handoff_digest_ref(
    item: &HandoffDigestRef,
) -> Result<(), ReturnDigestValidationError> {
    if item.handoff_id.trim().is_empty() {
        return Err(ReturnDigestValidationError::InvalidHandoffRef(
            "handoff_id is empty".into(),
        ));
    }
    if item.recipient_label.trim().is_empty() {
        return Err(ReturnDigestValidationError::InvalidHandoffRef(
            "recipient_label is empty".into(),
        ));
    }
    validate_utc_timestamp(&item.created_at)
        .map_err(ReturnDigestValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&item.updated_at)
        .map_err(ReturnDigestValidationError::InvalidTimestamp)?;
    Ok(())
}

pub fn validate_return_digest(digest: &ReturnDigest) -> Result<(), ReturnDigestValidationError> {
    if digest.workspace_id.trim().is_empty() {
        return Err(ReturnDigestValidationError::EmptyWorkspaceId);
    }
    if digest.protocol_version != RETURN_DIGEST_PROTOCOL_VERSION {
        return Err(ReturnDigestValidationError::UnsupportedProtocolVersion {
            found: digest.protocol_version.clone(),
            expected: RETURN_DIGEST_PROTOCOL_VERSION,
        });
    }
    validate_utc_timestamp(&digest.generated_at)
        .map_err(ReturnDigestValidationError::InvalidTimestamp)?;
    validate_window(&digest.window)?;
    if digest.summary.len() > MAX_DIGEST_SUMMARY_BYTES {
        return Err(ReturnDigestValidationError::SummaryTooLong {
            max: MAX_DIGEST_SUMMARY_BYTES,
        });
    }
    if digest.catch_up_summary.len() > MAX_DIGEST_SUMMARY_BYTES {
        return Err(ReturnDigestValidationError::SummaryTooLong {
            max: MAX_DIGEST_SUMMARY_BYTES,
        });
    }
    if digest.needs_me.len() > MAX_PENDING_QUESTION_ITEMS {
        return Err(ReturnDigestValidationError::TooManyPendingQuestions {
            max: MAX_PENDING_QUESTION_ITEMS,
        });
    }
    for item in &digest.needs_me {
        validate_pending_question_item(item)?;
    }
    if digest.handoffs.len() > MAX_DIGEST_HANDOFF_REFS {
        return Err(ReturnDigestValidationError::TooManyHandoffs {
            max: MAX_DIGEST_HANDOFF_REFS,
        });
    }
    for handoff in &digest.handoffs {
        validate_handoff_digest_ref(handoff)?;
    }
    if digest.evidence_refs.len() > MAX_DIGEST_EVIDENCE_REFS {
        return Err(ReturnDigestValidationError::TooManyEvidenceRefs {
            max: MAX_DIGEST_EVIDENCE_REFS,
        });
    }
    for evidence in &digest.evidence_refs {
        validate_evidence_ref(evidence).map_err(|e| {
            ReturnDigestValidationError::InvalidPendingQuestionItem(e.to_string())
        })?;
    }
    validate_limitations(&digest.limitations)?;
    Ok(())
}

fn validate_window(window: &CatchUpWindow) -> Result<(), ReturnDigestValidationError> {
    validate_utc_timestamp(&window.since)
        .map_err(ReturnDigestValidationError::InvalidCatchUpWindow)?;
    validate_utc_timestamp(&window.until)
        .map_err(ReturnDigestValidationError::InvalidCatchUpWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|_| ReturnDigestValidationError::InvalidCatchUpWindow(window.since.clone()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|_| ReturnDigestValidationError::InvalidCatchUpWindow(window.until.clone()))?;
    if since > until {
        return Err(ReturnDigestValidationError::CatchUpWindowInverted);
    }
    Ok(())
}

fn validate_limitations(limitations: &[String]) -> Result<(), ReturnDigestValidationError> {
    if limitations.len() > MAX_DIGEST_LIMITATIONS {
        return Err(ReturnDigestValidationError::TooManyLimitations {
            max: MAX_DIGEST_LIMITATIONS,
        });
    }
    for limitation in limitations {
        if limitation.len() > MAX_DIGEST_LIMITATION_BYTES {
            return Err(ReturnDigestValidationError::LimitationTooLong {
                max: MAX_DIGEST_LIMITATION_BYTES,
            });
        }
    }
    Ok(())
}
