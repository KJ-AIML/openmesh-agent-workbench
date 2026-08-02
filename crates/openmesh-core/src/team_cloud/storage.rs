//! Team cloud persistence under `.openmesh/team-cloud/`.

use crate::storage::{get_project_dir, read_project, Project};
use crate::team::read_team_workspace;
use crate::team_cloud::contract::{
    validate_team_cloud_config, TeamCloudConfig, TeamCloudMode, TEAM_CLOUD_DIR,
    TEAM_CLOUD_PROTOCOL_VERSION,
};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = "config.json";
const TEMP: &str = "team-cloud-tmp";

#[derive(Debug, thiserror::Error)]
pub enum TeamCloudStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("team cloud config missing")]
    Missing,
    #[error("team cloud already initialized")]
    AlreadyExists,
    #[error("team workspace required first (run team init)")]
    TeamRequired,
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

pub fn team_cloud_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(TEAM_CLOUD_DIR)
}

pub fn config_path(project_path: &str) -> PathBuf {
    team_cloud_dir(project_path).join(CONFIG_FILE)
}

fn load_project(project_path: &str) -> Result<Project, TeamCloudStorageError> {
    read_project(project_path, "project.json").ok_or(TeamCloudStorageError::ProjectNotInitialized)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), TeamCloudStorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| TeamCloudStorageError::Io)?;
    }
    let tmp = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{TEMP}-{}", std::process::id()));
    {
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&tmp)
            .map_err(|_| TeamCloudStorageError::Io)?;
        f.write_all(bytes).map_err(|_| TeamCloudStorageError::Io)?;
        f.sync_all().map_err(|_| TeamCloudStorageError::Io)?;
    }
    fs::rename(&tmp, path).map_err(|_| TeamCloudStorageError::Io)
}

/// Default selective paths for team cloud beta (evidence + team registry only).
pub fn default_sync_paths() -> Vec<String> {
    vec![
        ".openmesh/team".into(),
        ".openmesh/mesh".into(),
        ".openmesh/online-proxy".into(),
        ".openmesh/relay".into(),
    ]
}

pub fn init_team_cloud(
    project_path: &str,
    mode: TeamCloudMode,
    online_proxy_id: Option<String>,
) -> Result<TeamCloudConfig, TeamCloudStorageError> {
    let project = load_project(project_path)?;
    if config_path(project_path).exists() {
        return Err(TeamCloudStorageError::AlreadyExists);
    }
    let team = read_team_workspace(project_path).map_err(|_| TeamCloudStorageError::TeamRequired)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let cfg = TeamCloudConfig {
        protocol_version: TEAM_CLOUD_PROTOCOL_VERSION.into(),
        team_id: team.team_id,
        host_workspace_id: project.id.clone(),
        mode,
        online_proxy_id,
        selective_sync: true,
        sync_paths: default_sync_paths(),
        last_sync_at: None,
        limitations: vec![
            "local-sim / cloud-scaffold only — not multi-region SaaS".into(),
            "selective sync only — no full-repo upload".into(),
            "sync-scaffold is dry-run (no network)".into(),
        ],
        created_at: now.clone(),
        updated_at: now,
    };
    write_team_cloud(project_path, &cfg)?;
    Ok(cfg)
}

pub fn write_team_cloud(
    project_path: &str,
    cfg: &TeamCloudConfig,
) -> Result<(), TeamCloudStorageError> {
    let _ = load_project(project_path)?;
    validate_team_cloud_config(cfg)
        .map_err(|e| TeamCloudStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(team_cloud_dir(project_path)).map_err(|_| TeamCloudStorageError::Io)?;
    let bytes = serde_json::to_vec_pretty(cfg).map_err(|_| TeamCloudStorageError::MalformedJson)?;
    atomic_write(&config_path(project_path), &bytes)
}

pub fn read_team_cloud(project_path: &str) -> Result<TeamCloudConfig, TeamCloudStorageError> {
    let _ = load_project(project_path)?;
    let path = config_path(project_path);
    if !path.exists() {
        return Err(TeamCloudStorageError::Missing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| TeamCloudStorageError::Io)?;
    let cfg: TeamCloudConfig =
        serde_json::from_str(&raw).map_err(|_| TeamCloudStorageError::MalformedJson)?;
    validate_team_cloud_config(&cfg)
        .map_err(|e| TeamCloudStorageError::Validation(e.to_string()))?;
    Ok(cfg)
}
