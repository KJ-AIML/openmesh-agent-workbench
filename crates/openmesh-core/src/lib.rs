// openmesh-core: shared Rust foundation for OpenMesh Desktop and a future OpenMesh CLI.
// Populated incrementally across Dev Track 0.1.3.1's checkpoints.
//
// Dev Track 0.1.3.4 — Canonical WorkEvent & Evidence Ledger:
// - `domain` — wire types (`WorkEvent`, `EvidenceAttachment`, `EvidenceRef`) and
//   `validate_event_semantics`
// - `events` — project-scoped ledger persistence, recovery, and correction helpers
//
// Dev Track 0.1.3.5 Checkpoint A — promotion domain contracts (no I/O yet):
// - `promotion` — outcome taxonomy, decision/case/evidence contracts, idempotency key
// - `intelligence` — `ContinuityIntelligence` seam trait (no-op default)
//
// Dev Track 0.1.3.5 Checkpoint B — promotion audit persistence:
// - `promotion` — decision record storage under `.openmesh/events/promotion/decisions/`
//
// Dev Track 0.1.3.5 Checkpoint C — deterministic qualification (no I/O):
// - `promotion` — five-question scoring, kind matrix, `evaluate_promotion_case`
//
// Dev Track 0.1.3.5 Checkpoint D — correlation + duplicate/corroboration (no I/O):
// - `promotion` — `group_signals_by_correlation_hint`, `correlate_and_evaluate`
//
// Dev Track 0.1.3.5 Checkpoint E1 — WorkEvent protocol 1.1 compatibility:
// - `domain` — optional `WorkEvent.actor`, version-aware `validate_event_semantics`
// - `events` — ledger classification accepts protocol `1.0` and `1.1`
//
// Dev Track 0.1.3.5 Checkpoint E2 — promotion to WorkEvent ledger:
// - `promotion` — `apply_promotion_decision`, `compose_work_event_from_group`, idempotent append
//
// Dev Track 0.1.3.5 Checkpoint F — intelligence seam + boundary hardening:
// - `intelligence` — proposal-only `ContinuityIntelligence`, hardened `NoopContinuityIntelligence`
// - `promotion` — `resolve_ambiguous_with_intelligence`, audit/event consistency guards
//
// Ledger APIs are core-only in this track. CLI, Tauri, and Desktop do not expose
// WorkEvent ledger commands yet.

#[allow(dead_code)]
pub mod context;
pub mod context_service;
pub mod domain;
pub mod events;
pub mod index;
pub mod ingestion;
pub mod intelligence;
pub mod promotion;
pub mod signals;
pub mod storage;
