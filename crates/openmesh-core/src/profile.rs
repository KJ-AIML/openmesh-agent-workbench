// ============================================================================
// Dev Track 0.1.4 Checkpoint C — local Work Proxy Profile storage
// ============================================================================
// Project-scoped profile at `.openmesh/profile/work-proxy-profile.json`.
// Local only: validated before write, atomic replace, no continuity/signal/event I/O.

use crate::domain::{
    is_supported_work_proxy_profile_version, validate_work_proxy_profile, WorkProxyProfile,
};
use crate::profile_validation::validate_profile_policy;
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Canonical on-disk filename for the Work Proxy Profile.
pub const WORK_PROXY_PROFILE_FILENAME: &str = "work-proxy-profile.json";

#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    #[error("work proxy profile is missing")]
    ProfileMissing,
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("malformed profile JSON: {0}")]
    MalformedJson(String),
    #[error("unsupported profile_version {found}; accepted version is 1.0")]
    UnsupportedVersion { found: String },
    #[error("profile validation failed: {0}")]
    ValidationFailure(#[from] crate::domain::ProfileValidationError),
    #[error(
        "profile workspace_id does not match the project's id (expected {expected}, found {found})"
    )]
    WorkspaceMismatch { expected: String, found: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("atomic profile replacement failed: {0}")]
    AtomicReplaceFailed(String),
}

/// Returns `<project>/.openmesh/profile/`.
pub fn profile_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("profile")
}

/// Canonical profile path: `<project>/.openmesh/profile/work-proxy-profile.json`.
pub fn work_proxy_profile_path(project_path: &str) -> PathBuf {
    profile_dir(project_path).join(WORK_PROXY_PROFILE_FILENAME)
}

/// Returns whether the canonical profile file exists (does not create directories).
pub fn profile_exists(project_path: &str) -> Result<bool, ProfileError> {
    Ok(work_proxy_profile_path(project_path).exists())
}

/// Reads and validates the project's Work Proxy Profile without mutating disk.
pub fn read_work_proxy_profile(project_path: &str) -> Result<WorkProxyProfile, ProfileError> {
    let project = load_project(project_path)?;
    let path = work_proxy_profile_path(project_path);
    if !path.exists() {
        return Err(ProfileError::ProfileMissing);
    }

    let raw = fs::read_to_string(&path)?;
    let profile: WorkProxyProfile =
        serde_json::from_str(&raw).map_err(|err| ProfileError::MalformedJson(err.to_string()))?;

    if !is_supported_work_proxy_profile_version(&profile.profile_version) {
        return Err(ProfileError::UnsupportedVersion {
            found: profile.profile_version.clone(),
        });
    }

    validate_profile_for_storage(&profile)?;
    validate_profile_workspace(&profile, &project)?;
    Ok(profile)
}

/// Validates and atomically writes the canonical Work Proxy Profile.
pub fn write_work_proxy_profile(
    project_path: &str,
    profile: &WorkProxyProfile,
) -> Result<(), ProfileError> {
    let project = load_project(project_path)?;

    if !is_supported_work_proxy_profile_version(&profile.profile_version) {
        return Err(ProfileError::UnsupportedVersion {
            found: profile.profile_version.clone(),
        });
    }

    validate_profile_for_storage(profile)?;
    validate_profile_workspace(profile, &project)?;

    let path = work_proxy_profile_path(project_path);
    fs::create_dir_all(profile_dir(project_path))?;
    let payload = serialize_profile(profile)?;
    atomic_write_profile(&path, &payload)?;
    Ok(())
}

fn load_project(project_path: &str) -> Result<Project, ProfileError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or_else(|| ProfileError::ProjectNotInitialized(project_path.to_string()))
}

fn validate_profile_for_storage(profile: &WorkProxyProfile) -> Result<(), ProfileError> {
    validate_work_proxy_profile(profile)?;
    validate_profile_policy(profile)?;
    Ok(())
}

fn validate_profile_workspace(
    profile: &WorkProxyProfile,
    project: &Project,
) -> Result<(), ProfileError> {
    if profile.workspace_id != project.id {
        return Err(ProfileError::WorkspaceMismatch {
            expected: project.id.clone(),
            found: profile.workspace_id.clone(),
        });
    }
    Ok(())
}

fn serialize_profile(profile: &WorkProxyProfile) -> Result<String, ProfileError> {
    let mut payload = serde_json::to_string_pretty(profile)?;
    payload.push('\n');
    Ok(payload)
}

fn atomic_write_profile(path: &Path, content: &str) -> Result<(), ProfileError> {
    let temp_path = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(err) => {
            let _ = fs::remove_file(&temp_path);
            Err(ProfileError::AtomicReplaceFailed(err.to_string()))
        }
    }
}
