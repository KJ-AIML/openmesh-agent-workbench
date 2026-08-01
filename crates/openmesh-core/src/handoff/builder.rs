//! Dev Track 0.1.8 Checkpoint C — handoff note builder (pure, no persistence).

use crate::continuity::readers::ContinuityInputSnapshot;
use crate::continuity::{build_catch_up_view, ContinuityError};
use crate::domain::{
    CatchUpView, CatchUpWindow, ContinuitySourceKind, ContinuityStateItem, CurrentStateProjection,
    PendingAttentionItem, MAX_PROJECTION_LIMITATIONS,
};
use crate::handoff::contract::{
    validate_handoff_note, HandoffFreshness, HandoffNote, HandoffRecipient, HandoffSection,
    HandoffSectionItem, HandoffStatus, HandoffValidationError, HANDOFF_NOTE_PROTOCOL_VERSION,
    MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM, MAX_HANDOFF_LIMITATION_BYTES, MAX_HANDOFF_SECTION_ITEMS,
    MAX_HANDOFF_SOURCE_EVENT_IDS_PER_ITEM,
};

/// Inputs for building a draft handoff note from continuity projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildHandoffRequest {
    pub workspace_id: String,
    pub recipient: HandoffRecipient,
    pub window: CatchUpWindow,
    pub now_rfc3339: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HandoffBuildError {
    #[error("workspace_id does not match continuity snapshot")]
    WorkspaceMismatch,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("continuity build failed: {0}")]
    Continuity(String),
    #[error("handoff validation failed: {0}")]
    Validation(#[from] HandoffValidationError),
}

/// Builds a validated draft `HandoffNote` from continuity inputs.
pub fn build_handoff_note(
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    request: &BuildHandoffRequest,
) -> Result<HandoffNote, HandoffBuildError> {
    if request.workspace_id != snapshot.workspace_id {
        return Err(HandoffBuildError::WorkspaceMismatch);
    }

    let catch_up = build_catch_up_view(snapshot, current_state, &request.window)
        .map_err(map_continuity_error)?;

    let what_changed = section_from_items(catch_up.sections.changed.iter());
    let what_is_complete = section_from_items(
        catch_up
            .sections
            .completed
            .iter()
            .chain(current_state.sections.completed.iter()),
    );
    let what_is_blocked = section_from_items(
        catch_up
            .sections
            .blocked
            .iter()
            .chain(current_state.sections.blocked.iter()),
    );
    let what_needs_review = section_from_items(
        catch_up
            .sections
            .needs_attention
            .iter()
            .chain(current_state.sections.needs_attention.iter()),
    );
    let open_questions = section_from_items(
        catch_up
            .sections
            .still_open
            .iter()
            .chain(current_state.sections.still_open.iter()),
    );
    let safe_to_answer_context = section_from_items(
        catch_up
            .sections
            .decided
            .iter()
            .chain(current_state.sections.in_progress.iter())
            .chain(current_state.sections.decisions.iter()),
    );
    let next_suggested_step = section_from_attention(
        catch_up
            .next_suggested_attention
            .iter()
            .chain(current_state.pending_attention.iter()),
    );

    let mut limitations = merge_limitations(&catch_up, current_state);
    if section_is_empty(&what_changed)
        && section_is_empty(&what_is_complete)
        && section_is_empty(&what_is_blocked)
        && section_is_empty(&what_needs_review)
        && section_is_empty(&open_questions)
        && section_is_empty(&safe_to_answer_context)
        && section_is_empty(&next_suggested_step)
    {
        limitations.push(format!(
            "no continuity items in handoff window {} to {}",
            request.window.since, request.window.until
        ));
    }

    let freshness = build_freshness(snapshot, &request.window, &request.now_rfc3339)?;
    let handoff_id = deterministic_handoff_id(
        &request.now_rfc3339,
        &request.workspace_id,
        &request.window,
        &request.recipient,
    )?;

    let note = HandoffNote {
        protocol_version: HANDOFF_NOTE_PROTOCOL_VERSION.into(),
        handoff_id,
        workspace_id: request.workspace_id.clone(),
        status: HandoffStatus::Draft,
        recipient: request.recipient.clone(),
        window: request.window.clone(),
        what_changed,
        what_is_complete,
        what_is_blocked,
        what_needs_review,
        open_questions,
        safe_to_answer_context,
        next_suggested_step,
        freshness,
        limitations,
        created_at: request.now_rfc3339.clone(),
        updated_at: request.now_rfc3339.clone(),
        approved_at: None,
        work_event_id: None,
    };

    validate_handoff_note(&note)?;
    Ok(note)
}

fn map_continuity_error(err: ContinuityError) -> HandoffBuildError {
    HandoffBuildError::Continuity(err.to_string())
}

fn section_from_items<'a, I>(items: I) -> HandoffSection
where
    I: Iterator<Item = &'a ContinuityStateItem>,
{
    let mut section = HandoffSection::default();
    let mut seen = std::collections::BTreeSet::new();
    for item in items {
        if !seen.insert(item.source_id.clone()) {
            continue;
        }
        if section.items.len() >= MAX_HANDOFF_SECTION_ITEMS {
            break;
        }
        section.items.push(item_to_handoff_item(item));
    }
    section
}

fn section_from_attention<'a, I>(items: I) -> HandoffSection
where
    I: Iterator<Item = &'a PendingAttentionItem>,
{
    let mut section = HandoffSection::default();
    let mut seen = std::collections::BTreeSet::new();
    for item in items {
        if !seen.insert(item.id.clone()) {
            continue;
        }
        if section.items.len() >= MAX_HANDOFF_SECTION_ITEMS {
            break;
        }
        section.items.push(attention_to_handoff_item(item));
    }
    section
}

fn item_to_handoff_item(item: &ContinuityStateItem) -> HandoffSectionItem {
    HandoffSectionItem {
        summary: item.summary.clone(),
        evidence_refs: item
            .evidence_refs
            .iter()
            .take(MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM)
            .cloned()
            .collect(),
        source_event_ids: source_event_ids_for_item(item),
    }
}

fn attention_to_handoff_item(item: &PendingAttentionItem) -> HandoffSectionItem {
    HandoffSectionItem {
        summary: item.summary.clone(),
        evidence_refs: item
            .evidence_refs
            .iter()
            .take(MAX_HANDOFF_EVIDENCE_REFS_PER_ITEM)
            .cloned()
            .collect(),
        source_event_ids: match item.source {
            ContinuitySourceKind::WorkEvent => vec![item.source_id.clone()],
            _ => Vec::new(),
        }
        .into_iter()
        .take(MAX_HANDOFF_SOURCE_EVENT_IDS_PER_ITEM)
        .collect(),
    }
}

fn source_event_ids_for_item(item: &ContinuityStateItem) -> Vec<String> {
    if item.source == ContinuitySourceKind::WorkEvent {
        vec![item.source_id.clone()]
    } else {
        Vec::new()
    }
}

fn section_is_empty(section: &HandoffSection) -> bool {
    section.items.is_empty()
}

fn merge_limitations(
    catch_up: &CatchUpView,
    current_state: &CurrentStateProjection,
) -> Vec<String> {
    let mut merged = std::collections::BTreeSet::new();
    for entry in current_state
        .limitations
        .iter()
        .chain(catch_up.limitations.iter())
    {
        let trimmed = entry.trim();
        if trimmed.is_empty() {
            continue;
        }
        let bounded = if trimmed.len() > MAX_HANDOFF_LIMITATION_BYTES {
            trimmed[..MAX_HANDOFF_LIMITATION_BYTES].to_string()
        } else {
            trimmed.to_string()
        };
        merged.insert(bounded);
    }
    merged
        .into_iter()
        .take(MAX_PROJECTION_LIMITATIONS)
        .collect()
}

fn build_freshness(
    snapshot: &ContinuityInputSnapshot,
    window: &CatchUpWindow,
    now_rfc3339: &str,
) -> Result<HandoffFreshness, HandoffBuildError> {
    let observed = chrono::DateTime::parse_from_rfc3339(&snapshot.loaded_at)
        .map_err(|err| HandoffBuildError::InvalidTimestamp(err.to_string()))?;
    let generated = chrono::DateTime::parse_from_rfc3339(now_rfc3339)
        .map_err(|err| HandoffBuildError::InvalidTimestamp(err.to_string()))?;
    let age_seconds = generated
        .signed_duration_since(observed)
        .num_seconds()
        .max(0) as u64;
    Ok(HandoffFreshness {
        generated_at: now_rfc3339.to_string(),
        window: window.clone(),
        age_seconds,
        warnings: Vec::new(),
    })
}

fn deterministic_handoff_id(
    now_rfc3339: &str,
    workspace_id: &str,
    window: &CatchUpWindow,
    recipient: &HandoffRecipient,
) -> Result<String, HandoffBuildError> {
    let generated = chrono::DateTime::parse_from_rfc3339(now_rfc3339)
        .map_err(|err| HandoffBuildError::InvalidTimestamp(err.to_string()))?;
    let stamp = generated.format("%Y%m%d%H%M%S");
    let role = recipient.role_label.as_deref().unwrap_or("");
    let material = format!(
        "{}|{}|{}|{}|{}",
        workspace_id, window.since, window.until, recipient.label, role
    );
    let hash = fnv1a_hex(&material);
    Ok(format!("handoff-{stamp}-{}", &hash[..8]))
}

fn fnv1a_hex(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;
    let mut hash = FNV_OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}
