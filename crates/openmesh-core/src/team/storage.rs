//! Team workspace persistence under `.openmesh/team/`.

use crate::storage::{get_project_dir, read_project, Project};
use crate::team::contract::{
    validate_team_workspace, TeamMember, TeamMemberRole, TeamWorkspace, TEAM_DIR,
    TEAM_PROTOCOL_VERSION,
};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const WORKSPACE_FILE: &str = "workspace.json";
const TEMP: &str = "team-tmp";

#[derive(Debug, thiserror::Error)]
pub enum TeamStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("team workspace missing")]
    Missing,
    #[error("team already initialized")]
    AlreadyExists,
    #[error("validation: {0}")]
    Validation(String),
    #[error("member not found: {0}")]
    MemberNotFound(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

pub fn team_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(TEAM_DIR)
}

pub fn workspace_path(project_path: &str) -> PathBuf {
    team_dir(project_path).join(WORKSPACE_FILE)
}

fn load_project(project_path: &str) -> Result<Project, TeamStorageError> {
    read_project(project_path, "project.json").ok_or(TeamStorageError::ProjectNotInitialized)
}

pub fn init_team_workspace(
    project_path: &str,
    display_name: &str,
    owner_label: &str,
    team_id: Option<String>,
) -> Result<TeamWorkspace, TeamStorageError> {
    let project = load_project(project_path)?;
    let path = workspace_path(project_path);
    if path.exists() {
        return Err(TeamStorageError::AlreadyExists);
    }
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let team_id = team_id.unwrap_or_else(|| format!("team-{}", project.id));
    let owner = TeamMember {
        member_id: "owner-local".into(),
        label: owner_label.trim().to_string(),
        role: TeamMemberRole::Owner,
        mesh_peer_id: None,
        proxy_profile_id: None,
        remote_workspace_id: Some(project.id.clone()),
        joined_at: now.clone(),
    };
    let ws = TeamWorkspace {
        protocol_version: TEAM_PROTOCOL_VERSION.into(),
        team_id,
        display_name: display_name.trim().to_string(),
        host_workspace_id: project.id,
        members: vec![owner],
        created_at: now.clone(),
        updated_at: now,
        limitations: vec![
            "team workspace foundation alpha — local registry only".into(),
            "not multi-tenant cloud admin".into(),
        ],
    };
    write_team_workspace(project_path, &ws)?;
    Ok(ws)
}

pub fn write_team_workspace(
    project_path: &str,
    ws: &TeamWorkspace,
) -> Result<(), TeamStorageError> {
    let _ = load_project(project_path)?;
    validate_team_workspace(ws).map_err(|e| TeamStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(team_dir(project_path)).map_err(|_| TeamStorageError::Io)?;
    write_json_atomic(&workspace_path(project_path), ws)
}

pub fn read_team_workspace(project_path: &str) -> Result<TeamWorkspace, TeamStorageError> {
    let _ = load_project(project_path)?;
    let path = workspace_path(project_path);
    if !path.exists() {
        return Err(TeamStorageError::Missing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| TeamStorageError::Io)?;
    let ws: TeamWorkspace =
        serde_json::from_str(&raw).map_err(|_| TeamStorageError::MalformedJson)?;
    validate_team_workspace(&ws).map_err(|e| TeamStorageError::Validation(e.to_string()))?;
    Ok(ws)
}

pub fn list_team_members(project_path: &str) -> Result<Vec<TeamMember>, TeamStorageError> {
    Ok(read_team_workspace(project_path)?.members)
}

pub fn add_team_member(
    project_path: &str,
    member: TeamMember,
) -> Result<TeamWorkspace, TeamStorageError> {
    let mut ws = read_team_workspace(project_path)?;
    if ws.members.iter().any(|m| m.member_id == member.member_id) {
        return Err(TeamStorageError::Validation(format!(
            "member already exists: {}",
            member.member_id
        )));
    }
    ws.members.push(member);
    ws.updated_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_team_workspace(project_path, &ws)?;
    Ok(ws)
}

pub fn remove_team_member(
    project_path: &str,
    member_id: &str,
) -> Result<TeamWorkspace, TeamStorageError> {
    let mut ws = read_team_workspace(project_path)?;
    let before = ws.members.len();
    ws.members.retain(|m| m.member_id != member_id);
    if ws.members.len() == before {
        return Err(TeamStorageError::MemberNotFound(member_id.into()));
    }
    if !ws
        .members
        .iter()
        .any(|m| matches!(m.role, TeamMemberRole::Owner))
    {
        return Err(TeamStorageError::Validation(
            "cannot remove last owner".into(),
        ));
    }
    ws.updated_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_team_workspace(project_path, &ws)?;
    Ok(ws)
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), TeamStorageError> {
    let parent = path.parent().ok_or(TeamStorageError::Io)?;
    fs::create_dir_all(parent).map_err(|_| TeamStorageError::Io)?;
    let temp = path.with_extension(TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| TeamStorageError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| TeamStorageError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| TeamStorageError::Io)?;
        file.sync_all().map_err(|_| TeamStorageError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| TeamStorageError::Io)
}
