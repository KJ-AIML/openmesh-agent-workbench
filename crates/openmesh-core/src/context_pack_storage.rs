// ============================================================================
// Dev Track 0.1.5 Checkpoint E — Proxy Context Pack local persistence
// ============================================================================
// Canonical pack at `.openmesh/projections/proxy-context-pack.json`.
// Read paths never create directories; write validates before any mutation.

use crate::context_pack_validation::{
    validate_proxy_context_pack_complete, ContextPackValidationError,
};
use crate::domain::{is_supported_proxy_context_pack_protocol, ProxyContextPack};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Canonical on-disk filename for the Proxy Context Pack projection.
pub const PROXY_CONTEXT_PACK_FILENAME: &str = "proxy-context-pack.json";

/// Context-specific temp suffix for atomic replacement (not confused with the canonical pack).
const PROXY_CONTEXT_PACK_TEMP_EXTENSION: &str = "pack-tmp";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ContextPackStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("proxy context pack is missing")]
    PackNotFound,
    #[error("malformed context pack JSON")]
    MalformedJson,
    #[error("unsupported protocol version")]
    UnsupportedProtocolVersion,
    #[error("context pack validation failed")]
    ValidationFailed { category: String },
    #[error("workspace_id does not match project")]
    WorkspaceMismatch,
    #[error("failed to read context pack")]
    ReadFailed,
    #[error("failed to write context pack")]
    WriteFailed,
    #[error("atomic context pack replacement failed")]
    AtomicReplaceFailed,
}

impl ContextPackStorageError {
    /// Stable machine-readable category for CLI mapping.
    pub fn category(&self) -> &'static str {
        match self {
            Self::ProjectNotInitialized => "project-not-initialized",
            Self::PackNotFound => "pack-not-found",
            Self::MalformedJson => "malformed-json",
            Self::UnsupportedProtocolVersion => "unsupported-protocol-version",
            Self::ValidationFailed { .. } => "validation-failed",
            Self::WorkspaceMismatch => "workspace-mismatch",
            Self::ReadFailed => "read-failed",
            Self::WriteFailed => "write-failed",
            Self::AtomicReplaceFailed => "atomic-replace-failed",
        }
    }
}

/// Returns `<project>/.openmesh/projections/proxy-context-pack.json`.
pub fn proxy_context_pack_path(project_path: &str) -> PathBuf {
    context_pack_projections_dir(project_path).join(PROXY_CONTEXT_PACK_FILENAME)
}

/// Returns `<project>/.openmesh/projections/` (does not create it).
pub fn context_pack_projections_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("projections")
}

/// Returns whether the canonical pack file exists (does not create directories).
pub fn context_pack_exists(project_path: &str) -> bool {
    proxy_context_pack_path(project_path).exists()
}

/// Reads, validates, and returns the project's persisted Proxy Context Pack.
pub fn read_proxy_context_pack(
    project_path: &str,
) -> Result<ProxyContextPack, ContextPackStorageError> {
    let project = load_project(project_path)?;
    let path = proxy_context_pack_path(project_path);
    if !path.exists() {
        return Err(ContextPackStorageError::PackNotFound);
    }

    let pack = read_pack_file(&path)?;
    validate_pack_workspace(&pack, &project)?;
    Ok(pack)
}

/// Validates and atomically writes the canonical Proxy Context Pack.
pub fn write_proxy_context_pack(
    project_path: &str,
    pack: &ProxyContextPack,
) -> Result<(), ContextPackStorageError> {
    let project = load_project(project_path)?;
    if pack.workspace_id != project.id {
        return Err(ContextPackStorageError::WorkspaceMismatch);
    }
    validate_pack_for_storage(pack)?;

    let path = proxy_context_pack_path(project_path);
    fs::create_dir_all(context_pack_projections_dir(project_path))
        .map_err(|_| ContextPackStorageError::WriteFailed)?;
    let payload = serialize_pack(pack)?;
    atomic_write_pack(&path, &payload)?;
    Ok(())
}

/// Reads and validates a pack from an explicit file path (no project workspace check).
pub fn read_proxy_context_pack_file(
    path: &Path,
) -> Result<ProxyContextPack, ContextPackStorageError> {
    read_pack_file(path)
}

fn load_project(project_path: &str) -> Result<Project, ContextPackStorageError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(ContextPackStorageError::ProjectNotInitialized)
}

fn read_pack_file(path: &Path) -> Result<ProxyContextPack, ContextPackStorageError> {
    let raw = fs::read_to_string(path).map_err(|_| ContextPackStorageError::ReadFailed)?;
    let pack: ProxyContextPack =
        serde_json::from_str(&raw).map_err(|_| ContextPackStorageError::MalformedJson)?;

    if !is_supported_proxy_context_pack_protocol(&pack.protocol_version) {
        return Err(ContextPackStorageError::UnsupportedProtocolVersion);
    }

    validate_pack_for_storage(&pack)?;
    Ok(pack)
}

fn validate_pack_for_storage(pack: &ProxyContextPack) -> Result<(), ContextPackStorageError> {
    validate_proxy_context_pack_complete(pack).map_err(map_validation_error)
}

fn validate_pack_workspace(
    pack: &ProxyContextPack,
    project: &Project,
) -> Result<(), ContextPackStorageError> {
    if pack.workspace_id != project.id {
        return Err(ContextPackStorageError::WorkspaceMismatch);
    }
    Ok(())
}

fn map_validation_error(err: ContextPackValidationError) -> ContextPackStorageError {
    match err {
        ContextPackValidationError::UnsupportedProtocolVersion => {
            ContextPackStorageError::UnsupportedProtocolVersion
        }
        other => ContextPackStorageError::ValidationFailed {
            category: other.category().to_string(),
        },
    }
}

fn serialize_pack(pack: &ProxyContextPack) -> Result<String, ContextPackStorageError> {
    let mut payload =
        serde_json::to_string_pretty(pack).map_err(|_| ContextPackStorageError::WriteFailed)?;
    payload.push('\n');
    Ok(payload)
}

fn atomic_write_pack(path: &Path, content: &str) -> Result<(), ContextPackStorageError> {
    let temp_path = path.with_extension(PROXY_CONTEXT_PACK_TEMP_EXTENSION);
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temp_path)
        .map_err(|_| ContextPackStorageError::WriteFailed)?;
    file.write_all(content.as_bytes())
        .map_err(|_| ContextPackStorageError::WriteFailed)?;
    file.flush()
        .map_err(|_| ContextPackStorageError::WriteFailed)?;
    match fs::rename(&temp_path, path) {
        Ok(()) => Ok(()),
        Err(_) => {
            let _ = fs::remove_file(&temp_path);
            Err(ContextPackStorageError::AtomicReplaceFailed)
        }
    }
}
