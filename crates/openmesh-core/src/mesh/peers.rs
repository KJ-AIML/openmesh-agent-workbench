//! Dev Track 0.1.10 Checkpoint B — local peer registry under `.openmesh/mesh/peers/`.

use crate::domain::validate_utc_timestamp;
use crate::mesh::contract::{
    validate_mesh_peer_ref, MeshPeerRef, MeshValidationError, MESH_PEERS_DIR, MAX_PEER_LABEL_BYTES,
    MAX_PEER_PROFILE_ID_BYTES, MAX_WORKSPACE_ID_BYTES,
};
use crate::storage::{get_project_dir, read_project, Project};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Wire protocol for persisted peer records.
pub const MESH_PEER_RECORD_PROTOCOL_VERSION: &str = "1.0";

pub const MAX_PEER_ID_BYTES: usize = 128;
pub const MAX_PEER_NOTES_BYTES: usize = 512;

const PEER_TEMP_EXTENSION: &str = "peer-tmp";

/// Local registry entry for a mesh peer (not network identity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshPeerRecord {
    pub protocol_version: String,
    pub peer_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_profile_id: Option<String>,
    /// Foreign workspace id when known (importer may set later).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl MeshPeerRecord {
    /// View as a mesh peer ref for envelope wiring.
    pub fn as_peer_ref(&self) -> MeshPeerRef {
        MeshPeerRef {
            label: self.label.clone(),
            proxy_profile_id: self.proxy_profile_id.clone(),
            workspace_id: self.remote_workspace_id.clone(),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MeshPeerError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("mesh peer not found")]
    NotFound,
    #[error("mesh peer already exists: {0}")]
    AlreadyExists(String),
    #[error("malformed peer JSON")]
    MalformedJson,
    #[error("peer validation failed: {0}")]
    ValidationFailed(String),
    #[error("failed to read peer registry")]
    ReadFailed,
    #[error("failed to write peer record")]
    WriteFailed,
    #[error("atomic peer replacement failed")]
    AtomicReplaceFailed,
}

impl From<MeshValidationError> for MeshPeerError {
    fn from(value: MeshValidationError) -> Self {
        MeshPeerError::ValidationFailed(value.to_string())
    }
}

/// Returns `<project>/.openmesh/mesh/peers/`.
pub fn peers_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(MESH_PEERS_DIR)
}

/// Returns `<project>/.openmesh/mesh/peers/{peer_id}.json`.
pub fn peer_path(project_path: &str, peer_id: &str) -> PathBuf {
    peers_dir(project_path).join(format!("{peer_id}.json"))
}

/// Path-safe peer id for filenames.
pub fn validate_peer_id_for_storage(peer_id: &str) -> Result<(), MeshPeerError> {
    let trimmed = peer_id.trim();
    if trimmed.is_empty() {
        return Err(MeshPeerError::ValidationFailed(
            "peer_id is empty after trim".into(),
        ));
    }
    if trimmed.len() > MAX_PEER_ID_BYTES {
        return Err(MeshPeerError::ValidationFailed(format!(
            "peer_id exceeds the {MAX_PEER_ID_BYTES}-byte bound"
        )));
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(MeshPeerError::ValidationFailed(
            "peer_id must not contain path separators or '..'".into(),
        ));
    }
    Ok(())
}

/// Deterministic peer id from a label (lowercase alnum + hyphen).
pub fn peer_id_from_label(label: &str) -> String {
    let mut out = String::new();
    let mut last_hyphen = false;
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            last_hyphen = false;
        } else if !last_hyphen && !out.is_empty() {
            out.push('-');
            last_hyphen = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "peer".into()
    } else {
        out.chars().take(MAX_PEER_ID_BYTES).collect()
    }
}

pub fn validate_mesh_peer_record(record: &MeshPeerRecord) -> Result<(), MeshPeerError> {
    if record.protocol_version != MESH_PEER_RECORD_PROTOCOL_VERSION {
        return Err(MeshPeerError::ValidationFailed(format!(
            "unsupported peer protocol_version {}",
            record.protocol_version
        )));
    }
    validate_peer_id_for_storage(&record.peer_id)?;
    // Registry labels do not require remote workspace on the peer record.
    validate_mesh_peer_ref(
        &MeshPeerRef {
            label: record.label.clone(),
            proxy_profile_id: record.proxy_profile_id.clone(),
            workspace_id: record.remote_workspace_id.clone(),
        },
        false,
    )?;
    if record.label.len() > MAX_PEER_LABEL_BYTES {
        return Err(MeshPeerError::ValidationFailed(format!(
            "label exceeds {MAX_PEER_LABEL_BYTES} bytes"
        )));
    }
    if let Some(profile_id) = &record.proxy_profile_id {
        if profile_id.len() > MAX_PEER_PROFILE_ID_BYTES {
            return Err(MeshPeerError::ValidationFailed(format!(
                "proxy_profile_id exceeds {MAX_PEER_PROFILE_ID_BYTES} bytes"
            )));
        }
    }
    if let Some(ws) = &record.remote_workspace_id {
        if ws.len() > MAX_WORKSPACE_ID_BYTES {
            return Err(MeshPeerError::ValidationFailed(format!(
                "remote_workspace_id exceeds {MAX_WORKSPACE_ID_BYTES} bytes"
            )));
        }
    }
    if let Some(notes) = &record.notes {
        if notes.trim().is_empty() {
            return Err(MeshPeerError::ValidationFailed(
                "notes is empty after trim".into(),
            ));
        }
        if notes.len() > MAX_PEER_NOTES_BYTES {
            return Err(MeshPeerError::ValidationFailed(format!(
                "notes exceeds {MAX_PEER_NOTES_BYTES} bytes"
            )));
        }
    }
    validate_utc_timestamp(&record.created_at)
        .map_err(|e| MeshPeerError::ValidationFailed(format!("created_at: {e}")))?;
    validate_utc_timestamp(&record.updated_at)
        .map_err(|e| MeshPeerError::ValidationFailed(format!("updated_at: {e}")))?;
    Ok(())
}

/// Lists peer ids in deterministic lexicographic order.
pub fn list_peer_ids(project_path: &str) -> Result<Vec<String>, MeshPeerError> {
    let _ = load_project(project_path)?;
    let dir = peers_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| MeshPeerError::ReadFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| MeshPeerError::ReadFailed)?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        ids.push(stem.to_string());
    }
    ids.sort();
    Ok(ids)
}

/// Reads and validates a peer record.
pub fn read_peer(project_path: &str, peer_id: &str) -> Result<MeshPeerRecord, MeshPeerError> {
    let _ = load_project(project_path)?;
    validate_peer_id_for_storage(peer_id)?;
    let path = peer_path(project_path, peer_id);
    if !path.exists() {
        return Err(MeshPeerError::NotFound);
    }
    let raw = fs::read_to_string(&path).map_err(|_| MeshPeerError::ReadFailed)?;
    let record: MeshPeerRecord =
        serde_json::from_str(&raw).map_err(|_| MeshPeerError::MalformedJson)?;
    validate_mesh_peer_record(&record)?;
    if record.peer_id != peer_id {
        return Err(MeshPeerError::ValidationFailed(
            "peer_id inside file does not match filename".into(),
        ));
    }
    Ok(record)
}

/// Lists full peer records (skips malformed files with error aggregation via fail-closed read).
pub fn list_peers(project_path: &str) -> Result<Vec<MeshPeerRecord>, MeshPeerError> {
    let ids = list_peer_ids(project_path)?;
    let mut out = Vec::new();
    for id in ids {
        out.push(read_peer(project_path, &id)?);
    }
    Ok(out)
}

/// Creates a new peer record. Fails if peer_id already exists.
pub fn add_peer(
    project_path: &str,
    record: &MeshPeerRecord,
) -> Result<MeshPeerRecord, MeshPeerError> {
    let _ = load_project(project_path)?;
    validate_mesh_peer_record(record)?;
    let path = peer_path(project_path, &record.peer_id);
    if path.exists() {
        return Err(MeshPeerError::AlreadyExists(record.peer_id.clone()));
    }
    fs::create_dir_all(peers_dir(project_path)).map_err(|_| MeshPeerError::WriteFailed)?;
    write_json_atomic(&path, record)?;
    Ok(record.clone())
}

/// Overwrites an existing peer record (or creates if missing when allow_create).
pub fn write_peer(project_path: &str, record: &MeshPeerRecord) -> Result<(), MeshPeerError> {
    let _ = load_project(project_path)?;
    validate_mesh_peer_record(record)?;
    fs::create_dir_all(peers_dir(project_path)).map_err(|_| MeshPeerError::WriteFailed)?;
    let path = peer_path(project_path, &record.peer_id);
    write_json_atomic(&path, record)
}

fn load_project(project_path: &str) -> Result<Project, MeshPeerError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(MeshPeerError::ProjectNotInitialized)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), MeshPeerError> {
    let parent = path.parent().ok_or(MeshPeerError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| MeshPeerError::WriteFailed)?;
    let temp = path.with_extension(PEER_TEMP_EXTENSION);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| MeshPeerError::WriteFailed)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| MeshPeerError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| MeshPeerError::WriteFailed)?;
        file.sync_all().map_err(|_| MeshPeerError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| MeshPeerError::AtomicReplaceFailed)
}
