//! Dev Track 0.1.10 — Two-Person Mesh (local file envelopes).
//! Dev Track 0.1.14 — Ter × Yo remote peer query (read-only).
//!
//! Checkpoint A: domain contract + pure validators (no I/O).
//! Checkpoint B: local peer registry under `.openmesh/mesh/peers/`.
//! Checkpoint C: export builder + outbox write.
//! Checkpoint D: import into inbox.
//! Checkpoint E: list/show peer evidence read model.
//! 0.1.14: query offline peer proxy from imported envelopes.

pub mod contract;
pub mod export;
pub mod import;
pub mod peers;
pub mod query;
pub mod view;

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
pub use import::{
    import_mesh_envelope, import_mesh_envelope_from_file, inbox_dir, inbox_envelope_path,
    list_inbox_envelope_ids, load_envelope_from_file, read_inbox_envelope, write_inbox_envelope,
    ImportMeshOptions, MeshImportError,
};
pub use peers::{
    add_peer, list_peer_ids, list_peers, peer_id_from_label, peer_path, peers_dir, read_peer,
    validate_mesh_peer_record, validate_peer_id_for_storage, write_peer, MeshPeerError,
    MeshPeerRecord, MESH_PEER_RECORD_PROTOCOL_VERSION,
};
pub use query::{
    query_remote_peer_proxy, read_query_answer, resolve_peer, validate_mesh_remote_query_answer,
    write_query_answer, MeshQueryError, MeshRemoteQueryAnswer, MeshRemoteQueryRequest,
    MESH_QUERIES_DIR, MESH_QUERY_PROTOCOL_VERSION,
};
pub use view::{
    list_envelope_summaries, list_outbox_envelope_ids, show_envelope, MeshEnvelopeSummary,
    MeshMailbox, MeshViewError,
};
