//! Dev Track 0.1.17 — Trust, Privacy & Admin Beta.
//!
//! Explicit team trust/privacy/admin controls over sync and query.
//! Fail-closed on secrets; no enterprise IdP/SSO.

pub mod audit;
pub mod contract;
pub mod gate;
pub mod storage;

pub use audit::{append_audit_event, list_audit_events, AdminAuditEvent, AuditAction};
pub use contract::{
    validate_team_trust_policy, QueryAllowEntry, QueryAllowlistMode, TeamTrustPolicy,
    TRUST_ADMIN_DIR, TRUST_ADMIN_PROTOCOL_VERSION,
};
pub use gate::{evaluate_remote_query, QueryPermission, QueryPermissionDecision};
pub use storage::{
    init_trust_policy, read_trust_policy, trust_admin_dir, update_trust_policy, write_trust_policy,
    TrustAdminStorageError,
};
