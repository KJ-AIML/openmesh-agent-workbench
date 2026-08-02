//! Pure wire contracts for Team Cloud Beta (0.1.16).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const TEAM_CLOUD_PROTOCOL_VERSION: &str = "1.0";
pub const TEAM_CLOUD_DIR: &str = "team-cloud";
pub const MAX_TEAM_ID_BYTES: usize = 128;
pub const MAX_PATH_BYTES: usize = 512;
pub const MAX_SYNC_PATHS: usize = 32;
pub const MAX_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 256;

/// Deployment mode for the team cloud tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeamCloudMode {
    /// Local process simulating team cloud (default beta).
    LocalSim,
    /// Reserved remote cloud runtime (scaffold only; no multi-region).
    CloudScaffold,
}

/// Team-scoped cloud / online tier configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamCloudConfig {
    pub protocol_version: String,
    /// Must match or link to TeamWorkspace.team_id when team exists.
    pub team_id: String,
    pub host_workspace_id: String,
    pub mode: TeamCloudMode,
    /// Optional link to always-online proxy id (0.1.12).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub online_proxy_id: Option<String>,
    /// Always true in beta — full-repo upload is forbidden.
    pub selective_sync: bool,
    /// Relative paths under the project allowed for selective sync (scaffold).
    #[serde(default)]
    pub sync_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at: Option<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Dry-run / local-sim sync plan — never performs remote upload in beta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamCloudSyncPlan {
    pub protocol_version: String,
    pub team_id: String,
    pub mode: TeamCloudMode,
    pub generated_at: String,
    /// Paths that *would* be considered for selective sync.
    pub planned_paths: Vec<String>,
    /// True when this is a scaffold-only plan (no network).
    pub scaffold_only: bool,
    pub note: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TeamCloudValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("team_id invalid")]
    InvalidTeamId,
    #[error("host_workspace_id empty")]
    EmptyHostWorkspace,
    #[error("selective_sync must be true (full-repo upload forbidden)")]
    SelectiveSyncRequired,
    #[error("invalid sync path")]
    InvalidSyncPath,
    #[error("too many sync paths")]
    TooManySyncPaths,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("limitations bounds")]
    LimitationsBounds,
    #[error("online_proxy_id invalid")]
    InvalidOnlineProxyId,
}

pub fn validate_team_cloud_config(c: &TeamCloudConfig) -> Result<(), TeamCloudValidationError> {
    if c.protocol_version != TEAM_CLOUD_PROTOCOL_VERSION {
        return Err(TeamCloudValidationError::UnsupportedProtocol {
            found: c.protocol_version.clone(),
        });
    }
    if c.team_id.trim().is_empty()
        || c.team_id.len() > MAX_TEAM_ID_BYTES
        || c.team_id.contains("..")
        || c.team_id.contains('/')
    {
        return Err(TeamCloudValidationError::InvalidTeamId);
    }
    if c.host_workspace_id.trim().is_empty() {
        return Err(TeamCloudValidationError::EmptyHostWorkspace);
    }
    if !c.selective_sync {
        return Err(TeamCloudValidationError::SelectiveSyncRequired);
    }
    if c.sync_paths.len() > MAX_SYNC_PATHS {
        return Err(TeamCloudValidationError::TooManySyncPaths);
    }
    for p in &c.sync_paths {
        if p.trim().is_empty()
            || p.len() > MAX_PATH_BYTES
            || p.contains("..")
            || p.starts_with('/')
            || p.contains('\\')
        {
            return Err(TeamCloudValidationError::InvalidSyncPath);
        }
    }
    if let Some(id) = &c.online_proxy_id {
        if id.trim().is_empty() || id.len() > MAX_TEAM_ID_BYTES || id.contains("..") || id.contains('/')
        {
            return Err(TeamCloudValidationError::InvalidOnlineProxyId);
        }
    }
    if c.limitations.len() > MAX_LIMITATIONS
        || c.limitations.iter().any(|l| l.len() > MAX_LIMITATION_BYTES)
    {
        return Err(TeamCloudValidationError::LimitationsBounds);
    }
    validate_utc_timestamp(&c.created_at).map_err(TeamCloudValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&c.updated_at).map_err(TeamCloudValidationError::InvalidTimestamp)?;
    if let Some(t) = &c.last_sync_at {
        validate_utc_timestamp(t).map_err(TeamCloudValidationError::InvalidTimestamp)?;
    }
    Ok(())
}

pub fn validate_team_cloud_sync_plan(p: &TeamCloudSyncPlan) -> Result<(), TeamCloudValidationError> {
    if p.protocol_version != TEAM_CLOUD_PROTOCOL_VERSION {
        return Err(TeamCloudValidationError::UnsupportedProtocol {
            found: p.protocol_version.clone(),
        });
    }
    if p.team_id.trim().is_empty() {
        return Err(TeamCloudValidationError::InvalidTeamId);
    }
    validate_utc_timestamp(&p.generated_at).map_err(TeamCloudValidationError::InvalidTimestamp)?;
    if p.planned_paths.len() > MAX_SYNC_PATHS {
        return Err(TeamCloudValidationError::TooManySyncPaths);
    }
    for path in &p.planned_paths {
        if path.contains("..") || path.starts_with('/') {
            return Err(TeamCloudValidationError::InvalidSyncPath);
        }
    }
    if !p.scaffold_only {
        // Beta: only scaffold plans are valid wire shapes we emit.
        // (Remote upload is not implemented; refuse non-scaffold plans.)
        return Err(TeamCloudValidationError::SelectiveSyncRequired);
    }
    Ok(())
}
