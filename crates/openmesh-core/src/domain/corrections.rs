use super::events::{validate_event_semantics, WorkEvent};
use serde::{Deserialize, Serialize};

// ============================================================================
// Dev Track 0.1.3.8 Checkpoint A — WorkEvent correction semantics (pure, no I/O)
// ============================================================================

/// Authority/confidence for a continuity item — ambiguity must remain visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuityConfidence {
    High,
    Medium,
    Low,
    Ambiguous,
}

/// Default `kind` string for human-appended correction WorkEvents (Checkpoint B CLI).
pub const WORK_EVENT_CORRECTION_KIND: &str = "correction";

/// Diagnostic for invalid or unsupported correction relationships.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum CorrectionSemanticDiagnostic {
    SelfCorrection {
        event_id: String,
    },
    MissingTarget {
        correction_event_id: String,
        target_id: String,
    },
    CorrectionCycle {
        path: Vec<String>,
    },
    InvalidCorrectionSemantics {
        correction_event_id: String,
    },
}

/// Effective presentation derived from a WorkEvent and its direct correction chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EffectiveEventPresentation {
    pub event_id: String,
    pub effective_kind: String,
    pub effective_summary: String,
    pub original_kind: String,
    pub original_summary: String,
    pub is_corrected: bool,
    pub is_superseded_original: bool,
    pub correction_event_ids: Vec<String>,
    pub superseded_by_event_id: Option<String>,
    pub confidence: ContinuityConfidence,
    pub diagnostics: Vec<CorrectionSemanticDiagnostic>,
}

impl EffectiveEventPresentation {
    pub fn kind_text(&self) -> &str {
        &self.effective_kind
    }

    pub fn summary_text(&self) -> &str {
        &self.effective_summary
    }

    pub fn original_kind_text(&self) -> &str {
        &self.original_kind
    }

    pub fn original_summary_text(&self) -> &str {
        &self.original_summary
    }
}

/// Finds a WorkEvent by `event_id` in an in-memory set.
pub fn find_work_event<'a>(events: &'a [WorkEvent], event_id: &str) -> Option<&'a WorkEvent> {
    events.iter().find(|event| event.event_id == event_id)
}

/// Lists correction events that directly reference `target_id` via `correctsEventId`.
pub fn direct_corrections_for<'a>(events: &'a [WorkEvent], target_id: &str) -> Vec<&'a WorkEvent> {
    events
        .iter()
        .filter(|event| event.corrects_event_id.as_deref() == Some(target_id))
        .collect()
}

/// Sorts correction references by timestamp ascending, then `event_id` lexicographically.
pub fn sort_corrections_deterministic(corrections: &mut [&WorkEvent]) {
    corrections.sort_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
}

/// Picks the winning correction: latest `timestamp`, then lexicographic `event_id`.
pub fn select_latest_correction<'a>(corrections: &[&'a WorkEvent]) -> Option<&'a WorkEvent> {
    corrections.iter().copied().max_by(|left, right| {
        left.timestamp
            .cmp(&right.timestamp)
            .then_with(|| left.event_id.cmp(&right.event_id))
    })
}

/// Returns ordered correction event ids for `target_id` (deterministic sort).
pub fn correction_chain_event_ids(events: &[WorkEvent], target_id: &str) -> Vec<String> {
    let mut corrections: Vec<&WorkEvent> = direct_corrections_for(events, target_id);
    sort_corrections_deterministic(&mut corrections);
    corrections
        .into_iter()
        .map(|event| event.event_id.clone())
        .collect()
}

/// True when `event_id` is a correction target with at least one valid direct correction.
pub fn is_superseded_original(events: &[WorkEvent], event_id: &str) -> bool {
    effective_presentation(events, event_id)
        .map(|presentation| presentation.is_superseded_original)
        .unwrap_or(false)
}

/// Validates a would-be correction against an in-memory event set (no panic on missing target).
pub fn validate_correction_relationship(
    correction: &WorkEvent,
    events: &[WorkEvent],
) -> Result<(), CorrectionSemanticDiagnostic> {
    let Some(target_id) = correction.corrects_event_id.as_deref() else {
        return Ok(());
    };
    if correction.event_id == target_id {
        return Err(CorrectionSemanticDiagnostic::SelfCorrection {
            event_id: correction.event_id.clone(),
        });
    }
    if find_work_event(events, target_id).is_none() {
        return Err(CorrectionSemanticDiagnostic::MissingTarget {
            correction_event_id: correction.event_id.clone(),
            target_id: target_id.to_string(),
        });
    }
    if validate_event_semantics(correction).is_err() {
        return Err(CorrectionSemanticDiagnostic::InvalidCorrectionSemantics {
            correction_event_id: correction.event_id.clone(),
        });
    }
    let mut augmented = events.to_vec();
    if !augmented
        .iter()
        .any(|event| event.event_id == correction.event_id)
    {
        augmented.push(correction.clone());
    }
    if let Some(path) = correction_cycle_path(&augmented, &correction.event_id) {
        return Err(CorrectionSemanticDiagnostic::CorrectionCycle { path });
    }
    Ok(())
}

/// Returns valid direct corrections for `target_id` plus diagnostics for rejected candidates.
pub fn classify_direct_corrections<'a>(
    events: &'a [WorkEvent],
    target_id: &str,
) -> (Vec<&'a WorkEvent>, Vec<CorrectionSemanticDiagnostic>) {
    let mut diagnostics = Vec::new();
    if find_work_event(events, target_id).is_none() {
        return (Vec::new(), diagnostics);
    }

    let mut valid = Vec::new();
    for correction in direct_corrections_for(events, target_id) {
        match validate_correction_relationship(correction, events) {
            Ok(()) => valid.push(correction),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    sort_corrections_deterministic(&mut valid);
    (valid, diagnostics)
}

/// Effective `kind` for `original` using valid direct corrections only.
pub fn effective_kind_for(events: &[WorkEvent], original: &WorkEvent) -> String {
    if original.corrects_event_id.is_some() {
        return original.kind.clone();
    }
    let (valid, _) = classify_direct_corrections(events, &original.event_id);
    if let Some(latest) = select_latest_correction(&valid) {
        latest.kind.clone()
    } else {
        original.kind.clone()
    }
}

/// Effective `summary` for `original` using valid direct corrections only.
pub fn effective_summary_for(events: &[WorkEvent], original: &WorkEvent) -> String {
    if original.corrects_event_id.is_some() {
        return original.summary.clone();
    }
    let (valid, _) = classify_direct_corrections(events, &original.event_id);
    if let Some(latest) = select_latest_correction(&valid) {
        latest.summary.clone()
    } else {
        original.summary.clone()
    }
}

/// Confidence cap for effective presentation — corrected targets never exceed `medium`.
pub fn effective_confidence_for(
    events: &[WorkEvent],
    event_id: &str,
) -> Option<ContinuityConfidence> {
    effective_presentation(events, event_id).map(|presentation| presentation.confidence)
}

/// Builds effective presentation for `event_id` from an in-memory ledger snapshot.
pub fn effective_presentation(
    events: &[WorkEvent],
    event_id: &str,
) -> Option<EffectiveEventPresentation> {
    let original = find_work_event(events, event_id)?;
    let (valid_corrections, diagnostics) = classify_direct_corrections(events, event_id);

    if original.corrects_event_id.is_some() {
        return Some(EffectiveEventPresentation {
            event_id: original.event_id.clone(),
            effective_kind: original.kind.clone(),
            effective_summary: original.summary.clone(),
            original_kind: original.kind.clone(),
            original_summary: original.summary.clone(),
            is_corrected: false,
            is_superseded_original: false,
            correction_event_ids: Vec::new(),
            superseded_by_event_id: None,
            confidence: ContinuityConfidence::Medium,
            diagnostics,
        });
    }

    let is_corrected = !valid_corrections.is_empty();
    let latest = select_latest_correction(&valid_corrections);
    let correction_event_ids: Vec<String> = valid_corrections
        .iter()
        .map(|event| event.event_id.clone())
        .collect();
    let superseded_by_event_id = latest.map(|event| event.event_id.clone());

    let (effective_kind, effective_summary) = if let Some(latest) = latest {
        (latest.kind.clone(), latest.summary.clone())
    } else {
        (original.kind.clone(), original.summary.clone())
    };

    let confidence = if is_corrected {
        ContinuityConfidence::Medium
    } else {
        ContinuityConfidence::High
    };

    Some(EffectiveEventPresentation {
        event_id: original.event_id.clone(),
        effective_kind,
        effective_summary,
        original_kind: original.kind.clone(),
        original_summary: original.summary.clone(),
        is_corrected,
        is_superseded_original: is_corrected,
        correction_event_ids,
        superseded_by_event_id,
        confidence,
        diagnostics,
    })
}

/// Follows `correctsEventId` links from `start_event_id`; returns cycle path when found.
pub fn correction_cycle_path(events: &[WorkEvent], start_event_id: &str) -> Option<Vec<String>> {
    use std::collections::HashSet;

    let mut visited = HashSet::new();
    let mut path = Vec::new();
    let mut current = start_event_id.to_string();

    loop {
        if !visited.insert(current.clone()) {
            path.push(current);
            return Some(path);
        }
        path.push(current.clone());
        let event = find_work_event(events, &current)?;
        let target_id = event.corrects_event_id.as_deref()?;
        current = target_id.to_string();
    }
}
