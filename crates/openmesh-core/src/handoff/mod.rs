//! Dev Track 0.1.8 — Handoff Note Engine.
//!
//! Checkpoint A: domain contract + pure validators (no I/O).
//! Checkpoints B–E: scope, builder, storage, markdown projection.

pub mod builder;
pub mod contract;
pub mod markdown;
pub mod scope;
pub mod storage;

pub use builder::{build_handoff_note, BuildHandoffRequest, HandoffBuildError};
pub use contract::{
    validate_handoff_id_for_storage, validate_handoff_note, validate_recipient_fields,
    validate_window_fields, HandoffFreshness,
    HandoffNote, HandoffRecipient, HandoffSection, HandoffSectionItem, HandoffStatus,
    HandoffValidationError, HANDOFF_NOTE_PROTOCOL_VERSION, MAX_HANDOFF_ID_BYTES,
    MAX_HANDOFF_ITEM_SUMMARY_BYTES, MAX_HANDOFF_LIMITATION_BYTES,
    MAX_HANDOFF_RECIPIENT_LABEL_BYTES, MAX_HANDOFF_SECTION_ITEMS, MAX_HANDOFF_WARNINGS,
    WORK_EVENT_HANDOFF_KIND,
};
pub use markdown::render_handoff_markdown;
pub use scope::{build_handoff_recipient, resolve_handoff_window};
pub use storage::{
    approve_handoff_note, handoff_dir, handoff_note_path, handoff_relative_path,
    link_handoff_work_event, list_handoff_ids, read_handoff_note, write_handoff_note,
    HandoffStorageError, HANDOFF_DIR,
};
