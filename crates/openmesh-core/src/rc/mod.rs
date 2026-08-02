//! Dev Track 0.1.21 — 1.0 Release Candidate Program.
//!
//! RC checklist, regression matrix, freeze policy toward 1.0.
//! No feature expansion in this track.

pub mod contract;
pub mod evaluate;
pub mod storage;

pub use contract::{
    validate_rc_pack, RcCheckItem, RcCheckStatus, RcFreezePolicy, RcPack, RcRegressionRow,
    RcSeverity, RC_DIR, RC_PROTOCOL_VERSION,
};
pub use evaluate::{build_rc_pack, RcEvaluateError};
pub use storage::{read_rc_pack, write_rc_pack, RcStorageError};
