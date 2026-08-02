//! Pure wire contracts for Enterprise Pilot Readiness (0.1.20).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const PILOT_PROTOCOL_VERSION: &str = "1.0";
pub const PILOT_DIR: &str = "pilot";
pub const MAX_CHECKS: usize = 64;
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_TITLE_BYTES: usize = 256;
pub const MAX_DETAIL_BYTES: usize = 1024;
pub const MAX_THREATS: usize = 32;
pub const MAX_RUNBOOK: usize = 32;
pub const MAX_LIMITATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PilotCheckStatus {
    Pass,
    Warn,
    Fail,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PilotCheckItem {
    pub id: String,
    pub title: String,
    pub status: PilotCheckStatus,
    /// Evidence path, command, or short observation (no secrets).
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreatNote {
    pub id: String,
    pub title: String,
    pub summary: String,
    /// Residual risk after mitigations in this pilot scope.
    pub residual: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunbookStep {
    pub id: String,
    pub title: String,
    pub command_or_action: String,
    pub purpose: String,
}

/// Full pilot readiness pack snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PilotPack {
    pub protocol_version: String,
    pub workspace_id: String,
    pub generated_at: String,
    /// True when no Fail checks (Warn allowed).
    pub pilot_ready: bool,
    pub pass_count: u32,
    pub warn_count: u32,
    pub fail_count: u32,
    #[serde(default)]
    pub checks: Vec<PilotCheckItem>,
    #[serde(default)]
    pub threat_notes: Vec<ThreatNote>,
    #[serde(default)]
    pub runbook: Vec<RunbookStep>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PilotValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("workspace_id empty")]
    EmptyWorkspace,
    #[error("bounds exceeded")]
    Bounds,
    #[error("invalid check")]
    InvalidCheck,
    #[error("counts inconsistent with checks")]
    CountMismatch,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
}

pub fn validate_pilot_pack(p: &PilotPack) -> Result<(), PilotValidationError> {
    if p.protocol_version != PILOT_PROTOCOL_VERSION {
        return Err(PilotValidationError::UnsupportedProtocol {
            found: p.protocol_version.clone(),
        });
    }
    if p.workspace_id.trim().is_empty() {
        return Err(PilotValidationError::EmptyWorkspace);
    }
    if p.checks.len() > MAX_CHECKS
        || p.threat_notes.len() > MAX_THREATS
        || p.runbook.len() > MAX_RUNBOOK
        || p.limitations.len() > MAX_LIMITATIONS
    {
        return Err(PilotValidationError::Bounds);
    }
    validate_utc_timestamp(&p.generated_at).map_err(PilotValidationError::InvalidTimestamp)?;
    let mut pass = 0u32;
    let mut warn = 0u32;
    let mut fail = 0u32;
    for c in &p.checks {
        if c.id.trim().is_empty()
            || c.id.len() > MAX_ID_BYTES
            || c.title.trim().is_empty()
            || c.title.len() > MAX_TITLE_BYTES
            || c.evidence.len() > MAX_DETAIL_BYTES
        {
            return Err(PilotValidationError::InvalidCheck);
        }
        if let Some(d) = &c.detail {
            if d.len() > MAX_DETAIL_BYTES {
                return Err(PilotValidationError::InvalidCheck);
            }
        }
        match c.status {
            PilotCheckStatus::Pass => pass += 1,
            PilotCheckStatus::Warn => warn += 1,
            PilotCheckStatus::Fail => fail += 1,
            PilotCheckStatus::NotApplicable => {}
        }
    }
    if pass != p.pass_count || warn != p.warn_count || fail != p.fail_count {
        return Err(PilotValidationError::CountMismatch);
    }
    if p.pilot_ready != (fail == 0) {
        return Err(PilotValidationError::CountMismatch);
    }
    Ok(())
}
