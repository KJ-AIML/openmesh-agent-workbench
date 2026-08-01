//! Dev Track 0.1.8 Checkpoint D — handoff note storage and ledger linkage.

use crate::domain::{EvidenceAttachment, EvidenceRef, WorkEvent};
use crate::events::{append_event, get_event};
use crate::handoff::contract::{
    validate_handoff_id_for_storage, validate_handoff_note, HandoffNote, HandoffStatus,
    HandoffValidationError, WORK_EVENT_HANDOFF_KIND,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const HANDOFF_DIR: &str = "handoff";
const HANDOFF_TEMP_EXTENSION: &str = "handoff-tmp";
const HANDOFF_EVENT_ID_PREFIX: &str = "handoff-evt-";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HandoffStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("handoff note not found")]
    NotFound,
    #[error("malformed handoff JSON")]
    MalformedJson,
    #[error("handoff validation failed: {0}")]
    ValidationFailed(String),
    #[error("workspace_id does not match project")]
    WorkspaceMismatch,
    #[error("failed to read handoff note")]
    ReadFailed,
    #[error("failed to write handoff note")]
    WriteFailed,
    #[error("atomic handoff replacement failed")]
    AtomicReplaceFailed,
    #[error("ledger error: {0}")]
    Ledger(String),
    #[error("handoff is already linked to work event {0}")]
    AlreadyLinked(String),
}

/// Returns `<project>/.openmesh/handoff/`.
pub fn handoff_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(HANDOFF_DIR)
}

/// Returns `<project>/.openmesh/handoff/{handoff_id}.json`.
pub fn handoff_note_path(project_path: &str, handoff_id: &str) -> PathBuf {
    handoff_dir(project_path).join(format!("{handoff_id}.json"))
}

/// Relative evidence path for ledger linkage.
pub fn handoff_relative_path(handoff_id: &str) -> String {
    format!(".openmesh/handoff/{handoff_id}.json")
}

/// Lists persisted handoff ids in deterministic lexicographic order.
pub fn list_handoff_ids(project_path: &str) -> Result<Vec<String>, HandoffStorageError> {
    let dir = handoff_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| HandoffStorageError::ReadFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| HandoffStorageError::ReadFailed)?;
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

/// Validates and atomically writes a handoff note.
pub fn write_handoff_note(
    project_path: &str,
    note: &HandoffNote,
) -> Result<(), HandoffStorageError> {
    let project = load_project(project_path)?;
    if note.workspace_id != project.id {
        return Err(HandoffStorageError::WorkspaceMismatch);
    }
    validate_handoff_note(note).map_err(map_validation_error)?;

    let dir = handoff_dir(project_path);
    fs::create_dir_all(&dir).map_err(|_| HandoffStorageError::WriteFailed)?;
    let path = handoff_note_path(project_path, &note.handoff_id);
    write_json_atomic(&path, note)
}

/// Reads and validates a persisted handoff note.
pub fn read_handoff_note(
    project_path: &str,
    handoff_id: &str,
) -> Result<HandoffNote, HandoffStorageError> {
    validate_handoff_id_for_storage(handoff_id).map_err(map_validation_error)?;
    let project = load_project(project_path)?;
    let path = handoff_note_path(project_path, handoff_id);
    if !path.exists() {
        return Err(HandoffStorageError::NotFound);
    }
    let note = read_note_file(&path)?;
    if note.workspace_id != project.id {
        return Err(HandoffStorageError::WorkspaceMismatch);
    }
    validate_handoff_note(&note).map_err(map_validation_error)?;
    Ok(note)
}

/// Approves a persisted handoff note and rewrites it atomically.
pub fn approve_handoff_note(
    project_path: &str,
    handoff_id: &str,
    now_rfc3339: &str,
) -> Result<HandoffNote, HandoffStorageError> {
    let mut note = read_handoff_note(project_path, handoff_id)?;
    note.status = HandoffStatus::Approved;
    note.approved_at = Some(now_rfc3339.to_string());
    note.updated_at = now_rfc3339.to_string();
    write_handoff_note(project_path, &note)?;
    Ok(note)
}

/// Appends a `work.handoff` ledger event and links it on the note.
pub fn link_handoff_work_event(
    project_path: &str,
    mut note: HandoffNote,
) -> Result<HandoffNote, HandoffStorageError> {
    if let Some(existing) = &note.work_event_id {
        return Err(HandoffStorageError::AlreadyLinked(existing.clone()));
    }

    let event_id = handoff_work_event_id(&note.handoff_id);
    if let Some(existing) = get_event(project_path, &event_id)
        .map_err(|err| HandoffStorageError::Ledger(err.to_string()))?
    {
        note.work_event_id = Some(existing.event_id);
        write_handoff_note(project_path, &note)?;
        return Ok(note);
    }

    let relative_path = handoff_relative_path(&note.handoff_id);
    let event = WorkEvent::new(
        event_id,
        note.workspace_id.clone(),
        WORK_EVENT_HANDOFF_KIND,
        handoff_work_event_summary(&note),
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath(relative_path),
            observed_at: None,
        }],
        note.updated_at.clone(),
    );

    append_event(project_path, &event)
        .map_err(|err| HandoffStorageError::Ledger(err.to_string()))?;
    note.work_event_id = Some(event.event_id);
    write_handoff_note(project_path, &note)?;
    Ok(note)
}

fn handoff_work_event_id(handoff_id: &str) -> String {
    format!("{HANDOFF_EVENT_ID_PREFIX}{handoff_id}")
}

fn handoff_work_event_summary(note: &HandoffNote) -> String {
    let recipient = note.recipient.label.trim();
    let role = note
        .recipient
        .role_label
        .as_deref()
        .map(|value| format!(" ({value})"))
        .unwrap_or_default();
    format!("Handoff for {recipient}{role} ({})", note.handoff_id)
}

fn load_project(project_path: &str) -> Result<Project, HandoffStorageError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(HandoffStorageError::ProjectNotInitialized)
}

fn read_note_file(path: &Path) -> Result<HandoffNote, HandoffStorageError> {
    let raw = fs::read_to_string(path).map_err(|_| HandoffStorageError::ReadFailed)?;
    serde_json::from_str(&raw).map_err(|_| HandoffStorageError::MalformedJson)
}

fn map_validation_error(err: HandoffValidationError) -> HandoffStorageError {
    HandoffStorageError::ValidationFailed(err.to_string())
}

fn write_json_atomic<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), HandoffStorageError> {
    let parent = path.parent().ok_or(HandoffStorageError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| HandoffStorageError::WriteFailed)?;
    let temp = path.with_extension(HANDOFF_TEMP_EXTENSION);
    let mut json =
        serde_json::to_string_pretty(value).map_err(|_| HandoffStorageError::WriteFailed)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| HandoffStorageError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| HandoffStorageError::WriteFailed)?;
        file.sync_all()
            .map_err(|_| HandoffStorageError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| HandoffStorageError::AtomicReplaceFailed)
}
