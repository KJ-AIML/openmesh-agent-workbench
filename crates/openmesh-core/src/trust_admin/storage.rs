//! Trust policy persistence under `.openmesh/trust-admin/`.

use crate::storage::{get_project_dir, read_project, Project};
use crate::team::read_team_workspace;
use crate::trust_admin::contract::{
    validate_team_trust_policy, QueryAllowlistMode, TeamTrustPolicy, TRUST_ADMIN_DIR,
    TRUST_ADMIN_PROTOCOL_VERSION,
};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const POLICY_FILE: &str = "policy.json";
const TEMP: &str = "trust-admin-tmp";

#[derive(Debug, thiserror::Error)]
pub enum TrustAdminStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("trust policy missing")]
    Missing,
    #[error("trust policy already initialized")]
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

pub fn trust_admin_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(TRUST_ADMIN_DIR)
}

pub fn policy_path(project_path: &str) -> PathBuf {
    trust_admin_dir(project_path).join(POLICY_FILE)
}

fn load_project(project_path: &str) -> Result<Project, TrustAdminStorageError> {
    read_project(project_path, "project.json").ok_or(TrustAdminStorageError::ProjectNotInitialized)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), TrustAdminStorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| TrustAdminStorageError::Io)?;
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
            .map_err(|_| TrustAdminStorageError::Io)?;
        f.write_all(bytes).map_err(|_| TrustAdminStorageError::Io)?;
        f.sync_all().map_err(|_| TrustAdminStorageError::Io)?;
    }
    fs::rename(&tmp, path).map_err(|_| TrustAdminStorageError::Io)
}

pub fn init_trust_policy(project_path: &str) -> Result<TeamTrustPolicy, TrustAdminStorageError> {
    let project = load_project(project_path)?;
    if policy_path(project_path).exists() {
        return Err(TrustAdminStorageError::AlreadyExists);
    }
    let team = read_team_workspace(project_path).map_err(|_| TrustAdminStorageError::TeamRequired)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let admins: Vec<String> = team
        .members
        .iter()
        .filter(|m| matches!(m.role, crate::team::TeamMemberRole::Owner))
        .map(|m| m.member_id.clone())
        .collect();
    let policy = TeamTrustPolicy {
        protocol_version: TRUST_ADMIN_PROTOCOL_VERSION.into(),
        team_id: team.team_id,
        host_workspace_id: project.id,
        remote_query_enabled: true,
        query_allowlist_mode: QueryAllowlistMode::AllowAll,
        query_allowlist: vec![],
        secret_topics_fail_closed: true,
        allow_secret_export: false,
        sync_require_selective: true,
        admin_member_ids: if admins.is_empty() {
            vec!["owner-local".into()]
        } else {
            admins
        },
        limitations: vec![
            "trust-admin beta — no IdP/SSO".into(),
            "secret topics fail closed; secret export forbidden".into(),
            "selective sync required".into(),
        ],
        created_at: now.clone(),
        updated_at: now,
    };
    write_trust_policy(project_path, &policy)?;
    Ok(policy)
}

pub fn write_trust_policy(
    project_path: &str,
    policy: &TeamTrustPolicy,
) -> Result<(), TrustAdminStorageError> {
    let _ = load_project(project_path)?;
    validate_team_trust_policy(policy)
        .map_err(|e| TrustAdminStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(trust_admin_dir(project_path)).map_err(|_| TrustAdminStorageError::Io)?;
    let bytes =
        serde_json::to_vec_pretty(policy).map_err(|_| TrustAdminStorageError::MalformedJson)?;
    atomic_write(&policy_path(project_path), &bytes)
}

pub fn read_trust_policy(project_path: &str) -> Result<TeamTrustPolicy, TrustAdminStorageError> {
    let _ = load_project(project_path)?;
    let path = policy_path(project_path);
    if !path.exists() {
        return Err(TrustAdminStorageError::Missing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| TrustAdminStorageError::Io)?;
    let policy: TeamTrustPolicy =
        serde_json::from_str(&raw).map_err(|_| TrustAdminStorageError::MalformedJson)?;
    validate_team_trust_policy(&policy)
        .map_err(|e| TrustAdminStorageError::Validation(e.to_string()))?;
    Ok(policy)
}

/// Replace policy (caller must re-validate invariants). Touches updated_at.
pub fn update_trust_policy(
    project_path: &str,
    mut policy: TeamTrustPolicy,
) -> Result<TeamTrustPolicy, TrustAdminStorageError> {
    policy.updated_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    // Re-enforce hard invariants even if caller flipped them.
    policy.secret_topics_fail_closed = true;
    policy.allow_secret_export = false;
    policy.sync_require_selective = true;
    write_trust_policy(project_path, &policy)?;
    Ok(policy)
}
