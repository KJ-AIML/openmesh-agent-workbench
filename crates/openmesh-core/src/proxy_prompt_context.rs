//! Dev Track 0.1.6 Checkpoint B — model-facing prompt-safe context contracts (pure).

use crate::context_pack_validation::validate_proxy_context_pack_complete;
use crate::domain::{
    CommunicationPreferences, ContextPackContinuityItem, ContextPackItemProvenance,
    ContextPackPendingAttentionItem, ContinuityConfidence, DecisionPreferences,
    PendingAttentionReason, PendingAttentionSeverity, PendingAttentionStatus, ProxyContextPack,
};
use crate::proxy_draft_safety::filter_stale_runtime_limitations;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Wire-schema version for `ProxyPromptContext`.
pub const PROXY_PROMPT_CONTEXT_VERSION: &str = "1.0";

/// Frozen prompt template version carried in model-facing context.
pub const PROXY_PROMPT_TEMPLATE_VERSION: &str = "1.0";

/// Fixed authority execution constraint for model-facing context.
pub const PROXY_PROMPT_AUTHORITY_EXECUTION: &str = "disabled";

/// Frozen UTF-8 byte cap for serialized `ProxyPromptContext`.
pub const MAX_PROXY_PROMPT_CONTEXT_BYTES: usize = 32_768;

/// Frozen cap for total Current State prompt items.
pub const MAX_PROXY_PROMPT_STATE_ITEMS: usize = 64;

/// Frozen cap for each Catch-up section item list.
pub const MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION: usize = 32;

/// Frozen cap for prompt-safe limitation strings.
pub const MAX_PROXY_PROMPT_LIMITATIONS: usize = 32;

pub const MAX_PROXY_PROMPT_LABEL_BYTES: usize = 512;
pub const MAX_PROXY_PROMPT_SUMMARY_BYTES: usize = 512;
pub const MAX_PROXY_PROMPT_KIND_BYTES: usize = 128;
pub const MAX_PROXY_PROMPT_TIMESTAMP_BYTES: usize = 64;
pub const MAX_PROXY_PROMPT_WARNING_BYTES: usize = 512;
pub const MAX_PROXY_PROMPT_CATCHUP_SUMMARY_BYTES: usize = 1024;

/// Lowest-priority-first Current State section tail removal order for byte bounding.
const CURRENT_STATE_BOUNDING_SECTION_ORDER: [CurrentStatePromptSection; 6] = [
    CurrentStatePromptSection::Completed,
    CurrentStatePromptSection::Decisions,
    CurrentStatePromptSection::StillOpen,
    CurrentStatePromptSection::InProgress,
    CurrentStatePromptSection::NeedsAttention,
    CurrentStatePromptSection::Blocked,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CurrentStatePromptSection {
    Completed,
    InProgress,
    Blocked,
    Decisions,
    NeedsAttention,
    StillOpen,
}

/// Closed, allowlisted model-facing prompt context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptContext {
    pub prompt_context_version: String,
    pub prompt_template_version: String,
    pub owner_label: String,
    pub role_label: String,
    pub communication_preferences: CommunicationPreferences,
    pub decision_preferences: DecisionPreferences,
    pub current_state: ProxyPromptCurrentState,
    pub catch_up: ProxyPromptCatchUp,
    pub freshness: ProxyPromptFreshness,
    pub redaction_summary: ProxyPromptRedactionSummary,
    pub limitations: Vec<String>,
    pub authority_execution: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptCurrentState {
    pub sections: ProxyPromptCurrentStateSections,
    pub pending_attention: Vec<ProxyPromptPendingAttentionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptCurrentStateSections {
    pub completed: Vec<ProxyPromptStateItem>,
    pub in_progress: Vec<ProxyPromptStateItem>,
    pub blocked: Vec<ProxyPromptStateItem>,
    pub decisions: Vec<ProxyPromptStateItem>,
    pub needs_attention: Vec<ProxyPromptStateItem>,
    pub still_open: Vec<ProxyPromptStateItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptCatchUp {
    pub sections: ProxyPromptCatchUpSections,
    pub summary: String,
    pub next_suggested_attention: Vec<ProxyPromptPendingAttentionItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptCatchUpSections {
    pub completed: Vec<ProxyPromptStateItem>,
    pub changed: Vec<ProxyPromptStateItem>,
    pub blocked: Vec<ProxyPromptStateItem>,
    pub decided: Vec<ProxyPromptStateItem>,
    pub needs_attention: Vec<ProxyPromptStateItem>,
    pub still_open: Vec<ProxyPromptStateItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptStateItem {
    pub summary: String,
    pub kind: String,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub confidence: ContinuityConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unverified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction: Option<ProxyPromptCorrectionPresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptPendingAttentionItem {
    pub summary: String,
    pub reason: PendingAttentionReason,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub status: PendingAttentionStatus,
    pub severity: PendingAttentionSeverity,
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptCorrectionPresentation {
    pub is_corrected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptFreshness {
    pub snapshot_observed_at: String,
    pub current_state_generated_at: String,
    pub catch_up_since: String,
    pub catch_up_until: String,
    pub pack_generated_at: String,
    pub age_seconds: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyPromptRedactionSummary {
    pub secret_items_omitted: u32,
    pub policy_restricted_items_omitted: u32,
    pub malformed_items_omitted: u32,
    pub quarantined_items_omitted: u32,
    pub bounds_truncated_items: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyPromptContextValidationError {
    #[error("unsupported prompt_context_version")]
    UnsupportedPromptContextVersion,
    #[error("unsupported prompt_template_version")]
    UnsupportedPromptTemplateVersion,
    #[error("authority_execution must be disabled")]
    InvalidAuthorityExecution,
    #[error("owner_label is empty")]
    EmptyOwnerLabel,
    #[error("role_label is empty")]
    EmptyRoleLabel,
    #[error("limitations exceed the bound")]
    TooManyLimitations,
    #[error("current_state items exceed the bound")]
    TooManyStateItems,
    #[error("catch_up section exceeds the bound")]
    TooManyCatchUpItems,
    #[error("field exceeds byte bound")]
    FieldTooLong,
    #[error("empty required field")]
    EmptyField,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProxyPromptError {
    #[error("question is invalid")]
    InvalidQuestion,
    #[error("context pack is invalid")]
    InvalidContextPack,
    #[error("prompt context is invalid")]
    InvalidPromptContext,
    #[error("prompt context exceeds the byte bound")]
    ContextTooLarge,
    #[error("prompt context serialization failed")]
    SerializationFailed,
    #[error("prompt bundle is invalid")]
    InvalidPromptBundle,
}

/// Map a completely validated `ProxyContextPack` into model-facing `ProxyPromptContext`.
pub fn map_pack_to_proxy_prompt_context(
    pack: &ProxyContextPack,
) -> Result<ProxyPromptContext, ProxyPromptError> {
    validate_proxy_context_pack_complete(pack).map_err(|_| ProxyPromptError::InvalidContextPack)?;
    let context = ProxyPromptContext {
        prompt_context_version: PROXY_PROMPT_CONTEXT_VERSION.to_string(),
        prompt_template_version: PROXY_PROMPT_TEMPLATE_VERSION.to_string(),
        owner_label: pack.owner_identity.owner_label.clone(),
        role_label: pack.owner_identity.role_label.clone(),
        communication_preferences: pack.communication_preferences.clone(),
        decision_preferences: pack.decision_preferences.clone(),
        current_state: map_current_state(pack),
        catch_up: map_catch_up(pack),
        freshness: map_freshness(pack),
        redaction_summary: map_redaction_summary(pack),
        limitations: map_limitations(pack),
        authority_execution: PROXY_PROMPT_AUTHORITY_EXECUTION.to_string(),
    };
    validate_proxy_prompt_context(&context).map_err(|_| ProxyPromptError::InvalidPromptContext)?;
    Ok(context)
}

/// Apply deterministic count and byte bounds to a typed prompt context.
pub fn bound_proxy_prompt_context(
    mut context: ProxyPromptContext,
) -> Result<ProxyPromptContext, ProxyPromptError> {
    validate_proxy_prompt_context(&context).map_err(|_| ProxyPromptError::InvalidPromptContext)?;
    canonicalize_prompt_context(&mut context);
    apply_count_bounds(&mut context);
    canonicalize_prompt_context(&mut context);
    if serialized_prompt_context_bytes(&context)? <= MAX_PROXY_PROMPT_CONTEXT_BYTES {
        return Ok(context);
    }
    loop {
        if !remove_one_bounding_item(&mut context) {
            return Err(ProxyPromptError::ContextTooLarge);
        }
        canonicalize_prompt_context(&mut context);
        if serialized_prompt_context_bytes(&context)? <= MAX_PROXY_PROMPT_CONTEXT_BYTES {
            return Ok(context);
        }
    }
}

/// Serialize a canonical `ProxyPromptContext` to JSON bytes for measurement.
pub fn serialized_prompt_context_bytes(
    context: &ProxyPromptContext,
) -> Result<usize, ProxyPromptError> {
    let json = serde_json::to_string(context).map_err(|_| ProxyPromptError::SerializationFailed)?;
    Ok(json.len())
}

/// Serialize a canonical `ProxyPromptContext` to JSON text.
pub fn serialize_proxy_prompt_context(
    context: &ProxyPromptContext,
) -> Result<String, ProxyPromptError> {
    serde_json::to_string(context).map_err(|_| ProxyPromptError::SerializationFailed)
}

/// Structural validation for `ProxyPromptContext`.
pub fn validate_proxy_prompt_context(
    context: &ProxyPromptContext,
) -> Result<(), ProxyPromptContextValidationError> {
    if context.prompt_context_version != PROXY_PROMPT_CONTEXT_VERSION {
        return Err(ProxyPromptContextValidationError::UnsupportedPromptContextVersion);
    }
    if context.prompt_template_version != PROXY_PROMPT_TEMPLATE_VERSION {
        return Err(ProxyPromptContextValidationError::UnsupportedPromptTemplateVersion);
    }
    if context.authority_execution != PROXY_PROMPT_AUTHORITY_EXECUTION {
        return Err(ProxyPromptContextValidationError::InvalidAuthorityExecution);
    }
    validate_owner_label(&context.owner_label)?;
    validate_role_label(&context.role_label)?;
    validate_preferences(
        &context.communication_preferences,
        &context.decision_preferences,
    )?;
    validate_current_state(&context.current_state)?;
    validate_catch_up(&context.catch_up)?;
    validate_freshness(&context.freshness)?;
    let _ = &context.redaction_summary;
    if context.limitations.len() > MAX_PROXY_PROMPT_LIMITATIONS {
        return Err(ProxyPromptContextValidationError::TooManyLimitations);
    }
    for limitation in &context.limitations {
        validate_limitation_text(limitation)?;
    }
    Ok(())
}

fn map_current_state(pack: &ProxyContextPack) -> ProxyPromptCurrentState {
    let sections = &pack.current_state.sections;
    ProxyPromptCurrentState {
        sections: ProxyPromptCurrentStateSections {
            completed: map_state_items(&sections.completed),
            in_progress: map_state_items(&sections.in_progress),
            blocked: map_state_items(&sections.blocked),
            decisions: map_state_items(&sections.decisions),
            needs_attention: map_state_items(&sections.needs_attention),
            still_open: map_state_items(&sections.still_open),
        },
        pending_attention: map_pending_items(&pack.current_state.pending_attention),
    }
}

fn map_catch_up(pack: &ProxyContextPack) -> ProxyPromptCatchUp {
    let sections = &pack.catch_up.sections;
    ProxyPromptCatchUp {
        sections: ProxyPromptCatchUpSections {
            completed: map_state_items(&sections.completed),
            changed: map_state_items(&sections.changed),
            blocked: map_state_items(&sections.blocked),
            decided: map_state_items(&sections.decided),
            needs_attention: map_state_items(&sections.needs_attention),
            still_open: map_state_items(&sections.still_open),
        },
        summary: pack.catch_up.summary.clone(),
        next_suggested_attention: map_pending_items(&pack.catch_up.next_suggested_attention),
    }
}

fn map_state_items(items: &[ContextPackContinuityItem]) -> Vec<ProxyPromptStateItem> {
    let mut mapped = items.iter().filter_map(map_state_item).collect::<Vec<_>>();
    sort_state_items(&mut mapped);
    mapped
}

fn map_state_item(item: &ContextPackContinuityItem) -> Option<ProxyPromptStateItem> {
    if item.provenance == ContextPackItemProvenance::DiagnosticOnly {
        return None;
    }
    if item
        .correction
        .as_ref()
        .is_some_and(|correction| correction.is_superseded_original)
    {
        return None;
    }
    Some(ProxyPromptStateItem {
        summary: item.summary.clone(),
        kind: item.kind.clone(),
        provenance: item.provenance,
        timestamp: item.timestamp.clone(),
        confidence: item.confidence,
        unverified: item.unverified,
        correction: item
            .correction
            .as_ref()
            .map(|correction| ProxyPromptCorrectionPresentation {
                is_corrected: correction.is_corrected,
            }),
    })
}

fn map_pending_items(
    items: &[ContextPackPendingAttentionItem],
) -> Vec<ProxyPromptPendingAttentionItem> {
    let mut mapped = items
        .iter()
        .filter(|item| item.provenance != ContextPackItemProvenance::DiagnosticOnly)
        .map(|item| ProxyPromptPendingAttentionItem {
            summary: item.summary.clone(),
            reason: item.reason,
            provenance: item.provenance,
            timestamp: item.timestamp.clone(),
            status: item.status,
            severity: item.severity,
            priority: item.priority,
        })
        .collect::<Vec<_>>();
    sort_pending_items(&mut mapped);
    mapped
}

fn map_freshness(pack: &ProxyContextPack) -> ProxyPromptFreshness {
    ProxyPromptFreshness {
        snapshot_observed_at: pack.freshness.snapshot_observed_at.clone(),
        current_state_generated_at: pack.freshness.current_state_generated_at.clone(),
        catch_up_since: pack.freshness.catch_up_since.clone(),
        catch_up_until: pack.freshness.catch_up_until.clone(),
        pack_generated_at: pack.freshness.pack_generated_at.clone(),
        age_seconds: pack.freshness.age_seconds,
        warnings: pack.freshness.warnings.clone(),
    }
}

fn map_redaction_summary(pack: &ProxyContextPack) -> ProxyPromptRedactionSummary {
    let summary = &pack.redaction_summary;
    ProxyPromptRedactionSummary {
        secret_items_omitted: summary.secret_items_omitted,
        policy_restricted_items_omitted: summary.policy_restricted_items_omitted,
        malformed_items_omitted: summary.malformed_items_omitted,
        quarantined_items_omitted: summary.quarantined_items_omitted,
        bounds_truncated_items: summary.bounds_truncated_items,
    }
}

fn map_limitations(pack: &ProxyContextPack) -> Vec<String> {
    let mut merged = BTreeSet::new();
    for source in [
        pack.limitations.as_slice(),
        pack.current_state.limitations.as_slice(),
        pack.catch_up.limitations.as_slice(),
    ] {
        for entry in source {
            let trimmed = entry.trim();
            if !trimmed.is_empty() {
                merged.insert(trimmed.to_string());
            }
        }
    }
    if pack.redaction_summary.secret_items_omitted > 0 {
        merged.insert("some secret continuity material was omitted from this prompt".into());
    }
    if pack.redaction_summary.quarantined_items_omitted > 0
        || pack.redaction_summary.malformed_items_omitted > 0
    {
        merged.insert("some continuity inputs were omitted as incomplete or unsafe".into());
    }
    let mut limitations = merged.into_iter().collect::<Vec<_>>();
    limitations = filter_stale_runtime_limitations(&limitations);
    limitations.truncate(MAX_PROXY_PROMPT_LIMITATIONS);
    limitations
}

pub(crate) fn canonicalize_prompt_context(context: &mut ProxyPromptContext) {
    sort_state_items_in_sections(&mut context.current_state.sections);
    sort_pending_items(&mut context.current_state.pending_attention);
    sort_state_items_in_catchup_sections(&mut context.catch_up.sections);
    sort_pending_items(&mut context.catch_up.next_suggested_attention);
    context.limitations = dedupe_limitations(&context.limitations);
    context.freshness.warnings.sort();
    context.freshness.warnings.dedup();
}

fn apply_count_bounds(context: &mut ProxyPromptContext) {
    context.limitations.truncate(MAX_PROXY_PROMPT_LIMITATIONS);
    truncate_catchup_sections(&mut context.catch_up.sections);
    truncate_catchup_pending(&mut context.catch_up.next_suggested_attention);
    truncate_state_items(context);
}

fn truncate_catchup_sections(sections: &mut ProxyPromptCatchUpSections) {
    sections
        .completed
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
    sections
        .changed
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
    sections
        .blocked
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
    sections
        .decided
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
    sections
        .needs_attention
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
    sections
        .still_open
        .truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
}

fn truncate_catchup_pending(items: &mut Vec<ProxyPromptPendingAttentionItem>) {
    items.truncate(MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION);
}

fn truncate_state_items(context: &mut ProxyPromptContext) {
    while total_state_items(context) > MAX_PROXY_PROMPT_STATE_ITEMS {
        if !remove_one_state_count_item(context) {
            break;
        }
    }
}

fn total_state_items(context: &ProxyPromptContext) -> usize {
    let sections = &context.current_state.sections;
    sections.completed.len()
        + sections.in_progress.len()
        + sections.blocked.len()
        + sections.decisions.len()
        + sections.needs_attention.len()
        + sections.still_open.len()
        + context.current_state.pending_attention.len()
}

fn remove_one_state_count_item(context: &mut ProxyPromptContext) -> bool {
    for section in CURRENT_STATE_BOUNDING_SECTION_ORDER {
        if pop_state_section_tail(&mut context.current_state.sections, section) {
            return true;
        }
    }
    false
}

fn remove_one_bounding_item(context: &mut ProxyPromptContext) -> bool {
    if context.limitations.pop().is_some() {
        return true;
    }
    if context.catch_up.sections.still_open.pop().is_some() {
        return true;
    }
    if context.catch_up.sections.needs_attention.pop().is_some() {
        return true;
    }
    if context.catch_up.sections.changed.pop().is_some() {
        return true;
    }
    for section in CURRENT_STATE_BOUNDING_SECTION_ORDER {
        if pop_state_section_tail(&mut context.current_state.sections, section) {
            return true;
        }
    }
    false
}

fn pop_state_section_tail(
    sections: &mut ProxyPromptCurrentStateSections,
    section: CurrentStatePromptSection,
) -> bool {
    match section {
        CurrentStatePromptSection::Completed => sections.completed.pop(),
        CurrentStatePromptSection::InProgress => sections.in_progress.pop(),
        CurrentStatePromptSection::Blocked => sections.blocked.pop(),
        CurrentStatePromptSection::Decisions => sections.decisions.pop(),
        CurrentStatePromptSection::NeedsAttention => sections.needs_attention.pop(),
        CurrentStatePromptSection::StillOpen => sections.still_open.pop(),
    }
    .is_some()
}

fn sort_state_items_in_sections(sections: &mut ProxyPromptCurrentStateSections) {
    sort_state_items(&mut sections.completed);
    sort_state_items(&mut sections.in_progress);
    sort_state_items(&mut sections.blocked);
    sort_state_items(&mut sections.decisions);
    sort_state_items(&mut sections.needs_attention);
    sort_state_items(&mut sections.still_open);
}

fn sort_state_items_in_catchup_sections(sections: &mut ProxyPromptCatchUpSections) {
    sort_state_items(&mut sections.completed);
    sort_state_items(&mut sections.changed);
    sort_state_items(&mut sections.blocked);
    sort_state_items(&mut sections.decided);
    sort_state_items(&mut sections.needs_attention);
    sort_state_items(&mut sections.still_open);
}

fn sort_state_items(items: &mut [ProxyPromptStateItem]) {
    items.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.summary.cmp(&right.summary))
            .then_with(|| left.kind.cmp(&right.kind))
    });
}

fn sort_pending_items(items: &mut [ProxyPromptPendingAttentionItem]) {
    items.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.summary.cmp(&right.summary))
            .then_with(|| left.priority.cmp(&right.priority))
    });
}

fn dedupe_limitations(limitations: &[String]) -> Vec<String> {
    let mut set = BTreeSet::new();
    for entry in limitations {
        let trimmed = entry.trim();
        if !trimmed.is_empty() {
            set.insert(trimmed.to_string());
        }
    }
    set.into_iter().collect()
}

fn validate_owner_label(value: &str) -> Result<(), ProxyPromptContextValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyPromptContextValidationError::EmptyOwnerLabel);
    }
    if value.len() > MAX_PROXY_PROMPT_LABEL_BYTES {
        return Err(ProxyPromptContextValidationError::FieldTooLong);
    }
    Ok(())
}

fn validate_role_label(value: &str) -> Result<(), ProxyPromptContextValidationError> {
    if value.trim().is_empty() {
        return Err(ProxyPromptContextValidationError::EmptyRoleLabel);
    }
    if value.len() > MAX_PROXY_PROMPT_LABEL_BYTES {
        return Err(ProxyPromptContextValidationError::FieldTooLong);
    }
    Ok(())
}

fn validate_preferences(
    communication: &CommunicationPreferences,
    decision: &DecisionPreferences,
) -> Result<(), ProxyPromptContextValidationError> {
    for value in [
        &communication.tone,
        &communication.detail_level,
        &communication.async_preference,
        &communication.correction_preference,
        &decision.decision_style,
        &decision.escalation_preference,
    ] {
        if value.trim().is_empty() {
            return Err(ProxyPromptContextValidationError::EmptyField);
        }
        if value.len() > MAX_PROXY_PROMPT_LABEL_BYTES {
            return Err(ProxyPromptContextValidationError::FieldTooLong);
        }
    }
    Ok(())
}

fn validate_current_state(
    current_state: &ProxyPromptCurrentState,
) -> Result<(), ProxyPromptContextValidationError> {
    if total_state_items_from_parts(current_state) > MAX_PROXY_PROMPT_STATE_ITEMS {
        return Err(ProxyPromptContextValidationError::TooManyStateItems);
    }
    for item in current_state
        .sections
        .completed
        .iter()
        .chain(&current_state.sections.in_progress)
        .chain(&current_state.sections.blocked)
        .chain(&current_state.sections.decisions)
        .chain(&current_state.sections.needs_attention)
        .chain(&current_state.sections.still_open)
    {
        validate_state_item(item)?;
    }
    for item in &current_state.pending_attention {
        validate_pending_item(item)?;
    }
    Ok(())
}

fn total_state_items_from_parts(current_state: &ProxyPromptCurrentState) -> usize {
    let sections = &current_state.sections;
    sections.completed.len()
        + sections.in_progress.len()
        + sections.blocked.len()
        + sections.decisions.len()
        + sections.needs_attention.len()
        + sections.still_open.len()
        + current_state.pending_attention.len()
}

fn validate_catch_up(
    catch_up: &ProxyPromptCatchUp,
) -> Result<(), ProxyPromptContextValidationError> {
    if catch_up.summary.trim().is_empty()
        || catch_up.summary.len() > MAX_PROXY_PROMPT_CATCHUP_SUMMARY_BYTES
    {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    for items in [
        &catch_up.sections.completed,
        &catch_up.sections.changed,
        &catch_up.sections.blocked,
        &catch_up.sections.decided,
        &catch_up.sections.needs_attention,
        &catch_up.sections.still_open,
    ] {
        if items.len() > MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION {
            return Err(ProxyPromptContextValidationError::TooManyCatchUpItems);
        }
        for item in items {
            validate_state_item(item)?;
        }
    }
    if catch_up.next_suggested_attention.len() > MAX_PROXY_PROMPT_CATCHUP_ITEMS_PER_SECTION {
        return Err(ProxyPromptContextValidationError::TooManyCatchUpItems);
    }
    for item in &catch_up.next_suggested_attention {
        validate_pending_item(item)?;
    }
    Ok(())
}

fn validate_freshness(
    freshness: &ProxyPromptFreshness,
) -> Result<(), ProxyPromptContextValidationError> {
    for value in [
        &freshness.snapshot_observed_at,
        &freshness.current_state_generated_at,
        &freshness.catch_up_since,
        &freshness.catch_up_until,
        &freshness.pack_generated_at,
    ] {
        if value.trim().is_empty() || value.len() > MAX_PROXY_PROMPT_TIMESTAMP_BYTES {
            return Err(ProxyPromptContextValidationError::EmptyField);
        }
    }
    for warning in &freshness.warnings {
        if warning.trim().is_empty() || warning.len() > MAX_PROXY_PROMPT_WARNING_BYTES {
            return Err(ProxyPromptContextValidationError::FieldTooLong);
        }
    }
    Ok(())
}

fn validate_state_item(
    item: &ProxyPromptStateItem,
) -> Result<(), ProxyPromptContextValidationError> {
    if item.summary.trim().is_empty() || item.summary.len() > MAX_PROXY_PROMPT_SUMMARY_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    if item.kind.trim().is_empty() || item.kind.len() > MAX_PROXY_PROMPT_KIND_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    if item.timestamp.trim().is_empty() || item.timestamp.len() > MAX_PROXY_PROMPT_TIMESTAMP_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    Ok(())
}

fn validate_pending_item(
    item: &ProxyPromptPendingAttentionItem,
) -> Result<(), ProxyPromptContextValidationError> {
    if item.summary.trim().is_empty() || item.summary.len() > MAX_PROXY_PROMPT_SUMMARY_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    if item.timestamp.trim().is_empty() || item.timestamp.len() > MAX_PROXY_PROMPT_TIMESTAMP_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    Ok(())
}

fn validate_limitation_text(limitation: &str) -> Result<(), ProxyPromptContextValidationError> {
    if limitation.trim().is_empty() || limitation.len() > MAX_PROXY_PROMPT_WARNING_BYTES {
        return Err(ProxyPromptContextValidationError::EmptyField);
    }
    Ok(())
}
