//! Dev Track 0.1.10 Checkpoint E — peer evidence read model (inbox/outbox listing).

use crate::mesh::contract::{MeshEnvelope, MeshPeerRef};
use crate::mesh::export::{outbox_dir, read_outbox_envelope, MeshExportError};
use crate::mesh::import::{list_inbox_envelope_ids, read_inbox_envelope, MeshImportError};
use crate::storage::{read_project, Project};
use serde::{Deserialize, Serialize};
use std::fs;

/// Which mesh mailbox to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeshMailbox {
    Inbox,
    Outbox,
}

/// Compact envelope summary for list views (attributed foreign evidence).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshEnvelopeSummary {
    pub envelope_id: String,
    pub mailbox: MeshMailbox,
    pub from_peer: MeshPeerRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_peer: Option<MeshPeerRef>,
    pub generated_at: String,
    pub evidence_item_count: u32,
    pub handoff_id_count: u32,
    pub limitation_count: u32,
    /// Attribution label for human output (from peer).
    pub attributed_to: String,
}

#[derive(Debug, thiserror::Error)]
pub enum MeshViewError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("envelope not found")]
    NotFound,
    #[error("failed to read mesh mailbox")]
    ReadFailed,
    #[error("import error: {0}")]
    Import(#[from] MeshImportError),
    #[error("export error: {0}")]
    Export(#[from] MeshExportError),
}

/// List envelope summaries for inbox and/or outbox.
pub fn list_envelope_summaries(
    project_path: &str,
    mailbox: Option<MeshMailbox>,
) -> Result<Vec<MeshEnvelopeSummary>, MeshViewError> {
    let _ = load_project(project_path)?;
    let mut out = Vec::new();
    match mailbox {
        Some(MeshMailbox::Inbox) | None => {
            for id in list_inbox_envelope_ids(project_path)? {
                if let Ok(env) = read_inbox_envelope(project_path, &id) {
                    out.push(summary_from(&env, MeshMailbox::Inbox));
                }
            }
        }
        Some(MeshMailbox::Outbox) => {}
    }
    if matches!(mailbox, Some(MeshMailbox::Outbox) | None) {
        for id in list_outbox_envelope_ids(project_path)? {
            if let Ok(env) = read_outbox_envelope(project_path, &id) {
                out.push(summary_from(&env, MeshMailbox::Outbox));
            }
        }
    }
    out.sort_by(|a, b| {
        a.mailbox
            .as_sort_key()
            .cmp(&b.mailbox.as_sort_key())
            .then_with(|| a.envelope_id.cmp(&b.envelope_id))
    });
    Ok(out)
}

/// Show full envelope from inbox (preferred) or outbox.
pub fn show_envelope(
    project_path: &str,
    envelope_id: &str,
    mailbox: Option<MeshMailbox>,
) -> Result<(MeshEnvelope, MeshMailbox), MeshViewError> {
    let _ = load_project(project_path)?;
    match mailbox {
        Some(MeshMailbox::Inbox) => {
            let env = read_inbox_envelope(project_path, envelope_id)?;
            Ok((env, MeshMailbox::Inbox))
        }
        Some(MeshMailbox::Outbox) => {
            let env = read_outbox_envelope(project_path, envelope_id)?;
            Ok((env, MeshMailbox::Outbox))
        }
        None => {
            if let Ok(env) = read_inbox_envelope(project_path, envelope_id) {
                return Ok((env, MeshMailbox::Inbox));
            }
            if let Ok(env) = read_outbox_envelope(project_path, envelope_id) {
                return Ok((env, MeshMailbox::Outbox));
            }
            Err(MeshViewError::NotFound)
        }
    }
}

fn summary_from(envelope: &MeshEnvelope, mailbox: MeshMailbox) -> MeshEnvelopeSummary {
    MeshEnvelopeSummary {
        envelope_id: envelope.envelope_id.clone(),
        mailbox,
        from_peer: envelope.from_peer.clone(),
        to_peer: envelope.to_peer.clone(),
        generated_at: envelope.generated_at.clone(),
        evidence_item_count: envelope.evidence_items.len() as u32,
        handoff_id_count: envelope.handoff_ids.len() as u32,
        limitation_count: envelope.limitations.len() as u32,
        attributed_to: envelope.from_peer.label.clone(),
    }
}

impl MeshMailbox {
    fn as_sort_key(self) -> u8 {
        match self {
            MeshMailbox::Inbox => 0,
            MeshMailbox::Outbox => 1,
        }
    }
}

/// List outbox envelope ids (Checkpoint E helper).
pub fn list_outbox_envelope_ids(project_path: &str) -> Result<Vec<String>, MeshViewError> {
    let _ = load_project(project_path)?;
    let dir = outbox_dir(project_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut ids = Vec::new();
    let entries = fs::read_dir(&dir).map_err(|_| MeshViewError::ReadFailed)?;
    for entry in entries {
        let entry = entry.map_err(|_| MeshViewError::ReadFailed)?;
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

fn load_project(project_path: &str) -> Result<Project, MeshViewError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(MeshViewError::ProjectNotInitialized)
}
