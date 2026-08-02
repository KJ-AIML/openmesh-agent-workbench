//! Dev Track 0.1.11 Checkpoint D — approve gate for egress.

use crate::relay::audit::{append_audit_event, make_audit_event, RelayAuditError};
use crate::relay::contract::{
    is_package_approved, validate_package_id_for_storage, validate_relay_package, RelayAuditKind,
    RelayPackage, RELAY_APPROVED_DIR,
};
use crate::relay::package::{
    read_staging_package, staging_package_path, RelayPackageError,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const APPROVED_TEMP: &str = "relay-approved-tmp";

#[derive(Debug, thiserror::Error)]
pub enum RelayApproveError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("package error: {0}")]
    Package(#[from] RelayPackageError),
    #[error("audit error: {0}")]
    Audit(#[from] RelayAuditError),
    #[error("package already approved")]
    AlreadyApproved,
    #[error("io failed")]
    Io,
}

pub fn approved_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RELAY_APPROVED_DIR)
}

pub fn approved_package_path(project_path: &str, package_id: &str) -> PathBuf {
    approved_dir(project_path).join(format!("{package_id}.json"))
}

pub fn read_approved_package(
    project_path: &str,
    package_id: &str,
) -> Result<RelayPackage, RelayApproveError> {
    let _ = load_project(project_path)?;
    validate_package_id_for_storage(package_id).map_err(|e| {
        RelayApproveError::Package(RelayPackageError::Validation(e))
    })?;
    let path = approved_package_path(project_path, package_id);
    if !path.exists() {
        return Err(RelayApproveError::Package(RelayPackageError::NotFound(
            package_id.into(),
        )));
    }
    let raw = fs::read_to_string(&path).map_err(|_| RelayApproveError::Io)?;
    let pkg: RelayPackage = serde_json::from_str(&raw).map_err(|_| RelayApproveError::Io)?;
    validate_relay_package(&pkg)
        .map_err(|e| RelayApproveError::Package(RelayPackageError::Validation(e)))?;
    Ok(pkg)
}

/// Approve a staged package for egress: set approval fields, write approved/, audit.
pub fn approve_relay_package(
    project_path: &str,
    package_id: &str,
    approved_at: &str,
    approved_by: &str,
) -> Result<RelayPackage, RelayApproveError> {
    let _ = load_project(project_path)?;
    let mut pkg = read_staging_package(project_path, package_id)?;
    if is_package_approved(&pkg) {
        return Err(RelayApproveError::AlreadyApproved);
    }
    pkg.approved_at = Some(approved_at.to_string());
    pkg.approved_by = Some(approved_by.to_string());
    validate_relay_package(&pkg)
        .map_err(|e| RelayApproveError::Package(RelayPackageError::Validation(e)))?;

    fs::create_dir_all(approved_dir(project_path)).map_err(|_| RelayApproveError::Io)?;
    let path = approved_package_path(project_path, package_id);
    if path.exists() {
        return Err(RelayApproveError::AlreadyApproved);
    }
    write_json_atomic(&path, &pkg)?;

    // Keep staging copy updated too (approved snapshot).
    let staging = staging_package_path(project_path, package_id);
    if staging.exists() {
        let _ = write_json_atomic(&staging, &pkg);
    }

    let audit = make_audit_event(
        format!("audit-approved-{}", package_id),
        package_id,
        RelayAuditKind::Approved,
        approved_at,
        format!("approved by {approved_by}"),
        Some(approved_by.to_string()),
        Some(pkg.sensitivity_max),
    );
    append_audit_event(project_path, &audit)?;
    Ok(pkg)
}

fn load_project(project_path: &str) -> Result<Project, RelayApproveError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(RelayApproveError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), RelayApproveError> {
    let parent = path.parent().ok_or(RelayApproveError::Io)?;
    fs::create_dir_all(parent).map_err(|_| RelayApproveError::Io)?;
    let temp = path.with_extension(APPROVED_TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| RelayApproveError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| RelayApproveError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| RelayApproveError::Io)?;
        file.sync_all().map_err(|_| RelayApproveError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| RelayApproveError::Io)
}
