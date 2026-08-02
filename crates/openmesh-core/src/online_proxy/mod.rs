//! Dev Track 0.1.12 — Always-Online Work Proxy Alpha.
//!
//! Scaffold for an always-available proxy runtime that answers with explicit
//! evidence-freshness disclosure. Alpha uses local + relay-received evidence;
//! not a multi-tenant cloud deployment.

pub mod ask;
pub mod contract;
pub mod storage;

pub use ask::{ask_online_proxy, OnlineProxyAskError, OnlineProxyAskRequest};
pub use contract::{
    validate_evidence_freshness_statement, validate_online_proxy_answer,
    validate_online_proxy_config, EvidenceFreshnessStatement, OnlineProxyAnswer,
    OnlineProxyConfig, OnlineProxyMode, ONLINE_PROXY_PROTOCOL_VERSION,
};
pub use storage::{
    config_path, online_proxy_dir, read_answer, read_config, write_answer, write_config,
    OnlineProxyStorageError, ONLINE_PROXY_DIR,
};
