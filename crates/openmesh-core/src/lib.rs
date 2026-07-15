// openmesh-core: shared Rust foundation for OpenMesh Desktop and a future OpenMesh CLI.
// Populated incrementally across Dev Track 0.1.3.1's checkpoints.
//
// Dev Track 0.1.3.4 — Canonical WorkEvent & Evidence Ledger:
// - `domain` — wire types (`WorkEvent`, `EvidenceAttachment`, `EvidenceRef`) and
//   `validate_event_semantics`
// - `events` — project-scoped ledger persistence, recovery, and correction helpers
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
pub mod signals;
pub mod storage;
