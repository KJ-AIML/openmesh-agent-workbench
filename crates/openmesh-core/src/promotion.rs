// ============================================================================
// Signal Promotion — Dev Track 0.1.3.5
// ============================================================================
// Checkpoint A: domain contracts and deterministic key material.
// Checkpoint B: project-scoped promotion audit persistence + idempotency.
// Checkpoint E2: promotion application — audit + protocol 1.1 WorkEvent append.
// Checkpoint F: intelligence seam wiring + audit/event consistency guards.
// See: .heli-harness/state/reports/openmesh-0.1.3.5-execution-plan.md

use crate::context::Sensitivity;
use crate::domain::{
    validate_event_semantics, ActorRef, EvidenceAttachment, EvidenceRef, ProducerRef, WorkEvent,
    WorkSignalKind, WORK_EVENT_PROTOCOL_VERSION_PROMOTED,
};
use crate::events::{append_event, get_event, EventError};
use crate::intelligence::{
    validate_proposal_contract, ContinuityIntelligence, NoopContinuityIntelligence,
};
use crate::storage::{get_project_dir, read_project, Project};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Frozen prefix for promotion idempotency key material (plan §3.3.1).
pub const PROMOTION_KEY_PREFIX: &str = "promote-v1";

/// Frozen prefix for deterministic promoted WorkEvent ids (Checkpoint E2).
pub const PROMOTED_EVENT_ID_PREFIX: &str = "promoted-";

/// Note for contract consumers: promoted WorkEvents use protocol `1.1` with a
/// required `actor` at composition time (Checkpoint E2).
pub const PROMOTED_EVENT_PROTOCOL_NOTE: &str = "protocol 1.1 with composed actor — Checkpoint E2";

/// Terminal promotion outcomes (plan §3.7). No projection / catch-up variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromotionOutcome {
    Promote,
    Suppress,
    Defer,
    Ambiguous,
}

/// Frozen minimum reason codes (plan §3.7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PromotionReasonCode {
    QualificationFailed,
    ActivitySpam,
    SameOriginSemanticDuplicate,
    KindConflict,
    MissingEvidence,
    SeamNoProposal,
    SeamAmbiguous,
    /// Checkpoint C — signal passed qualification and kind matrix.
    Qualifies,
    /// Checkpoint C — five-question score below the promotion threshold.
    BelowThreshold,
    /// Checkpoint C — evidence or completeness insufficient for promotion.
    InsufficientEvidence,
    /// Checkpoint C — case is open or incomplete.
    UnresolvedOrIncomplete,
    /// Checkpoint C — deterministic rules cannot decide; intelligence seam path only.
    AmbiguousRequiresIntelligence,
    /// Checkpoint D — signals grouped by shared correlation_hint.
    CorrelatedByHint,
    /// Checkpoint D — independent producers corroborate the same conclusion.
    IndependentCorroboration,
    /// Checkpoint D — signal has no correlation_hint and forms a solo group.
    Uncorrelated,
    /// Checkpoint D — duplicate/corroboration relationship cannot be determined.
    AmbiguousCorrelation,
}

/// Deterministic idempotency key material for a promotion decision batch.
///
/// Checkpoint A defines the canonical preimage string. Checkpoint B persists
/// `sha256_hex(material)` per the approved execution plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PromotionKey(pub String);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PromotionKeyError {
    #[error("promotion key material is empty")]
    Empty,
    #[error("promotion key material exceeds {max} bytes")]
    TooLong { max: usize },
    #[error("workspace_id is empty")]
    EmptyWorkspaceId,
    #[error("signal_ids must not be empty")]
    EmptySignalIds,
    #[error("signal_id is empty")]
    EmptySignalId,
}

impl PromotionKey {
    pub const MAX_MATERIAL_BYTES: usize = 4096;

    /// Build validated key material from workspace + sorted signal ids.
    pub fn from_inputs(
        workspace_id: &str,
        signal_ids: &[String],
    ) -> Result<Self, PromotionKeyError> {
        let material = promotion_key_material(workspace_id, signal_ids)?;
        Self::from_material(material)
    }

    pub fn from_material(material: String) -> Result<Self, PromotionKeyError> {
        if material.is_empty() {
            return Err(PromotionKeyError::Empty);
        }
        if material.len() > Self::MAX_MATERIAL_BYTES {
            return Err(PromotionKeyError::TooLong {
                max: Self::MAX_MATERIAL_BYTES,
            });
        }
        Ok(Self(sha256_hex(material.as_bytes())))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Canonical deterministic preimage: `promote-v1|{workspaceId}|{sorted signalIds}`.
pub fn promotion_key_material(
    workspace_id: &str,
    signal_ids: &[String],
) -> Result<String, PromotionKeyError> {
    if workspace_id.trim().is_empty() {
        return Err(PromotionKeyError::EmptyWorkspaceId);
    }
    if signal_ids.is_empty() {
        return Err(PromotionKeyError::EmptySignalIds);
    }
    let mut ids = signal_ids.to_vec();
    for id in &ids {
        if id.trim().is_empty() {
            return Err(PromotionKeyError::EmptySignalId);
        }
    }
    ids.sort();
    Ok(format!(
        "{}|{}|{}",
        PROMOTION_KEY_PREFIX,
        workspace_id,
        ids.join(",")
    ))
}

/// Lightweight signal reference for promotion evaluation — no inbox file reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalRef {
    pub signal_id: String,
    pub kind: WorkSignalKind,
    pub summary: String,
    pub producer: ProducerRef,
    pub actor: ActorRef,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    pub evidence_refs: Vec<EvidenceRef>,
}

/// A promotion evaluation unit: one or more correlated signal references.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionCase {
    pub workspace_id: String,
    pub signals: Vec<SignalRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
}

impl PromotionCase {
    pub fn signal_ids(&self) -> Vec<String> {
        self.signals.iter().map(|s| s.signal_id.clone()).collect()
    }
}

/// Contract-level evidence grouping for promotion (not persisted in Checkpoint A).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceRelationship {
    /// Same claim + same causal origin — suppress after first (plan §3.6).
    SameOriginDuplicate,
    /// Same correlation hint + independent producers — merge corroboration (plan §3.6).
    IndependentCorroboration,
    Unrelated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionEvidence {
    pub signal_refs: Vec<String>,
    pub relationship: EvidenceRelationship,
    /// `ProducerSignal` evidence refs supporting one composed event (CC-1).
    pub producer_signal_attachments: Vec<String>,
}

/// Non-canonical proposed fields for a future WorkEvent — no write authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedEventComposition {
    pub kind: String,
    pub summary: String,
    pub timestamp: String,
    pub producer_signal_evidence_ids: Vec<String>,
    pub file_evidence_paths: Vec<String>,
    pub sensitivity: Sensitivity,
    /// Documents deferred wire work: actor + protocol 1.1 land in Checkpoint E2.
    pub composition_note: String,
}

impl ProposedEventComposition {
    pub fn placeholder_for_promote() -> Self {
        Self {
            kind: String::new(),
            summary: String::new(),
            timestamp: String::new(),
            producer_signal_evidence_ids: Vec::new(),
            file_evidence_paths: Vec::new(),
            sensitivity: Sensitivity::Private,
            composition_note: PROMOTED_EVENT_PROTOCOL_NOTE.to_string(),
        }
    }
}

/// A promotion decision contract — records outcome and rationale without writing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionDecision {
    pub promotion_key: PromotionKey,
    pub outcome: PromotionOutcome,
    pub source_signal_ids: Vec<String>,
    pub reason_code: Option<PromotionReasonCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposed_composition: Option<ProposedEventComposition>,
    pub ambiguous: bool,
}

impl PromotionDecision {
    pub fn promote(
        promotion_key: PromotionKey,
        source_signal_ids: Vec<String>,
        proposed: ProposedEventComposition,
    ) -> Self {
        Self {
            promotion_key,
            outcome: PromotionOutcome::Promote,
            source_signal_ids,
            reason_code: None,
            reason_detail: None,
            correlation_hint: None,
            correlation_group_id: None,
            proposed_composition: Some(proposed),
            ambiguous: false,
        }
    }

    pub fn suppress(
        promotion_key: PromotionKey,
        source_signal_ids: Vec<String>,
        reason_code: PromotionReasonCode,
    ) -> Self {
        Self {
            promotion_key,
            outcome: PromotionOutcome::Suppress,
            source_signal_ids,
            reason_code: Some(reason_code),
            reason_detail: None,
            correlation_hint: None,
            correlation_group_id: None,
            proposed_composition: None,
            ambiguous: false,
        }
    }

    pub fn defer(
        promotion_key: PromotionKey,
        source_signal_ids: Vec<String>,
        reason_code: PromotionReasonCode,
    ) -> Self {
        Self {
            promotion_key,
            outcome: PromotionOutcome::Defer,
            source_signal_ids,
            reason_code: Some(reason_code),
            reason_detail: None,
            correlation_hint: None,
            correlation_group_id: None,
            proposed_composition: None,
            ambiguous: false,
        }
    }

    pub fn ambiguous(
        promotion_key: PromotionKey,
        source_signal_ids: Vec<String>,
        reason_detail: String,
    ) -> Self {
        Self {
            promotion_key,
            outcome: PromotionOutcome::Ambiguous,
            source_signal_ids,
            reason_code: Some(PromotionReasonCode::SeamAmbiguous),
            reason_detail: Some(reason_detail),
            correlation_hint: None,
            correlation_group_id: None,
            proposed_composition: None,
            ambiguous: true,
        }
    }
}

/// Inputs surfaced to the intelligence seam when deterministic rules cannot decide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AmbiguousPromotionCase {
    pub case: PromotionCase,
    pub reason: String,
    pub qualification_notes: Vec<String>,
}

/// Proposal-only output from the intelligence seam — never a canonical write.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionProposal {
    pub has_proposal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_outcome: Option<PromotionOutcome>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_composition: Option<ProposedEventComposition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

impl PromotionProposal {
    pub fn none() -> Self {
        Self {
            has_proposal: false,
            suggested_outcome: None,
            suggested_composition: None,
            rationale: None,
        }
    }

    /// Proposals are in-memory only and never perform canonical writes.
    pub fn is_side_effect_free(&self) -> bool {
        true
    }
}

// ============================================================================
// Checkpoint C — deterministic qualification + suppression
// ============================================================================

/// Minimum number of `true` answers on the five-question test for promotion.
pub const QUALIFICATION_PASS_THRESHOLD: u8 = 3;

/// Context for evaluating a signal within a promotion case (no cross-case grouping).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualificationContext {
    pub group_size: usize,
    pub any_peer_has_evidence: bool,
}

impl QualificationContext {
    pub fn from_case(case: &PromotionCase) -> Self {
        Self {
            group_size: case.signals.len(),
            any_peer_has_evidence: case.signals.iter().any(|s| !s.evidence_refs.is_empty()),
        }
    }
}

/// Product Bible §8 five-question qualification test — deterministic booleans only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualificationScore {
    pub materially_changed: bool,
    pub affects_continuity: bool,
    pub teammate_needs_to_know: bool,
    pub has_evidence_or_accountable_source: bool,
    pub durable_memory_reduces_harm: bool,
}

impl QualificationScore {
    pub fn true_count(&self) -> u8 {
        [
            self.materially_changed,
            self.affects_continuity,
            self.teammate_needs_to_know,
            self.has_evidence_or_accountable_source,
            self.durable_memory_reduces_harm,
        ]
        .into_iter()
        .filter(|v| *v)
        .count() as u8
    }

    pub fn passes_threshold(&self) -> bool {
        self.true_count() >= QUALIFICATION_PASS_THRESHOLD
    }
}

fn signal_has_meaningful_progress_content(signal: &SignalRef) -> bool {
    signal.summary.chars().count() >= 20
        || !signal.evidence_refs.is_empty()
        || signal
            .correlation_hint
            .as_ref()
            .is_some_and(|h| !h.trim().is_empty())
}

fn has_effective_evidence(signal: &SignalRef, ctx: &QualificationContext) -> bool {
    !signal.evidence_refs.is_empty() || (ctx.group_size > 1 && ctx.any_peer_has_evidence)
}

fn is_activity_spam_kind(kind: WorkSignalKind) -> bool {
    matches!(
        kind,
        WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch
    )
}

fn is_high_value_kind(kind: WorkSignalKind) -> bool {
    matches!(
        kind,
        WorkSignalKind::Decision
            | WorkSignalKind::Blocker
            | WorkSignalKind::BlockerResolved
            | WorkSignalKind::Milestone
            | WorkSignalKind::ScopeChange
            | WorkSignalKind::Handoff
            | WorkSignalKind::ReviewRequired
            | WorkSignalKind::UnresolvedQuestion
    )
}

/// Deterministic five-question scoring for one signal reference.
pub fn qualification_score(signal: &SignalRef, ctx: &QualificationContext) -> QualificationScore {
    let meaningful = signal_has_meaningful_progress_content(signal);
    let spam = is_activity_spam_kind(signal.kind);
    let high_value = is_high_value_kind(signal.kind);

    let materially_changed = !spam && (high_value || meaningful);

    let affects_continuity = matches!(
        signal.kind,
        WorkSignalKind::Blocker
            | WorkSignalKind::BlockerResolved
            | WorkSignalKind::Decision
            | WorkSignalKind::Handoff
            | WorkSignalKind::UnresolvedQuestion
            | WorkSignalKind::Milestone
            | WorkSignalKind::ScopeChange
    ) || (signal.kind == WorkSignalKind::Progress && meaningful);

    let teammate_needs_to_know = affects_continuity
        || (signal.kind == WorkSignalKind::ReviewRequired && has_effective_evidence(signal, ctx))
        || (spam
            && ctx.group_size > 1
            && signal
                .correlation_hint
                .as_ref()
                .is_some_and(|h| !h.trim().is_empty()));

    let has_evidence_or_accountable_source = !signal.evidence_refs.is_empty()
        || matches!(signal.actor, ActorRef::Person(_))
        || matches!(signal.producer, ProducerRef::Reporter(_));

    let durable_memory_reduces_harm = high_value
        || (signal.kind == WorkSignalKind::Progress && meaningful)
        || (signal.kind == WorkSignalKind::ReviewRequired && has_effective_evidence(signal, ctx));

    QualificationScore {
        materially_changed,
        affects_continuity,
        teammate_needs_to_know,
        has_evidence_or_accountable_source,
        durable_memory_reduces_harm,
    }
}

fn kind_priority(kind: WorkSignalKind) -> u8 {
    match kind {
        WorkSignalKind::Decision => 6,
        WorkSignalKind::Blocker => 5,
        WorkSignalKind::Milestone | WorkSignalKind::ScopeChange => 4,
        WorkSignalKind::Handoff
        | WorkSignalKind::ReviewRequired
        | WorkSignalKind::UnresolvedQuestion => 3,
        WorkSignalKind::Progress | WorkSignalKind::BlockerResolved => 2,
        WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch => 0,
    }
}

fn preliminary_kind_conflict(signals: &[SignalRef]) -> bool {
    if signals.len() < 2 {
        return false;
    }
    let max_priority = signals
        .iter()
        .map(|s| kind_priority(s.kind))
        .max()
        .unwrap_or(0);
    let top_kinds: std::collections::BTreeSet<_> = signals
        .iter()
        .filter(|s| kind_priority(s.kind) == max_priority)
        .map(|s| format!("{:?}", s.kind))
        .collect();
    top_kinds.len() > 1
}

fn select_dominant_signal(signals: &[SignalRef]) -> &SignalRef {
    signals
        .iter()
        .max_by_key(|s| (kind_priority(s.kind), s.signal_id.as_str()))
        .expect("non-empty signals")
}

/// Frozen kind-matrix outcome when the five-question score passes (plan §3.9).
pub fn matrix_outcome_when_qualification_passes(
    kind: WorkSignalKind,
    has_effective_evidence: bool,
    summary_char_count: usize,
    has_correlation_hint: bool,
    group_size: usize,
) -> PromotionOutcome {
    match kind {
        WorkSignalKind::Progress => {
            if summary_char_count >= 20 || has_effective_evidence || has_correlation_hint {
                PromotionOutcome::Promote
            } else {
                PromotionOutcome::Suppress
            }
        }
        WorkSignalKind::Decision
        | WorkSignalKind::Blocker
        | WorkSignalKind::BlockerResolved
        | WorkSignalKind::ScopeChange
        | WorkSignalKind::Milestone
        | WorkSignalKind::Handoff => PromotionOutcome::Promote,
        WorkSignalKind::ReviewRequired | WorkSignalKind::UnresolvedQuestion => {
            if has_effective_evidence {
                PromotionOutcome::Promote
            } else {
                PromotionOutcome::Defer
            }
        }
        WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch => {
            if has_correlation_hint && group_size > 1 {
                PromotionOutcome::Promote
            } else {
                PromotionOutcome::Suppress
            }
        }
    }
}

/// Frozen kind-matrix outcome when the five-question score fails (plan §3.9).
pub fn matrix_outcome_when_qualification_fails(
    kind: WorkSignalKind,
    has_effective_evidence: bool,
) -> PromotionOutcome {
    match kind {
        WorkSignalKind::Progress | WorkSignalKind::SessionEnd | WorkSignalKind::AgentSwitch => {
            PromotionOutcome::Suppress
        }
        WorkSignalKind::Decision => {
            if has_effective_evidence {
                PromotionOutcome::Suppress
            } else {
                PromotionOutcome::Defer
            }
        }
        WorkSignalKind::Blocker | WorkSignalKind::BlockerResolved => PromotionOutcome::Defer,
        WorkSignalKind::ScopeChange
        | WorkSignalKind::Milestone
        | WorkSignalKind::ReviewRequired
        | WorkSignalKind::UnresolvedQuestion
        | WorkSignalKind::Handoff => PromotionOutcome::Defer,
    }
}

fn reason_for_outcome(
    outcome: PromotionOutcome,
    kind: WorkSignalKind,
    score_passed: bool,
    has_effective_evidence: bool,
) -> PromotionReasonCode {
    match outcome {
        PromotionOutcome::Promote => PromotionReasonCode::Qualifies,
        PromotionOutcome::Suppress => {
            if is_activity_spam_kind(kind) || kind == WorkSignalKind::Progress {
                PromotionReasonCode::ActivitySpam
            } else if !score_passed {
                PromotionReasonCode::BelowThreshold
            } else {
                PromotionReasonCode::QualificationFailed
            }
        }
        PromotionOutcome::Defer => {
            if matches!(
                kind,
                WorkSignalKind::Decision
                    | WorkSignalKind::Blocker
                    | WorkSignalKind::BlockerResolved
            ) && !has_effective_evidence
            {
                PromotionReasonCode::MissingEvidence
            } else if !score_passed {
                PromotionReasonCode::BelowThreshold
            } else {
                PromotionReasonCode::UnresolvedOrIncomplete
            }
        }
        PromotionOutcome::Ambiguous => PromotionReasonCode::AmbiguousRequiresIntelligence,
    }
}

fn normalized_event_kind(kind: WorkSignalKind) -> &'static str {
    match kind {
        WorkSignalKind::Progress => "work.progress",
        WorkSignalKind::Decision => "work.decision",
        WorkSignalKind::Blocker => "work.blocked",
        WorkSignalKind::BlockerResolved => "work.unblocked",
        WorkSignalKind::ScopeChange => "work.scope-changed",
        WorkSignalKind::Milestone => "work.milestone",
        WorkSignalKind::ReviewRequired => "work.review-required",
        WorkSignalKind::UnresolvedQuestion => "work.question-open",
        WorkSignalKind::Handoff => "work.handoff",
        WorkSignalKind::SessionEnd => "work.session-end",
        WorkSignalKind::AgentSwitch => "work.agent-switch",
    }
}

fn build_proposed_composition(
    signal: &SignalRef,
    source_signal_ids: &[String],
) -> ProposedEventComposition {
    let file_evidence_paths = signal
        .evidence_refs
        .iter()
        .filter_map(|ev| match ev {
            EvidenceRef::FilePath(path) => Some(path.clone()),
            EvidenceRef::ProducerSignal(_) => None,
        })
        .collect();

    ProposedEventComposition {
        kind: normalized_event_kind(signal.kind).to_string(),
        summary: signal.summary.clone(),
        timestamp: signal.timestamp.clone(),
        producer_signal_evidence_ids: source_signal_ids.to_vec(),
        file_evidence_paths,
        sensitivity: Sensitivity::Private,
        composition_note: PROMOTED_EVENT_PROTOCOL_NOTE.to_string(),
    }
}

/// Classify a single signal using the five-question score and kind matrix.
pub fn classify_signal(
    signal: &SignalRef,
    ctx: &QualificationContext,
) -> (PromotionOutcome, PromotionReasonCode) {
    let score = qualification_score(signal, ctx);
    let evidence = has_effective_evidence(signal, ctx);
    let summary_chars = signal.summary.chars().count();
    let has_hint = signal
        .correlation_hint
        .as_ref()
        .is_some_and(|h| !h.trim().is_empty());

    let outcome = if score.passes_threshold() {
        matrix_outcome_when_qualification_passes(
            signal.kind,
            evidence,
            summary_chars,
            has_hint,
            ctx.group_size,
        )
    } else {
        matrix_outcome_when_qualification_fails(signal.kind, evidence)
    };

    let reason = reason_for_outcome(outcome, signal.kind, score.passes_threshold(), evidence);
    (outcome, reason)
}

/// Pure qualification evaluation for a lightweight promotion case — no I/O.
pub fn evaluate_promotion_case(
    case: &PromotionCase,
) -> Result<PromotionDecision, PromotionKeyError> {
    let source_signal_ids = case.signal_ids();
    let promotion_key = PromotionKey::from_inputs(&case.workspace_id, &source_signal_ids)?;

    if case.signals.is_empty() {
        return Ok(PromotionDecision {
            promotion_key,
            outcome: PromotionOutcome::Defer,
            source_signal_ids,
            reason_code: Some(PromotionReasonCode::UnresolvedOrIncomplete),
            reason_detail: Some("promotion case has no signals".into()),
            correlation_hint: case.correlation_hint.clone(),
            correlation_group_id: None,
            proposed_composition: None,
            ambiguous: false,
        });
    }

    if preliminary_kind_conflict(&case.signals) {
        return Ok(PromotionDecision {
            promotion_key,
            outcome: PromotionOutcome::Ambiguous,
            source_signal_ids,
            reason_code: Some(PromotionReasonCode::AmbiguousRequiresIntelligence),
            reason_detail: Some(
                "dominant kind conflict — ContinuityIntelligence seam proposal path only".into(),
            ),
            correlation_hint: case.correlation_hint.clone(),
            correlation_group_id: None,
            proposed_composition: None,
            ambiguous: true,
        });
    }

    let ctx = QualificationContext::from_case(case);
    let dominant = select_dominant_signal(&case.signals);
    let (outcome, reason_code) = classify_signal(dominant, &ctx);

    let proposed_composition = if outcome == PromotionOutcome::Promote {
        Some(build_proposed_composition(dominant, &source_signal_ids))
    } else {
        None
    };

    Ok(PromotionDecision {
        promotion_key,
        outcome,
        source_signal_ids,
        reason_code: Some(reason_code),
        reason_detail: None,
        correlation_hint: case
            .correlation_hint
            .clone()
            .or(dominant.correlation_hint.clone()),
        correlation_group_id: None,
        proposed_composition,
        ambiguous: outcome == PromotionOutcome::Ambiguous,
    })
}

// ============================================================================
// Checkpoint D — correlation + duplicate vs corroboration
// ============================================================================

/// Bucket label for signals without a correlation hint (solo groups).
pub const UNCORRELATED_BUCKET_PREFIX: &str = "__uncorrelated__";

/// One correlated batch of signal references — no WorkEvent payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelatedPromotionGroup {
    pub correlation_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    pub workspace_id: String,
    pub signals: Vec<SignalRef>,
    pub group_order_key: String,
}

/// Inspectable relationship refs within a correlation batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignalRelationshipRef {
    pub signal_ids: Vec<String>,
    pub relationship: EvidenceRelationship,
}

/// Grouping output with relationship metadata — pure in-memory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationResult {
    pub groups: Vec<CorrelatedPromotionGroup>,
    pub duplicate_refs: Vec<SignalRelationshipRef>,
    pub corroboration_refs: Vec<SignalRelationshipRef>,
    pub unrelated_refs: Vec<SignalRelationshipRef>,
}

/// One group's qualification decision enriched with correlation evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationDecision {
    pub group: CorrelatedPromotionGroup,
    pub decision: PromotionDecision,
    pub evidence: PromotionEvidence,
    pub duplicate_signal_ids: Vec<String>,
    pub corroborating_signal_ids: Vec<String>,
}

/// Full correlation pass over lightweight signal refs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrelationBatchResult {
    pub correlation: CorrelationResult,
    pub decisions: Vec<CorrelationDecision>,
}

fn uncorrelated_key(signal_id: &str) -> String {
    format!("{UNCORRELATED_BUCKET_PREFIX}/{signal_id}")
}

fn normalize_summary(summary: &str) -> String {
    summary.trim().to_lowercase()
}

fn producer_key(producer: &ProducerRef) -> String {
    match producer {
        ProducerRef::Native => "native".into(),
        ProducerRef::Heli => "heli".into(),
        ProducerRef::Git => "git".into(),
        ProducerRef::Reporter(name) => format!("reporter:{name}"),
    }
}

fn semantic_origin_key(signal: &SignalRef) -> String {
    format!(
        "{}|{}|{}",
        producer_key(&signal.producer),
        normalize_summary(&signal.summary),
        signal
            .correlation_hint
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
    )
}

/// Group signal refs by exact non-empty `correlation_hint`; missing hints stay solo.
pub fn group_signals_by_correlation_hint(
    workspace_id: &str,
    signals: &[SignalRef],
) -> CorrelationResult {
    let mut hint_groups: std::collections::BTreeMap<String, Vec<SignalRef>> =
        std::collections::BTreeMap::new();
    let mut solo_groups = Vec::new();

    for signal in signals {
        if signal
            .correlation_hint
            .as_ref()
            .is_some_and(|h| !h.trim().is_empty())
        {
            let hint = signal.correlation_hint.clone().unwrap();
            hint_groups.entry(hint).or_default().push(signal.clone());
        } else {
            solo_groups.push(CorrelatedPromotionGroup {
                correlation_key: uncorrelated_key(&signal.signal_id),
                correlation_hint: None,
                workspace_id: workspace_id.to_string(),
                signals: vec![signal.clone()],
                group_order_key: signal.signal_id.clone(),
            });
        }
    }

    let mut groups = Vec::new();
    for (hint, mut members) in hint_groups {
        members.sort_by(|a, b| a.signal_id.cmp(&b.signal_id));
        let order_key = members[0].signal_id.clone();
        groups.push(CorrelatedPromotionGroup {
            correlation_key: hint.clone(),
            correlation_hint: Some(hint),
            workspace_id: workspace_id.to_string(),
            signals: members,
            group_order_key: order_key,
        });
    }

    groups.append(&mut solo_groups);
    groups.sort_by(|a, b| a.group_order_key.cmp(&b.group_order_key));

    let mut duplicate_refs = Vec::new();
    let mut corroboration_refs = Vec::new();
    let mut unrelated_refs = Vec::new();
    for group in &groups {
        let relationships = classify_group_relationships(group);
        duplicate_refs.extend(relationships.duplicate_refs);
        corroboration_refs.extend(relationships.corroboration_refs);
        unrelated_refs.extend(relationships.unrelated_refs);
    }

    CorrelationResult {
        groups,
        duplicate_refs,
        corroboration_refs,
        unrelated_refs,
    }
}

struct GroupRelationships {
    duplicate_refs: Vec<SignalRelationshipRef>,
    corroboration_refs: Vec<SignalRelationshipRef>,
    unrelated_refs: Vec<SignalRelationshipRef>,
}

fn detect_same_origin_semantic_duplicates(signals: &[SignalRef]) -> (Vec<String>, Vec<String>) {
    let mut buckets: std::collections::BTreeMap<String, Vec<&SignalRef>> =
        std::collections::BTreeMap::new();
    for signal in signals {
        buckets
            .entry(semantic_origin_key(signal))
            .or_default()
            .push(signal);
    }

    let mut canonical = Vec::new();
    let mut duplicates = Vec::new();
    for mut cluster in buckets.into_values() {
        cluster.sort_by(|a, b| a.signal_id.cmp(&b.signal_id));
        canonical.push(cluster[0].signal_id.clone());
        for dup in cluster.iter().skip(1) {
            duplicates.push(dup.signal_id.clone());
        }
    }
    canonical.sort();
    duplicates.sort();
    (canonical, duplicates)
}

fn detect_independent_corroboration(signals: &[SignalRef]) -> Vec<String> {
    if signals.len() < 2 {
        return Vec::new();
    }
    let has_hint = signals.iter().any(|s| {
        s.correlation_hint
            .as_ref()
            .is_some_and(|h| !h.trim().is_empty())
    });
    if !has_hint {
        return Vec::new();
    }

    let mut by_summary: std::collections::BTreeMap<String, Vec<&SignalRef>> =
        std::collections::BTreeMap::new();
    for signal in signals {
        by_summary
            .entry(normalize_summary(&signal.summary))
            .or_default()
            .push(signal);
    }

    let mut corroborating = Vec::new();
    for members in by_summary.into_values() {
        let producers: std::collections::BTreeSet<_> =
            members.iter().map(|s| producer_key(&s.producer)).collect();
        if producers.len() > 1 {
            let mut ids: Vec<_> = members.iter().map(|s| s.signal_id.clone()).collect();
            ids.sort();
            corroborating.extend(ids);
        }
    }
    corroborating.sort();
    corroborating.dedup();
    corroborating
}

fn is_ambiguous_correlation(signals: &[SignalRef]) -> bool {
    if signals.len() < 2 {
        return false;
    }
    let has_hint = signals.iter().any(|s| {
        s.correlation_hint
            .as_ref()
            .is_some_and(|h| !h.trim().is_empty())
    });
    if !has_hint {
        return false;
    }
    for i in 0..signals.len() {
        for j in (i + 1)..signals.len() {
            if producer_key(&signals[i].producer) == producer_key(&signals[j].producer)
                && normalize_summary(&signals[i].summary) != normalize_summary(&signals[j].summary)
            {
                return true;
            }
        }
    }
    false
}

fn classify_group_relationships(group: &CorrelatedPromotionGroup) -> GroupRelationships {
    let (_, duplicate_ids) = detect_same_origin_semantic_duplicates(&group.signals);
    let corroborating = detect_independent_corroboration(&group.signals);

    let mut duplicate_refs = Vec::new();
    if !duplicate_ids.is_empty() {
        duplicate_refs.push(SignalRelationshipRef {
            signal_ids: duplicate_ids.clone(),
            relationship: EvidenceRelationship::SameOriginDuplicate,
        });
    }

    let mut corroboration_refs = Vec::new();
    if corroborating.len() > 1 {
        corroboration_refs.push(SignalRelationshipRef {
            signal_ids: corroborating.clone(),
            relationship: EvidenceRelationship::IndependentCorroboration,
        });
    }

    let mut unrelated_refs = Vec::new();
    if group.signals.len() > 1
        && duplicate_ids.is_empty()
        && corroborating.len() <= 1
        && !is_ambiguous_correlation(&group.signals)
    {
        let ids: Vec<_> = group.signals.iter().map(|s| s.signal_id.clone()).collect();
        unrelated_refs.push(SignalRelationshipRef {
            signal_ids: ids,
            relationship: EvidenceRelationship::Unrelated,
        });
    }

    GroupRelationships {
        duplicate_refs,
        corroboration_refs,
        unrelated_refs,
    }
}

/// Prepare a many-signals-one-event candidate case — no WorkEvent composition.
pub fn prepare_future_event_candidate(group: &CorrelatedPromotionGroup) -> PromotionCase {
    PromotionCase {
        workspace_id: group.workspace_id.clone(),
        signals: group.signals.clone(),
        correlation_hint: group.correlation_hint.clone(),
    }
}

fn build_group_evidence(
    group: &CorrelatedPromotionGroup,
    duplicate_ids: &[String],
    corroborating_ids: &[String],
) -> PromotionEvidence {
    let relationship = if corroborating_ids.len() > 1 {
        EvidenceRelationship::IndependentCorroboration
    } else if !duplicate_ids.is_empty() {
        EvidenceRelationship::SameOriginDuplicate
    } else {
        EvidenceRelationship::Unrelated
    };

    let signal_refs: Vec<_> = group.signals.iter().map(|s| s.signal_id.clone()).collect();
    PromotionEvidence {
        signal_refs: signal_refs.clone(),
        relationship,
        producer_signal_attachments: if corroborating_ids.len() > 1 {
            corroborating_ids.to_vec()
        } else {
            signal_refs
        },
    }
}

fn suppress_duplicate_decision(
    workspace_id: &str,
    signal: &SignalRef,
) -> Result<PromotionDecision, PromotionKeyError> {
    let key = PromotionKey::from_inputs(workspace_id, std::slice::from_ref(&signal.signal_id))?;
    Ok(PromotionDecision {
        promotion_key: key,
        outcome: PromotionOutcome::Suppress,
        source_signal_ids: vec![signal.signal_id.clone()],
        reason_code: Some(PromotionReasonCode::SameOriginSemanticDuplicate),
        reason_detail: Some("same producer, summary, and correlation hint".into()),
        correlation_hint: signal.correlation_hint.clone(),
        correlation_group_id: None,
        proposed_composition: None,
        ambiguous: false,
    })
}

/// Evaluate one correlated group — pure logic, no storage or ledger writes.
pub fn evaluate_correlated_group(
    group: &CorrelatedPromotionGroup,
) -> Result<Vec<CorrelationDecision>, PromotionKeyError> {
    let all_source_ids: Vec<_> = group.signals.iter().map(|s| s.signal_id.clone()).collect();

    if is_ambiguous_correlation(&group.signals) {
        let promotion_key = PromotionKey::from_inputs(&group.workspace_id, &all_source_ids)?;
        let decision = PromotionDecision {
            promotion_key,
            outcome: PromotionOutcome::Ambiguous,
            source_signal_ids: all_source_ids.clone(),
            reason_code: Some(PromotionReasonCode::AmbiguousCorrelation),
            reason_detail: Some(
                "same producer emitted conflicting summaries under one correlation hint".into(),
            ),
            correlation_hint: group.correlation_hint.clone(),
            correlation_group_id: Some(group.correlation_key.clone()),
            proposed_composition: None,
            ambiguous: true,
        };
        let evidence = build_group_evidence(group, &[], &[]);
        return Ok(vec![CorrelationDecision {
            group: group.clone(),
            decision,
            evidence,
            duplicate_signal_ids: vec![],
            corroborating_signal_ids: vec![],
        }]);
    }

    let (_, duplicate_ids) = detect_same_origin_semantic_duplicates(&group.signals);
    let corroborating_ids = detect_independent_corroboration(&group.signals);

    let canonical_signals: Vec<SignalRef> = group
        .signals
        .iter()
        .filter(|s| !duplicate_ids.contains(&s.signal_id))
        .cloned()
        .collect();

    let case = PromotionCase {
        workspace_id: group.workspace_id.clone(),
        signals: if canonical_signals.is_empty() {
            group.signals.clone()
        } else {
            canonical_signals.clone()
        },
        correlation_hint: group.correlation_hint.clone(),
    };

    let mut decision = evaluate_promotion_case(&case)?;
    decision.source_signal_ids = all_source_ids.clone();
    decision.promotion_key =
        PromotionKey::from_inputs(&group.workspace_id, &decision.source_signal_ids)?;
    decision.correlation_group_id = Some(group.correlation_key.clone());

    if group.correlation_hint.is_some() {
        if corroborating_ids.len() > 1 {
            decision.reason_code = Some(PromotionReasonCode::IndependentCorroboration);
            if decision.outcome == PromotionOutcome::Promote {
                if let Some(composition) = decision.proposed_composition.as_mut() {
                    composition.producer_signal_evidence_ids = corroborating_ids.clone();
                }
            }
        } else if group.signals.len() > 1 {
            decision.reason_code = Some(PromotionReasonCode::CorrelatedByHint);
        }
    } else {
        decision.reason_code = decision
            .reason_code
            .or(Some(PromotionReasonCode::Uncorrelated));
    }

    let evidence = build_group_evidence(group, &duplicate_ids, &corroborating_ids);
    let mut results = vec![CorrelationDecision {
        group: group.clone(),
        decision,
        evidence,
        duplicate_signal_ids: duplicate_ids.clone(),
        corroborating_signal_ids: corroborating_ids.clone(),
    }];

    for signal in &group.signals {
        if duplicate_ids.contains(&signal.signal_id) {
            results.push(CorrelationDecision {
                group: CorrelatedPromotionGroup {
                    correlation_key: uncorrelated_key(&signal.signal_id),
                    correlation_hint: signal.correlation_hint.clone(),
                    workspace_id: group.workspace_id.clone(),
                    signals: vec![signal.clone()],
                    group_order_key: signal.signal_id.clone(),
                },
                decision: suppress_duplicate_decision(&group.workspace_id, signal)?,
                evidence: PromotionEvidence {
                    signal_refs: vec![signal.signal_id.clone()],
                    relationship: EvidenceRelationship::SameOriginDuplicate,
                    producer_signal_attachments: vec![signal.signal_id.clone()],
                },
                duplicate_signal_ids: vec![signal.signal_id.clone()],
                corroborating_signal_ids: vec![],
            });
        }
    }

    Ok(results)
}

/// Group, classify relationships, and evaluate all correlated batches.
pub fn correlate_and_evaluate(
    workspace_id: &str,
    signals: &[SignalRef],
) -> Result<CorrelationBatchResult, PromotionKeyError> {
    let correlation = group_signals_by_correlation_hint(workspace_id, signals);
    let mut decisions = Vec::new();
    for group in &correlation.groups {
        decisions.extend(evaluate_correlated_group(group)?);
    }
    Ok(CorrelationBatchResult {
        correlation,
        decisions,
    })
}

// ============================================================================
// Checkpoint B — promotion audit persistence
// ============================================================================

/// Frozen audit wire protocol for promotion decision records.
pub const PROMOTION_AUDIT_PROTOCOL_VERSION: &str = "1.0";

/// Frozen bound aligned with the WorkEvent ledger record cap.
pub const MAX_AUDIT_RECORD_BYTES: usize = 256 * 1024;

/// Persisted promotion audit record — no WorkEvent payload or actor wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromotionDecisionRecord {
    pub audit_protocol_version: String,
    pub promotion_key: PromotionKey,
    pub workspace_id: String,
    pub outcome: PromotionOutcome,
    pub source_signal_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<PromotionEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_code: Option<PromotionReasonCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correlation_group_id: Option<String>,
    pub ambiguous: bool,
    pub recorded_at: String,
}

impl PromotionDecisionRecord {
    pub fn from_decision(
        workspace_id: String,
        decision: PromotionDecision,
        evidence: Option<PromotionEvidence>,
        recorded_at: String,
    ) -> Self {
        Self {
            audit_protocol_version: PROMOTION_AUDIT_PROTOCOL_VERSION.to_string(),
            promotion_key: decision.promotion_key,
            workspace_id,
            outcome: decision.outcome,
            source_signal_ids: decision.source_signal_ids,
            evidence,
            reason_code: decision.reason_code,
            reason_detail: decision.reason_detail,
            correlation_hint: decision.correlation_hint,
            correlation_group_id: decision.correlation_group_id,
            ambiguous: decision.ambiguous,
            recorded_at,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PromotionAuditError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("decision workspace_id does not match the project's id")]
    WorkspaceMismatch,
    #[error("promotion key does not match derived idempotency key")]
    KeyMismatch,
    #[error("promotion key is not safe for audit storage: {0}")]
    UnsafePromotionKey(String),
    #[error("audit record failed validation: {0}")]
    InvalidRecord(String),
    #[error("audit record exceeds the {max}-byte bound (was {actual} bytes)")]
    RecordTooLarge { actual: usize, max: usize },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("promotion key error: {0}")]
    Key(#[from] PromotionKeyError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteDecisionOutcome {
    Created(PromotionDecisionRecord),
    Existing(PromotionDecisionRecord),
}

pub fn promotion_dir(project_path: &str) -> PathBuf {
    events_root(project_path).join("promotion")
}

pub fn promotion_decisions_dir(project_path: &str) -> PathBuf {
    promotion_dir(project_path).join("decisions")
}

fn events_root(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("events")
}

fn ensure_promotion_directories(project_path: &str) -> std::io::Result<()> {
    fs::create_dir_all(promotion_decisions_dir(project_path))
}

fn decision_file_path(project_path: &str, promotion_key: &PromotionKey) -> PathBuf {
    promotion_decisions_dir(project_path).join(format!("{}.json", promotion_key.as_str()))
}

fn load_project(project_path: &str) -> Result<Project, PromotionAuditError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or_else(|| PromotionAuditError::ProjectNotInitialized(project_path.to_string()))
}

fn write_all_and_flush(mut file: fs::File, content: &str) -> Result<(), PromotionAuditError> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn validate_promotion_key_for_storage(key: &str) -> Result<(), PromotionAuditError> {
    if key.len() != 64 || !key.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(PromotionAuditError::UnsafePromotionKey(
            "promotion key must be a 64-character lowercase hex digest".into(),
        ));
    }
    Ok(())
}

pub fn validate_decision_record(
    record: &PromotionDecisionRecord,
    project: &Project,
) -> Result<(), PromotionAuditError> {
    if record.workspace_id != project.id {
        return Err(PromotionAuditError::WorkspaceMismatch);
    }
    if record.audit_protocol_version != PROMOTION_AUDIT_PROTOCOL_VERSION {
        return Err(PromotionAuditError::InvalidRecord(format!(
            "unsupported audit protocol version: {}",
            record.audit_protocol_version
        )));
    }
    validate_promotion_key_for_storage(record.promotion_key.as_str())?;
    let derived = PromotionKey::from_inputs(&record.workspace_id, &record.source_signal_ids)?;
    if derived != record.promotion_key {
        return Err(PromotionAuditError::KeyMismatch);
    }
    if record.source_signal_ids.is_empty() {
        return Err(PromotionAuditError::InvalidRecord(
            "source_signal_ids must not be empty".into(),
        ));
    }
    Ok(())
}

pub fn classify_decision_record(
    path: &Path,
    project: &Project,
) -> Result<PromotionDecisionRecord, PromotionAuditError> {
    let metadata = fs::metadata(path)?;
    if metadata.len() as usize > MAX_AUDIT_RECORD_BYTES {
        return Err(PromotionAuditError::RecordTooLarge {
            actual: metadata.len() as usize,
            max: MAX_AUDIT_RECORD_BYTES,
        });
    }

    let raw = fs::read_to_string(path)?;
    let record: PromotionDecisionRecord = serde_json::from_str(&raw)?;
    validate_decision_record(&record, project)?;

    let filename_key = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| PromotionAuditError::InvalidRecord("invalid audit filename".into()))?;
    if filename_key != record.promotion_key.as_str() {
        return Err(PromotionAuditError::InvalidRecord(
            "audit filename does not match promotionKey".into(),
        ));
    }

    Ok(record)
}

fn list_canonical_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "tmp").unwrap_or(false) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

/// Persists one promotion decision audit record under
/// `<project>/.openmesh/events/promotion/decisions/{promotionKey}.json`.
///
/// Same `promotionKey` returns the existing record without overwriting bytes.
pub fn write_decision_record(
    project_path: &str,
    record: &PromotionDecisionRecord,
) -> Result<WriteDecisionOutcome, PromotionAuditError> {
    let project = load_project(project_path)?;
    validate_decision_record(record, &project)?;

    let final_path = decision_file_path(project_path, &record.promotion_key);
    if final_path.exists() {
        let existing = classify_decision_record(&final_path, &project)?;
        return Ok(WriteDecisionOutcome::Existing(existing));
    }

    let payload = serde_json::to_string_pretty(record)?;
    let payload_len = payload.len();
    if payload_len > MAX_AUDIT_RECORD_BYTES {
        return Err(PromotionAuditError::RecordTooLarge {
            actual: payload_len,
            max: MAX_AUDIT_RECORD_BYTES,
        });
    }

    ensure_promotion_directories(project_path)?;
    let decisions = promotion_decisions_dir(project_path);
    let temp_path = decisions.join(format!("{}.tmp", record.promotion_key.as_str()));

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;
    write_all_and_flush(file, &payload)?;

    match fs::rename(&temp_path, &final_path) {
        Ok(()) => Ok(WriteDecisionOutcome::Created(record.clone())),
        Err(_err) if final_path.exists() => {
            let _ = fs::remove_file(&temp_path);
            let existing = classify_decision_record(&final_path, &project)?;
            Ok(WriteDecisionOutcome::Existing(existing))
        }
        Err(err) => Err(PromotionAuditError::Io(err)),
    }
}

pub fn get_decision_record(
    project_path: &str,
    promotion_key: &PromotionKey,
) -> Result<Option<PromotionDecisionRecord>, PromotionAuditError> {
    let path = decision_file_path(project_path, promotion_key);
    if !path.exists() {
        return Ok(None);
    }
    let project = load_project(project_path)?;
    Ok(Some(classify_decision_record(&path, &project)?))
}

pub fn list_decision_records(
    project_path: &str,
) -> Result<Vec<PromotionDecisionRecord>, PromotionAuditError> {
    let project = load_project(project_path)?;
    let dir = promotion_decisions_dir(project_path);
    let mut records = Vec::new();
    for path in list_canonical_files(&dir)? {
        records.push(classify_decision_record(&path, &project)?);
    }
    Ok(records)
}

// ============================================================================
// Checkpoint E2 — promotion application + WorkEvent composition
// ============================================================================

/// Deterministic WorkEvent id for a promotion decision (idempotent append).
pub fn promoted_event_id(promotion_key: &PromotionKey) -> String {
    format!("{PROMOTED_EVENT_ID_PREFIX}{}", promotion_key.as_str())
}

/// Input for core-only promotion application — no inbox reads or signal moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionApplyRequest {
    pub workspace_id: String,
    pub decision: PromotionDecision,
    /// Source signals for actor, summary, and evidence composition (`Promote` only).
    pub signals: Vec<SignalRef>,
    pub evidence: Option<PromotionEvidence>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyPromotionOutcome {
    pub audit: WriteDecisionOutcome,
    pub event: Option<PromotedEventOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromotedEventOutcome {
    Created(WorkEvent),
    Existing(WorkEvent),
}

#[derive(Debug, thiserror::Error)]
pub enum PromotionApplyError {
    #[error("decision workspace_id does not match the project's id")]
    WorkspaceMismatch,
    #[error("promotion key does not match derived idempotency key")]
    KeyMismatch,
    #[error("promotion composition failed: {0}")]
    Composition(String),
    #[error("promotion audit error: {0}")]
    Audit(#[from] PromotionAuditError),
    #[error("ledger error: {0}")]
    Event(#[from] EventError),
    #[error("promotion key error: {0}")]
    Key(#[from] PromotionKeyError),
    #[error("intelligence proposal violates seam contract")]
    InvalidIntelligenceProposal,
    #[error("promote audit exists but WorkEvent {0} is not materialized in the ledger")]
    PromoteEventNotMaterialized(String),
    #[error("existing promote audit disagrees with requested outcome")]
    AuditOutcomeMismatch,
}

/// Composes `WorkEvent.actor` per plan §3.4.4 (deterministic, no inference).
pub fn compose_event_actor(signals: &[SignalRef]) -> ActorRef {
    if signals.is_empty() {
        return ActorRef::Unknown;
    }
    if signals.len() == 1 {
        return signals[0].actor.clone();
    }

    let mut qualifying: Vec<&SignalRef> = signals
        .iter()
        .filter(|s| {
            matches!(s.actor, ActorRef::Person(_))
                && matches!(s.kind, WorkSignalKind::Decision | WorkSignalKind::Handoff)
        })
        .collect();
    if !qualifying.is_empty() {
        qualifying.sort_by(|a, b| a.signal_id.cmp(&b.signal_id));
        return qualifying[0].actor.clone();
    }

    signals
        .iter()
        .max_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.signal_id.cmp(&b.signal_id))
        })
        .map(|s| s.actor.clone())
        .unwrap_or(ActorRef::Unknown)
}

fn compose_summary(signals: &[SignalRef]) -> String {
    let mut qualifying: Vec<&SignalRef> = signals
        .iter()
        .filter(|s| {
            matches!(s.actor, ActorRef::Person(_))
                && matches!(s.kind, WorkSignalKind::Decision | WorkSignalKind::Handoff)
        })
        .collect();
    if !qualifying.is_empty() {
        qualifying.sort_by(|a, b| a.signal_id.cmp(&b.signal_id));
        return qualifying[0].summary.clone();
    }

    signals
        .iter()
        .max_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.signal_id.cmp(&b.signal_id))
        })
        .map(|s| s.summary.clone())
        .unwrap_or_default()
}

fn compose_timestamp(signals: &[SignalRef]) -> String {
    signals
        .iter()
        .max_by(|a, b| {
            a.timestamp
                .cmp(&b.timestamp)
                .then_with(|| a.signal_id.cmp(&b.signal_id))
        })
        .map(|s| s.timestamp.clone())
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".into())
}

fn compose_evidence_attachments(
    signals: &[SignalRef],
    producer_signal_ids: &[String],
) -> Vec<EvidenceAttachment> {
    let mut attachments = Vec::new();
    for id in producer_signal_ids {
        attachments.push(EvidenceAttachment {
            evidence_ref: EvidenceRef::ProducerSignal(id.clone()),
            observed_at: None,
        });
    }
    for signal in signals {
        for ev in &signal.evidence_refs {
            if let EvidenceRef::FilePath(path) = ev {
                attachments.push(EvidenceAttachment {
                    evidence_ref: EvidenceRef::FilePath(path.clone()),
                    observed_at: None,
                });
            }
        }
    }
    attachments
}

/// Builds a protocol `1.1` WorkEvent from correlated promotion inputs.
pub fn compose_work_event_from_group(
    workspace_id: &str,
    promotion_key: &PromotionKey,
    signals: &[SignalRef],
    proposed: &ProposedEventComposition,
) -> Result<WorkEvent, PromotionApplyError> {
    if signals.is_empty() {
        return Err(PromotionApplyError::Composition(
            "cannot compose promoted event without source signals".into(),
        ));
    }
    if proposed.kind.trim().is_empty() || proposed.summary.trim().is_empty() {
        return Err(PromotionApplyError::Composition(
            "proposed composition is incomplete".into(),
        ));
    }

    let actor = compose_event_actor(signals);
    let evidence = compose_evidence_attachments(signals, &proposed.producer_signal_evidence_ids);
    if evidence.is_empty() {
        return Err(PromotionApplyError::Composition(
            "promoted event evidence must not be empty".into(),
        ));
    }

    let mut event = WorkEvent::new(
        promoted_event_id(promotion_key),
        workspace_id,
        proposed.kind.clone(),
        compose_summary(signals),
        evidence,
        compose_timestamp(signals),
    );
    event.protocol_version = WORK_EVENT_PROTOCOL_VERSION_PROMOTED.to_string();
    event.actor = Some(actor);
    event.sensitivity = proposed.sensitivity.clone();

    validate_event_semantics(&event)
        .map_err(|e| PromotionApplyError::Composition(e.to_string()))?;
    Ok(event)
}

fn validate_promote_request(request: &PromotionApplyRequest) -> Result<(), PromotionApplyError> {
    let proposed = request
        .decision
        .proposed_composition
        .as_ref()
        .ok_or_else(|| {
            PromotionApplyError::Composition(
                "Promote outcome requires proposed_composition before WorkEvent write".into(),
            )
        })?;
    compose_work_event_from_group(
        &request.workspace_id,
        &request.decision.promotion_key,
        &request.signals,
        proposed,
    )?;
    Ok(())
}

fn append_promoted_event(
    project_path: &str,
    event: &WorkEvent,
) -> Result<PromotedEventOutcome, PromotionApplyError> {
    let event_id = event.event_id.clone();
    if let Some(existing) = get_event(project_path, &event_id)? {
        return Ok(PromotedEventOutcome::Existing(existing));
    }

    match append_event(project_path, event) {
        Ok(()) => Ok(PromotedEventOutcome::Created(event.clone())),
        Err(EventError::DuplicateEventId(_)) => {
            let existing = get_event(project_path, &event_id)?.ok_or_else(|| {
                PromotionApplyError::Composition(
                    "duplicate event id without readable ledger record".into(),
                )
            })?;
            Ok(PromotedEventOutcome::Existing(existing))
        }
        Err(err) => Err(PromotionApplyError::Event(err)),
    }
}

// ============================================================================
// Checkpoint F — intelligence seam wiring + audit/event consistency
// ============================================================================

/// Builds an `AmbiguousPromotionCase` snapshot from a promotion decision.
pub fn ambiguous_case_from_decision(
    workspace_id: &str,
    decision: &PromotionDecision,
    signals: &[SignalRef],
) -> AmbiguousPromotionCase {
    AmbiguousPromotionCase {
        case: PromotionCase {
            workspace_id: workspace_id.to_string(),
            signals: signals.to_vec(),
            correlation_hint: decision.correlation_hint.clone(),
        },
        reason: decision
            .reason_detail
            .clone()
            .unwrap_or_else(|| "ambiguous promotion case".into()),
        qualification_notes: decision
            .reason_code
            .as_ref()
            .map(|code| vec![format!("{code:?}")])
            .unwrap_or_default(),
    }
}

/// Builds an `AmbiguousPromotionCase` with explicit workspace id.
pub fn ambiguous_case_from_request(request: &PromotionApplyRequest) -> AmbiguousPromotionCase {
    AmbiguousPromotionCase {
        case: PromotionCase {
            workspace_id: request.workspace_id.clone(),
            signals: request.signals.clone(),
            correlation_hint: request.decision.correlation_hint.clone(),
        },
        reason: request
            .decision
            .reason_detail
            .clone()
            .unwrap_or_else(|| "ambiguous promotion case".into()),
        qualification_notes: request
            .decision
            .reason_code
            .as_ref()
            .map(|code| vec![format!("{code:?}")])
            .unwrap_or_default(),
    }
}

/// Invokes the intelligence seam for an ambiguous decision. No canonical writes.
pub fn resolve_ambiguous_with_intelligence(
    decision: &PromotionDecision,
    ambiguous: &AmbiguousPromotionCase,
    intelligence: &dyn ContinuityIntelligence,
) -> Result<PromotionDecision, PromotionApplyError> {
    if decision.outcome != PromotionOutcome::Ambiguous {
        return Ok(decision.clone());
    }

    let proposal = intelligence.propose(ambiguous);
    if !validate_proposal_contract(&proposal) {
        return Err(PromotionApplyError::InvalidIntelligenceProposal);
    }

    if !proposal.has_proposal {
        return Ok(decision.clone());
    }

    let Some(suggested) = proposal.suggested_outcome else {
        return Ok(PromotionDecision {
            reason_code: Some(PromotionReasonCode::SeamNoProposal),
            ..decision.clone()
        });
    };

    if suggested == PromotionOutcome::Ambiguous {
        return Ok(decision.clone());
    }

    if suggested == PromotionOutcome::Promote {
        let composition = proposal
            .suggested_composition
            .as_ref()
            .ok_or(PromotionApplyError::InvalidIntelligenceProposal)?;
        return Ok(PromotionDecision::promote(
            decision.promotion_key.clone(),
            decision.source_signal_ids.clone(),
            composition.clone(),
        ));
    }

    if suggested == PromotionOutcome::Suppress {
        return Ok(PromotionDecision::suppress(
            decision.promotion_key.clone(),
            decision.source_signal_ids.clone(),
            PromotionReasonCode::SeamAmbiguous,
        ));
    }

    Ok(PromotionDecision::defer(
        decision.promotion_key.clone(),
        decision.source_signal_ids.clone(),
        PromotionReasonCode::SeamAmbiguous,
    ))
}

fn ensure_existing_promote_audit_matches(
    audit: &WriteDecisionOutcome,
    requested: &PromotionDecision,
) -> Result<(), PromotionApplyError> {
    if let WriteDecisionOutcome::Existing(existing) = audit {
        if existing.outcome == PromotionOutcome::Promote
            && requested.outcome != PromotionOutcome::Promote
        {
            return Err(PromotionApplyError::AuditOutcomeMismatch);
        }
    }
    Ok(())
}

fn ensure_promote_event_materialized(
    project_path: &str,
    promotion_key: &PromotionKey,
    event: &Option<PromotedEventOutcome>,
) -> Result<(), PromotionApplyError> {
    let event_id = promoted_event_id(promotion_key);
    if get_event(project_path, &event_id)?.is_none() {
        return Err(PromotionApplyError::PromoteEventNotMaterialized(event_id));
    }
    if event.is_none() {
        return Err(PromotionApplyError::PromoteEventNotMaterialized(event_id));
    }
    Ok(())
}

/// Applies one promotion decision with intelligence seam wiring for ambiguous cases.
pub fn apply_promotion_decision_with_intelligence(
    project_path: &str,
    request: &PromotionApplyRequest,
    intelligence: &dyn ContinuityIntelligence,
) -> Result<ApplyPromotionOutcome, PromotionApplyError> {
    let mut request = request.clone();
    if request.decision.outcome == PromotionOutcome::Ambiguous {
        let ambiguous = ambiguous_case_from_request(&request);
        request.decision =
            resolve_ambiguous_with_intelligence(&request.decision, &ambiguous, intelligence)?;
    }

    let outcome = apply_promotion_decision_core(project_path, &request)?;
    ensure_existing_promote_audit_matches(&outcome.audit, &request.decision)?;

    if request.decision.outcome == PromotionOutcome::Promote {
        ensure_promote_event_materialized(
            project_path,
            &request.decision.promotion_key,
            &outcome.event,
        )?;
    }

    Ok(outcome)
}

/// Applies one promotion decision using the default noop intelligence seam.
pub fn apply_promotion_decision(
    project_path: &str,
    request: &PromotionApplyRequest,
) -> Result<ApplyPromotionOutcome, PromotionApplyError> {
    apply_promotion_decision_with_intelligence(project_path, request, &NoopContinuityIntelligence)
}

/// Applies one promotion decision: audit for every outcome; WorkEvent append for `Promote` only.
///
/// Does not read the signal inbox, call `process_pending`, replay signals, or move files.
fn apply_promotion_decision_core(
    project_path: &str,
    request: &PromotionApplyRequest,
) -> Result<ApplyPromotionOutcome, PromotionApplyError> {
    let project = load_project(project_path).map_err(PromotionApplyError::Audit)?;
    if request.workspace_id != project.id {
        return Err(PromotionApplyError::WorkspaceMismatch);
    }

    let derived_key =
        PromotionKey::from_inputs(&request.workspace_id, &request.decision.source_signal_ids)?;
    if derived_key != request.decision.promotion_key {
        return Err(PromotionApplyError::KeyMismatch);
    }

    if request.decision.outcome == PromotionOutcome::Promote {
        validate_promote_request(request)?;
    }

    let record = PromotionDecisionRecord::from_decision(
        request.workspace_id.clone(),
        request.decision.clone(),
        request.evidence.clone(),
        request.recorded_at.clone(),
    );
    let audit = write_decision_record(project_path, &record)?;

    let event = if request.decision.outcome == PromotionOutcome::Promote {
        let proposed = request
            .decision
            .proposed_composition
            .as_ref()
            .expect("validated above");
        let composed = compose_work_event_from_group(
            &request.workspace_id,
            &request.decision.promotion_key,
            &request.signals,
            proposed,
        )?;
        Some(append_promoted_event(project_path, &composed)?)
    } else {
        None
    };

    Ok(ApplyPromotionOutcome { audit, event })
}

/// Convenience wrapper for a correlated promotion result from Checkpoint D.
pub fn apply_correlation_decision(
    project_path: &str,
    correlated: &CorrelationDecision,
    recorded_at: &str,
) -> Result<ApplyPromotionOutcome, PromotionApplyError> {
    apply_promotion_decision(
        project_path,
        &PromotionApplyRequest {
            workspace_id: correlated.group.workspace_id.clone(),
            decision: correlated.decision.clone(),
            signals: correlated.group.signals.clone(),
            evidence: Some(correlated.evidence.clone()),
            recorded_at: recorded_at.to_string(),
        },
    )
}

// ============================================================================
// SHA-256 (no external dependency; frozen promotionKey digest)
// ============================================================================

fn sha256_hex(input: &[u8]) -> String {
    sha256(input)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut padded = data.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in chunk.chunks(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    let mut out = [0u8; 32];
    for (i, word) in state.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn promotion_key_material_is_deterministic() {
        let a = promotion_key_material("ws-1", &["sig-b".into(), "sig-a".into()]).unwrap();
        let b = promotion_key_material("ws-1", &["sig-a".into(), "sig-b".into()]).unwrap();
        assert_eq!(a, b);
        assert_eq!(a, "promote-v1|ws-1|sig-a,sig-b");
    }

    #[test]
    fn promotion_key_from_inputs_hashes_material() {
        let key = PromotionKey::from_inputs("ws-1", &["sig-a".into()]).unwrap();
        assert_eq!(key.as_str().len(), 64);
        assert_eq!(
            key,
            PromotionKey::from_material("promote-v1|ws-1|sig-a".into()).unwrap()
        );
    }
}
