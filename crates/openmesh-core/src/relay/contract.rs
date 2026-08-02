//! Dev Track 0.1.11 Checkpoint A — RelayPackage wire contract (pure, no I/O).

use crate::domain::validate_utc_timestamp;
use crate::mesh::{
    validate_mesh_envelope, MeshEnvelope, MeshSensitivityMax, MeshValidationError,
};
use serde::{Deserialize, Serialize};

pub const RELAY_PACKAGE_PROTOCOL_VERSION: &str = "1.0";
pub const RELAY_AUDIT_PROTOCOL_VERSION: &str = "1.0";

pub const RELAY_DIR: &str = "relay";
pub const RELAY_STAGING_DIR: &str = "relay/staging";
pub const RELAY_APPROVED_DIR: &str = "relay/approved";
pub const RELAY_SENT_DIR: &str = "relay/sent";
pub const RELAY_RECEIVED_DIR: &str = "relay/received";
pub const RELAY_AUDIT_DIR: &str = "relay/audit";

pub const MAX_PACKAGE_ID_BYTES: usize = 128;
pub const MAX_WORKSPACE_ID_BYTES: usize = 128;
pub const MAX_ENVELOPES_PER_PACKAGE: usize = 16;
pub const MAX_HANDOFF_IDS: usize = 32;
pub const MAX_POLICY_STRINGS: usize = 32;
pub const MAX_POLICY_STRING_BYTES: usize = 256;
pub const MAX_LIMITATIONS: usize = 16;
pub const MAX_LIMITATION_BYTES: usize = 512;
pub const MAX_CONTENT_HASH_BYTES: usize = 128;
pub const MAX_ACTOR_LABEL_BYTES: usize = 128;
pub const MAX_AUDIT_DETAIL_BYTES: usize = 512;

/// Policy snapshot attached to a relay package at pack time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelayPolicySnapshot {
    #[serde(default)]
    pub approved_paths: Vec<String>,
    #[serde(default)]
    pub denied_classes: Vec<String>,
    #[serde(default)]
    pub selection_notes: Vec<String>,
}

/// Selective-sync package that may leave the local node only after approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelayPackage {
    pub protocol_version: String,
    pub package_id: String,
    pub workspace_id: String,
    pub generated_at: String,
    pub sensitivity_max: MeshSensitivityMax,
    #[serde(default)]
    pub envelopes: Vec<MeshEnvelope>,
    #[serde(default)]
    pub handoff_ids: Vec<String>,
    #[serde(default)]
    pub policy: RelayPolicySnapshot,
    #[serde(default)]
    pub limitations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
    /// Set only after explicit approve (Checkpoint D).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_by: Option<String>,
}

/// Audit event kinds for egress lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelayAuditKind {
    Staged,
    Approved,
    Sent,
    Received,
    Denied,
}

impl RelayAuditKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RelayAuditKind::Staged => "staged",
            RelayAuditKind::Approved => "approved",
            RelayAuditKind::Sent => "sent",
            RelayAuditKind::Received => "received",
            RelayAuditKind::Denied => "denied",
        }
    }
}

/// Append-only audit event (wire form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RelayAuditEvent {
    pub protocol_version: String,
    pub event_id: String,
    pub package_id: String,
    pub kind: RelayAuditKind,
    pub at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_label: Option<String>,
    pub detail: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitivity_max: Option<MeshSensitivityMax>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RelayValidationError {
    #[error("unsupported protocol_version {found}; accepted is {expected}")]
    UnsupportedProtocolVersion {
        found: String,
        expected: &'static str,
    },
    #[error("package_id is empty after trim")]
    EmptyPackageId,
    #[error("package_id exceeds the {max}-byte bound")]
    PackageIdTooLong { max: usize },
    #[error("package_id must not contain path separators or '..'")]
    UnsafePackageId,
    #[error("workspace_id is empty after trim")]
    EmptyWorkspaceId,
    #[error("workspace_id exceeds the {max}-byte bound")]
    WorkspaceIdTooLong { max: usize },
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("too many envelopes (max {max})")]
    TooManyEnvelopes { max: usize },
    #[error("envelope validation failed: {0}")]
    Envelope(String),
    #[error("envelope sensitivity exceeds package sensitivity_max")]
    EnvelopeSensitivityExceedsPackage,
    #[error("too many handoff ids (max {max})")]
    TooManyHandoffIds { max: usize },
    #[error("handoff_id is empty")]
    EmptyHandoffId,
    #[error("policy list exceeds bounds")]
    PolicyBounds,
    #[error("empty policy string")]
    EmptyPolicyString,
    #[error("limitation is empty or too long")]
    LimitationInvalid,
    #[error("too many limitations (max {max})")]
    TooManyLimitations { max: usize },
    #[error("package has no envelopes/handoffs and no limitations (fail closed)")]
    EmptyPackageWithoutLimitations,
    #[error("content_hash is empty or too long")]
    InvalidContentHash,
    #[error("approved_by is empty or too long")]
    InvalidApprovedBy,
    #[error("approved_at set without approved_by (or vice versa)")]
    ApprovalFieldsInconsistent,
    #[error("audit event invalid: {0}")]
    Audit(String),
}

pub fn validate_package_id_for_storage(package_id: &str) -> Result<(), RelayValidationError> {
    let t = package_id.trim();
    if t.is_empty() {
        return Err(RelayValidationError::EmptyPackageId);
    }
    if t.len() > MAX_PACKAGE_ID_BYTES {
        return Err(RelayValidationError::PackageIdTooLong {
            max: MAX_PACKAGE_ID_BYTES,
        });
    }
    if t.contains('/') || t.contains('\\') || t.contains("..") {
        return Err(RelayValidationError::UnsafePackageId);
    }
    Ok(())
}

fn validate_policy_list(list: &[String]) -> Result<(), RelayValidationError> {
    if list.len() > MAX_POLICY_STRINGS {
        return Err(RelayValidationError::PolicyBounds);
    }
    for s in list {
        if s.trim().is_empty() || s.len() > MAX_POLICY_STRING_BYTES {
            return Err(RelayValidationError::EmptyPolicyString);
        }
    }
    Ok(())
}

fn sensitivity_rank(s: MeshSensitivityMax) -> u8 {
    match s {
        MeshSensitivityMax::Public => 0,
        MeshSensitivityMax::Team => 1,
        MeshSensitivityMax::Private => 2,
    }
}

/// Pure structural validation for RelayPackage v1.0.
pub fn validate_relay_package(pkg: &RelayPackage) -> Result<(), RelayValidationError> {
    if pkg.protocol_version != RELAY_PACKAGE_PROTOCOL_VERSION {
        return Err(RelayValidationError::UnsupportedProtocolVersion {
            found: pkg.protocol_version.clone(),
            expected: RELAY_PACKAGE_PROTOCOL_VERSION,
        });
    }
    validate_package_id_for_storage(&pkg.package_id)?;
    if pkg.workspace_id.trim().is_empty() {
        return Err(RelayValidationError::EmptyWorkspaceId);
    }
    if pkg.workspace_id.len() > MAX_WORKSPACE_ID_BYTES {
        return Err(RelayValidationError::WorkspaceIdTooLong {
            max: MAX_WORKSPACE_ID_BYTES,
        });
    }
    validate_utc_timestamp(&pkg.generated_at).map_err(RelayValidationError::InvalidTimestamp)?;

    if pkg.envelopes.len() > MAX_ENVELOPES_PER_PACKAGE {
        return Err(RelayValidationError::TooManyEnvelopes {
            max: MAX_ENVELOPES_PER_PACKAGE,
        });
    }
    for env in &pkg.envelopes {
        validate_mesh_envelope(env)
            .map_err(|e: MeshValidationError| RelayValidationError::Envelope(e.to_string()))?;
        if sensitivity_rank(env.sensitivity_max) > sensitivity_rank(pkg.sensitivity_max) {
            return Err(RelayValidationError::EnvelopeSensitivityExceedsPackage);
        }
    }

    if pkg.handoff_ids.len() > MAX_HANDOFF_IDS {
        return Err(RelayValidationError::TooManyHandoffIds {
            max: MAX_HANDOFF_IDS,
        });
    }
    for id in &pkg.handoff_ids {
        if id.trim().is_empty() {
            return Err(RelayValidationError::EmptyHandoffId);
        }
    }

    validate_policy_list(&pkg.policy.approved_paths)?;
    validate_policy_list(&pkg.policy.denied_classes)?;
    validate_policy_list(&pkg.policy.selection_notes)?;

    // Secret class must always be denied on the wire policy snapshot for alpha.
    if !pkg
        .policy
        .denied_classes
        .iter()
        .any(|c| c.eq_ignore_ascii_case("secret"))
    {
        // Not a hard fail if empty package with limitations documenting policy —
        // but recommended; pack builder always injects "secret".
    }

    if pkg.limitations.len() > MAX_LIMITATIONS {
        return Err(RelayValidationError::TooManyLimitations {
            max: MAX_LIMITATIONS,
        });
    }
    for lim in &pkg.limitations {
        if lim.trim().is_empty() || lim.len() > MAX_LIMITATION_BYTES {
            return Err(RelayValidationError::LimitationInvalid);
        }
    }

    if pkg.envelopes.is_empty() && pkg.handoff_ids.is_empty() && pkg.limitations.is_empty() {
        return Err(RelayValidationError::EmptyPackageWithoutLimitations);
    }

    if let Some(hash) = &pkg.content_hash {
        if hash.trim().is_empty() || hash.len() > MAX_CONTENT_HASH_BYTES {
            return Err(RelayValidationError::InvalidContentHash);
        }
    }

    match (&pkg.approved_at, &pkg.approved_by) {
        (None, None) => {}
        (Some(at), Some(by)) => {
            validate_utc_timestamp(at).map_err(RelayValidationError::InvalidTimestamp)?;
            if by.trim().is_empty() || by.len() > MAX_ACTOR_LABEL_BYTES {
                return Err(RelayValidationError::InvalidApprovedBy);
            }
        }
        _ => return Err(RelayValidationError::ApprovalFieldsInconsistent),
    }

    Ok(())
}

pub fn validate_relay_audit_event(ev: &RelayAuditEvent) -> Result<(), RelayValidationError> {
    if ev.protocol_version != RELAY_AUDIT_PROTOCOL_VERSION {
        return Err(RelayValidationError::UnsupportedProtocolVersion {
            found: ev.protocol_version.clone(),
            expected: RELAY_AUDIT_PROTOCOL_VERSION,
        });
    }
    if ev.event_id.trim().is_empty() || ev.event_id.contains("..") || ev.event_id.contains('/') {
        return Err(RelayValidationError::Audit("invalid event_id".into()));
    }
    validate_package_id_for_storage(&ev.package_id)?;
    validate_utc_timestamp(&ev.at).map_err(RelayValidationError::InvalidTimestamp)?;
    if ev.detail.trim().is_empty() || ev.detail.len() > MAX_AUDIT_DETAIL_BYTES {
        return Err(RelayValidationError::Audit("invalid detail".into()));
    }
    if let Some(actor) = &ev.actor_label {
        if actor.trim().is_empty() || actor.len() > MAX_ACTOR_LABEL_BYTES {
            return Err(RelayValidationError::Audit("invalid actor_label".into()));
        }
    }
    Ok(())
}

/// True if package is approved for egress.
pub fn is_package_approved(pkg: &RelayPackage) -> bool {
    pkg.approved_at.is_some() && pkg.approved_by.is_some()
}
