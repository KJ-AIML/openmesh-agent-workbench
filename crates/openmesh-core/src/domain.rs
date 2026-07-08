// ============================================================================
// OpenMesh Work Continuity Domain Contracts — Dev Track 0.1.3.1
// ============================================================================
// Minimum ownership-boundary contracts only. No serialization schema frozen,
// no persistence, no promotion logic. See:
//   .heli-harness/state/reports/openmesh-0.1.3.1-execution-plan.md, section 5.
//
// What this module deliberately does NOT introduce (Category B, see the
// execution plan's section 6 and Dev Track 0.1.3.1's non-goals):
//   - CurrentStateProjection (owned by 0.1.3.7)
//   - PendingAttention (owned by 0.1.3.7)
//   - a concrete correction-link field on WorkEvent (owned by 0.1.3.4)
//   - a Git-specific EvidenceRef variant (owned by 0.1.3.6)
// ============================================================================

use crate::context::Sensitivity;

/// Which system/integration emitted a WorkSignal — for later dedup/correlation
/// (Classification Pack cases WEC-26/WEC-32). Distinct from `ActorRef`: this is
/// about the producer, not who the claim/action belongs to.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProducerRef {
    /// OpenMesh's own native producer (e.g. manual checkpoints, snapshot creation).
    Native,
    /// Heli, read independently as an evidence producer.
    Heli,
    /// Local Git evidence (owned by 0.1.3.6 — this variant exists so ProducerRef
    /// can already be typed against it; no Git-specific producer logic runs yet).
    Git,
    /// An external agent Reporter Skill, identified by agent/tool name.
    Reporter(String),
}

/// Who a WorkSignal's claim or action is attributed to — distinct from
/// `ProducerRef` (which system emitted it). A bare identity discriminant only;
/// not the "Person and Proxy Identity" / role / responsibility profile system,
/// which is 0.1.4's job ("My Work Proxy Profile"). No authority logic here.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActorRef {
    Person(String),
    Device(String),
    Proxy(String),
    Unknown,
}

/// Why OpenMesh believes a claim/event — a reference, not the evidence content
/// itself. Deliberately `#[non_exhaustive]` so a future Git-ref variant (needed
/// by 0.1.3.6, Classification Pack case WEC-33) can be added without a breaking
/// change to any downstream consumer of this crate.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvidenceRef {
    /// A file/path reference (e.g. a relative path within a project).
    FilePath(String),
    /// A reference to another WorkSignal by its `signal_id` (corroboration).
    ProducerSignal(String),
}

/// WorkSignal's semantic kind. Fixed to the categories already settled in
/// Development Spec v1.6 Decision 1 / the Continuity Runtime Architecture §6 —
/// not an open/extensible list, since that list is already canonical.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkSignalKind {
    Progress,
    Decision,
    Blocker,
    BlockerResolved,
    ScopeChange,
    Milestone,
    ReviewRequired,
    UnresolvedQuestion,
    Handoff,
    SessionEnd,
    AgentSwitch,
}

/// An unpromoted claim/observation entering the continuity domain. Not yet
/// durable truth — only the Deterministic Continuity Pipeline (0.1.3.5, not
/// implemented here) decides whether this becomes a WorkEvent.
///
/// `producer` identifies which system/integration emitted this signal.
/// `actor` identifies whose claim or action this represents (an agent, a
/// human, or unknown) — a distinct field, not folded into `producer`. This
/// distinction is what lets an agent's completion claim, a verification
/// result, and a human's acceptance exist as separately-attributed records
/// (Classification Pack case WEC-29) instead of being inferred from evidence
/// presence alone.
#[derive(Debug, Clone)]
pub struct WorkSignal {
    pub signal_id: String,
    pub workspace_id: String,
    pub producer: ProducerRef,
    pub actor: ActorRef,
    pub kind: WorkSignalKind,
    pub summary: String,
    pub timestamp: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub correlation_hint: Option<String>,
    pub sensitivity: Sensitivity,
    pub protocol_version: String,
}

/// A durable, evidence-backed meaningful transition. `evidence_refs` is a
/// **list**, not a single reference, so that many signals may support one
/// WorkEvent (Classification Pack case CC-1) without forcing a cardinality.
///
/// Deliberately carries no `actor` field in this track (see `ActorRef`'s own
/// doc comment for why) and no concrete correction-link field (see the
/// `#[non_exhaustive]` attribute and the `new` constructor below, which leave
/// room for a future field without a breaking change — the exact correction
/// representation is 0.1.3.4's decision, not frozen here).
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WorkEvent {
    pub kind: String,
    pub summary: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub timestamp: String,
}

impl WorkEvent {
    pub fn new(
        kind: impl Into<String>,
        summary: impl Into<String>,
        evidence_refs: Vec<EvidenceRef>,
        timestamp: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            summary: summary.into(),
            evidence_refs,
            timestamp: timestamp.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal(
        id: &str,
        producer: ProducerRef,
        actor: ActorRef,
        kind: WorkSignalKind,
        correlation_hint: Option<&str>,
        evidence_refs: Vec<EvidenceRef>,
    ) -> WorkSignal {
        WorkSignal {
            signal_id: id.to_string(),
            workspace_id: "ws-1".to_string(),
            producer,
            actor,
            kind,
            summary: format!("signal {id}"),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            evidence_refs,
            correlation_hint: correlation_hint.map(|s| s.to_string()),
            sensitivity: Sensitivity::Private,
            protocol_version: "0.1.0".to_string(),
        }
    }

    // Category A — Positive Representability Tests (execution plan §6/§9.D1).

    /// WEC-29: an agent's completion claim, a verification result, and a
    /// human's acceptance exist as three separately-attributed records linked
    /// by a shared correlation hint — not one record with a boolean flag.
    #[test]
    fn wec_29_claim_verification_and_human_acceptance_are_distinct_records() {
        let claim = signal(
            "s-claim",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![],
        );
        let verification = signal(
            "s-verify",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Progress,
            Some("corr-1"),
            vec![EvidenceRef::FilePath("tests/output.log".into())],
        );
        let acceptance = signal(
            "s-accept",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            Some("corr-1"),
            vec![EvidenceRef::ProducerSignal("s-verify".into())],
        );

        // All three share the same correlation hint, so they're correlatable...
        assert_eq!(claim.correlation_hint, verification.correlation_hint);
        assert_eq!(verification.correlation_hint, acceptance.correlation_hint);
        // ...but remain three distinct records, not one record with a status flag.
        assert_ne!(claim.signal_id, verification.signal_id);
        assert_ne!(verification.signal_id, acceptance.signal_id);
        // Acceptance is distinctly human-attributed; the claim is not.
        assert!(matches!(acceptance.actor, ActorRef::Person(_)));
        assert!(!matches!(claim.actor, ActorRef::Person(_)));
    }

    /// CC-1: many signals may support one WorkEvent, and producer count does
    /// not determine event count — both a single event with several evidence
    /// refs and several single-evidence events must be constructible.
    #[test]
    fn cc_1_many_signals_can_support_one_event_or_several() {
        let refs: Vec<EvidenceRef> = (0..4)
            .map(|i| EvidenceRef::ProducerSignal(format!("s-{i}")))
            .collect();

        let one_event_many_refs = WorkEvent::new(
            "bugfix",
            "clear_project fixed",
            refs.clone(),
            "2026-07-08T00:00:00Z",
        );
        assert_eq!(one_event_many_refs.evidence_refs.len(), 4);

        let four_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| {
                WorkEvent::new(
                    "bugfix",
                    "clear_project fixed",
                    vec![r],
                    "2026-07-08T00:00:00Z",
                )
            })
            .collect();
        assert_eq!(four_events.len(), 4);
        assert!(four_events.iter().all(|e| e.evidence_refs.len() == 1));
    }

    /// WEC-26 + WEC-32: same-producer duplicate vs. cross-producer
    /// corroboration must be distinguishable data, not the same shape.
    #[test]
    fn wec_26_32_duplicate_vs_corroborating_signals_are_distinguishable() {
        let duplicate_a = signal(
            "s-dup-a",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        let duplicate_b = signal(
            "s-dup-b",
            ProducerRef::Reporter("codex".into()),
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-dup"),
            vec![],
        );
        assert_eq!(duplicate_a.producer, duplicate_b.producer);

        let corroborating_a = signal(
            "s-corr-a",
            ProducerRef::Native,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        let corroborating_b = signal(
            "s-corr-b",
            ProducerRef::Git,
            ActorRef::Unknown,
            WorkSignalKind::Milestone,
            Some("corr-shared"),
            vec![],
        );
        assert_ne!(corroborating_a.producer, corroborating_b.producer);
        assert_eq!(
            corroborating_a.correlation_hint,
            corroborating_b.correlation_hint
        );
    }

    /// WEC-15: a claim existing only in conversation, not yet backed by
    /// durable evidence, is representable as a WorkSignal with no evidence
    /// refs — distinct from (and never auto-promoted to) a WorkEvent.
    #[test]
    fn wec_15_conversation_only_claim_has_no_evidence_refs() {
        let conversation_only = signal(
            "s-convo",
            ProducerRef::Native,
            ActorRef::Person("ter".into()),
            WorkSignalKind::Decision,
            None,
            vec![],
        );
        assert!(conversation_only.evidence_refs.is_empty());
    }

    /// WEC-30: a genuinely ambiguous case (one event or six?) must remain
    /// representable either way — the contracts must not resolve ambiguity.
    #[test]
    fn wec_30_ambiguous_one_event_or_six_both_remain_constructible() {
        let refs: Vec<EvidenceRef> = (0..6)
            .map(|i| EvidenceRef::ProducerSignal(format!("round-{i}")))
            .collect();

        let one_event = WorkEvent::new("stabilized", "0.1.2.5 stabilized", refs.clone(), "t");
        assert_eq!(one_event.evidence_refs.len(), 6);

        let six_events: Vec<WorkEvent> = refs
            .into_iter()
            .map(|r| WorkEvent::new("bugfix-round", "one closure round", vec![r], "t"))
            .collect();
        assert_eq!(six_events.len(), 6);
    }

    // Generic structural test (not a Classification Pack case). WEC-33
    // (Git-state evidence representability) remains Category B / deferred to
    // 0.1.3.6 — this test only exercises the two variants that already exist
    // today (`FilePath`, `ProducerSignal`); it makes no claim about Git,
    // uncommitted, or unpushed state representability.
    #[test]
    fn evidence_ref_variants_construct_and_compare() {
        let a = EvidenceRef::FilePath("docs/overview.md".into());
        let b = EvidenceRef::FilePath("docs/overview.md".into());
        let c = EvidenceRef::ProducerSignal("s-1".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
