//! Dev Track 0.1.8 Checkpoint A — Handoff Note wire contract (pure, no I/O).
//!
//! Source of truth for a local handoff package. Markdown projection and storage
//! are later checkpoints; this module only defines shape + fail-closed validation.

use crate::domain::{validate_evidence_ref, validate_utc_timestamp, CatchUpWindow, EvidenceRef};
use serde::{Deserialize, Serialize};

/// Wire protocol for `HandoffNote`.
pub const HANDOFF_NOTE_PROTOCOL_VERSION: &str = "1.0";

/// Canonical WorkEvent kind used when a handoff is linked into the ledger.
pub const WORK_EVENT_HANDOFF_KIND: &str = "work.handoff";

pub const MAX_HANDOFF_ID_BYTES: usize = 128;
pub const MAX_HANDOFF_RECIPIENT_LABEL_BYTES: usize = 128;
pub const MAX_HANDOFF_ITEM_SUMMARY_BYTES: usize = 512;
pub const MAX_HANDOFF_LIMITATION_BYTES: usize = 512;
pub const MAX_HANDOFF_SECTION_ITEMS: usize = 64;
pub const MAX_HANDOFF_WARNINGS: usize = 32;
pub const MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM: usize = 16;
pub const MAX_HANDOFF_SOURCE_EVENT_IDS_PER_ITEM: usize = 16;

/// Lifecycle of a handoff note before any remote share (local-only in 0.1.8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HandoffStatus {
    Draft,
    Approved,
}

/// Named teammate or role the handoff is prepared for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffRecipient {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_label: Option<String>,
}

/// One reconstructable bullet under a handoff section.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffSectionItem {
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    /// Ledger event ids this item was derived from (reconstructability).
    #[serde(default)]
    pub source_event_ids: Vec<String>,
}

/// Bounded section body (what changed / blocked / …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffSection {
    #[serde(default)]
    pub items: Vec<HandoffSectionItem>,
}

/// Freshness metadata for the window used to build the note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffFreshness {
    pub generated_at: String,
    pub window: CatchUpWindow,
    pub age_seconds: u64,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// Structured, evidence-backed local handoff note (protocol 1.0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandoffNote {
    pub protocol_version: String,
    pub handoff_id: String,
    pub workspace_id: String,
    pub status: HandoffStatus,
    pub recipient: HandoffRecipient,
    pub window: CatchUpWindow,
    pub what_changed: HandoffSection,
    pub what_is_complete: HandoffSection,
    pub what_is_blocked: HandoffSection,
    pub what_needs_review: HandoffSection,
    pub open_questions: HandoffSection,
    pub safe_to_answer_context: HandoffSection,
    pub next_suggested_step: HandoffSection,
    pub freshness: HandoffFreshness,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    /// Optional ledger linkage once Checkpoint D persists a WorkEvent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_event_id: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HandoffValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("handoff_id is empty after trim")]
    EmptyHandoffId,
    #[error("handoff_id exceeds the {max}-byte bound")]
    HandoffIdTooLong { max: usize },
    #[error("handoff_id must not contain path separators or '..'")]
    UnsafeHandoffId,
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("recipient.label is empty after trim")]
    EmptyRecipientLabel,
    #[error("recipient.label exceeds the {max}-byte bound")]
    RecipientLabelTooLong { max: usize },
    #[error("recipient.role_label is empty after trim")]
    EmptyRecipientRoleLabel,
    #[error("recipient.role_label exceeds the {max}-byte bound")]
    RecipientRoleLabelTooLong { max: usize },
    #[error("invalid catch-up window: {0}")]
    InvalidWindow(String),
    #[error("catch-up window is inverted (since >= until)")]
    WindowInverted,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("approved handoff requires approved_at")]
    ApprovedMissingApprovedAt,
    #[error("draft handoff must not set approved_at")]
    DraftHasApprovedAt,
    #[error("updated_at must not precede created_at")]
    UpdatedBeforeCreated,
    #[error("section `{section}` exceeds the {max}-item bound")]
    TooManySectionItems { section: &'static str, max: usize },
    #[error("section `{section}` item summary is empty")]
    EmptyItemSummary { section: &'static str },
    #[error("section `{section}` item summary exceeds the {max}-byte bound")]
    ItemSummaryTooLong { section: &'static str, max: usize },
    #[error("section `{section}` item has too many evidence refs (max {max})")]
    TooManyItemEvidenceRefs { section: &'static str, max: usize },
    #[error("section `{section}` item has too many source event ids (max {max})")]
    TooManyItemSourceEventIds { section: &'static str, max: usize },
    #[error("section `{section}` item has empty source_event_id")]
    EmptySourceEventId { section: &'static str },
    #[error("item evidence is invalid: {0}")]
    InvalidItemEvidence(String),
    #[error("too many freshness warnings (max {max})")]
    TooManyWarnings { max: usize },
    #[error("freshness warning is empty")]
    EmptyWarning,
    #[error("limitation is empty")]
    EmptyLimitation,
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
    #[error("handoff has no section content and no limitations (fail closed)")]
    EmptyHandoffWithoutLimitations,
    #[error("work_event_id is empty after trim")]
    EmptyWorkEventId,
}

/// Structural validation for `HandoffNote` v1.0 (pure, no I/O).
pub fn validate_handoff_note(note: &HandoffNote) -> Result<(), HandoffValidationError> {
    if note.protocol_version != HANDOFF_NOTE_PROTOCOL_VERSION {
        return Err(HandoffValidationError::UnsupportedProtocolVersion {
            found: note.protocol_version.clone(),
            expected: HANDOFF_NOTE_PROTOCOL_VERSION,
        });
    }

    validate_handoff_id_for_storage(&note.handoff_id)?;

    if note.workspace_id.trim().is_empty() {
        return Err(HandoffValidationError::EmptyWorkspaceId);
    }

    validate_recipient_fields(&note.recipient)?;
    validate_window_fields(&note.window)?;
    validate_utc_timestamp(&note.freshness.generated_at)
        .map_err(HandoffValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&note.created_at).map_err(HandoffValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&note.updated_at).map_err(HandoffValidationError::InvalidTimestamp)?;

    let created = chrono::DateTime::parse_from_rfc3339(&note.created_at)
        .map_err(|err| HandoffValidationError::InvalidTimestamp(err.to_string()))?;
    let updated = chrono::DateTime::parse_from_rfc3339(&note.updated_at)
        .map_err(|err| HandoffValidationError::InvalidTimestamp(err.to_string()))?;
    if updated < created {
        return Err(HandoffValidationError::UpdatedBeforeCreated);
    }

    match note.status {
        HandoffStatus::Approved => {
            let approved_at = note
                .approved_at
                .as_deref()
                .ok_or(HandoffValidationError::ApprovedMissingApprovedAt)?;
            validate_utc_timestamp(approved_at)
                .map_err(HandoffValidationError::InvalidTimestamp)?;
        }
        HandoffStatus::Draft => {
            if note.approved_at.is_some() {
                return Err(HandoffValidationError::DraftHasApprovedAt);
            }
        }
    }

    if let Some(event_id) = &note.work_event_id {
        if event_id.trim().is_empty() {
            return Err(HandoffValidationError::EmptyWorkEventId);
        }
    }

    validate_section("whatChanged", &note.what_changed)?;
    validate_section("whatIsComplete", &note.what_is_complete)?;
    validate_section("whatIsBlocked", &note.what_is_blocked)?;
    validate_section("whatNeedsReview", &note.what_needs_review)?;
    validate_section("openQuestions", &note.open_questions)?;
    validate_section("safeToAnswerContext", &note.safe_to_answer_context)?;
    validate_section("nextSuggestedStep", &note.next_suggested_step)?;

    if note.freshness.warnings.len() > MAX_HANDOFF_WARNINGS {
        return Err(HandoffValidationError::TooManyWarnings {
            max: MAX_HANDOFF_WARNINGS,
        });
    }
    for warning in &note.freshness.warnings {
        if warning.trim().is_empty() {
            return Err(HandoffValidationError::EmptyWarning);
        }
    }

    for limitation in &note.limitations {
        if limitation.trim().is_empty() {
            return Err(HandoffValidationError::EmptyLimitation);
        }
        if limitation.len() > MAX_HANDOFF_LIMITATION_BYTES {
            return Err(HandoffValidationError::LimitationTooLong {
                max: MAX_HANDOFF_LIMITATION_BYTES,
            });
        }
    }

    let total_items = section_item_count(note);
    if total_items == 0 && note.limitations.is_empty() {
        return Err(HandoffValidationError::EmptyHandoffWithoutLimitations);
    }

    Ok(())
}

/// Validates a handoff id for persistence lookups (filename-safe, bounded).
pub fn validate_handoff_id_for_storage(handoff_id: &str) -> Result<(), HandoffValidationError> {
    validate_non_empty_bounded(
        handoff_id,
        MAX_HANDOFF_ID_BYTES,
        HandoffValidationError::EmptyHandoffId,
        HandoffValidationError::HandoffIdTooLong {
            max: MAX_HANDOFF_ID_BYTES,
        },
    )?;
    if handoff_id.contains('/') || handoff_id.contains('\\') || handoff_id.contains("..") {
        return Err(HandoffValidationError::UnsafeHandoffId);
    }
    Ok(())
}

/// Validates recipient label and optional role label bounds.
pub fn validate_recipient_fields(
    recipient: &HandoffRecipient,
) -> Result<(), HandoffValidationError> {
    validate_non_empty_bounded(
        &recipient.label,
        MAX_HANDOFF_RECIPIENT_LABEL_BYTES,
        HandoffValidationError::EmptyRecipientLabel,
        HandoffValidationError::RecipientLabelTooLong {
            max: MAX_HANDOFF_RECIPIENT_LABEL_BYTES,
        },
    )?;
    if let Some(role) = &recipient.role_label {
        validate_non_empty_bounded(
            role,
            MAX_HANDOFF_RECIPIENT_LABEL_BYTES,
            HandoffValidationError::EmptyRecipientRoleLabel,
            HandoffValidationError::RecipientRoleLabelTooLong {
                max: MAX_HANDOFF_RECIPIENT_LABEL_BYTES,
            },
        )?;
    }
    Ok(())
}

/// Validates catch-up window timestamps and ordering.
pub fn validate_window_fields(window: &CatchUpWindow) -> Result<(), HandoffValidationError> {
    validate_utc_timestamp(&window.since).map_err(HandoffValidationError::InvalidWindow)?;
    validate_utc_timestamp(&window.until).map_err(HandoffValidationError::InvalidWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|_| HandoffValidationError::InvalidWindow(window.since.clone()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|_| HandoffValidationError::InvalidWindow(window.until.clone()))?;
    if since >= until {
        return Err(HandoffValidationError::WindowInverted);
    }
    Ok(())
}

fn validate_section(
    section: &'static str,
    body: &HandoffSection,
) -> Result<(), HandoffValidationError> {
    if body.items.len() > MAX_HANDOFF_SECTION_ITEMS {
        return Err(HandoffValidationError::TooManySectionItems {
            section,
            max: MAX_HANDOFF_SECTION_ITEMS,
        });
    }
    for item in &body.items {
        if item.summary.trim().is_empty() {
            return Err(HandoffValidationError::EmptyItemSummary { section });
        }
        if item.summary.len() > MAX_HANDOFF_ITEM_SUMMARY_BYTES {
            return Err(HandoffValidationError::ItemSummaryTooLong {
                section,
                max: MAX_HANDOFF_ITEM_SUMMARY_BYTES,
            });
        }
        if item.evidence_refs.len() > MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM {
            return Err(HandoffValidationError::TooManyItemEvidenceRefs {
                section,
                max: MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM,
            });
        }
        if item.source_event_ids.len() > MAX_HANDOFF_SOURCE_EVENT_IDS_PER_ITEM {
            return Err(HandoffValidationError::TooManyItemSourceEventIds {
                section,
                max: MAX_HANDOFF_SOURCE_EVENT_IDS_PER_ITEM,
            });
        }
        for evidence in &item.evidence_refs {
            validate_evidence_ref(evidence)
                .map_err(|err| HandoffValidationError::InvalidItemEvidence(err.to_string()))?;
        }
        for event_id in &item.source_event_ids {
            if event_id.trim().is_empty() {
                return Err(HandoffValidationError::EmptySourceEventId { section });
            }
        }
    }
    Ok(())
}

fn section_item_count(note: &HandoffNote) -> usize {
    note.what_changed.items.len()
        + note.what_is_complete.items.len()
        + note.what_is_blocked.items.len()
        + note.what_needs_review.items.len()
        + note.open_questions.items.len()
        + note.safe_to_answer_context.items.len()
        + note.next_suggested_step.items.len()
}

fn validate_non_empty_bounded(
    value: &str,
    max: usize,
    empty: HandoffValidationError,
    too_long: HandoffValidationError,
) -> Result<(), HandoffValidationError> {
    if value.trim().is_empty() {
        return Err(empty);
    }
    if value.len() > max {
        return Err(too_long);
    }
    Ok(())
}
