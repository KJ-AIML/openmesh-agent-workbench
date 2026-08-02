//! Dev Track 0.1.10 — Two-Person Mesh (local file envelopes).
//!
//! Checkpoint A: domain contract + pure validators (no I/O).
//! Later: peers, export, import, CLI.

pub mod contract;

pub use contract::{
    validate_envelope_id_for_storage, validate_mesh_envelope, validate_mesh_peer_ref, MeshEnvelope,
    MeshEvidenceItem, MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax, MeshValidationError,
    MESH_DIR, MESH_ENVELOPE_PROTOCOL_VERSION, MESH_INBOX_DIR, MESH_OUTBOX_DIR, MESH_PEERS_DIR,
    MAX_ENVELOPE_ID_BYTES, MAX_EVIDENCE_ITEMS, MAX_PEER_LABEL_BYTES,
};
