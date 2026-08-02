//! Dev Track 0.1.10 — Two-Person Mesh (local file envelopes).
//!
//! Checkpoint A: domain contract + pure validators (no I/O).
//! Checkpoint B: local peer registry under `.openmesh/mesh/peers/`.
//! Checkpoint C: export builder + outbox write.
//! Later: import, list/show envelopes.

pub mod contract;
pub mod export;
pub mod peers;

pub use contract::{
    validate_envelope_id_for_storage, validate_mesh_envelope, validate_mesh_peer_ref, MeshEnvelope,
    MeshEvidenceItem, MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax, MeshValidationError,
    MESH_DIR, MESH_ENVELOPE_PROTOCOL_VERSION, MESH_INBOX_DIR, MESH_OUTBOX_DIR, MESH_PEERS_DIR,
    MAX_ENVELOPE_ID_BYTES, MAX_EVIDENCE_ITEMS, MAX_PEER_LABEL_BYTES,
};
pub use export::{
    build_mesh_export_envelope, export_mesh_envelope_to_outbox, outbox_dir, outbox_envelope_path,
    read_outbox_envelope, to_peer_from_registry, write_outbox_envelope, BuildMeshExportRequest,
    MeshExportError,
};
pub use peers::{
    add_peer, list_peer_ids, list_peers, peer_id_from_label, peer_path, peers_dir, read_peer,
    validate_mesh_peer_record, validate_peer_id_for_storage, write_peer, MeshPeerError,
    MeshPeerRecord, MESH_PEER_RECORD_PROTOCOL_VERSION,
};
