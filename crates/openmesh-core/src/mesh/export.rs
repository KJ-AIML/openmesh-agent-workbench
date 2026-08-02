//! Dev Track 0.1.10 Checkpoint C — mesh envelope export builder + outbox write.

use crate::continuity::readers::ContinuityInputSnapshot;
use crate::continuity::{build_catch_up_view, ContinuityError};
use crate::domain::{
    CatchUpWindow, ContinuityStateItem, CurrentStateProjection, PendingAttentionItem,
};
use crate::handoff::{list_handoff_ids, HandoffStorageError};
use crate::mesh::contract::{
    validate_envelope_id_for_storage, validate_mesh_envelope, MeshEnvelope, MeshEvidenceItem,
    MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax, MeshValidationError,
    MESH_ENVELOPE_PROTOCOL_VERSION, MESH_OUTBOX_DIR, MAX_EVIDENCE_ITEMS, MAX_HANDOFF_IDS,
    MAX_LIMITATIONS,
};
use crate::mesh::peers::{read_peer, MeshPeerError};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const OUTBOX_TEMP_EXTENSION: &str = "envelope-tmp";

/// Inputs for building a mesh export envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildMeshExportRequest {
    pub workspace_id: String,
    pub from_peer: MeshPeerRef,
    pub to_peer: Option<MeshPeerRef>,
    pub window: Option<CatchUpWindow>,
    pub now_rfc3339: String,
    pub envelope_id: String,
    pub sensitivity_max: MeshSensitivityMax,
    /// When true, attach approved/draft handoff ids present in the project.
    pub include_handoff_ids: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MeshExportError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("workspace_id does not match continuity snapshot")]
    WorkspaceMismatch,
    #[error("mesh peer error: {0}")]
    Peer(#[from] MeshPeerError),
    #[error("continuity build failed: {0}")]
    Continuity(String),
    #[error("handoff list failed: {0}")]
    Handoff(String),
    #[error("envelope validation failed: {0}")]
    Validation(#[from] MeshValidationError),
    #[error("failed to write envelope to outbox")]
    WriteFailed,
    #[error("atomic envelope write failed")]
    AtomicReplaceFailed,
    #[error("envelope already exists in outbox: {0}")]
    AlreadyExists(String),
    #[error("envelope not found in outbox")]
    NotFound,
}

impl From<ContinuityError> for MeshExportError {
    fn from(value: ContinuityError) -> Self {
        MeshExportError::Continuity(value.to_string())
    }
}

impl From<HandoffStorageError> for MeshExportError {
    fn from(value: HandoffStorageError) -> Self {
        MeshExportError::Handoff(value.to_string())
    }
}

/// Returns `<project>/.openmesh/mesh/outbox/`.
pub fn outbox_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(MESH_OUTBOX_DIR)
}

/// Returns `<project>/.openmesh/mesh/outbox/{envelope_id}.json`.
pub fn outbox_envelope_path(project_path: &str, envelope_id: &str) -> PathBuf {
    outbox_dir(project_path).join(format!("{envelope_id}.json"))
}

/// Build a validated envelope from continuity (+ optional handoff ids). Pure of disk except optional handoff list via request flags handled by caller.
pub fn build_mesh_export_envelope(
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    handoff_ids: &[String],
    request: &BuildMeshExportRequest,
) -> Result<MeshEnvelope, MeshExportError> {
    if request.workspace_id != snapshot.workspace_id {
        return Err(MeshExportError::WorkspaceMismatch);
    }
    validate_envelope_id_for_storage(&request.envelope_id)?;

    let mut evidence_items = Vec::new();
    let mut limitations = Vec::new();

    if let Some(window) = &request.window {
        let catch_up = build_catch_up_view(snapshot, current_state, window)?;
        push_items(
            &mut evidence_items,
            catch_up.sections.completed.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            catch_up.sections.changed.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            catch_up.sections.blocked.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            catch_up.sections.decided.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            catch_up.sections.needs_attention.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            catch_up.sections.still_open.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        for attention in &catch_up.next_suggested_attention {
            push_attention(&mut evidence_items, attention);
        }
        limitations.extend(catch_up.limitations);
    } else {
        // No window: export current-state snapshot sections only.
        push_items(
            &mut evidence_items,
            current_state.sections.completed.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            current_state.sections.in_progress.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            current_state.sections.blocked.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            current_state.sections.decisions.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            current_state.sections.needs_attention.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        push_items(
            &mut evidence_items,
            current_state.sections.still_open.iter(),
            MeshEvidenceSourceKind::ContinuityItem,
        );
        for attention in &current_state.pending_attention {
            push_attention(&mut evidence_items, attention);
        }
        limitations.extend(current_state.limitations.clone());
    }

    evidence_items.truncate(MAX_EVIDENCE_ITEMS);

    let mut selected_handoffs: Vec<String> = handoff_ids.iter().cloned().collect();
    selected_handoffs.sort();
    selected_handoffs.dedup();
    selected_handoffs.truncate(MAX_HANDOFF_IDS);

    if evidence_items.is_empty() && selected_handoffs.is_empty() {
        limitations.push("export contained no continuity items or handoff ids".into());
    }
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_LIMITATIONS);

    let mut from_peer = request.from_peer.clone();
    if from_peer.workspace_id.is_none() {
        from_peer.workspace_id = Some(request.workspace_id.clone());
    }

    let envelope = MeshEnvelope {
        protocol_version: MESH_ENVELOPE_PROTOCOL_VERSION.into(),
        envelope_id: request.envelope_id.clone(),
        from_peer,
        to_peer: request.to_peer.clone(),
        generated_at: request.now_rfc3339.clone(),
        window: request.window.clone(),
        evidence_items,
        handoff_ids: selected_handoffs,
        limitations,
        sensitivity_max: request.sensitivity_max,
    };
    validate_mesh_envelope(&envelope)?;
    Ok(envelope)
}

/// Build from project continuity + handoff list + peer registry, then write outbox.
pub fn export_mesh_envelope_to_outbox(
    project_path: &str,
    snapshot: &ContinuityInputSnapshot,
    current_state: &CurrentStateProjection,
    request: &BuildMeshExportRequest,
) -> Result<MeshEnvelope, MeshExportError> {
    let _ = load_project(project_path)?;
    let handoff_ids = if request.include_handoff_ids {
        list_handoff_ids(project_path)?
    } else {
        Vec::new()
    };
    let envelope =
        build_mesh_export_envelope(snapshot, current_state, &handoff_ids, request)?;
    write_outbox_envelope(project_path, &envelope)?;
    Ok(envelope)
}

/// Resolve `to_peer` from the local peer registry by peer id.
pub fn to_peer_from_registry(
    project_path: &str,
    peer_id: &str,
) -> Result<MeshPeerRef, MeshExportError> {
    let peer = read_peer(project_path, peer_id)?;
    Ok(peer.as_peer_ref())
}

/// Persist envelope under outbox (fail if id already exists).
pub fn write_outbox_envelope(
    project_path: &str,
    envelope: &MeshEnvelope,
) -> Result<(), MeshExportError> {
    let _ = load_project(project_path)?;
    validate_mesh_envelope(envelope)?;
    let path = outbox_envelope_path(project_path, &envelope.envelope_id);
    if path.exists() {
        return Err(MeshExportError::AlreadyExists(envelope.envelope_id.clone()));
    }
    fs::create_dir_all(outbox_dir(project_path)).map_err(|_| MeshExportError::WriteFailed)?;
    write_json_atomic(&path, envelope)
}

/// Read an outbox envelope (Checkpoint C helper for tests/show later).
pub fn read_outbox_envelope(
    project_path: &str,
    envelope_id: &str,
) -> Result<MeshEnvelope, MeshExportError> {
    let _ = load_project(project_path)?;
    validate_envelope_id_for_storage(envelope_id)?;
    let path = outbox_envelope_path(project_path, envelope_id);
    if !path.exists() {
        return Err(MeshExportError::NotFound);
    }
    let raw = fs::read_to_string(&path).map_err(|_| MeshExportError::WriteFailed)?;
    let envelope: MeshEnvelope =
        serde_json::from_str(&raw).map_err(|_| MeshExportError::WriteFailed)?;
    validate_mesh_envelope(&envelope)?;
    Ok(envelope)
}

fn load_project(project_path: &str) -> Result<Project, MeshExportError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(MeshExportError::ProjectNotInitialized)
}

fn push_items<'a, I>(
    out: &mut Vec<MeshEvidenceItem>,
    items: I,
    source_kind: MeshEvidenceSourceKind,
) where
    I: Iterator<Item = &'a ContinuityStateItem>,
{
    for item in items {
        if out.len() >= MAX_EVIDENCE_ITEMS {
            break;
        }
        out.push(MeshEvidenceItem {
            summary: item.summary.clone(),
            evidence_refs: item.evidence_refs.clone(),
            source_kind,
            source_id: item.source_id.clone(),
        });
    }
}

fn push_attention(out: &mut Vec<MeshEvidenceItem>, attention: &PendingAttentionItem) {
    if out.len() >= MAX_EVIDENCE_ITEMS {
        return;
    }
    out.push(MeshEvidenceItem {
        summary: attention.summary.clone(),
        evidence_refs: attention.evidence_refs.clone(),
        source_kind: MeshEvidenceSourceKind::PendingQuestion,
        source_id: attention.source_id.clone(),
    });
}

fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), MeshExportError> {
    let parent = path.parent().ok_or(MeshExportError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| MeshExportError::WriteFailed)?;
    let temp = path.with_extension(OUTBOX_TEMP_EXTENSION);
    let mut json =
        serde_json::to_string_pretty(value).map_err(|_| MeshExportError::WriteFailed)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| MeshExportError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| MeshExportError::WriteFailed)?;
        file.sync_all().map_err(|_| MeshExportError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| MeshExportError::AtomicReplaceFailed)
}
