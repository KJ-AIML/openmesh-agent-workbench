//! Dev Track 0.1.18 — Connector Layer.
//!
//! External system-of-record connectors as **evidence producers only**.
//! Does not replace GitHub/Linear as source of record.

pub mod contract;
pub mod github_stub;
pub mod storage;

pub use contract::{
    validate_connector_descriptor, validate_connector_run, ConnectorDescriptor, ConnectorKind,
    ConnectorRole, ConnectorRun, EvidenceItemKind, ExternalEvidenceItem, CONNECTORS_DIR,
    CONNECTOR_PROTOCOL_VERSION,
};
pub use github_stub::collect_github_stub;
pub use storage::{
    init_or_register_connector, list_connectors, read_connector, write_connector_run,
    ConnectorStorageError,
};
