//! Pilot pack persistence under `.openmesh/pilot/`.

use crate::pilot::contract::{validate_pilot_pack, PilotPack, PILOT_DIR};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const PACK_FILE: &str = "pack.json";
const TEMP: &str = "pilot-tmp";

#[derive(Debug, thiserror::Error)]
pub enum PilotStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("pilot pack missing")]
    Missing,
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

pub fn pilot_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(PILOT_DIR)
}

fn pack_path(project_path: &str) -> PathBuf {
    pilot_dir(project_path).join(PACK_FILE)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), PilotStorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| PilotStorageError::Io)?;
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
            .map_err(|_| PilotStorageError::Io)?;
        f.write_all(bytes).map_err(|_| PilotStorageError::Io)?;
        f.sync_all().map_err(|_| PilotStorageError::Io)?;
    }
    fs::rename(&tmp, path).map_err(|_| PilotStorageError::Io)
}

pub fn write_pilot_pack(project_path: &str, pack: &PilotPack) -> Result<(), PilotStorageError> {
    let _: Project = read_project(project_path, "project.json")
        .ok_or(PilotStorageError::ProjectNotInitialized)?;
    validate_pilot_pack(pack).map_err(|e| PilotStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(pilot_dir(project_path)).map_err(|_| PilotStorageError::Io)?;
    let bytes = serde_json::to_vec_pretty(pack).map_err(|_| PilotStorageError::MalformedJson)?;
    atomic_write(&pack_path(project_path), &bytes)
}

pub fn read_pilot_pack(project_path: &str) -> Result<PilotPack, PilotStorageError> {
    let _: Project = read_project(project_path, "project.json")
        .ok_or(PilotStorageError::ProjectNotInitialized)?;
    let path = pack_path(project_path);
    if !path.exists() {
        return Err(PilotStorageError::Missing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| PilotStorageError::Io)?;
    let pack: PilotPack =
        serde_json::from_str(&raw).map_err(|_| PilotStorageError::MalformedJson)?;
    validate_pilot_pack(&pack).map_err(|e| PilotStorageError::Validation(e.to_string()))?;
    Ok(pack)
}
