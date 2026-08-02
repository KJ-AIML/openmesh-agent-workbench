//! Admin audit log (append-only JSONL under trust-admin).

use crate::domain::validate_utc_timestamp;
use crate::storage::get_project_dir;
use crate::trust_admin::contract::TRUST_ADMIN_DIR;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

const AUDIT_FILE: &str = "audit.jsonl";
const MAX_AUDIT_READ: usize = 500;
const MAX_DETAIL_BYTES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AuditAction {
    PolicyInit,
    PolicyUpdate,
    AllowlistAdd,
    AllowlistRemove,
    QueryAllowed,
    QueryDenied,
    AuditList,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdminAuditEvent {
    pub event_id: String,
    pub team_id: String,
    pub actor_member_id: String,
    pub action: AuditAction,
    pub detail: String,
    pub at: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("io failed")]
    Io,
    #[error("invalid event: {0}")]
    Invalid(String),
    #[error("malformed audit line")]
    Malformed,
}

fn audit_path(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(TRUST_ADMIN_DIR).join(AUDIT_FILE)
}

pub fn append_audit_event(project_path: &str, event: &AdminAuditEvent) -> Result<(), AuditError> {
    if event.event_id.trim().is_empty() || event.team_id.trim().is_empty() {
        return Err(AuditError::Invalid("ids".into()));
    }
    if event.actor_member_id.trim().is_empty() {
        return Err(AuditError::Invalid("actor".into()));
    }
    if event.detail.len() > MAX_DETAIL_BYTES {
        return Err(AuditError::Invalid("detail too long".into()));
    }
    validate_utc_timestamp(&event.at).map_err(|e| AuditError::Invalid(e))?;
    let path = audit_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| AuditError::Io)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|_| AuditError::Io)?;
    let line = serde_json::to_string(event).map_err(|_| AuditError::Malformed)?;
    writeln!(f, "{line}").map_err(|_| AuditError::Io)?;
    f.sync_all().map_err(|_| AuditError::Io)?;
    Ok(())
}

/// List newest-first audit events (capped).
pub fn list_audit_events(
    project_path: &str,
    limit: Option<usize>,
) -> Result<Vec<AdminAuditEvent>, AuditError> {
    let path = audit_path(project_path);
    if !path.exists() {
        return Ok(vec![]);
    }
    let f = fs::File::open(&path).map_err(|_| AuditError::Io)?;
    let reader = BufReader::new(f);
    let mut events = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|_| AuditError::Io)?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let ev: AdminAuditEvent = serde_json::from_str(line).map_err(|_| AuditError::Malformed)?;
        events.push(ev);
    }
    events.reverse();
    let cap = limit.unwrap_or(50).min(MAX_AUDIT_READ);
    events.truncate(cap);
    Ok(events)
}
