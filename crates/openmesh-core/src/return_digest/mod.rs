//! Dev Track 0.1.9 — Pending Questions & Return Digest.
//!
//! Bridges existing pending sources by projection (proxy pending, continuity
//! attention, unresolved signals) and builds an on-demand return digest on top
//! of Catch-up + handoff notes. No new mesh/sync; local-only.

pub mod contract;
pub mod digest;
pub mod pending;

pub use contract::{
    validate_pending_question_item, validate_pending_questions_view, validate_return_digest,
    HandoffDigestRef, PendingQuestionItem, PendingQuestionSourceCounts, PendingQuestionSourceKind,
    PendingQuestionsView, ReturnDigest, ReturnDigestValidationError,
    PENDING_QUESTIONS_PROTOCOL_VERSION, RETURN_DIGEST_PROTOCOL_VERSION,
};
pub use digest::{build_return_digest, ReturnDigestError};
pub use pending::{
    build_pending_questions_for_project, build_pending_questions_view, PendingQuestionsError,
};
