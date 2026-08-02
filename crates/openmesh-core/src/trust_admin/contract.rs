//! Pure wire contracts for Trust, Privacy & Admin Beta (0.1.17).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const TRUST_ADMIN_PROTOCOL_VERSION: &str = "1.0";
pub const TRUST_ADMIN_DIR: &str = "trust-admin";
pub const MAX_TEAM_ID_BYTES: usize = 128;
pub const MAX_MEMBER_ID_BYTES: usize = 128;
pub const MAX_ALLOWLIST: usize = 128;
pub const MAX_ADMINS: usize = 32;
pub const MAX_NOTE_BYTES: usize = 256;
pub const MAX_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 256;

/// How remote team/mesh queries are authorized.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum QueryAllowlistMode {
    /// Any linked member/peer may be queried (subject to other fail-closed rules).
    AllowAll,
    /// Only entries on the allowlist may be queried.
    AllowlistOnly,
    /// Remote query is disabled for the team.
    DenyAll,
}

/// One allowlisted query target (member and/or mesh peer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueryAllowEntry {
    /// Team member id (preferred).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_id: Option<String>,
    /// Mesh peer id (optional; matches team member mesh_peer_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_peer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    pub added_at: String,
}

/// Team trust / privacy / admin policy snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamTrustPolicy {
    pub protocol_version: String,
    pub team_id: String,
    pub host_workspace_id: String,
    /// When false, remote query is refused regardless of allowlist.
    pub remote_query_enabled: bool,
    pub query_allowlist_mode: QueryAllowlistMode,
    #[serde(default)]
    pub query_allowlist: Vec<QueryAllowEntry>,
    /// Always true in beta — secrets fail closed (enforced by validator).
    pub secret_topics_fail_closed: bool,
    /// Always false in beta — secret export forbidden (enforced by validator).
    pub allow_secret_export: bool,
    /// Always true — selective sync required (ties to team cloud).
    pub sync_require_selective: bool,
    /// Member ids authorized to mutate policy (admin audit actors).
    #[serde(default)]
    pub admin_member_ids: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TrustAdminValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("team_id invalid")]
    InvalidTeamId,
    #[error("host_workspace_id empty")]
    EmptyHostWorkspace,
    #[error("secret_topics_fail_closed must be true")]
    SecretFailClosedRequired,
    #[error("allow_secret_export must be false")]
    SecretExportForbidden,
    #[error("sync_require_selective must be true")]
    SelectiveSyncRequired,
    #[error("invalid allowlist entry")]
    InvalidAllowEntry,
    #[error("too many allowlist entries")]
    TooManyAllowEntries,
    #[error("too many admins")]
    TooManyAdmins,
    #[error("invalid admin member id")]
    InvalidAdminId,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("limitations bounds")]
    LimitationsBounds,
}

pub fn validate_query_allow_entry(e: &QueryAllowEntry) -> Result<(), TrustAdminValidationError> {
    let has_member = e
        .member_id
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    let has_peer = e
        .mesh_peer_id
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if !has_member && !has_peer {
        return Err(TrustAdminValidationError::InvalidAllowEntry);
    }
    if let Some(m) = &e.member_id {
        if m.len() > MAX_MEMBER_ID_BYTES || m.contains("..") || m.contains('/') {
            return Err(TrustAdminValidationError::InvalidAllowEntry);
        }
    }
    if let Some(p) = &e.mesh_peer_id {
        if p.len() > MAX_MEMBER_ID_BYTES || p.contains("..") || p.contains('/') {
            return Err(TrustAdminValidationError::InvalidAllowEntry);
        }
    }
    if let Some(n) = &e.note {
        if n.len() > MAX_NOTE_BYTES {
            return Err(TrustAdminValidationError::InvalidAllowEntry);
        }
    }
    validate_utc_timestamp(&e.added_at).map_err(TrustAdminValidationError::InvalidTimestamp)?;
    Ok(())
}

pub fn validate_team_trust_policy(p: &TeamTrustPolicy) -> Result<(), TrustAdminValidationError> {
    if p.protocol_version != TRUST_ADMIN_PROTOCOL_VERSION {
        return Err(TrustAdminValidationError::UnsupportedProtocol {
            found: p.protocol_version.clone(),
        });
    }
    if p.team_id.trim().is_empty()
        || p.team_id.len() > MAX_TEAM_ID_BYTES
        || p.team_id.contains("..")
        || p.team_id.contains('/')
    {
        return Err(TrustAdminValidationError::InvalidTeamId);
    }
    if p.host_workspace_id.trim().is_empty() {
        return Err(TrustAdminValidationError::EmptyHostWorkspace);
    }
    if !p.secret_topics_fail_closed {
        return Err(TrustAdminValidationError::SecretFailClosedRequired);
    }
    if p.allow_secret_export {
        return Err(TrustAdminValidationError::SecretExportForbidden);
    }
    if !p.sync_require_selective {
        return Err(TrustAdminValidationError::SelectiveSyncRequired);
    }
    if p.query_allowlist.len() > MAX_ALLOWLIST {
        return Err(TrustAdminValidationError::TooManyAllowEntries);
    }
    for e in &p.query_allowlist {
        validate_query_allow_entry(e)?;
    }
    if p.admin_member_ids.len() > MAX_ADMINS {
        return Err(TrustAdminValidationError::TooManyAdmins);
    }
    for a in &p.admin_member_ids {
        if a.trim().is_empty() || a.len() > MAX_MEMBER_ID_BYTES || a.contains("..") || a.contains('/')
        {
            return Err(TrustAdminValidationError::InvalidAdminId);
        }
    }
    if p.limitations.len() > MAX_LIMITATIONS
        || p.limitations.iter().any(|l| l.len() > MAX_LIMITATION_BYTES)
    {
        return Err(TrustAdminValidationError::LimitationsBounds);
    }
    validate_utc_timestamp(&p.created_at).map_err(TrustAdminValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&p.updated_at).map_err(TrustAdminValidationError::InvalidTimestamp)?;
    Ok(())
}
