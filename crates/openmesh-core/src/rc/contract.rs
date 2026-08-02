//! Pure wire contracts for 1.0 RC Program (0.1.21).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const RC_PROTOCOL_VERSION: &str = "1.0";
pub const RC_DIR: &str = "rc";
pub const MAX_CHECKS: usize = 64;
pub const MAX_MATRIX: usize = 64;
pub const MAX_ID_BYTES: usize = 128;
pub const MAX_TITLE_BYTES: usize = 256;
pub const MAX_DETAIL_BYTES: usize = 1024;
pub const MAX_LIMITATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RcSeverity {
    P0,
    P1,
    P2,
    P3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RcCheckStatus {
    Pass,
    Fail,
    Warn,
    Open,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RcCheckItem {
    pub id: String,
    pub title: String,
    pub severity: RcSeverity,
    pub status: RcCheckStatus,
    pub evidence: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RcRegressionRow {
    pub id: String,
    pub area: String,
    pub surface: String,
    pub status: RcCheckStatus,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RcFreezePolicy {
    /// Feature expansion frozen for RC window.
    pub features_frozen: bool,
    #[serde(default)]
    pub allowed: Vec<String>,
    #[serde(default)]
    pub forbidden: Vec<String>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RcPack {
    pub protocol_version: String,
    pub workspace_id: String,
    pub generated_at: String,
    /// True when no P0/P1 Fail checks.
    pub rc_ready: bool,
    pub p0_fail_count: u32,
    pub p1_fail_count: u32,
    pub open_count: u32,
    #[serde(default)]
    pub checks: Vec<RcCheckItem>,
    #[serde(default)]
    pub regression_matrix: Vec<RcRegressionRow>,
    pub freeze_policy: RcFreezePolicy,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RcValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("workspace_id empty")]
    EmptyWorkspace,
    #[error("bounds exceeded")]
    Bounds,
    #[error("invalid item")]
    InvalidItem,
    #[error("rc_ready inconsistent with P0/P1 fails")]
    ReadyMismatch,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("freeze policy must freeze features")]
    FreezeRequired,
}

pub fn validate_rc_pack(p: &RcPack) -> Result<(), RcValidationError> {
    if p.protocol_version != RC_PROTOCOL_VERSION {
        return Err(RcValidationError::UnsupportedProtocol {
            found: p.protocol_version.clone(),
        });
    }
    if p.workspace_id.trim().is_empty() {
        return Err(RcValidationError::EmptyWorkspace);
    }
    if p.checks.len() > MAX_CHECKS
        || p.regression_matrix.len() > MAX_MATRIX
        || p.limitations.len() > MAX_LIMITATIONS
    {
        return Err(RcValidationError::Bounds);
    }
    validate_utc_timestamp(&p.generated_at).map_err(RcValidationError::InvalidTimestamp)?;
    if !p.freeze_policy.features_frozen {
        return Err(RcValidationError::FreezeRequired);
    }
    let mut p0 = 0u32;
    let mut p1 = 0u32;
    let mut open = 0u32;
    for c in &p.checks {
        if c.id.trim().is_empty()
            || c.id.len() > MAX_ID_BYTES
            || c.title.trim().is_empty()
            || c.title.len() > MAX_TITLE_BYTES
            || c.evidence.len() > MAX_DETAIL_BYTES
        {
            return Err(RcValidationError::InvalidItem);
        }
        if matches!(c.status, RcCheckStatus::Open) {
            open += 1;
        }
        if matches!(c.status, RcCheckStatus::Fail) {
            match c.severity {
                RcSeverity::P0 => p0 += 1,
                RcSeverity::P1 => p1 += 1,
                _ => {}
            }
        }
    }
    for r in &p.regression_matrix {
        if r.id.trim().is_empty() || r.area.trim().is_empty() || r.surface.trim().is_empty() {
            return Err(RcValidationError::InvalidItem);
        }
    }
    if p.p0_fail_count != p0 || p.p1_fail_count != p1 || p.open_count != open {
        return Err(RcValidationError::ReadyMismatch);
    }
    if p.rc_ready != (p0 == 0 && p1 == 0) {
        return Err(RcValidationError::ReadyMismatch);
    }
    Ok(())
}
