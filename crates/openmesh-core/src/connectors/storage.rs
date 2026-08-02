//! Connector registry persistence under `.openmesh/connectors/`.

use crate::connectors::contract::{
    validate_connector_descriptor, validate_connector_run, ConnectorDescriptor, ConnectorKind,
    ConnectorRole, ConnectorRun, CONNECTORS_DIR, CONNECTOR_PROTOCOL_VERSION,
};
use crate::storage::{get_project_dir, read_project, Project};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const REGISTRY_FILE: &str = "registry.json";
const RUNS_DIR: &str = "runs";
const TEMP: &str = "connectors-tmp";

#[derive(Debug, thiserror::Error)]
pub enum ConnectorStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("connector not found: {0}")]
    NotFound(String),
    #[error("connector already registered: {0}")]
    AlreadyExists(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ConnectorRegistry {
    #[serde(default)]
    connectors: Vec<ConnectorDescriptor>,
}

fn load_project(project_path: &str) -> Result<Project, ConnectorStorageError> {
    read_project(project_path, "project.json").ok_or(ConnectorStorageError::ProjectNotInitialized)
}

pub fn connectors_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(CONNECTORS_DIR)
}

fn registry_path(project_path: &str) -> PathBuf {
    connectors_dir(project_path).join(REGISTRY_FILE)
}

fn runs_dir(project_path: &str) -> PathBuf {
    connectors_dir(project_path).join(RUNS_DIR)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ConnectorStorageError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| ConnectorStorageError::Io)?;
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
            .map_err(|_| ConnectorStorageError::Io)?;
        f.write_all(bytes).map_err(|_| ConnectorStorageError::Io)?;
        f.sync_all().map_err(|_| ConnectorStorageError::Io)?;
    }
    fs::rename(&tmp, path).map_err(|_| ConnectorStorageError::Io)
}

fn read_registry(project_path: &str) -> Result<ConnectorRegistry, ConnectorStorageError> {
    let path = registry_path(project_path);
    if !path.exists() {
        return Ok(ConnectorRegistry::default());
    }
    let raw = fs::read_to_string(&path).map_err(|_| ConnectorStorageError::Io)?;
    serde_json::from_str(&raw).map_err(|_| ConnectorStorageError::MalformedJson)
}

fn write_registry(
    project_path: &str,
    reg: &ConnectorRegistry,
) -> Result<(), ConnectorStorageError> {
    for c in &reg.connectors {
        validate_connector_descriptor(c)
            .map_err(|e| ConnectorStorageError::Validation(e.to_string()))?;
    }
    fs::create_dir_all(connectors_dir(project_path)).map_err(|_| ConnectorStorageError::Io)?;
    let bytes = serde_json::to_vec_pretty(reg).map_err(|_| ConnectorStorageError::MalformedJson)?;
    atomic_write(&registry_path(project_path), &bytes)
}

pub fn list_connectors(project_path: &str) -> Result<Vec<ConnectorDescriptor>, ConnectorStorageError> {
    let _ = load_project(project_path)?;
    Ok(read_registry(project_path)?.connectors)
}

pub fn read_connector(
    project_path: &str,
    connector_id: &str,
) -> Result<ConnectorDescriptor, ConnectorStorageError> {
    let _ = load_project(project_path)?;
    read_registry(project_path)?
        .connectors
        .into_iter()
        .find(|c| c.connector_id == connector_id)
        .ok_or_else(|| ConnectorStorageError::NotFound(connector_id.into()))
}

/// Register a connector (defaults to GitHub stub when kind omitted by caller).
pub fn init_or_register_connector(
    project_path: &str,
    connector_id: &str,
    display_name: &str,
    kind: ConnectorKind,
    external_ref: Option<String>,
) -> Result<ConnectorDescriptor, ConnectorStorageError> {
    let _ = load_project(project_path)?;
    let mut reg = read_registry(project_path)?;
    if reg.connectors.iter().any(|c| c.connector_id == connector_id) {
        return Err(ConnectorStorageError::AlreadyExists(connector_id.into()));
    }
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let d = ConnectorDescriptor {
        protocol_version: CONNECTOR_PROTOCOL_VERSION.into(),
        connector_id: connector_id.trim().to_string(),
        kind,
        display_name: display_name.trim().to_string(),
        role: ConnectorRole::EvidenceProducerOnly,
        enabled: true,
        external_ref,
        limitations: vec![
            "evidence producer only — does not replace external SoR".into(),
            "github-stub is offline; no live API calls".into(),
        ],
        created_at: now.clone(),
        updated_at: now,
    };
    validate_connector_descriptor(&d)
        .map_err(|e| ConnectorStorageError::Validation(e.to_string()))?;
    reg.connectors.push(d.clone());
    write_registry(project_path, &reg)?;
    Ok(d)
}

pub fn write_connector_run(
    project_path: &str,
    run: &ConnectorRun,
) -> Result<PathBuf, ConnectorStorageError> {
    let _ = load_project(project_path)?;
    validate_connector_run(run).map_err(|e| ConnectorStorageError::Validation(e.to_string()))?;
    let dir = runs_dir(project_path);
    fs::create_dir_all(&dir).map_err(|_| ConnectorStorageError::Io)?;
    let path = dir.join(format!("{}.json", run.run_id.replace(':', "-")));
    let bytes = serde_json::to_vec_pretty(run).map_err(|_| ConnectorStorageError::MalformedJson)?;
    atomic_write(&path, &bytes)?;
    Ok(path)
}
