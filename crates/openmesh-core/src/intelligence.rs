// ============================================================================
// Local Continuity Intelligence seam — Dev Track 0.1.3.5
// ============================================================================
// Checkpoint A: proposal-only contract.
// Checkpoint F: hardened noop seam + deterministic proposal validation.
// No model runtime, no canonical writes, no I/O, no network.
// See: .heli-harness/state/reports/openmesh-0.1.3.5-execution-plan.md §3.8

use crate::promotion::{
    AmbiguousPromotionCase, PromotionOutcome, PromotionProposal, ProposedEventComposition,
};

/// Frozen contract note for consumers and boundary guards.
pub const INTELLIGENCE_SEAM_CONTRACT_NOTE: &str =
    "proposal-only; no I/O, network, model calls, or canonical writes";

/// Extension point for future Local Continuity Intelligence (Runtime Architecture §15).
///
/// Implementations may suggest how to resolve ambiguous promotion cases. They
/// must never write canonical truth directly.
pub trait ContinuityIntelligence {
    /// Returns a side-effect-free proposal. Must not perform file I/O, network
    /// calls, model inference, promotion audit writes, WorkEvent appends, or
    /// signal inbox mutation.
    fn propose(&self, case: &AmbiguousPromotionCase) -> PromotionProposal;
}

/// Default seam: no model, no network, no side effects — returns no proposal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoopContinuityIntelligence;

impl ContinuityIntelligence for NoopContinuityIntelligence {
    fn propose(&self, case: &AmbiguousPromotionCase) -> PromotionProposal {
        let proposal = PromotionProposal::none();
        debug_assert!(proposal_preserves_source_signal_refs(&proposal, case));
        debug_assert!(validate_proposal_contract(&proposal));
        proposal
    }
}

/// Returns true when a proposal obeys the seam contract (proposal-only, no fake certainty).
pub fn validate_proposal_contract(proposal: &PromotionProposal) -> bool {
    if !proposal.is_side_effect_free() {
        return false;
    }
    if !proposal.has_proposal {
        return proposal.suggested_outcome.is_none() && proposal.suggested_composition.is_none();
    }
    if proposal.suggested_outcome == Some(PromotionOutcome::Ambiguous) {
        return false;
    }
    if proposal.suggested_outcome == Some(PromotionOutcome::Promote) {
        return proposal
            .suggested_composition
            .as_ref()
            .is_some_and(proposed_composition_is_complete);
    }
    true
}

fn proposed_composition_is_complete(composition: &ProposedEventComposition) -> bool {
    !composition.kind.trim().is_empty()
        && !composition.summary.trim().is_empty()
        && !composition.timestamp.trim().is_empty()
        && !composition.producer_signal_evidence_ids.is_empty()
}

/// Proposals must not drop or rewrite source signal ids from the ambiguous case.
pub fn proposal_preserves_source_signal_refs(
    proposal: &PromotionProposal,
    case: &AmbiguousPromotionCase,
) -> bool {
    let source_ids = case.case.signal_ids();
    if !proposal.has_proposal {
        return true;
    }
    let Some(composition) = proposal.suggested_composition.as_ref() else {
        return true;
    };
    composition
        .producer_signal_evidence_ids
        .iter()
        .all(|id| source_ids.iter().any(|src| src == id))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::domain::{ActorRef, EvidenceRef, ProducerRef, WorkSignalKind};
    use crate::promotion::{PromotionCase, SignalRef};

    fn sample_case() -> AmbiguousPromotionCase {
        AmbiguousPromotionCase {
            case: PromotionCase {
                workspace_id: "ws-test".into(),
                signals: vec![SignalRef {
                    signal_id: "sig-a".into(),
                    kind: WorkSignalKind::Handoff,
                    summary: "handoff one".into(),
                    producer: ProducerRef::Reporter("codex".into()),
                    actor: ActorRef::Unknown,
                    timestamp: "2026-07-15T12:00:00Z".into(),
                    correlation_hint: Some("feat/x".into()),
                    evidence_refs: vec![EvidenceRef::ProducerSignal("sig-a".into())],
                }],
                correlation_hint: Some("feat/x".into()),
            },
            reason: "kind conflict".into(),
            qualification_notes: vec!["dominant kind conflict".into()],
        }
    }

    #[test]
    fn noop_proposal_is_deterministic() {
        let case = sample_case();
        let seam = NoopContinuityIntelligence;
        let first = seam.propose(&case);
        let second = seam.propose(&case);
        assert_eq!(first, second);
        assert_eq!(first, PromotionProposal::none());
    }
}
