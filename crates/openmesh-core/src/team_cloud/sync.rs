//! Selective sync scaffold for Team Cloud Beta (dry-run only).

use crate::team_cloud::contract::{
    validate_team_cloud_sync_plan, TeamCloudConfig, TeamCloudSyncPlan, TEAM_CLOUD_PROTOCOL_VERSION,
};
use crate::team_cloud::storage::{read_team_cloud, write_team_cloud, TeamCloudStorageError};
use chrono::Utc;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum TeamCloudSyncError {
    #[error(transparent)]
    Storage(#[from] TeamCloudStorageError),
    #[error("validation: {0}")]
    Validation(String),
}

/// Build a scaffold-only selective sync plan. Never uploads.
pub fn build_sync_scaffold(project_path: &str) -> Result<TeamCloudSyncPlan, TeamCloudSyncError> {
    let mut cfg = read_team_cloud(project_path)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // Filter planned paths to those that currently exist under the project root.
    let root = Path::new(project_path);
    let planned: Vec<String> = cfg
        .sync_paths
        .iter()
        .filter(|p| root.join(p).exists())
        .cloned()
        .collect();

    let plan = TeamCloudSyncPlan {
        protocol_version: TEAM_CLOUD_PROTOCOL_VERSION.into(),
        team_id: cfg.team_id.clone(),
        mode: cfg.mode,
        generated_at: now.clone(),
        planned_paths: planned,
        scaffold_only: true,
        note: "Team Cloud Beta scaffold — dry-run only; no remote upload performed".into(),
    };
    validate_team_cloud_sync_plan(&plan)
        .map_err(|e| TeamCloudSyncError::Validation(e.to_string()))?;

    // Record last scaffold timestamp (not a real sync).
    cfg.last_sync_at = Some(now);
    cfg.updated_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_team_cloud(project_path, &cfg)?;

    let _ = cfg; // silence if optimized
    Ok(plan)
}

/// Helper for tests / desktop: expose config snapshot after scaffold.
pub fn team_cloud_status(project_path: &str) -> Result<TeamCloudConfig, TeamCloudSyncError> {
    Ok(read_team_cloud(project_path)?)
}
