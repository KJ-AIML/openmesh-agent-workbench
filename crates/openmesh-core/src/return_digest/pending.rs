//! Project unified pending questions from proxy / continuity / signal sources.

use crate::continuity::current_state::ContinuityError;
use crate::continuity::readers::ContinuityInputSnapshot;
use crate::domain::{
    CurrentStateProjection, PendingAttentionItem, PendingAttentionReason,
    PendingAttentionStatus, WorkSignal, WorkSignalKind,
};
use crate::pending_proxy_question::{list_pending_proxy_questions, PendingProxyQuestion};
use crate::return_digest::contract::{
    attention_status_wire, is_open_status, severity_wire, validate_pending_questions_view,
    PendingQuestionItem, PendingQuestionSourceCounts, PendingQuestionSourceKind,
    PendingQuestionsView, ReturnDigestValidationError, MAX_DIGEST_LIMITATIONS,
    MAX_PENDING_QUESTION_ITEMS, MAX_PENDING_QUESTION_REASON_BYTES,
    MAX_PENDING_QUESTION_SUMMARY_BYTES, PENDING_QUESTIONS_PROTOCOL_VERSION,
};
use crate::storage::{read_project, Project};
use chrono::Utc;

#[derive(Debug, thiserror::Error)]
pub enum PendingQuestionsError {
    #[error(transparent)]
    Continuity(#[from] ContinuityError),
    #[error(transparent)]
    Validation(#[from] ReturnDigestValidationError),
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("proxy pending read failed")]
    ProxyPendingRead,
}

/// Build an on-demand pending-questions view for a project.
pub fn build_pending_questions_view(
    project_path: &str,
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
) -> Result<PendingQuestionsView, PendingQuestionsError> {
    let project: Project = read_project(project_path, "project.json")
        .ok_or(PendingQuestionsError::ProjectNotInitialized)?;
    let generated_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut limitations = Vec::new();
    let mut items = Vec::new();
    let mut seen_source_ids = std::collections::BTreeSet::new();

    // 1. Proxy must-ask / denied pending questions
    match list_pending_proxy_questions(project_path) {
        Ok(proxy_items) => {
            for record in proxy_items {
                let item = item_from_proxy_pending(&record);
                if seen_source_ids.insert(item.source_id.clone()) {
                    items.push(item);
                }
            }
        }
        Err(_) => {
            limitations.push(
                "proxy pending directory unreadable; proxy questions omitted".to_string(),
            );
        }
    }

    // 2. Continuity pending attention (current state)
    for attention in &current_state.pending_attention {
        if attention.status == PendingAttentionStatus::Resolved {
            continue;
        }
        let item = item_from_attention(attention);
        if seen_source_ids.insert(item.source_id.clone()) {
            items.push(item);
        }
    }

    // 3. Unresolved-question WorkSignals in pending + processed buckets
    for signal in snapshot
        .pending_signals
        .iter()
        .chain(snapshot.processed_signals.iter())
    {
        if signal.kind != WorkSignalKind::UnresolvedQuestion {
            continue;
        }
        let item = item_from_unresolved_signal(signal);
        if seen_source_ids.insert(item.source_id.clone()) {
            items.push(item);
        }
    }

    // Deterministic order: severity priority then id
    items.sort_by(|a, b| {
        severity_rank(&a.severity)
            .cmp(&severity_rank(&b.severity))
            .then_with(|| a.id.cmp(&b.id))
    });
    if items.len() > MAX_PENDING_QUESTION_ITEMS {
        limitations.push(format!(
            "pending questions truncated to {MAX_PENDING_QUESTION_ITEMS} items"
        ));
        items.truncate(MAX_PENDING_QUESTION_ITEMS);
    }

    let mut source_counts = PendingQuestionSourceCounts::default();
    let mut open_count = 0u32;
    for item in &items {
        if !is_open_status(&item.status) {
            continue;
        }
        open_count += 1;
        match item.source {
            PendingQuestionSourceKind::ProxyPending => source_counts.proxy_pending += 1,
            PendingQuestionSourceKind::ContinuityAttention => {
                source_counts.continuity_attention += 1
            }
            PendingQuestionSourceKind::UnresolvedSignal => source_counts.unresolved_signal += 1,
        }
    }

    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_DIGEST_LIMITATIONS);

    let view = PendingQuestionsView {
        workspace_id: project.id,
        generated_at,
        protocol_version: PENDING_QUESTIONS_PROTOCOL_VERSION.into(),
        items,
        open_count,
        source_counts,
        limitations,
    };
    validate_pending_questions_view(&view)?;
    Ok(view)
}

/// Convenience loader that rebuilds continuity inputs + current state then projects.
pub fn build_pending_questions_for_project(
    project_path: &str,
    current_state: &CurrentStateProjection,
    snapshot: &ContinuityInputSnapshot,
) -> Result<PendingQuestionsView, PendingQuestionsError> {
    build_pending_questions_view(project_path, snapshot, current_state)
}

fn item_from_proxy_pending(record: &PendingProxyQuestion) -> PendingQuestionItem {
    let summary = truncate_bytes(&record.question_text, MAX_PENDING_QUESTION_SUMMARY_BYTES);
    let reason = truncate_bytes(&record.reason, MAX_PENDING_QUESTION_REASON_BYTES);
    let severity = match record.resolved_authority.as_str() {
        "cannot-answer" => "critical",
        "must-ask-human" => "high",
        _ => "medium",
    }
    .to_string();
    PendingQuestionItem {
        id: format!("pq-proxy-{}", record.pending_id),
        summary,
        source: PendingQuestionSourceKind::ProxyPending,
        source_id: record.pending_id.clone(),
        status: record.status.clone(),
        severity,
        created_at: record.created_at.clone(),
        reason,
        risk: Some(record.risk.clone()),
        resolved_authority: Some(record.resolved_authority.clone()),
        evidence_refs: Vec::new(),
    }
}

fn item_from_attention(attention: &PendingAttentionItem) -> PendingQuestionItem {
    PendingQuestionItem {
        id: format!("pq-attention-{}", attention.id),
        summary: truncate_bytes(&attention.summary, MAX_PENDING_QUESTION_SUMMARY_BYTES),
        source: PendingQuestionSourceKind::ContinuityAttention,
        source_id: attention.source_id.clone(),
        status: attention_status_wire(attention.status).to_string(),
        severity: severity_wire(attention.severity).to_string(),
        created_at: attention.timestamp.clone(),
        reason: attention_reason_wire(attention.reason).to_string(),
        risk: None,
        resolved_authority: None,
        evidence_refs: attention.evidence_refs.clone(),
    }
}

fn item_from_unresolved_signal(signal: &WorkSignal) -> PendingQuestionItem {
    PendingQuestionItem {
        id: format!("pq-signal-{}", signal.signal_id),
        summary: truncate_bytes(&signal.summary, MAX_PENDING_QUESTION_SUMMARY_BYTES),
        source: PendingQuestionSourceKind::UnresolvedSignal,
        source_id: signal.signal_id.clone(),
        status: "open".to_string(),
        severity: "medium".to_string(),
        created_at: signal.timestamp.clone(),
        reason: "unresolved-question signal".to_string(),
        risk: None,
        resolved_authority: None,
        evidence_refs: signal.evidence_refs.clone(),
    }
}

fn attention_reason_wire(reason: PendingAttentionReason) -> &'static str {
    match reason {
        PendingAttentionReason::PendingSignal => "pending-signal",
        PendingAttentionReason::ReviewRequired => "review-required",
        PendingAttentionReason::Blocker => "blocker",
        PendingAttentionReason::UnresolvedQuestion => "unresolved-question",
        PendingAttentionReason::AmbiguousPromotion => "ambiguous-promotion",
        PendingAttentionReason::SuppressedPromotion => "suppressed-promotion",
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 1,
        "high" => 2,
        "medium" => 3,
        "low" => 4,
        _ => 5,
    }
}

fn truncate_bytes(input: &str, max: usize) -> String {
    if input.len() <= max {
        return input.to_string();
    }
    let mut end = max;
    while end > 0 && !input.is_char_boundary(end) {
        end -= 1;
    }
    input[..end].to_string()
}
