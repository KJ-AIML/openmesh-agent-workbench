//! Dev Track 0.1.11 Checkpoints B–C — pack selection + staging storage.

use crate::mesh::{
    read_outbox_envelope, MeshEnvelope, MeshSensitivityMax, MESH_OUTBOX_DIR,
};
use crate::relay::contract::{
    validate_package_id_for_storage, validate_relay_package, RelayPackage, RelayPolicySnapshot,
    RelayValidationError, RELAY_PACKAGE_PROTOCOL_VERSION, RELAY_STAGING_DIR, MAX_ENVELOPES_PER_PACKAGE,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const STAGING_TEMP: &str = "relay-staging-tmp";

#[derive(Debug, thiserror::Error)]
pub enum RelayPackageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("validation: {0}")]
    Validation(#[from] RelayValidationError),
    #[error("envelope not found in outbox: {0}")]
    EnvelopeNotFound(String),
    #[error("package already exists: {0}")]
    AlreadyExists(String),
    #[error("package not found: {0}")]
    NotFound(String),
    #[error("io failed")]
    Io,
    #[error("malformed package JSON")]
    MalformedJson,
}

/// Inputs for building a staging relay package.
#[derive(Debug, Clone)]
pub struct BuildRelayPackRequest {
    pub package_id: String,
    pub workspace_id: String,
    pub generated_at: String,
    pub sensitivity_max: MeshSensitivityMax,
    /// Envelope ids from local mesh outbox to include.
    pub envelope_ids: Vec<String>,
    pub handoff_ids: Vec<String>,
    pub selection_notes: Vec<String>,
}

pub fn staging_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RELAY_STAGING_DIR)
}

pub fn staging_package_path(project_path: &str, package_id: &str) -> PathBuf {
    staging_dir(project_path).join(format!("{package_id}.json"))
}

/// Build a validated staging package from selected mesh outbox envelopes.
pub fn build_relay_package(
    project_path: &str,
    request: &BuildRelayPackRequest,
) -> Result<RelayPackage, RelayPackageError> {
    let _ = load_project(project_path)?;
    validate_package_id_for_storage(&request.package_id)?;

    let mut envelopes: Vec<MeshEnvelope> = Vec::new();
    for id in &request.envelope_ids {
        if envelopes.len() >= MAX_ENVELOPES_PER_PACKAGE {
            break;
        }
        let env = read_outbox_envelope(project_path, id).map_err(|_| {
            RelayPackageError::EnvelopeNotFound(id.clone())
        })?;
        envelopes.push(env);
    }

    let mut denied = vec!["secret".to_string()];
    denied.sort();
    denied.dedup();

    let mut limitations = Vec::new();
    if envelopes.is_empty() && request.handoff_ids.is_empty() {
        limitations.push("relay pack selected no envelopes or handoff ids".into());
    }

    let mut notes = request.selection_notes.clone();
    notes.push(format!(
        "packed {} envelope(s) under sensitivity_max={}",
        envelopes.len(),
        request.sensitivity_max.as_str()
    ));

    let mut pkg = RelayPackage {
        protocol_version: RELAY_PACKAGE_PROTOCOL_VERSION.into(),
        package_id: request.package_id.clone(),
        workspace_id: request.workspace_id.clone(),
        generated_at: request.generated_at.clone(),
        sensitivity_max: request.sensitivity_max,
        envelopes,
        handoff_ids: request.handoff_ids.clone(),
        policy: RelayPolicySnapshot {
            approved_paths: vec![format!(".openmesh/{MESH_OUTBOX_DIR}")],
            denied_classes: denied,
            selection_notes: notes,
        },
        limitations,
        content_hash: None,
        approved_at: None,
        approved_by: None,
    };

    // Simple stable content fingerprint (not crypto-secure; alpha audit helper).
    let body = serde_json::to_string(&pkg).map_err(|_| RelayPackageError::Io)?;
    pkg.content_hash = Some(simple_hash(&body));

    validate_relay_package(&pkg)?;
    Ok(pkg)
}

/// Write package to staging (fail if exists).
pub fn write_staging_package(
    project_path: &str,
    pkg: &RelayPackage,
) -> Result<(), RelayPackageError> {
    let _ = load_project(project_path)?;
    validate_relay_package(pkg)?;
    let path = staging_package_path(project_path, &pkg.package_id);
    if path.exists() {
        return Err(RelayPackageError::AlreadyExists(pkg.package_id.clone()));
    }
    fs::create_dir_all(staging_dir(project_path)).map_err(|_| RelayPackageError::Io)?;
    write_json_atomic(&path, pkg)
}

pub fn read_staging_package(
    project_path: &str,
    package_id: &str,
) -> Result<RelayPackage, RelayPackageError> {
    let _ = load_project(project_path)?;
    validate_package_id_for_storage(package_id)?;
    let path = staging_package_path(project_path, package_id);
    if !path.exists() {
        return Err(RelayPackageError::NotFound(package_id.into()));
    }
    let raw = fs::read_to_string(&path).map_err(|_| RelayPackageError::Io)?;
    let pkg: RelayPackage =
        serde_json::from_str(&raw).map_err(|_| RelayPackageError::MalformedJson)?;
    validate_relay_package(&pkg)?;
    Ok(pkg)
}

pub fn pack_to_staging(
    project_path: &str,
    request: &BuildRelayPackRequest,
) -> Result<RelayPackage, RelayPackageError> {
    let pkg = build_relay_package(project_path, request)?;
    write_staging_package(project_path, &pkg)?;
    Ok(pkg)
}

fn load_project(project_path: &str) -> Result<Project, RelayPackageError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(RelayPackageError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), RelayPackageError> {
    let parent = path.parent().ok_or(RelayPackageError::Io)?;
    fs::create_dir_all(parent).map_err(|_| RelayPackageError::Io)?;
    let temp = path.with_extension(STAGING_TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| RelayPackageError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| RelayPackageError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| RelayPackageError::Io)?;
        file.sync_all().map_err(|_| RelayPackageError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| RelayPackageError::Io)
}

fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}
