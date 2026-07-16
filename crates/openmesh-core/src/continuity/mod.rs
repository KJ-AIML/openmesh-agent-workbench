//! Dev Track 0.1.3.7 — Current State & Catch-up read models.
//!
//! Checkpoint B: read-only loaders for local continuity inputs.
//! Checkpoint C: Current State builder and projection persistence.
//! Checkpoint D: on-demand Catch-up view builder.

pub mod catch_up;
pub mod current_state;
pub mod readers;

pub use catch_up::build_catch_up_view;
pub use current_state::{
    build_current_state_projection, current_state_projection_path,
    projections_dir as current_state_projections_dir, read_current_state_projection,
    rebuild_current_state_projection, write_current_state_projection, ContinuityError,
};
pub use readers::{
    classify_producer_signal_bucket, compute_source_counts, corrections_for_event,
    list_duplicate_signals, list_pending_signals, list_processed_signals, list_quarantine_signals,
    list_signal_bucket, load_continuity_input_snapshot, load_promotion_audit_records,
    load_work_events, projections_dir, ContinuityDiagnostic, ContinuityDiagnosticKind,
    ContinuityInputSnapshot, ContinuityReaderError, LoadedPromotionAudit, LoadedSignalBucket,
    LoadedWorkEvents, SignalBucket,
};
