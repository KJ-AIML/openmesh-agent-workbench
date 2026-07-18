//! Dev Track 0.1.5 Checkpoint B — pure deterministic context-pack selection.
//!
//! Operates only on already-loaded values. No filesystem, profile loading, continuity
//! building, persistence, CLI, or authority execution.

use crate::context::Sensitivity;
use crate::domain::{
    validate_evidence_ref, validate_utc_timestamp, ContextPackContinuityItem,
    ContextPackCorrectionProvenance, ContextPackDiagnostic, ContextPackDiagnosticSeverity,
    ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin, ContextPackItemProvenance,
    ContextPackPendingAttentionItem, ContextPackRedactionSummary, ContinuityConfidence,
    ContinuitySourceKind, EvidenceRef, PendingAttentionReason, PendingAttentionSeverity,
    PendingAttentionStatus, MAX_CONTEXT_PACK_DIAGNOSTICS, MAX_CONTEXT_PACK_EVIDENCE_INDEX,
    MAX_CONTEXT_PACK_EVIDENCE_LABEL_BYTES,
};

use std::collections::BTreeMap;

/// Classification for an already-loaded evidence candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextPackEvidenceCandidateKind {
    Normal,
    Malformed,
    Quarantined,
}

/// Already-loaded evidence candidate for deterministic index assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackEvidenceCandidate {
    pub evidence_ref: EvidenceRef,
    pub origin: ContextPackEvidenceOrigin,
    pub sensitivity: Sensitivity,
    pub safe_label: String,
    pub timestamp: Option<String>,
    pub provenance: ContextPackItemProvenance,
    pub correction: Option<ContextPackCorrectionProvenance>,
    pub policy_eligible: bool,
    pub kind: ContextPackEvidenceCandidateKind,
}

/// Options for evidence-index assembly (reserved for future deterministic filters).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ContextPackEvidenceSelectionOptions {}

/// Output of deterministic evidence-index assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackEvidenceSelectionResult {
    pub evidence_index: Vec<ContextPackEvidenceIndexEntry>,
    pub redaction_summary: ContextPackRedactionSummary,
    pub diagnostics: Vec<ContextPackDiagnostic>,
}

/// Already-loaded continuity item values for pack-safe sanitization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackContinuityItemInput {
    pub id: String,
    pub summary: String,
    pub kind: String,
    pub source: ContinuitySourceKind,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub confidence: ContinuityConfidence,
    pub unverified: Option<bool>,
    pub correction: Option<ContextPackCorrectionProvenance>,
    pub sensitivity: Sensitivity,
    pub policy_restricted: bool,
    pub malformed: bool,
    pub quarantined: bool,
}

/// Already-loaded pending-attention values for pack-safe sanitization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackPendingAttentionItemInput {
    pub id: String,
    pub summary: String,
    pub reason: PendingAttentionReason,
    pub provenance: ContextPackItemProvenance,
    pub timestamp: String,
    pub status: PendingAttentionStatus,
    pub severity: PendingAttentionSeverity,
    pub priority: u8,
    pub evidence_refs: Vec<EvidenceRef>,
    pub sensitivity: Sensitivity,
    pub policy_restricted: bool,
    pub malformed: bool,
    pub quarantined: bool,
}

/// Sanitized continuity items plus aggregate omission metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackContinuitySanitizationResult {
    pub items: Vec<ContextPackContinuityItem>,
    pub redaction_summary: ContextPackRedactionSummary,
    pub diagnostics: Vec<ContextPackDiagnostic>,
}

/// Sanitized pending-attention items plus aggregate omission metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPackPendingSanitizationResult {
    pub items: Vec<ContextPackPendingAttentionItem>,
    pub redaction_summary: ContextPackRedactionSummary,
    pub diagnostics: Vec<ContextPackDiagnostic>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextPackSelectionError {
    #[error("invalid candidate contract: {0}")]
    InvalidCandidate(String),
    #[error("unsupported evidence origin")]
    UnsupportedOrigin,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("inconsistent provenance for duplicate evidence ref")]
    InconsistentProvenance,
    #[error("impossible duplicate classification")]
    ImpossibleDuplicateClassification,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct RedactionAccumulator {
    secret_items_omitted: u32,
    policy_restricted_items_omitted: u32,
    malformed_items_omitted: u32,
    quarantined_items_omitted: u32,
    bounds_truncated_items: u32,
}

impl RedactionAccumulator {
    fn into_summary(self) -> ContextPackRedactionSummary {
        ContextPackRedactionSummary {
            secret_items_omitted: self.secret_items_omitted,
            policy_restricted_items_omitted: self.policy_restricted_items_omitted,
            malformed_items_omitted: self.malformed_items_omitted,
            quarantined_items_omitted: self.quarantined_items_omitted,
            bounds_truncated_items: self.bounds_truncated_items,
        }
    }

    fn merge(&mut self, other: &RedactionAccumulator) {
        self.secret_items_omitted = self
            .secret_items_omitted
            .saturating_add(other.secret_items_omitted);
        self.policy_restricted_items_omitted = self
            .policy_restricted_items_omitted
            .saturating_add(other.policy_restricted_items_omitted);
        self.malformed_items_omitted = self
            .malformed_items_omitted
            .saturating_add(other.malformed_items_omitted);
        self.quarantined_items_omitted = self
            .quarantined_items_omitted
            .saturating_add(other.quarantined_items_omitted);
        self.bounds_truncated_items = self
            .bounds_truncated_items
            .saturating_add(other.bounds_truncated_items);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MergeableEvidenceEntry {
    evidence_ref: EvidenceRef,
    origin: ContextPackEvidenceOrigin,
    sensitivity: Sensitivity,
    safe_label: String,
    timestamp: Option<String>,
    provenance: ContextPackItemProvenance,
    correction: Option<ContextPackCorrectionProvenance>,
}

/// Build a deterministic, bounded evidence index from already-loaded candidates.
pub fn build_context_pack_evidence_index(
    candidates: &[ContextPackEvidenceCandidate],
    options: &ContextPackEvidenceSelectionOptions,
) -> Result<ContextPackEvidenceSelectionResult, ContextPackSelectionError> {
    let _ = options;
    let mut redaction = RedactionAccumulator::default();
    let mut diagnostics = Vec::new();
    let mut groups: BTreeMap<String, Vec<ContextPackEvidenceCandidate>> = BTreeMap::new();

    for candidate in candidates {
        match candidate.kind {
            ContextPackEvidenceCandidateKind::Malformed => {
                redaction.malformed_items_omitted += 1;
                push_diagnostic(
                    &mut diagnostics,
                    "evidence-malformed",
                    "malformed evidence candidate omitted",
                    ContextPackDiagnosticSeverity::Warning,
                );
                continue;
            }
            ContextPackEvidenceCandidateKind::Quarantined => {
                redaction.quarantined_items_omitted += 1;
                push_diagnostic(
                    &mut diagnostics,
                    "evidence-quarantined",
                    "quarantined evidence candidate omitted",
                    ContextPackDiagnosticSeverity::Warning,
                );
                continue;
            }
            ContextPackEvidenceCandidateKind::Normal => {}
        }

        if let Some(correction) = &candidate.correction {
            if correction.is_superseded_original {
                continue;
            }
        }

        let key = match canonical_evidence_ref_key(&candidate.evidence_ref) {
            Ok(key) => key,
            Err(_) => {
                redaction.malformed_items_omitted += 1;
                push_diagnostic(
                    &mut diagnostics,
                    "evidence-malformed",
                    "malformed evidence candidate omitted",
                    ContextPackDiagnosticSeverity::Warning,
                );
                continue;
            }
        };

        validate_candidate_contract(candidate)?;

        groups.entry(key).or_default().push(candidate.clone());
    }

    let mut merged: BTreeMap<String, MergeableEvidenceEntry> = BTreeMap::new();
    for (key, group) in groups {
        if group
            .iter()
            .any(|candidate| candidate.sensitivity == Sensitivity::Secret)
        {
            redaction.secret_items_omitted += group.len() as u32;
            continue;
        }
        if group.iter().any(|candidate| !candidate.policy_eligible) {
            redaction.policy_restricted_items_omitted += group
                .iter()
                .filter(|candidate| !candidate.policy_eligible)
                .count() as u32;
            continue;
        }
        merged.insert(key, merge_candidate_group(&group)?);
    }

    let mut entries: Vec<MergeableEvidenceEntry> = merged.into_values().collect();
    entries.sort_by(|left, right| {
        canonical_evidence_ref_key(&left.evidence_ref)
            .unwrap()
            .cmp(&canonical_evidence_ref_key(&right.evidence_ref).unwrap())
    });

    if entries.len() > MAX_CONTEXT_PACK_EVIDENCE_INDEX {
        let omitted = entries.len() - MAX_CONTEXT_PACK_EVIDENCE_INDEX;
        redaction.bounds_truncated_items += omitted as u32;
        entries.truncate(MAX_CONTEXT_PACK_EVIDENCE_INDEX);
        push_diagnostic(
            &mut diagnostics,
            "evidence-truncated",
            "evidence index truncated to bound",
            ContextPackDiagnosticSeverity::Warning,
        );
    }

    let evidence_index = entries
        .into_iter()
        .enumerate()
        .map(|(index, entry)| ContextPackEvidenceIndexEntry {
            ref_id: stable_ref_id(index),
            evidence_ref: entry.evidence_ref,
            origin: entry.origin,
            sensitivity: entry.sensitivity,
            label: entry.safe_label,
            timestamp: entry.timestamp,
        })
        .collect();

    diagnostics.truncate(MAX_CONTEXT_PACK_DIAGNOSTICS);

    Ok(ContextPackEvidenceSelectionResult {
        evidence_index,
        redaction_summary: redaction.into_summary(),
        diagnostics,
    })
}

/// Sanitize already-loaded continuity items for pack embedding.
pub fn sanitize_context_pack_continuity_items(
    inputs: &[ContextPackContinuityItemInput],
) -> Result<ContextPackContinuitySanitizationResult, ContextPackSelectionError> {
    let mut redaction = RedactionAccumulator::default();
    let mut diagnostics = Vec::new();
    let mut items = Vec::new();

    for input in inputs {
        if input.malformed {
            redaction.malformed_items_omitted += 1;
            push_diagnostic(
                &mut diagnostics,
                "continuity-malformed",
                "malformed continuity item omitted",
                ContextPackDiagnosticSeverity::Warning,
            );
            continue;
        }
        if input.quarantined {
            redaction.quarantined_items_omitted += 1;
            push_diagnostic(
                &mut diagnostics,
                "continuity-quarantined",
                "quarantined continuity item omitted",
                ContextPackDiagnosticSeverity::Warning,
            );
            continue;
        }
        if input.sensitivity == Sensitivity::Secret || input.policy_restricted {
            if input.sensitivity == Sensitivity::Secret {
                redaction.secret_items_omitted += 1;
            } else {
                redaction.policy_restricted_items_omitted += 1;
            }
            continue;
        }
        if let Some(correction) = &input.correction {
            if correction.is_superseded_original {
                continue;
            }
        }

        let evidence = build_context_pack_evidence_index(
            &evidence_candidates_from_refs(
                &input.evidence_refs,
                input.provenance,
                input.correction.clone(),
                input.sensitivity.clone(),
            ),
            &ContextPackEvidenceSelectionOptions::default(),
        )?;
        redaction.merge(&RedactionAccumulator {
            secret_items_omitted: evidence.redaction_summary.secret_items_omitted,
            policy_restricted_items_omitted: evidence
                .redaction_summary
                .policy_restricted_items_omitted,
            malformed_items_omitted: evidence.redaction_summary.malformed_items_omitted,
            quarantined_items_omitted: evidence.redaction_summary.quarantined_items_omitted,
            bounds_truncated_items: evidence.redaction_summary.bounds_truncated_items,
        });
        diagnostics.extend(evidence.diagnostics);

        items.push(ContextPackContinuityItem {
            id: input.id.clone(),
            summary: input.summary.clone(),
            kind: input.kind.clone(),
            source: input.source,
            provenance: input.provenance,
            timestamp: input.timestamp.clone(),
            evidence_refs: evidence
                .evidence_index
                .into_iter()
                .map(|entry| entry.evidence_ref)
                .collect(),
            confidence: input.confidence,
            unverified: input.unverified,
            correction: input.correction.clone(),
        });
    }

    diagnostics.truncate(MAX_CONTEXT_PACK_DIAGNOSTICS);

    Ok(ContextPackContinuitySanitizationResult {
        items,
        redaction_summary: redaction.into_summary(),
        diagnostics,
    })
}

/// Sanitize already-loaded pending-attention items for pack embedding.
pub fn sanitize_context_pack_pending_items(
    inputs: &[ContextPackPendingAttentionItemInput],
) -> Result<ContextPackPendingSanitizationResult, ContextPackSelectionError> {
    let mut redaction = RedactionAccumulator::default();
    let mut diagnostics = Vec::new();
    let mut items = Vec::new();

    for input in inputs {
        if input.malformed {
            redaction.malformed_items_omitted += 1;
            push_diagnostic(
                &mut diagnostics,
                "pending-malformed",
                "malformed pending item omitted",
                ContextPackDiagnosticSeverity::Warning,
            );
            continue;
        }
        if input.quarantined {
            redaction.quarantined_items_omitted += 1;
            push_diagnostic(
                &mut diagnostics,
                "pending-quarantined",
                "quarantined pending item omitted",
                ContextPackDiagnosticSeverity::Warning,
            );
            continue;
        }
        if input.sensitivity == Sensitivity::Secret || input.policy_restricted {
            if input.sensitivity == Sensitivity::Secret {
                redaction.secret_items_omitted += 1;
            } else {
                redaction.policy_restricted_items_omitted += 1;
            }
            continue;
        }

        let evidence = build_context_pack_evidence_index(
            &evidence_candidates_from_refs(
                &input.evidence_refs,
                input.provenance,
                None,
                input.sensitivity.clone(),
            ),
            &ContextPackEvidenceSelectionOptions::default(),
        )?;
        redaction.merge(&RedactionAccumulator {
            secret_items_omitted: evidence.redaction_summary.secret_items_omitted,
            policy_restricted_items_omitted: evidence
                .redaction_summary
                .policy_restricted_items_omitted,
            malformed_items_omitted: evidence.redaction_summary.malformed_items_omitted,
            quarantined_items_omitted: evidence.redaction_summary.quarantined_items_omitted,
            bounds_truncated_items: evidence.redaction_summary.bounds_truncated_items,
        });
        diagnostics.extend(evidence.diagnostics);

        items.push(ContextPackPendingAttentionItem {
            id: input.id.clone(),
            summary: input.summary.clone(),
            reason: input.reason,
            provenance: input.provenance,
            timestamp: input.timestamp.clone(),
            status: input.status,
            severity: input.severity,
            priority: input.priority,
            evidence_refs: evidence
                .evidence_index
                .into_iter()
                .map(|entry| entry.evidence_ref)
                .collect(),
        });
    }

    diagnostics.truncate(MAX_CONTEXT_PACK_DIAGNOSTICS);

    Ok(ContextPackPendingSanitizationResult {
        items,
        redaction_summary: redaction.into_summary(),
        diagnostics,
    })
}

pub fn canonical_evidence_ref_key(
    evidence_ref: &EvidenceRef,
) -> Result<String, ContextPackSelectionError> {
    serde_json::to_string(evidence_ref).map_err(|err| {
        ContextPackSelectionError::InvalidCandidate(format!("evidence ref not canonical: {err}"))
    })
}

fn evidence_candidates_from_refs(
    evidence_refs: &[EvidenceRef],
    provenance: ContextPackItemProvenance,
    correction: Option<ContextPackCorrectionProvenance>,
    sensitivity: Sensitivity,
) -> Vec<ContextPackEvidenceCandidate> {
    evidence_refs
        .iter()
        .cloned()
        .map(|evidence_ref| ContextPackEvidenceCandidate {
            evidence_ref,
            origin: ContextPackEvidenceOrigin::ContinuityItem,
            sensitivity: sensitivity.clone(),
            safe_label: "continuity evidence".into(),
            timestamp: None,
            provenance,
            correction: correction.clone(),
            policy_eligible: sensitivity != Sensitivity::Secret,
            kind: ContextPackEvidenceCandidateKind::Normal,
        })
        .collect()
}

fn validate_candidate_contract(
    candidate: &ContextPackEvidenceCandidate,
) -> Result<(), ContextPackSelectionError> {
    if candidate.origin != ContextPackEvidenceOrigin::ContinuityItem {
        return Err(ContextPackSelectionError::UnsupportedOrigin);
    }
    if candidate.safe_label.trim().is_empty() {
        return Err(ContextPackSelectionError::InvalidCandidate(
            "safe_label is empty".into(),
        ));
    }
    if candidate.safe_label.len() > MAX_CONTEXT_PACK_EVIDENCE_LABEL_BYTES {
        return Err(ContextPackSelectionError::InvalidCandidate(format!(
            "safe_label exceeds {} bytes",
            MAX_CONTEXT_PACK_EVIDENCE_LABEL_BYTES
        )));
    }
    validate_evidence_ref(&candidate.evidence_ref).map_err(|err| {
        ContextPackSelectionError::InvalidCandidate(format!("invalid evidence ref: {err}"))
    })?;
    if let Some(timestamp) = &candidate.timestamp {
        validate_utc_timestamp(timestamp).map_err(ContextPackSelectionError::InvalidTimestamp)?;
    }
    if candidate.provenance == ContextPackItemProvenance::Confirmed
        && candidate.correction.is_some()
    {
        // corrected effective presentation may remain confirmed when explicitly supplied
    }
    Ok(())
}

fn merge_candidate_group(
    group: &[ContextPackEvidenceCandidate],
) -> Result<MergeableEvidenceEntry, ContextPackSelectionError> {
    if group.is_empty() {
        return Err(ContextPackSelectionError::InvalidCandidate(
            "empty merge group".into(),
        ));
    }

    let mut ordered = group.to_vec();
    ordered.sort_by(|left, right| {
        canonical_evidence_ref_key(&left.evidence_ref)
            .unwrap()
            .cmp(&canonical_evidence_ref_key(&right.evidence_ref).unwrap())
            .then_with(|| left.safe_label.cmp(&right.safe_label))
            .then_with(|| left.timestamp.cmp(&right.timestamp))
    });

    let base = ordered
        .first()
        .expect("non-empty group after empty check")
        .clone();

    let provenance = ordered
        .iter()
        .map(|candidate| candidate.provenance)
        .reduce(merge_provenance)
        .ok_or(ContextPackSelectionError::InconsistentProvenance)?;

    let sensitivity = ordered
        .iter()
        .map(|candidate| candidate.sensitivity.clone())
        .reduce(merge_sensitivity)
        .unwrap_or(Sensitivity::Private);

    let safe_label = ordered
        .iter()
        .map(|candidate| candidate.safe_label.as_str())
        .min()
        .unwrap_or("")
        .to_string();

    let timestamp = ordered
        .iter()
        .filter_map(|candidate| candidate.timestamp.as_ref())
        .min()
        .cloned();

    let correction = ordered
        .iter()
        .filter_map(|candidate| candidate.correction.clone())
        .max_by(|left, right| left.correction_event_ids.cmp(&right.correction_event_ids));

    Ok(MergeableEvidenceEntry {
        evidence_ref: base.evidence_ref,
        origin: base.origin,
        sensitivity,
        safe_label,
        timestamp,
        provenance,
        correction,
    })
}

fn merge_provenance(
    left: ContextPackItemProvenance,
    right: ContextPackItemProvenance,
) -> ContextPackItemProvenance {
    use ContextPackItemProvenance::*;
    match (left, right) {
        (DiagnosticOnly, _) | (_, DiagnosticOnly) => DiagnosticOnly,
        (Unconfirmed, _) | (_, Unconfirmed) => Unconfirmed,
        (Pending, _) | (_, Pending) => Pending,
        (Confirmed, Confirmed) => Confirmed,
    }
}

fn merge_sensitivity(left: Sensitivity, right: Sensitivity) -> Sensitivity {
    match (left, right) {
        (Sensitivity::Secret, _) | (_, Sensitivity::Secret) => Sensitivity::Secret,
        (Sensitivity::Private, _) | (_, Sensitivity::Private) => Sensitivity::Private,
        (Sensitivity::Team, _) | (_, Sensitivity::Team) => Sensitivity::Team,
        (Sensitivity::Public, Sensitivity::Public) => Sensitivity::Public,
    }
}

fn stable_ref_id(index: usize) -> String {
    format!("ref-{:03}", index + 1)
}

fn push_diagnostic(
    diagnostics: &mut Vec<ContextPackDiagnostic>,
    code: &str,
    message: &str,
    severity: ContextPackDiagnosticSeverity,
) {
    if diagnostics.len() >= MAX_CONTEXT_PACK_DIAGNOSTICS {
        return;
    }
    diagnostics.push(ContextPackDiagnostic {
        code: code.into(),
        message: message.into(),
        severity,
    });
}
