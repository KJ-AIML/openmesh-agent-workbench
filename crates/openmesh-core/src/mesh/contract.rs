//! Dev Track 0.1.10 Checkpoint A — Mesh envelope wire contract (pure, no I/O).
//!
//! Local two-person mesh uses file envelopes only. This module freezes the
//! wire shape + fail-closed validation; export/import/peers are later checkpoints.

use crate::context::Sensitivity;
use crate::domain::{validate_evidence_ref, validate_utc_timestamp, CatchUpWindow, EvidenceRef};
use serde::{Deserialize, Serialize};

/// Wire protocol for `MeshEnvelope`.
pub const MESH_ENVELOPE_PROTOCOL_VERSION: &str = "1.0";

/// Project-local mesh storage root (relative under `.openmesh/`).
pub const MESH_DIR: &str = "mesh";
pub const MESH_OUTBOX_DIR: &str = "mesh/outbox";
pub const MESH_INBOX_DIR: &str = "mesh/inbox";
pub const MESH_PEERS_DIR: &str = "mesh/peers";

pub const MAX_ENVELOPE_ID_BYTES: usize = 128;
pub const MAX_PEER_LABEL_BYTES: usize = 128;
pub const MAX_PEER_PROFILE_ID_BYTES: usize = 128;
pub const MAX_WORKSPACE_ID_BYTES: usize = 128;
pub const MAX_EVIDENCE_ITEM_SUMMARY_BYTES: usize = 512;
pub const MAX_EVIDENCE_ITEMS: usize = 64;
pub const MAX_EVIDENCE_REFS_PER_ITEM: usize = 16;
pub const MAX_SOURCE_ID_BYTES: usize = 256;
pub const MAX_HANDOFF_IDS: usize = 32;
pub const MAX_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 512;

/// Where a mesh evidence item was projected from on the exporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MeshEvidenceSourceKind {
    WorkEvent,
    ContinuityItem,
    Handoff,
    PendingQuestion,
    ContextPack,
    Other,
}

impl MeshEvidenceSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MeshEvidenceSourceKind::WorkEvent => "work-event",
            MeshEvidenceSourceKind::ContinuityItem => "continuity-item",
            MeshEvidenceSourceKind::Handoff => "handoff",
            MeshEvidenceSourceKind::PendingQuestion => "pending-question",
            MeshEvidenceSourceKind::ContextPack => "context-pack",
            MeshEvidenceSourceKind::Other => "other",
        }
    }
}

/// Local peer label for envelope from/to (not network authentication).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshPeerRef {
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy_profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
}

/// One attributed evidence bullet carried in an envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshEvidenceItem {
    pub summary: String,
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    pub source_kind: MeshEvidenceSourceKind,
    pub source_id: String,
}

/// Highest sensitivity allowed in this envelope (secret is never permitted).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MeshSensitivityMax {
    Public,
    Team,
    Private,
}

impl MeshSensitivityMax {
    pub fn as_str(self) -> &'static str {
        match self {
            MeshSensitivityMax::Public => "public",
            MeshSensitivityMax::Team => "team",
            MeshSensitivityMax::Private => "private",
        }
    }

    /// True if `item` is allowed at or below this envelope max.
    pub fn allows(self, item: Sensitivity) -> bool {
        if matches!(item, Sensitivity::Secret) {
            return false;
        }
        match self {
            MeshSensitivityMax::Public => matches!(item, Sensitivity::Public),
            MeshSensitivityMax::Team => matches!(item, Sensitivity::Public | Sensitivity::Team),
            MeshSensitivityMax::Private => {
                matches!(
                    item,
                    Sensitivity::Public | Sensitivity::Team | Sensitivity::Private
                )
            }
        }
    }
}

/// File-exchange package between two local Work Proxies (protocol 1.0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshEnvelope {
    pub protocol_version: String,
    pub envelope_id: String,
    pub from_peer: MeshPeerRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_peer: Option<MeshPeerRef>,
    pub generated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<CatchUpWindow>,
    #[serde(default)]
    pub evidence_items: Vec<MeshEvidenceItem>,
    #[serde(default)]
    pub handoff_ids: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub sensitivity_max: MeshSensitivityMax,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MeshValidationError {
    #[error("unsupported protocol_version {found}; accepted version is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("envelope_id is empty after trim")]
    EmptyEnvelopeId,
    #[error("envelope_id exceeds the {max}-byte bound")]
    EnvelopeIdTooLong { max: usize },
    #[error("envelope_id must not contain path separators or '..'")]
    UnsafeEnvelopeId,
    #[error("peer label is empty after trim")]
    EmptyPeerLabel,
    #[error("peer label exceeds the {max}-byte bound")]
    PeerLabelTooLong { max: usize },
    #[error("peer proxy_profile_id is empty after trim")]
    EmptyProxyProfileId,
    #[error("peer proxy_profile_id exceeds the {max}-byte bound")]
    ProxyProfileIdTooLong { max: usize },
    #[error("peer workspace_id is empty after trim")]
    EmptyPeerWorkspaceId,
    #[error("peer workspace_id exceeds the {max}-byte bound")]
    PeerWorkspaceIdTooLong { max: usize },
    #[error("from_peer.workspace_id is required")]
    FromPeerWorkspaceRequired,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("invalid catch-up window: {0}")]
    InvalidWindow(String),
    #[error("catch-up window is inverted (since > until)")]
    WindowInverted,
    #[error("evidence_items exceed the {max}-entry bound")]
    TooManyEvidenceItems { max: usize },
    #[error("evidence item summary is empty")]
    EmptyEvidenceSummary,
    #[error("evidence item summary exceeds the {max}-byte bound")]
    EvidenceSummaryTooLong { max: usize },
    #[error("evidence item source_id is empty after trim")]
    EmptySourceId,
    #[error("evidence item source_id exceeds the {max}-byte bound")]
    SourceIdTooLong { max: usize },
    #[error("evidence item has too many evidence refs (max {max})")]
    TooManyEvidenceRefs { max: usize },
    #[error("item evidence is invalid: {0}")]
    InvalidItemEvidence(String),
    #[error("handoff_ids exceed the {max}-entry bound")]
    TooManyHandoffIds { max: usize },
    #[error("handoff_id is empty after trim")]
    EmptyHandoffId,
    #[error("limitations exceed the {max}-entry bound")]
    TooManyLimitations { max: usize },
    #[error("limitation is empty")]
    EmptyLimitation,
    #[error("limitation exceeds the {max}-byte bound")]
    LimitationTooLong { max: usize },
    #[error("envelope has no evidence items, handoff ids, or limitations (fail closed)")]
    EmptyEnvelopeWithoutLimitations,
}

/// Path-safe envelope id for future outbox/inbox filenames.
pub fn validate_envelope_id_for_storage(envelope_id: &str) -> Result<(), MeshValidationError> {
    let trimmed = envelope_id.trim();
    if trimmed.is_empty() {
        return Err(MeshValidationError::EmptyEnvelopeId);
    }
    if trimmed.len() > MAX_ENVELOPE_ID_BYTES {
        return Err(MeshValidationError::EnvelopeIdTooLong {
            max: MAX_ENVELOPE_ID_BYTES,
        });
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err(MeshValidationError::UnsafeEnvelopeId);
    }
    Ok(())
}

pub fn validate_mesh_peer_ref(
    peer: &MeshPeerRef,
    require_workspace: bool,
) -> Result<(), MeshValidationError> {
    if peer.label.trim().is_empty() {
        return Err(MeshValidationError::EmptyPeerLabel);
    }
    if peer.label.len() > MAX_PEER_LABEL_BYTES {
        return Err(MeshValidationError::PeerLabelTooLong {
            max: MAX_PEER_LABEL_BYTES,
        });
    }
    if let Some(profile_id) = &peer.proxy_profile_id {
        if profile_id.trim().is_empty() {
            return Err(MeshValidationError::EmptyProxyProfileId);
        }
        if profile_id.len() > MAX_PEER_PROFILE_ID_BYTES {
            return Err(MeshValidationError::ProxyProfileIdTooLong {
                max: MAX_PEER_PROFILE_ID_BYTES,
            });
        }
    }
    match &peer.workspace_id {
        Some(ws) => {
            if ws.trim().is_empty() {
                return Err(MeshValidationError::EmptyPeerWorkspaceId);
            }
            if ws.len() > MAX_WORKSPACE_ID_BYTES {
                return Err(MeshValidationError::PeerWorkspaceIdTooLong {
                    max: MAX_WORKSPACE_ID_BYTES,
                });
            }
        }
        None if require_workspace => {
            return Err(MeshValidationError::FromPeerWorkspaceRequired);
        }
        None => {}
    }
    Ok(())
}

fn validate_window(window: &CatchUpWindow) -> Result<(), MeshValidationError> {
    validate_utc_timestamp(&window.since).map_err(MeshValidationError::InvalidWindow)?;
    validate_utc_timestamp(&window.until).map_err(MeshValidationError::InvalidWindow)?;
    let since = chrono::DateTime::parse_from_rfc3339(&window.since)
        .map_err(|e| MeshValidationError::InvalidWindow(e.to_string()))?;
    let until = chrono::DateTime::parse_from_rfc3339(&window.until)
        .map_err(|e| MeshValidationError::InvalidWindow(e.to_string()))?;
    if since > until {
        return Err(MeshValidationError::WindowInverted);
    }
    Ok(())
}

fn validate_evidence_item(item: &MeshEvidenceItem) -> Result<(), MeshValidationError> {
    if item.summary.trim().is_empty() {
        return Err(MeshValidationError::EmptyEvidenceSummary);
    }
    if item.summary.len() > MAX_EVIDENCE_ITEM_SUMMARY_BYTES {
        return Err(MeshValidationError::EvidenceSummaryTooLong {
            max: MAX_EVIDENCE_ITEM_SUMMARY_BYTES,
        });
    }
    if item.source_id.trim().is_empty() {
        return Err(MeshValidationError::EmptySourceId);
    }
    if item.source_id.len() > MAX_SOURCE_ID_BYTES {
        return Err(MeshValidationError::SourceIdTooLong {
            max: MAX_SOURCE_ID_BYTES,
        });
    }
    if item.evidence_refs.len() > MAX_EVIDENCE_REFS_PER_ITEM {
        return Err(MeshValidationError::TooManyEvidenceRefs {
            max: MAX_EVIDENCE_REFS_PER_ITEM,
        });
    }
    for evidence in &item.evidence_refs {
        validate_evidence_ref(evidence)
            .map_err(|e| MeshValidationError::InvalidItemEvidence(e.to_string()))?;
    }
    Ok(())
}

/// Structural validation for `MeshEnvelope` v1.0 (pure, no I/O).
pub fn validate_mesh_envelope(envelope: &MeshEnvelope) -> Result<(), MeshValidationError> {
    if envelope.protocol_version != MESH_ENVELOPE_PROTOCOL_VERSION {
        return Err(MeshValidationError::UnsupportedProtocolVersion {
            found: envelope.protocol_version.clone(),
            expected: MESH_ENVELOPE_PROTOCOL_VERSION,
        });
    }
    validate_envelope_id_for_storage(&envelope.envelope_id)?;
    validate_mesh_peer_ref(&envelope.from_peer, true)?;
    if let Some(to) = &envelope.to_peer {
        validate_mesh_peer_ref(to, false)?;
    }
    validate_utc_timestamp(&envelope.generated_at)
        .map_err(MeshValidationError::InvalidTimestamp)?;
    if let Some(window) = &envelope.window {
        validate_window(window)?;
    }
    if envelope.evidence_items.len() > MAX_EVIDENCE_ITEMS {
        return Err(MeshValidationError::TooManyEvidenceItems {
            max: MAX_EVIDENCE_ITEMS,
        });
    }
    for item in &envelope.evidence_items {
        validate_evidence_item(item)?;
    }
    if envelope.handoff_ids.len() > MAX_HANDOFF_IDS {
        return Err(MeshValidationError::TooManyHandoffIds {
            max: MAX_HANDOFF_IDS,
        });
    }
    for handoff_id in &envelope.handoff_ids {
        if handoff_id.trim().is_empty() {
            return Err(MeshValidationError::EmptyHandoffId);
        }
    }
    if envelope.limitations.len() > MAX_LIMITATIONS {
        return Err(MeshValidationError::TooManyLimitations {
            max: MAX_LIMITATIONS,
        });
    }
    for limitation in &envelope.limitations {
        if limitation.trim().is_empty() {
            return Err(MeshValidationError::EmptyLimitation);
        }
        if limitation.len() > MAX_LIMITATION_BYTES {
            return Err(MeshValidationError::LimitationTooLong {
                max: MAX_LIMITATION_BYTES,
            });
        }
    }
    // Empty package without limitations is fail-closed (same spirit as handoff).
    if envelope.evidence_items.is_empty()
        && envelope.handoff_ids.is_empty()
        && envelope.limitations.is_empty()
    {
        return Err(MeshValidationError::EmptyEnvelopeWithoutLimitations);
    }
    // sensitivity_max is Public|Team|Private only — Secret cannot be expressed on the wire.
    let _ = envelope.sensitivity_max;
    Ok(())
}

#[cfg(test)]
mod sensitivity_unit_tests {
    use super::*;

    #[test]
    fn private_max_rejects_secret() {
        assert!(!MeshSensitivityMax::Private.allows(Sensitivity::Secret));
        assert!(MeshSensitivityMax::Private.allows(Sensitivity::Private));
    }

    #[test]
    fn public_max_rejects_team() {
        assert!(!MeshSensitivityMax::Public.allows(Sensitivity::Team));
    }
}
