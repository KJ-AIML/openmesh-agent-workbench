// ============================================================================
// OpenMesh Work Continuity Domain Contracts — Dev Track 0.1.3.1
// ============================================================================
// Minimum ownership-boundary contracts only. No serialization schema frozen,
// no persistence, no promotion logic. See:
//   .heli-harness/state/reports/openmesh-0.1.3.1-execution-plan.md, section 5.
//
// What this module deliberately did NOT introduce in 0.1.3.1 (Category B):
//   CurrentStateProjection and PendingAttention were deferred to 0.1.3.7.
// Dev Track 0.1.3.6 Checkpoint A — `EvidenceRef::GitState` (WEC-33) and WorkSignal
// protocol `1.1` compatibility for Git evidence producers.
// Dev Track 0.1.3.7 Checkpoint A — `CurrentStateProjection`, `PendingAttentionItem`,
// and `CatchUpView` wire contracts (pure types + validation; no I/O).
//
// Dev Track 0.1.3.4 Checkpoint A hardens the serializable WorkEvent wire shape
// and EvidenceAttachment model. Ledger persistence is Checkpoint B.
// ============================================================================

pub mod context_pack;
pub mod corrections;
pub mod events;
pub mod profile;
pub mod projections;
pub mod proxy_draft;
pub mod signals;

pub use context_pack::*;
pub use corrections::*;
pub use events::*;
pub use profile::*;
pub use projections::*;
pub use proxy_draft::*;
pub use signals::*;

#[cfg(test)]
mod tests;
