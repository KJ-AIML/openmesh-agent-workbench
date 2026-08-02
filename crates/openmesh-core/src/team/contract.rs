//! Pure wire contracts for Team Workspace Foundation (0.1.15).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const TEAM_PROTOCOL_VERSION: &str = "1.0";
pub const TEAM_DIR: &str = "team";
pub const MAX_TEAM_ID_BYTES: usize = 128;
pub const MAX_TEAM_NAME_BYTES: usize = 256;
pub const MAX_MEMBER_LABEL_BYTES: usize = 128;
pub const MAX_MEMBERS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TeamMemberRole {
    Owner,
    Member,
    Observer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamMember {
    pub member_id: String,
    pub label: String,
    pub role: TeamMemberRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh_peer_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_workspace_id: Option<String>,
    pub joined_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TeamWorkspace {
    pub protocol_version: String,
    pub team_id: String,
    pub display_name: String,
    /// Local OpenMesh project / workspace id that hosts this team registry.
    pub host_workspace_id: String,
    #[serde(default)]
    pub members: Vec<TeamMember>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TeamValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("team_id invalid")]
    InvalidTeamId,
    #[error("display_name empty or too long")]
    InvalidDisplayName,
    #[error("host_workspace_id empty")]
    EmptyHostWorkspace,
    #[error("too many members (max {max})")]
    TooManyMembers { max: usize },
    #[error("member invalid: {0}")]
    Member(String),
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("duplicate member_id {0}")]
    DuplicateMember(String),
    #[error("team must have at least one owner")]
    NoOwner,
}

pub fn validate_team_member(m: &TeamMember) -> Result<(), TeamValidationError> {
    if m.member_id.trim().is_empty()
        || m.member_id.len() > MAX_TEAM_ID_BYTES
        || m.member_id.contains("..")
        || m.member_id.contains('/')
    {
        return Err(TeamValidationError::Member("member_id".into()));
    }
    if m.label.trim().is_empty() || m.label.len() > MAX_MEMBER_LABEL_BYTES {
        return Err(TeamValidationError::Member("label".into()));
    }
    validate_utc_timestamp(&m.joined_at).map_err(TeamValidationError::InvalidTimestamp)?;
    if let Some(p) = &m.mesh_peer_id {
        if p.trim().is_empty() || p.contains("..") || p.contains('/') {
            return Err(TeamValidationError::Member("mesh_peer_id".into()));
        }
    }
    Ok(())
}

pub fn validate_team_workspace(t: &TeamWorkspace) -> Result<(), TeamValidationError> {
    if t.protocol_version != TEAM_PROTOCOL_VERSION {
        return Err(TeamValidationError::UnsupportedProtocol {
            found: t.protocol_version.clone(),
        });
    }
    if t.team_id.trim().is_empty()
        || t.team_id.len() > MAX_TEAM_ID_BYTES
        || t.team_id.contains("..")
        || t.team_id.contains('/')
    {
        return Err(TeamValidationError::InvalidTeamId);
    }
    if t.display_name.trim().is_empty() || t.display_name.len() > MAX_TEAM_NAME_BYTES {
        return Err(TeamValidationError::InvalidDisplayName);
    }
    if t.host_workspace_id.trim().is_empty() {
        return Err(TeamValidationError::EmptyHostWorkspace);
    }
    if t.members.len() > MAX_MEMBERS {
        return Err(TeamValidationError::TooManyMembers { max: MAX_MEMBERS });
    }
    validate_utc_timestamp(&t.created_at).map_err(TeamValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&t.updated_at).map_err(TeamValidationError::InvalidTimestamp)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut owners = 0u32;
    for m in &t.members {
        validate_team_member(m)?;
        if !seen.insert(m.member_id.clone()) {
            return Err(TeamValidationError::DuplicateMember(m.member_id.clone()));
        }
        if matches!(m.role, TeamMemberRole::Owner) {
            owners += 1;
        }
    }
    if !t.members.is_empty() && owners == 0 {
        return Err(TeamValidationError::NoOwner);
    }
    Ok(())
}
