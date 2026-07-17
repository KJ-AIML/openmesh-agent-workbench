//! Catch-up view builder (Dev Track 0.1.3.7 Checkpoint D) — on-demand only, no persistence.

use crate::continuity::current_state::ContinuityError;
use crate::continuity::readers::{ContinuityDiagnostic, ContinuityInputSnapshot};
use crate::domain::ProducerRef;
use crate::domain::{
    effective_presentation, validate_catch_up_view, validate_correction_relationship,
    CatchUpSections, CatchUpView, CatchUpWindow, ContinuityConfidence, ContinuitySourceKind,
    ContinuityStateItem, ContinuityValidationError, CorrectionSemanticDiagnostic,
    CurrentStateProjection, EffectiveEventPresentation, EvidenceRef, PendingAttentionItem,
    WorkEvent, WorkSignal, WorkSignalKind, CATCH_UP_VIEW_PROTOCOL_VERSION,
    MAX_CATCH_UP_EVIDENCE_REFS, MAX_CATCH_UP_SUMMARY_BYTES, MAX_CONTINUITY_ITEM_EVIDENCE_REFS,
    MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES, MAX_NEXT_SUGGESTED_ATTENTION,
    MAX_PROJECTION_LIMITATIONS,
};
use crate::promotion::{PromotionDecisionRecord, PromotionOutcome};
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatchUpSection {
    Completed,
    Changed,
    Blocked,
    Decided,
    NeedsAttention,
    StillOpen,
}

/// Builds a deterministic on-demand `CatchUpView` from loaded inputs and current state.
pub fn build_catch_up_view(
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    window: &CatchUpWindow,
) -> Result<CatchUpView, ContinuityError> {
    validate_window(window)?;

    let mut sections = CatchUpSections {
        completed: Vec::new(),
        changed: Vec::new(),
        blocked: Vec::new(),
        decided: Vec::new(),
        needs_attention: Vec::new(),
        still_open: Vec::new(),
    };
    let mut limitations = current_state.limitations.clone();
    let mut evidence_set: BTreeSet<String> = BTreeSet::new();
    let ambiguity = detect_correlation_ambiguity(snapshot);

    for event in &snapshot.work_events {
        if event.corrects_event_id.is_some() {
            if let Err(diagnostic) = validate_correction_relationship(event, &snapshot.work_events)
            {
                limitations.push(limitation_from_correction_diagnostic(&diagnostic));
            }
        } else if let Some(presentation) =
            effective_presentation(&snapshot.work_events, &event.event_id)
        {
            for diagnostic in &presentation.diagnostics {
                limitations.push(limitation_from_correction_diagnostic(diagnostic));
            }
        }
    }

    for event in &snapshot.work_events {
        let Some(target_id) = event.corrects_event_id.as_deref() else {
            continue;
        };
        if !timestamp_in_window(&event.timestamp, window) {
            continue;
        }
        if validate_correction_relationship(event, &snapshot.work_events).is_err() {
            continue;
        }
        let Some(target) = snapshot
            .work_events
            .iter()
            .find(|candidate| candidate.event_id == target_id)
        else {
            continue;
        };
        let Some(presentation) = effective_presentation(&snapshot.work_events, target_id) else {
            continue;
        };
        let item = item_from_correction_in_window(event, target, &presentation);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_catch_up_section(&mut sections, CatchUpSection::Changed, item);
    }

    for event in &snapshot.work_events {
        if event.corrects_event_id.is_some() {
            continue;
        }
        if !timestamp_in_window(&event.timestamp, window) {
            continue;
        }
        let Some(presentation) = effective_presentation(&snapshot.work_events, &event.event_id)
        else {
            continue;
        };
        if presentation.is_corrected {
            limitations.push(correction_visibility_limitation(&presentation));
        }
        let item = item_from_work_event(event, &presentation);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_catch_up_section(
            &mut sections,
            catch_up_section_for_event(presentation.kind_text()),
            item,
        );
    }

    for signal in snapshot
        .processed_signals
        .iter()
        .chain(snapshot.pending_signals.iter())
    {
        if !timestamp_in_window(&signal.timestamp, window) {
            continue;
        }
        let source = if snapshot
            .pending_signals
            .iter()
            .any(|s| s.signal_id == signal.signal_id)
        {
            ContinuitySourceKind::PendingSignal
        } else {
            ContinuitySourceKind::ProcessedSignal
        };
        let item = item_from_signal(signal, source, &ambiguity);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_catch_up_section(
            &mut sections,
            catch_up_section_for_signal_kind(signal.kind),
            item,
        );
    }

    for record in &snapshot.promotion_audit_records {
        if !timestamp_in_window(&record.recorded_at, window) {
            continue;
        }
        let item = item_from_promotion_audit(record);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_catch_up_section(
            &mut sections,
            catch_up_section_for_promotion_outcome(record.outcome),
            item,
        );
    }

    for diagnostic in &snapshot.diagnostics {
        if !timestamp_in_window(&snapshot.loaded_at, window) {
            continue;
        }
        let item = item_from_diagnostic(diagnostic, &snapshot.loaded_at);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_catch_up_section(&mut sections, CatchUpSection::NeedsAttention, item);
        limitations.push(limitation_from_diagnostic(diagnostic));
    }

    for item in &current_state.sections.still_open {
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        sections.still_open.push(item.clone());
    }

    for attention in &current_state.pending_attention {
        let item = item_from_pending_attention(attention);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        if !sections
            .needs_attention
            .iter()
            .any(|existing| existing.source_id == item.source_id)
        {
            push_catch_up_section(&mut sections, CatchUpSection::NeedsAttention, item);
        }
    }

    for (hint, source_ids) in &ambiguity {
        limitations.push(format!(
            "ambiguous correlation hint {hint}: conflicting sources {}",
            source_ids.join(", ")
        ));
    }

    let window_item_count = sections.completed.len()
        + sections.changed.len()
        + sections.blocked.len()
        + sections.decided.len()
        + sections.needs_attention.len();
    if window_item_count == 0 {
        limitations.push(format!(
            "no continuity records fell within window {} to {}",
            window.since, window.until
        ));
    }

    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_PROJECTION_LIMITATIONS);

    sort_catch_up_sections(&mut sections);

    let mut next_suggested_attention: Vec<PendingAttentionItem> =
        current_state.pending_attention.clone();
    next_suggested_attention.sort_by(|a, b| a.id.cmp(&b.id));
    next_suggested_attention.truncate(MAX_NEXT_SUGGESTED_ATTENTION);

    let summary = build_summary(&sections, window_item_count);
    let evidence_refs = decode_evidence_set(&evidence_set);

    let view = CatchUpView {
        workspace_id: snapshot.workspace_id.clone(),
        generated_at: window.until.clone(),
        protocol_version: CATCH_UP_VIEW_PROTOCOL_VERSION.into(),
        window: window.clone(),
        sections,
        summary,
        next_suggested_attention,
        evidence_refs,
        limitations,
    };

    validate_catch_up_view(&view)?;
    Ok(view)
}

fn validate_window(window: &CatchUpWindow) -> Result<(), ContinuityValidationError> {
    validate_utc_timestamp(&window.since)
        .map_err(ContinuityValidationError::InvalidCatchUpWindow)?;
    validate_utc_timestamp(&window.until)
        .map_err(ContinuityValidationError::InvalidCatchUpWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|_| ContinuityValidationError::InvalidCatchUpWindow(window.since.clone()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|_| ContinuityValidationError::InvalidCatchUpWindow(window.until.clone()))?;
    if since > until {
        return Err(ContinuityValidationError::CatchUpWindowInverted);
    }
    Ok(())
}

fn validate_utc_timestamp(timestamp: &str) -> Result<(), String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).map_err(|e| e.to_string())?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err("timestamp must be UTC".into());
    }
    Ok(())
}

fn timestamp_in_window(timestamp: &str, window: &CatchUpWindow) -> bool {
    let Ok(value) = chrono::DateTime::parse_from_rfc3339(timestamp) else {
        return false;
    };
    let Ok(since) = chrono::DateTime::parse_from_rfc3339(&window.since) else {
        return false;
    };
    let Ok(until) = chrono::DateTime::parse_from_rfc3339(&window.until) else {
        return false;
    };
    value >= since && value <= until
}

fn build_summary(sections: &CatchUpSections, window_item_count: usize) -> String {
    if window_item_count == 0 {
        return "No changes found in this window".into();
    }
    let summary = format!(
        "{} completed, {} changed, {} blocked, {} decided, {} need attention",
        sections.completed.len(),
        sections.changed.len(),
        sections.blocked.len(),
        sections.decided.len(),
        sections.needs_attention.len(),
    );
    if summary.len() <= MAX_CATCH_UP_SUMMARY_BYTES {
        summary
    } else {
        summary[..MAX_CATCH_UP_SUMMARY_BYTES].to_string()
    }
}

fn detect_correlation_ambiguity(
    snapshot: &ContinuityInputSnapshot,
) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, (BTreeSet<String>, BTreeSet<String>)> = HashMap::new();
    for signal in snapshot
        .processed_signals
        .iter()
        .chain(snapshot.pending_signals.iter())
    {
        let Some(hint) = signal
            .correlation_hint
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let entry = groups.entry(hint.to_string()).or_default();
        entry.0.insert(signal_kind_label(signal.kind));
        entry.1.insert(format!("signal:{}", signal.signal_id));
    }
    groups
        .into_iter()
        .filter(|(_, (kinds, _))| kinds.len() > 1)
        .map(|(hint, (_, ids))| (hint, ids.into_iter().collect()))
        .collect()
}

fn is_ambiguous_source(source_id: &str, ambiguity: &HashMap<String, Vec<String>>) -> bool {
    ambiguity
        .values()
        .any(|ids| ids.iter().any(|id| id.ends_with(source_id)))
}

fn correction_ledger_evidence_ref(event_id: &str) -> EvidenceRef {
    EvidenceRef::FilePath(format!(".openmesh/events/ledger/{event_id}.json"))
}

fn correction_visibility_limitation(presentation: &EffectiveEventPresentation) -> String {
    truncate_summary(&format!(
        "work event {} presentation corrected by {}",
        presentation.event_id,
        presentation.correction_event_ids.join(", ")
    ))
}

fn limitation_from_correction_diagnostic(diagnostic: &CorrectionSemanticDiagnostic) -> String {
    match diagnostic {
        CorrectionSemanticDiagnostic::SelfCorrection { event_id } => {
            format!("invalid correction: event {event_id} cannot correct itself")
        }
        CorrectionSemanticDiagnostic::MissingTarget {
            correction_event_id,
            target_id,
        } => format!("invalid correction {correction_event_id}: missing target {target_id}"),
        CorrectionSemanticDiagnostic::CorrectionCycle { path } => {
            format!("correction cycle detected: {}", path.join(" -> "))
        }
        CorrectionSemanticDiagnostic::InvalidCorrectionSemantics {
            correction_event_id,
        } => {
            format!("invalid correction semantics for {correction_event_id}")
        }
    }
}

fn item_from_work_event(
    event: &WorkEvent,
    presentation: &EffectiveEventPresentation,
) -> ContinuityStateItem {
    let mut evidence_refs: Vec<EvidenceRef> = event
        .evidence
        .iter()
        .map(|attachment| attachment.evidence_ref.clone())
        .collect();
    for correction_id in &presentation.correction_event_ids {
        evidence_refs.push(correction_ledger_evidence_ref(correction_id));
    }
    ContinuityStateItem {
        id: format!("event:{}", event.event_id),
        summary: truncate_summary(presentation.summary_text()),
        kind: presentation.kind_text().to_string(),
        source: ContinuitySourceKind::WorkEvent,
        source_id: event.event_id.clone(),
        producer: "work-event-ledger".into(),
        timestamp: event.timestamp.clone(),
        evidence_refs: bound_evidence_refs(&evidence_refs),
        confidence: presentation.confidence,
        correlation_hint: None,
        unverified: None,
    }
}

fn item_from_correction_in_window(
    correction: &WorkEvent,
    target: &WorkEvent,
    presentation: &EffectiveEventPresentation,
) -> ContinuityStateItem {
    let mut evidence_refs: Vec<EvidenceRef> = target
        .evidence
        .iter()
        .map(|attachment| attachment.evidence_ref.clone())
        .collect();
    for attachment in &correction.evidence {
        evidence_refs.push(attachment.evidence_ref.clone());
    }
    evidence_refs.push(correction_ledger_evidence_ref(&target.event_id));
    evidence_refs.push(correction_ledger_evidence_ref(&correction.event_id));
    ContinuityStateItem {
        id: format!(
            "event:{}:corrected-by:{}",
            target.event_id, correction.event_id
        ),
        summary: truncate_summary(&format!(
            "corrected {} (effective: {})",
            target.event_id,
            presentation.summary_text()
        )),
        kind: presentation.kind_text().to_string(),
        source: ContinuitySourceKind::WorkEvent,
        source_id: correction.event_id.clone(),
        producer: "work-event-ledger".into(),
        timestamp: correction.timestamp.clone(),
        evidence_refs: bound_evidence_refs(&evidence_refs),
        confidence: ContinuityConfidence::Medium,
        correlation_hint: Some(target.event_id.clone()),
        unverified: None,
    }
}

fn item_from_signal(
    signal: &WorkSignal,
    source: ContinuitySourceKind,
    ambiguity: &HashMap<String, Vec<String>>,
) -> ContinuityStateItem {
    let ambiguous = is_ambiguous_source(&signal.signal_id, ambiguity);
    ContinuityStateItem {
        id: format!("signal:{}", signal.signal_id),
        summary: truncate_summary(&signal.summary),
        kind: signal_kind_label(signal.kind),
        source,
        source_id: signal.signal_id.clone(),
        producer: producer_label(&signal.producer),
        timestamp: signal.timestamp.clone(),
        evidence_refs: bound_evidence_refs(&signal.evidence_refs),
        confidence: if ambiguous {
            ContinuityConfidence::Ambiguous
        } else if source == ContinuitySourceKind::PendingSignal {
            ContinuityConfidence::Low
        } else {
            ContinuityConfidence::Medium
        },
        correlation_hint: signal.correlation_hint.clone(),
        unverified: if source == ContinuitySourceKind::PendingSignal {
            Some(true)
        } else {
            None
        },
    }
}

fn item_from_promotion_audit(record: &PromotionDecisionRecord) -> ContinuityStateItem {
    ContinuityStateItem {
        id: format!("audit:{}", record.promotion_key.as_str()),
        summary: promotion_summary(record),
        kind: promotion_kind_label(record.outcome),
        source: ContinuitySourceKind::PromotionAudit,
        source_id: record.promotion_key.as_str().to_string(),
        producer: "promotion-audit".into(),
        timestamp: record.recorded_at.clone(),
        evidence_refs: Vec::new(),
        confidence: match record.outcome {
            PromotionOutcome::Ambiguous => ContinuityConfidence::Ambiguous,
            PromotionOutcome::Suppress => ContinuityConfidence::Low,
            _ => ContinuityConfidence::Medium,
        },
        correlation_hint: record.correlation_hint.clone(),
        unverified: None,
    }
}

fn item_from_diagnostic(diagnostic: &ContinuityDiagnostic, timestamp: &str) -> ContinuityStateItem {
    ContinuityStateItem {
        id: format!("signal:diagnostic-{}", fnv1a_hex(&diagnostic.location)),
        summary: truncate_summary(&diagnostic.message),
        kind: "review-required".into(),
        source: ContinuitySourceKind::ProcessedSignal,
        source_id: diagnostic.location.clone(),
        producer: "continuity-reader".into(),
        timestamp: timestamp.to_string(),
        evidence_refs: Vec::new(),
        confidence: ContinuityConfidence::Low,
        correlation_hint: None,
        unverified: None,
    }
}

fn item_from_pending_attention(attention: &PendingAttentionItem) -> ContinuityStateItem {
    let id_prefix = match attention.source {
        ContinuitySourceKind::WorkEvent => "event",
        ContinuitySourceKind::ProcessedSignal | ContinuitySourceKind::PendingSignal => "signal",
        ContinuitySourceKind::PromotionAudit => "audit",
    };
    ContinuityStateItem {
        id: format!("{id_prefix}:{}", attention.source_id),
        summary: truncate_summary(&attention.summary),
        kind: pending_attention_kind_label(attention.reason),
        source: attention.source,
        source_id: attention.source_id.clone(),
        producer: "pending-attention".into(),
        timestamp: attention.timestamp.clone(),
        evidence_refs: bound_evidence_refs(&attention.evidence_refs),
        confidence: ContinuityConfidence::Medium,
        correlation_hint: None,
        unverified: if attention.source == ContinuitySourceKind::PendingSignal {
            Some(true)
        } else {
            None
        },
    }
}

fn pending_attention_kind_label(reason: crate::domain::PendingAttentionReason) -> String {
    serde_json::to_value(reason)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "review-required".into())
}

fn promotion_summary(record: &PromotionDecisionRecord) -> String {
    if let Some(detail) = &record.reason_detail {
        if !detail.trim().is_empty() {
            return truncate_summary(detail);
        }
    }
    truncate_summary(&format!(
        "promotion {:?} for {} signal(s)",
        record.outcome,
        record.source_signal_ids.len()
    ))
}

fn promotion_kind_label(outcome: PromotionOutcome) -> String {
    match outcome {
        PromotionOutcome::Promote => "promoted".into(),
        PromotionOutcome::Suppress => "suppressed".into(),
        PromotionOutcome::Defer => "deferred".into(),
        PromotionOutcome::Ambiguous => "ambiguous".into(),
    }
}

fn signal_kind_label(kind: WorkSignalKind) -> String {
    serde_json::to_value(kind)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "unknown".into())
}

fn producer_label(producer: &ProducerRef) -> String {
    match producer {
        ProducerRef::Native => "native".into(),
        ProducerRef::Heli => "heli".into(),
        ProducerRef::Git => "git".into(),
        ProducerRef::Reporter(name) => format!("reporter:{name}"),
    }
}

fn catch_up_section_for_signal_kind(kind: WorkSignalKind) -> CatchUpSection {
    match kind {
        WorkSignalKind::Milestone | WorkSignalKind::BlockerResolved => CatchUpSection::Completed,
        WorkSignalKind::Progress | WorkSignalKind::Handoff | WorkSignalKind::ScopeChange => {
            CatchUpSection::Changed
        }
        WorkSignalKind::Blocker => CatchUpSection::Blocked,
        WorkSignalKind::Decision => CatchUpSection::Decided,
        WorkSignalKind::ReviewRequired | WorkSignalKind::UnresolvedQuestion => {
            CatchUpSection::NeedsAttention
        }
        WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch => CatchUpSection::StillOpen,
    }
}

fn catch_up_section_for_event(kind: &str) -> CatchUpSection {
    let normalized = kind.to_ascii_lowercase();
    if normalized.contains("completed")
        || normalized.contains("milestone")
        || normalized.contains("resolved")
    {
        CatchUpSection::Completed
    } else if normalized.contains("block") {
        CatchUpSection::Blocked
    } else if normalized.contains("decision") {
        CatchUpSection::Decided
    } else if normalized.contains("progress")
        || normalized.contains("handoff")
        || normalized.contains("scope")
        || normalized.contains("changed")
    {
        CatchUpSection::Changed
    } else if normalized.contains("question")
        || normalized.contains("review")
        || normalized.contains("attention")
        || normalized.contains("unresolved")
    {
        CatchUpSection::NeedsAttention
    } else {
        CatchUpSection::StillOpen
    }
}

fn catch_up_section_for_promotion_outcome(outcome: PromotionOutcome) -> CatchUpSection {
    match outcome {
        PromotionOutcome::Promote => CatchUpSection::Decided,
        PromotionOutcome::Suppress | PromotionOutcome::Ambiguous => CatchUpSection::NeedsAttention,
        PromotionOutcome::Defer => CatchUpSection::StillOpen,
    }
}

fn push_catch_up_section(
    sections: &mut CatchUpSections,
    target: CatchUpSection,
    item: ContinuityStateItem,
) {
    match target {
        CatchUpSection::Completed => sections.completed.push(item),
        CatchUpSection::Changed => sections.changed.push(item),
        CatchUpSection::Blocked => sections.blocked.push(item),
        CatchUpSection::Decided => sections.decided.push(item),
        CatchUpSection::NeedsAttention => sections.needs_attention.push(item),
        CatchUpSection::StillOpen => sections.still_open.push(item),
    }
}

fn sort_catch_up_sections(sections: &mut CatchUpSections) {
    for items in [
        &mut sections.completed,
        &mut sections.changed,
        &mut sections.blocked,
        &mut sections.decided,
        &mut sections.needs_attention,
        &mut sections.still_open,
    ] {
        items.sort_by(|a, b| a.id.cmp(&b.id));
        items.dedup_by(|a, b| a.id == b.id);
    }
}

fn limitation_from_diagnostic(diagnostic: &ContinuityDiagnostic) -> String {
    truncate_summary(&format!("{}: {}", diagnostic.location, diagnostic.message))
}

fn truncate_summary(summary: &str) -> String {
    if summary.len() <= MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES {
        summary.to_string()
    } else {
        summary[..MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES].to_string()
    }
}

fn bound_evidence_refs(refs: &[EvidenceRef]) -> Vec<EvidenceRef> {
    refs.iter()
        .take(MAX_CONTINUITY_ITEM_EVIDENCE_REFS)
        .cloned()
        .collect()
}

fn collect_evidence_refs(refs: &[EvidenceRef], set: &mut BTreeSet<String>) {
    for evidence in refs {
        if let Ok(encoded) = serde_json::to_string(evidence) {
            set.insert(encoded);
        }
    }
}

fn decode_evidence_set(set: &BTreeSet<String>) -> Vec<EvidenceRef> {
    set.iter()
        .filter_map(|encoded| serde_json::from_str(encoded).ok())
        .take(MAX_CATCH_UP_EVIDENCE_REFS)
        .collect()
}

fn fnv1a_hex(input: &str) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SourceCounts;

    fn empty_snapshot(workspace_id: &str) -> ContinuityInputSnapshot {
        ContinuityInputSnapshot {
            workspace_id: workspace_id.into(),
            loaded_at: "2026-07-16T10:00:00Z".into(),
            pending_signals: vec![],
            processed_signals: vec![],
            quarantine_signals: vec![],
            duplicate_signals: vec![],
            work_events: vec![],
            promotion_audit_records: vec![],
            diagnostics: vec![],
            source_counts: SourceCounts {
                work_events: 0,
                processed_signals: 0,
                pending_signals: 0,
                promotion_audit_records: 0,
                quarantine_signals: 0,
                duplicate_signals: 0,
                reporter_signals: 0,
                git_signals: 0,
                heli_signals: 0,
                unknown_producer_signals: 0,
                other_producer_signals: 0,
            },
        }
    }

    fn empty_current_state(workspace_id: &str) -> CurrentStateProjection {
        crate::continuity::build_current_state_projection(&empty_snapshot(workspace_id)).unwrap()
    }

    #[test]
    fn rejects_inverted_window() {
        let window = CatchUpWindow {
            since: "2026-07-16T12:00:00Z".into(),
            until: "2026-07-16T10:00:00Z".into(),
        };
        let err = build_catch_up_view(
            &empty_snapshot("ws-1"),
            &empty_current_state("ws-1"),
            &window,
        )
        .expect_err("inverted");
        assert!(matches!(
            err,
            ContinuityError::Validation(ContinuityValidationError::CatchUpWindowInverted)
        ));
    }
}
