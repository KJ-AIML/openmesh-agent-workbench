//! Pure wire contracts for Connector Layer (0.1.18).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const CONNECTOR_PROTOCOL_VERSION: &str = "1.0";
pub const CONNECTORS_DIR: &str = "connectors";
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_NAME_BYTES: usize = 256;
pub const MAX_URL_BYTES: usize = 512;
pub const MAX_SUMMARY_BYTES: usize = 1024;
pub const MAX_ITEMS: usize = 64;
pub const MAX_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 256;

/// Connector kinds available in beta (stubs / offline-shaped).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorKind {
    /// Offline GitHub-shaped evidence producer (no live API; not SoR).
    GithubStub,
}

/// Always evidence-producer — never system-of-record replacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorRole {
    EvidenceProducerOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceItemKind {
    Issue,
    PullRequest,
    Comment,
    Status,
    Other,
}

/// Registered connector descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectorDescriptor {
    pub protocol_version: String,
    pub connector_id: String,
    pub kind: ConnectorKind,
    pub display_name: String,
    pub role: ConnectorRole,
    pub enabled: bool,
    /// e.g. `owner/repo` for GitHub-shaped stub.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// One external evidence item produced by a connector collect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalEvidenceItem {
    pub external_id: String,
    pub title: String,
    pub kind: EvidenceItemKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    pub summary: String,
    pub observed_at: String,
}

/// Result of a connector collect (evidence producer run).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectorRun {
    pub protocol_version: String,
    pub run_id: String,
    pub connector_id: String,
    pub kind: ConnectorKind,
    pub collected_at: String,
    /// Always true in beta — no live SoR mutation.
    pub evidence_only: bool,
    /// Stub/fixture source label.
    pub source: String,
    #[serde(default)]
    pub items: Vec<ExternalEvidenceItem>,
    pub note: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConnectorValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("connector_id invalid")]
    InvalidConnectorId,
    #[error("display_name empty or too long")]
    InvalidDisplayName,
    #[error("role must be evidence-producer-only")]
    InvalidRole,
    #[error("external_ref invalid")]
    InvalidExternalRef,
    #[error("invalid item")]
    InvalidItem,
    #[error("too many items")]
    TooManyItems,
    #[error("evidence_only must be true")]
    EvidenceOnlyRequired,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("limitations bounds")]
    LimitationsBounds,
}

pub fn validate_connector_descriptor(
    d: &ConnectorDescriptor,
) -> Result<(), ConnectorValidationError> {
    if d.protocol_version != CONNECTOR_PROTOCOL_VERSION {
        return Err(ConnectorValidationError::UnsupportedProtocol {
            found: d.protocol_version.clone(),
        });
    }
    if d.connector_id.trim().is_empty()
        || d.connector_id.len() > MAX_ID_BYTES
        || d.connector_id.contains("..")
        || d.connector_id.contains('/')
    {
        return Err(ConnectorValidationError::InvalidConnectorId);
    }
    if d.display_name.trim().is_empty() || d.display_name.len() > MAX_NAME_BYTES {
        return Err(ConnectorValidationError::InvalidDisplayName);
    }
    if !matches!(d.role, ConnectorRole::EvidenceProducerOnly) {
        return Err(ConnectorValidationError::InvalidRole);
    }
    if let Some(r) = &d.external_ref {
        if r.trim().is_empty()
            || r.len() > MAX_NAME_BYTES
            || r.contains("..")
            || r.starts_with('/')
        {
            return Err(ConnectorValidationError::InvalidExternalRef);
        }
    }
    if d.limitations.len() > MAX_LIMITATIONS
        || d.limitations.iter().any(|l| l.len() > MAX_LIMITATION_BYTES)
    {
        return Err(ConnectorValidationError::LimitationsBounds);
    }
    validate_utc_timestamp(&d.created_at).map_err(ConnectorValidationError::InvalidTimestamp)?;
    validate_utc_timestamp(&d.updated_at).map_err(ConnectorValidationError::InvalidTimestamp)?;
    Ok(())
}

pub fn validate_evidence_item(i: &ExternalEvidenceItem) -> Result<(), ConnectorValidationError> {
    if i.external_id.trim().is_empty() || i.external_id.len() > MAX_ID_BYTES {
        return Err(ConnectorValidationError::InvalidItem);
    }
    if i.title.trim().is_empty() || i.title.len() > MAX_NAME_BYTES {
        return Err(ConnectorValidationError::InvalidItem);
    }
    if i.summary.len() > MAX_SUMMARY_BYTES {
        return Err(ConnectorValidationError::InvalidItem);
    }
    if let Some(u) = &i.url {
        if u.len() > MAX_URL_BYTES || u.contains("..") {
            return Err(ConnectorValidationError::InvalidItem);
        }
    }
    validate_utc_timestamp(&i.observed_at).map_err(ConnectorValidationError::InvalidTimestamp)?;
    Ok(())
}

pub fn validate_connector_run(r: &ConnectorRun) -> Result<(), ConnectorValidationError> {
    if r.protocol_version != CONNECTOR_PROTOCOL_VERSION {
        return Err(ConnectorValidationError::UnsupportedProtocol {
            found: r.protocol_version.clone(),
        });
    }
    if r.run_id.trim().is_empty() || r.connector_id.trim().is_empty() {
        return Err(ConnectorValidationError::InvalidConnectorId);
    }
    if !r.evidence_only {
        return Err(ConnectorValidationError::EvidenceOnlyRequired);
    }
    if r.items.len() > MAX_ITEMS {
        return Err(ConnectorValidationError::TooManyItems);
    }
    for i in &r.items {
        validate_evidence_item(i)?;
    }
    validate_utc_timestamp(&r.collected_at).map_err(ConnectorValidationError::InvalidTimestamp)?;
    Ok(())
}
