//! Dev Track 0.1.20 — Enterprise Pilot Readiness.
//!
//! Security/privacy/admin pilot checklist with local evidence.
//! Not a customer production SLA.

pub mod contract;
pub mod evaluate;
pub mod storage;

pub use contract::{
    validate_pilot_pack, PilotCheckItem, PilotCheckStatus, PilotPack, RunbookStep, ThreatNote,
    PILOT_DIR, PILOT_PROTOCOL_VERSION,
};
pub use evaluate::{build_pilot_pack, PilotEvaluateError};
pub use storage::{read_pilot_pack, write_pilot_pack, PilotStorageError};
