//! RC pack persistence under `.openmesh/rc/`.

use crate::rc::contract::{validate_rc_pack, RcPack, RC_DIR};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const PACK_FILE: &str = "pack.json";
const TEMP: &str = "rc-tmp";

#[derive(Debug, thiserror::Error)]
pub enum RcStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("rc pack missing")]
    Missing,
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

pub fn rc_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RC_DIR)
}

fn pack_path(project_path: &str) -> PathBuf {
    rc_dir(project_path).join(PACK_FILE)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), RcStorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| RcStorageError::Io)?;
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
            .map_err(|_| RcStorageError::Io)?;
        f.write_all(bytes).map_err(|_| RcStorageError::Io)?;
        f.sync_all().map_err(|_| RcStorageError::Io)?;
    }
    fs::rename(&tmp, path).map_err(|_| RcStorageError::Io)
}

pub fn write_rc_pack(project_path: &str, pack: &RcPack) -> Result<(), RcStorageError> {
    let _: Project = read_project(project_path, "project.json")
        .ok_or(RcStorageError::ProjectNotInitialized)?;
    validate_rc_pack(pack).map_err(|e| RcStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(rc_dir(project_path)).map_err(|_| RcStorageError::Io)?;
    let bytes = serde_json::to_vec_pretty(pack).map_err(|_| RcStorageError::MalformedJson)?;
    atomic_write(&pack_path(project_path), &bytes)
}

pub fn read_rc_pack(project_path: &str) -> Result<RcPack, RcStorageError> {
    let _: Project = read_project(project_path, "project.json")
        .ok_or(RcStorageError::ProjectNotInitialized)?;
    let path = pack_path(project_path);
    if !path.exists() {
        return Err(RcStorageError::Missing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| RcStorageError::Io)?;
    let pack: RcPack = serde_json::from_str(&raw).map_err(|_| RcStorageError::MalformedJson)?;
    validate_rc_pack(&pack).map_err(|e| RcStorageError::Validation(e.to_string()))?;
    Ok(pack)
}
