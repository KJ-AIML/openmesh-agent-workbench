//! Dev Track 0.1.10 Checkpoint D — mesh envelope import into inbox.

use crate::mesh::contract::{
    validate_envelope_id_for_storage, validate_mesh_envelope, MeshEnvelope, MeshValidationError,
    MESH_INBOX_DIR,
};
use crate::mesh::peers::{
    add_peer, peer_id_from_label, read_peer, MeshPeerError, MeshPeerRecord,
    MESH_PEER_RECORD_PROTOCOL_VERSION,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const INBOX_TEMP_EXTENSION: &str = "inbox-tmp";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMeshOptions {
    /// When true, register `from_peer` into the local peer registry if missing.
    pub register_from_peer: bool,
    /// When true, allow importing an envelope whose from workspace matches this project (usually false).
    pub allow_self_workspace: bool,
}

impl Default for ImportMeshOptions {
    fn default() -> Self {
        Self {
            register_from_peer: false,
            allow_self_workspace: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MeshImportError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("import file not found")]
    FileNotFound,
    #[error("failed to read import file")]
    ReadFailed,
    #[error("malformed envelope JSON")]
    MalformedJson,
    #[error("envelope validation failed: {0}")]
    Validation(#[from] MeshValidationError),
    #[error("envelope from_peer.workspace_id matches local workspace (refuse self-import)")]
    SelfWorkspaceImport,
    #[error("envelope already exists in inbox: {0}")]
    AlreadyExists(String),
    #[error("failed to write inbox envelope")]
    WriteFailed,
    #[error("atomic inbox write failed")]
    AtomicReplaceFailed,
    #[error("peer registry error: {0}")]
    Peer(#[from] MeshPeerError),
}

/// Returns `<project>/.openmesh/mesh/inbox/`.
pub fn inbox_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(MESH_INBOX_DIR)
}

/// Returns `<project>/.openmesh/mesh/inbox/{envelope_id}.json`.
pub fn inbox_envelope_path(project_path: &str, envelope_id: &str) -> PathBuf {
    inbox_dir(project_path).join(format!("{envelope_id}.json"))
}

/// Load + validate a mesh envelope from an arbitrary file path (no project write).
pub fn load_envelope_from_file(path: &Path) -> Result<MeshEnvelope, MeshImportError> {
    if !path.exists() {
        return Err(MeshImportError::FileNotFound);
    }
    let raw = fs::read_to_string(path).map_err(|_| MeshImportError::ReadFailed)?;
    let envelope: MeshEnvelope =
        serde_json::from_str(&raw).map_err(|_| MeshImportError::MalformedJson)?;
    validate_mesh_envelope(&envelope)?;
    Ok(envelope)
}

/// Import an already-parsed envelope into the project's inbox.
pub fn import_mesh_envelope(
    project_path: &str,
    envelope: &MeshEnvelope,
    options: &ImportMeshOptions,
) -> Result<MeshEnvelope, MeshImportError> {
    let project = load_project(project_path)?;
    validate_mesh_envelope(envelope)?;
    validate_envelope_id_for_storage(&envelope.envelope_id)?;

    if !options.allow_self_workspace {
        if let Some(from_ws) = envelope.from_peer.workspace_id.as_deref() {
            if from_ws == project.id {
                return Err(MeshImportError::SelfWorkspaceImport);
            }
        }
    }

    if options.register_from_peer {
        maybe_register_from_peer(project_path, envelope)?;
    }

    write_inbox_envelope(project_path, envelope)?;
    Ok(envelope.clone())
}

/// Import envelope JSON from a file path into the local inbox.
pub fn import_mesh_envelope_from_file(
    project_path: &str,
    file_path: &Path,
    options: &ImportMeshOptions,
) -> Result<MeshEnvelope, MeshImportError> {
    let envelope = load_envelope_from_file(file_path)?;
    import_mesh_envelope(project_path, &envelope, options)
}

/// Persist envelope under inbox (fail if id already exists).
pub fn write_inbox_envelope(
    project_path: &str,
    envelope: &MeshEnvelope,
) -> Result<(), MeshImportError> {
    let _ = load_project(project_path)?;
    validate_mesh_envelope(envelope)?;
    let path = inbox_envelope_path(project_path, &envelope.envelope_id);
    if path.exists() {
        return Err(MeshImportError::AlreadyExists(envelope.envelope_id.clone()));
    }
    fs::create_dir_all(inbox_dir(project_path)).map_err(|_| MeshImportError::WriteFailed)?;
    write_json_atomic(&path, envelope)
}

/// Read an inbox envelope.
pub fn read_inbox_envelope(
    project_path: &str,
    envelope_id: &str,
) -> Result<MeshEnvelope, MeshImportError> {
    let _ = load_project(project_path)?;
    validate_envelope_id_for_storage(envelope_id)?;
    let path = inbox_envelope_path(project_path, envelope_id);
    if !path.exists() {
        return Err(MeshImportError::FileNotFound);
    }
    let raw = fs::read_to_string(&path).map_err(|_| MeshImportError::ReadFailed)?;
    let envelope: MeshEnvelope =
        serde_json::from_str(&raw).map_err(|_| MeshImportError::MalformedJson)?;
    validate_mesh_envelope(&envelope)?;
    Ok(envelope)
}

/// List inbox envelope ids (lexicographic).
pub fn list_inbox_envelope_ids(project_path: &str) -> Result<Vec<String>, MeshImportError> {
    let _ = load_project(project_path)?;
    let dir = inbox_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| MeshImportError::ReadFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| MeshImportError::ReadFailed)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            ids.push(stem.to_string());
        }
    }
    ids.sort();
    Ok(ids)
}

fn maybe_register_from_peer(
    project_path: &str,
    envelope: &MeshEnvelope,
) -> Result<(), MeshImportError> {
    let peer_id = peer_id_from_label(&envelope.from_peer.label);
    if read_peer(project_path, &peer_id).is_ok() {
        return Ok(());
    }
    let now = envelope.generated_at.clone();
    let record = MeshPeerRecord {
        protocol_version: MESH_PEER_RECORD_PROTOCOL_VERSION.into(),
        peer_id,
        label: envelope.from_peer.label.clone(),
        proxy_profile_id: envelope.from_peer.proxy_profile_id.clone(),
        remote_workspace_id: envelope.from_peer.workspace_id.clone(),
        notes: Some("auto-registered from mesh import".into()),
        created_at: now.clone(),
        updated_at: now,
    };
    match add_peer(project_path, &record) {
        Ok(_) => Ok(()),
        Err(MeshPeerError::AlreadyExists(_)) => Ok(()),
        Err(err) => Err(MeshImportError::Peer(err)),
    }
}

fn load_project(project_path: &str) -> Result<Project, MeshImportError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(MeshImportError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), MeshImportError> {
    let parent = path.parent().ok_or(MeshImportError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| MeshImportError::WriteFailed)?;
    let temp = path.with_extension(INBOX_TEMP_EXTENSION);
    let mut json =
        serde_json::to_string_pretty(value).map_err(|_| MeshImportError::WriteFailed)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| MeshImportError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| MeshImportError::WriteFailed)?;
        file.sync_all().map_err(|_| MeshImportError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| MeshImportError::AtomicReplaceFailed)
}
