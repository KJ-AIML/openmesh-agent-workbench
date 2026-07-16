//! Current State projection builder and persistence (Dev Track 0.1.3.7 Checkpoint C).

use crate::continuity::readers::{
    load_continuity_input_snapshot, ContinuityDiagnostic, ContinuityInputSnapshot,
};
use crate::domain::ProducerRef;
use crate::domain::{
    pending_attention_priority_for_severity, validate_current_state_projection,
    ContinuityConfidence, ContinuitySourceKind, ContinuityStateItem, ContinuityValidationError,
    CurrentStateProjection, CurrentStateSections, EvidenceRef, PendingAttentionItem,
    PendingAttentionReason, PendingAttentionSeverity, PendingAttentionStatus, WorkEvent,
    WorkSignal, WorkSignalKind, CURRENT_STATE_PROJECTION_PROTOCOL_VERSION,
    MAX_CONTINUITY_ITEM_EVIDENCE_REFS, MAX_CONTINUITY_STATE_ITEM_SUMMARY_BYTES,
    MAX_PROJECTION_EVIDENCE_REFS, MAX_PROJECTION_LIMITATIONS,
};
use crate::promotion::{PromotionDecisionRecord, PromotionOutcome};
use crate::storage::get_project_dir;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;

const CURRENT_STATE_FILENAME: &str = "current-state.json";

/// Errors from Current State build or persistence.
#[derive(Debug, thiserror::Error)]
pub enum ContinuityError {
    #[error("continuity reader error: {0}")]
    Reader(#[from] crate::continuity::readers::ContinuityReaderError),
    #[error("continuity validation error: {0}")]
    Validation(#[from] ContinuityValidationError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("current state projection not found")]
    ProjectionNotFound,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionTarget {
    Completed,
    InProgress,
    Blocked,
    Decisions,
    NeedsAttention,
    StillOpen,
}

/// Path to the persisted Current State projection file.
pub fn current_state_projection_path(project_path: &str) -> PathBuf {
    projections_dir(project_path).join(CURRENT_STATE_FILENAME)
}

/// Directory for rebuildable continuity projections.
pub fn projections_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("projections")
}

/// Builds a `CurrentStateProjection` deterministically from a loaded input snapshot.
pub fn build_current_state_projection(
    snapshot: &ContinuityInputSnapshot,
) -> Result<CurrentStateProjection, ContinuityError> {
    let mut sections = CurrentStateSections {
        completed: Vec::new(),
        in_progress: Vec::new(),
        blocked: Vec::new(),
        decisions: Vec::new(),
        needs_attention: Vec::new(),
        still_open: Vec::new(),
    };
    let mut pending_attention = Vec::new();
    let mut limitations = Vec::new();
    let mut evidence_set: BTreeSet<String> = BTreeSet::new();

    let ambiguity = detect_correlation_ambiguity(snapshot);

    for event in &snapshot.work_events {
        let item = item_from_work_event(event);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_to_section(&mut sections, section_for_work_event(&event.kind), item);
    }

    for signal in &snapshot.processed_signals {
        let item = item_from_signal(signal, ContinuitySourceKind::ProcessedSignal, &ambiguity);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_to_section(&mut sections, section_for_signal_kind(signal.kind), item);
    }

    for signal in &snapshot.pending_signals {
        let item = item_from_signal(signal, ContinuitySourceKind::PendingSignal, &ambiguity);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_to_section(&mut sections, SectionTarget::StillOpen, item.clone());
        pending_attention.push(attention_from_pending_signal(signal, &item));
    }

    for record in &snapshot.promotion_audit_records {
        let item = item_from_promotion_audit(record);
        collect_evidence_refs(&item.evidence_refs, &mut evidence_set);
        push_to_section(
            &mut sections,
            section_for_promotion_outcome(record.outcome),
            item.clone(),
        );
        if let Some(attention) = attention_from_promotion_audit(record, &item) {
            pending_attention.push(attention);
        }
    }

    for signal in snapshot
        .processed_signals
        .iter()
        .chain(snapshot.pending_signals.iter())
    {
        if matches!(
            signal.kind,
            WorkSignalKind::Blocker
                | WorkSignalKind::UnresolvedQuestion
                | WorkSignalKind::ReviewRequired
        ) {
            let item = item_from_signal(
                signal,
                if snapshot
                    .pending_signals
                    .iter()
                    .any(|s| s.signal_id == signal.signal_id)
                {
                    ContinuitySourceKind::PendingSignal
                } else {
                    ContinuitySourceKind::ProcessedSignal
                },
                &ambiguity,
            );
            if let Some(attention) = attention_from_signal_kind(signal, &item) {
                if !pending_attention
                    .iter()
                    .any(|a| a.source_id == signal.signal_id)
                {
                    pending_attention.push(attention);
                }
            }
        }
    }

    for diagnostic in &snapshot.diagnostics {
        limitations.push(limitation_from_diagnostic(diagnostic));
        pending_attention.push(attention_from_diagnostic(diagnostic, &snapshot.loaded_at));
    }

    if snapshot.work_events.is_empty()
        && (!snapshot.processed_signals.is_empty() || !snapshot.pending_signals.is_empty())
    {
        limitations
            .push("projection built from signals only; no WorkEvents exist in the ledger".into());
    }

    for (hint, source_ids) in &ambiguity {
        limitations.push(format!(
            "ambiguous correlation hint {hint}: conflicting sources {}",
            source_ids.join(", ")
        ));
    }

    if snapshot.diagnostics.is_empty()
        && snapshot.work_events.is_empty()
        && snapshot.processed_signals.is_empty()
        && snapshot.pending_signals.is_empty()
        && snapshot.promotion_audit_records.is_empty()
    {
        limitations.push("no continuity inputs were available for projection".into());
    }

    sort_section_items(&mut sections);
    pending_attention.sort_by(|a, b| a.id.cmp(&b.id));
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_PROJECTION_LIMITATIONS);

    let evidence_refs = decode_evidence_set(&evidence_set);
    let rebuild_inputs_hash = compute_rebuild_inputs_hash(snapshot);

    let projection = CurrentStateProjection {
        workspace_id: snapshot.workspace_id.clone(),
        generated_at: utc_now(),
        protocol_version: CURRENT_STATE_PROJECTION_PROTOCOL_VERSION.into(),
        sections,
        pending_attention,
        source_counts: snapshot.source_counts.clone(),
        evidence_refs,
        limitations,
        rebuild_inputs_hash,
    };

    validate_current_state_projection(&projection)?;
    Ok(projection)
}

/// Rebuilds and persists the Current State projection from local records.
pub fn rebuild_current_state_projection(
    project_path: &str,
) -> Result<CurrentStateProjection, ContinuityError> {
    let snapshot = load_continuity_input_snapshot(project_path)?;
    let projection = build_current_state_projection(&snapshot)?;
    write_current_state_projection(project_path, &projection)?;
    Ok(projection)
}

/// Reads a persisted Current State projection when present.
pub fn read_current_state_projection(
    project_path: &str,
) -> Result<CurrentStateProjection, ContinuityError> {
    let path = current_state_projection_path(project_path);
    if !path.exists() {
        return Err(ContinuityError::ProjectionNotFound);
    }
    let raw = fs::read_to_string(&path)?;
    let projection: CurrentStateProjection = serde_json::from_str(&raw)?;
    validate_current_state_projection(&projection)?;
    Ok(projection)
}

/// Validates and atomically writes the Current State projection file.
pub fn write_current_state_projection(
    project_path: &str,
    projection: &CurrentStateProjection,
) -> Result<(), ContinuityError> {
    validate_current_state_projection(projection)?;
    let dir = projections_dir(project_path);
    fs::create_dir_all(&dir)?;
    let final_path = dir.join(CURRENT_STATE_FILENAME);
    let temp_path = dir.join(format!("{CURRENT_STATE_FILENAME}.tmp"));
    let payload = serde_json::to_string_pretty(projection)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;
    file.write_all(payload.as_bytes())?;
    file.flush()?;
    match fs::rename(&temp_path, &final_path) {
        Ok(()) => Ok(()),
        Err(_err) if final_path.exists() => {
            let _ = fs::remove_file(&temp_path);
            fs::write(&final_path, payload)?;
            Ok(())
        }
        Err(err) => Err(ContinuityError::Io(err)),
    }
}

fn utc_now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
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

fn compute_rebuild_inputs_hash(snapshot: &ContinuityInputSnapshot) -> String {
    let mut signal_ids: Vec<String> = snapshot
        .processed_signals
        .iter()
        .chain(snapshot.pending_signals.iter())
        .map(|s| s.signal_id.clone())
        .collect();
    signal_ids.sort();
    let mut event_ids: Vec<String> = snapshot
        .work_events
        .iter()
        .map(|e| e.event_id.clone())
        .collect();
    event_ids.sort();
    let mut audit_keys: Vec<String> = snapshot
        .promotion_audit_records
        .iter()
        .map(|r| r.promotion_key.as_str().to_string())
        .collect();
    audit_keys.sort();
    let material = format!(
        "ws={};signals={};events={};audit={};qa={};du={};diag={}",
        snapshot.workspace_id,
        signal_ids.join(","),
        event_ids.join(","),
        audit_keys.join(","),
        snapshot.quarantine_signals.len(),
        snapshot.duplicate_signals.len(),
        snapshot.diagnostics.len(),
    );
    format!("fnv1a-{}", fnv1a_hex(&material))
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

fn item_from_work_event(event: &WorkEvent) -> ContinuityStateItem {
    let evidence_refs = bound_evidence_refs(
        &event
            .evidence
            .iter()
            .map(|attachment| attachment.evidence_ref.clone())
            .collect::<Vec<_>>(),
    );
    ContinuityStateItem {
        id: format!("event:{}", event.event_id),
        summary: truncate_summary(&event.summary),
        kind: event.kind.clone(),
        source: ContinuitySourceKind::WorkEvent,
        source_id: event.event_id.clone(),
        producer: "work-event-ledger".into(),
        timestamp: event.timestamp.clone(),
        evidence_refs,
        confidence: if event.corrects_event_id.is_some() {
            ContinuityConfidence::Medium
        } else {
            ContinuityConfidence::High
        },
        correlation_hint: None,
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

fn attention_from_pending_signal(
    signal: &WorkSignal,
    item: &ContinuityStateItem,
) -> PendingAttentionItem {
    PendingAttentionItem {
        id: attention_id(ContinuitySourceKind::PendingSignal, &signal.signal_id),
        summary: item.summary.clone(),
        reason: PendingAttentionReason::PendingSignal,
        source: ContinuitySourceKind::PendingSignal,
        source_id: signal.signal_id.clone(),
        timestamp: signal.timestamp.clone(),
        evidence_refs: item.evidence_refs.clone(),
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::Medium,
        priority: pending_attention_priority_for_severity(PendingAttentionSeverity::Medium),
    }
}

fn attention_from_signal_kind(
    signal: &WorkSignal,
    item: &ContinuityStateItem,
) -> Option<PendingAttentionItem> {
    let (reason, severity) = match signal.kind {
        WorkSignalKind::Blocker => (
            PendingAttentionReason::Blocker,
            PendingAttentionSeverity::High,
        ),
        WorkSignalKind::UnresolvedQuestion => (
            PendingAttentionReason::UnresolvedQuestion,
            PendingAttentionSeverity::High,
        ),
        WorkSignalKind::ReviewRequired => (
            PendingAttentionReason::ReviewRequired,
            PendingAttentionSeverity::Medium,
        ),
        _ => return None,
    };
    let source = item.source;
    Some(PendingAttentionItem {
        id: attention_id(source, &signal.signal_id),
        summary: item.summary.clone(),
        reason,
        source,
        source_id: signal.signal_id.clone(),
        timestamp: signal.timestamp.clone(),
        evidence_refs: item.evidence_refs.clone(),
        status: PendingAttentionStatus::Open,
        severity,
        priority: pending_attention_priority_for_severity(severity),
    })
}

fn attention_from_promotion_audit(
    record: &PromotionDecisionRecord,
    item: &ContinuityStateItem,
) -> Option<PendingAttentionItem> {
    let (reason, severity) = match record.outcome {
        PromotionOutcome::Ambiguous => (
            PendingAttentionReason::AmbiguousPromotion,
            PendingAttentionSeverity::High,
        ),
        PromotionOutcome::Suppress => (
            PendingAttentionReason::SuppressedPromotion,
            PendingAttentionSeverity::Medium,
        ),
        _ => return None,
    };
    Some(PendingAttentionItem {
        id: attention_id(
            ContinuitySourceKind::PromotionAudit,
            record.promotion_key.as_str(),
        ),
        summary: item.summary.clone(),
        reason,
        source: ContinuitySourceKind::PromotionAudit,
        source_id: record.promotion_key.as_str().to_string(),
        timestamp: record.recorded_at.clone(),
        evidence_refs: item.evidence_refs.clone(),
        status: PendingAttentionStatus::Open,
        severity,
        priority: pending_attention_priority_for_severity(severity),
    })
}

fn attention_from_diagnostic(
    diagnostic: &ContinuityDiagnostic,
    loaded_at: &str,
) -> PendingAttentionItem {
    let source_id = diagnostic.location.clone();
    PendingAttentionItem {
        id: attention_id(ContinuitySourceKind::ProcessedSignal, &source_id),
        summary: truncate_summary(&diagnostic.message),
        reason: PendingAttentionReason::ReviewRequired,
        source: ContinuitySourceKind::ProcessedSignal,
        source_id,
        timestamp: loaded_at.to_string(),
        evidence_refs: Vec::new(),
        status: PendingAttentionStatus::Open,
        severity: PendingAttentionSeverity::Medium,
        priority: pending_attention_priority_for_severity(PendingAttentionSeverity::Medium),
    }
}

fn attention_id(source: ContinuitySourceKind, source_id: &str) -> String {
    let prefix = match source {
        ContinuitySourceKind::WorkEvent => "event",
        ContinuitySourceKind::ProcessedSignal | ContinuitySourceKind::PendingSignal => "signal",
        ContinuitySourceKind::PromotionAudit => "audit",
    };
    format!("attention:{prefix}:{source_id}")
}

fn limitation_from_diagnostic(diagnostic: &ContinuityDiagnostic) -> String {
    truncate_summary(&format!("{}: {}", diagnostic.location, diagnostic.message))
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

fn section_for_signal_kind(kind: WorkSignalKind) -> SectionTarget {
    match kind {
        WorkSignalKind::Milestone | WorkSignalKind::BlockerResolved => SectionTarget::Completed,
        WorkSignalKind::Progress | WorkSignalKind::Handoff => SectionTarget::InProgress,
        WorkSignalKind::Blocker => SectionTarget::Blocked,
        WorkSignalKind::Decision | WorkSignalKind::ScopeChange => SectionTarget::Decisions,
        WorkSignalKind::ReviewRequired | WorkSignalKind::UnresolvedQuestion => {
            SectionTarget::NeedsAttention
        }
        WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch => SectionTarget::StillOpen,
    }
}

fn section_for_work_event(kind: &str) -> SectionTarget {
    let normalized = kind.to_ascii_lowercase();
    if normalized.contains("completed")
        || normalized.contains("milestone")
        || normalized.contains("resolved")
    {
        SectionTarget::Completed
    } else if normalized.contains("block") {
        SectionTarget::Blocked
    } else if normalized.contains("decision") || normalized.contains("scope") {
        SectionTarget::Decisions
    } else if normalized.contains("progress") || normalized.contains("handoff") {
        SectionTarget::InProgress
    } else if normalized.contains("question")
        || normalized.contains("review")
        || normalized.contains("attention")
        || normalized.contains("unresolved")
    {
        SectionTarget::NeedsAttention
    } else {
        SectionTarget::StillOpen
    }
}

fn section_for_promotion_outcome(outcome: PromotionOutcome) -> SectionTarget {
    match outcome {
        PromotionOutcome::Promote => SectionTarget::Decisions,
        PromotionOutcome::Suppress | PromotionOutcome::Ambiguous => SectionTarget::NeedsAttention,
        PromotionOutcome::Defer => SectionTarget::StillOpen,
    }
}

fn push_to_section(
    sections: &mut CurrentStateSections,
    target: SectionTarget,
    item: ContinuityStateItem,
) {
    match target {
        SectionTarget::Completed => sections.completed.push(item),
        SectionTarget::InProgress => sections.in_progress.push(item),
        SectionTarget::Blocked => sections.blocked.push(item),
        SectionTarget::Decisions => sections.decisions.push(item),
        SectionTarget::NeedsAttention => sections.needs_attention.push(item),
        SectionTarget::StillOpen => sections.still_open.push(item),
    }
}

fn sort_section_items(sections: &mut CurrentStateSections) {
    for items in [
        &mut sections.completed,
        &mut sections.in_progress,
        &mut sections.blocked,
        &mut sections.decisions,
        &mut sections.needs_attention,
        &mut sections.still_open,
    ] {
        items.sort_by(|a, b| a.id.cmp(&b.id));
    }
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
        .take(MAX_PROJECTION_EVIDENCE_REFS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::SourceCounts;

    fn empty_snapshot(workspace_id: &str) -> ContinuityInputSnapshot {
        ContinuityInputSnapshot {
            workspace_id: workspace_id.into(),
            loaded_at: "2026-07-16T10:00:00Z".into(),
            pending_signals: Vec::new(),
            processed_signals: Vec::new(),
            quarantine_signals: Vec::new(),
            duplicate_signals: Vec::new(),
            work_events: Vec::new(),
            promotion_audit_records: Vec::new(),
            diagnostics: Vec::new(),
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

    #[test]
    fn build_empty_snapshot_produces_valid_projection() {
        let snapshot = empty_snapshot("ws-empty");
        let projection = build_current_state_projection(&snapshot).expect("valid");
        assert_eq!(projection.workspace_id, "ws-empty");
        assert!(projection
            .limitations
            .iter()
            .any(|l| l.contains("no continuity inputs")));
    }

    #[test]
    fn rebuild_inputs_hash_is_prefixed() {
        let snapshot = empty_snapshot("ws-hash");
        let projection = build_current_state_projection(&snapshot).expect("valid");
        assert!(projection.rebuild_inputs_hash.starts_with("fnv1a-"));
    }
}
