//! Build an on-demand Return Digest (what I missed + what needs me).

use crate::continuity::build_catch_up_view;
use crate::continuity::current_state::ContinuityError;
use crate::continuity::readers::ContinuityInputSnapshot;
use crate::domain::{CatchUpWindow, CurrentStateProjection};
use crate::handoff::{list_handoff_ids, read_handoff_note, HandoffStatus};
use crate::return_digest::contract::{
    is_open_status, validate_return_digest, HandoffDigestRef, ReturnDigest,
    ReturnDigestValidationError, MAX_DIGEST_EVIDENCE_REFS, MAX_DIGEST_HANDOFF_REFS,
    MAX_DIGEST_LIMITATIONS, MAX_DIGEST_SUMMARY_BYTES, RETURN_DIGEST_PROTOCOL_VERSION,
};
use crate::return_digest::pending::{
    build_pending_questions_view, PendingQuestionsError,
};

#[derive(Debug, thiserror::Error)]
pub enum ReturnDigestError {
    #[error(transparent)]
    Continuity(#[from] ContinuityError),
    #[error(transparent)]
    Validation(#[from] ReturnDigestValidationError),
    #[error(transparent)]
    Pending(#[from] PendingQuestionsError),
}

/// Build a return digest for an absence window using continuity + pending + handoffs.
pub fn build_return_digest(
    project_path: &str,
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    window: &CatchUpWindow,
) -> Result<ReturnDigest, ReturnDigestError> {
    let catch_up = build_catch_up_view(snapshot, current_state, window)?;
    let pending = build_pending_questions_view(project_path, snapshot, current_state)?;

    let needs_me: Vec<_> = pending
        .items
        .into_iter()
        .filter(|item| is_open_status(&item.status))
        .collect();

    let handoffs = load_handoff_refs(project_path);
    let mut limitations = catch_up.limitations.clone();
    limitations.extend(pending.limitations);
    if handoffs.1 {
        limitations.push("one or more handoff notes could not be read".to_string());
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_DIGEST_LIMITATIONS);

    let mut evidence_refs = catch_up.evidence_refs;
    for item in &needs_me {
        for evidence in &item.evidence_refs {
            if !evidence_refs.iter().any(|existing| existing == evidence) {
                evidence_refs.push(evidence.clone());
            }
        }
    }
    evidence_refs.truncate(MAX_DIGEST_EVIDENCE_REFS);

    let missed_count = count_missed(&catch_up.sections);
    let summary = build_digest_summary(needs_me.len(), missed_count, handoffs.0.len());

    let digest = ReturnDigest {
        workspace_id: catch_up.workspace_id,
        generated_at: catch_up.generated_at,
        protocol_version: RETURN_DIGEST_PROTOCOL_VERSION.into(),
        window: window.clone(),
        summary,
        needs_me,
        what_i_missed: catch_up.sections,
        catch_up_summary: catch_up.summary,
        handoffs: handoffs.0,
        evidence_refs,
        limitations,
    };
    validate_return_digest(&digest)?;
    Ok(digest)
}

fn count_missed(sections: &crate::domain::CatchUpSections) -> usize {
    sections.completed.len()
        + sections.changed.len()
        + sections.blocked.len()
        + sections.decided.len()
        + sections.needs_attention.len()
        + sections.still_open.len()
}

fn build_digest_summary(needs_me: usize, missed: usize, handoffs: usize) -> String {
    let text = format!(
        "Return digest: {needs_me} item(s) need you; {missed} continuity item(s) in the absence window; {handoffs} handoff note(s)."
    );
    if text.len() <= MAX_DIGEST_SUMMARY_BYTES {
        text
    } else {
        text[..MAX_DIGEST_SUMMARY_BYTES].to_string()
    }
}

/// Returns (refs, had_read_errors).
fn load_handoff_refs(project_path: &str) -> (Vec<HandoffDigestRef>, bool) {
    let mut had_errors = false;
    let Ok(ids) = list_handoff_ids(project_path) else {
        return (Vec::new(), false);
    };
    let mut refs = Vec::new();
    for id in ids {
        match read_handoff_note(project_path, &id) {
            Ok(note) => {
                // Include draft + approved local notes; skip nothing else in v1.
                let status = match note.status {
                    HandoffStatus::Draft => "draft",
                    HandoffStatus::Approved => "approved",
                };
                refs.push(HandoffDigestRef {
                    handoff_id: note.handoff_id,
                    status: status.to_string(),
                    recipient_label: note.recipient.label,
                    created_at: note.created_at,
                    updated_at: note.updated_at,
                    window_since: Some(note.freshness.window.since),
                    window_until: Some(note.freshness.window.until),
                });
            }
            Err(_) => had_errors = true,
        }
        if refs.len() >= MAX_DIGEST_HANDOFF_REFS {
            break;
        }
    }
    refs.sort_by(|a, b| a.handoff_id.cmp(&b.handoff_id));
    (refs, had_errors)
}
