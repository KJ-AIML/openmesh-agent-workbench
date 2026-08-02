//! Dev Track 0.1.11 Checkpoint D — append-only relay audit log.

use crate::mesh::MeshSensitivityMax;
use crate::relay::contract::{
    validate_package_id_for_storage, validate_relay_audit_event, RelayAuditEvent, RelayAuditKind,
    RELAY_AUDIT_DIR, RELAY_AUDIT_PROTOCOL_VERSION,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const AUDIT_TEMP: &str = "relay-audit-tmp";

#[derive(Debug, thiserror::Error)]
pub enum RelayAuditError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed audit JSON")]
    MalformedJson,
}

pub fn audit_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RELAY_AUDIT_DIR)
}

pub fn audit_event_path(project_path: &str, event_id: &str) -> PathBuf {
    audit_dir(project_path).join(format!("{event_id}.json"))
}

pub fn append_audit_event(
    project_path: &str,
    event: &RelayAuditEvent,
) -> Result<(), RelayAuditError> {
    let _ = load_project(project_path)?;
    validate_relay_audit_event(event).map_err(|e| RelayAuditError::Validation(e.to_string()))?;
    validate_package_id_for_storage(&event.package_id)
        .map_err(|e| RelayAuditError::Validation(e.to_string()))?;
    fs::create_dir_all(audit_dir(project_path)).map_err(|_| RelayAuditError::Io)?;
    let path = audit_event_path(project_path, &event.event_id);
    if path.exists() {
        return Err(RelayAuditError::Validation(format!(
            "audit event already exists: {}",
            event.event_id
        )));
    }
    write_json_atomic(&path, event)
}

pub fn list_audit_events(project_path: &str) -> Result<Vec<RelayAuditEvent>, RelayAuditError> {
    let _ = load_project(project_path)?;
    let dir = audit_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut events = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| RelayAuditError::Io)?;
    for entry in entries {
        let entry = entry.map_err(|_| RelayAuditError::Io)?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).map_err(|_| RelayAuditError::Io)?;
        let ev: RelayAuditEvent =
            serde_json::from_str(&raw).map_err(|_| RelayAuditError::MalformedJson)?;
        validate_relay_audit_event(&ev).map_err(|e| RelayAuditError::Validation(e.to_string()))?;
        events.push(ev);
    }
    events.sort_by(|a, b| a.at.cmp(&b.at).then_with(|| a.event_id.cmp(&b.event_id)));
    Ok(events)
}

pub fn make_audit_event(
    event_id: impl Into<String>,
    package_id: impl Into<String>,
    kind: RelayAuditKind,
    at: impl Into<String>,
    detail: impl Into<String>,
    actor_label: Option<String>,
    sensitivity_max: Option<MeshSensitivityMax>,
) -> RelayAuditEvent {
    RelayAuditEvent {
        protocol_version: RELAY_AUDIT_PROTOCOL_VERSION.into(),
        event_id: event_id.into(),
        package_id: package_id.into(),
        kind,
        at: at.into(),
        actor_label,
        detail: detail.into(),
        sensitivity_max,
    }
}

fn load_project(project_path: &str) -> Result<Project, RelayAuditError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(RelayAuditError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), RelayAuditError> {
    let parent = path.parent().ok_or(RelayAuditError::Io)?;
    fs::create_dir_all(parent).map_err(|_| RelayAuditError::Io)?;
    let temp = path.with_extension(AUDIT_TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| RelayAuditError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| RelayAuditError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| RelayAuditError::Io)?;
        file.sync_all().map_err(|_| RelayAuditError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| RelayAuditError::Io)
}
