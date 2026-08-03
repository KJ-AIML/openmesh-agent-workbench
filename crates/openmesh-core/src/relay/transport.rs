//! Dev Track 0.1.11 Checkpoint E — filesystem relay-root transport.

use crate::relay::approve::{approved_package_path, read_approved_package, RelayApproveError};
use crate::relay::audit::{append_audit_event, make_audit_event, RelayAuditError};
use crate::relay::contract::{
    is_package_approved, validate_package_id_for_storage, validate_relay_package, RelayAuditKind,
    RelayPackage, RELAY_RECEIVED_DIR, RELAY_SENT_DIR,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

const TEMP: &str = "relay-transport-tmp";

#[derive(Debug, thiserror::Error)]
pub enum RelayTransportError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("package not approved for egress")]
    NotApproved,
    #[error("approve error: {0}")]
    Approve(#[from] RelayApproveError),
    #[error("audit error: {0}")]
    Audit(#[from] RelayAuditError),
    #[error("validation: {0}")]
    Validation(String),
    #[error("relay root missing or not a directory")]
    InvalidRelayRoot,
    #[error("package already present at destination")]
    AlreadyExists,
    #[error("package not found at relay root")]
    NotFound,
    #[error("io failed")]
    Io,
}

pub fn sent_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RELAY_SENT_DIR)
}

pub fn received_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(RELAY_RECEIVED_DIR)
}

/// Drop directory under relay root for outbound packages.
pub fn relay_root_drop_dir(relay_root: &Path) -> PathBuf {
    relay_root.join("drop")
}

/// Send an approved package to a filesystem relay root drop directory.
pub fn send_package_to_relay_root(
    project_path: &str,
    package_id: &str,
    relay_root: &Path,
    sent_at: &str,
    actor_label: Option<&str>,
) -> Result<RelayPackage, RelayTransportError> {
    let _ = load_project(project_path)?;
    if !relay_root.is_dir() {
        // create drop tree if parent exists
        fs::create_dir_all(relay_root_drop_dir(relay_root)).map_err(|_| RelayTransportError::Io)?;
    }
    let pkg = read_approved_package(project_path, package_id)?;
    if !is_package_approved(&pkg) {
        return Err(RelayTransportError::NotApproved);
    }

    let drop_dir = relay_root_drop_dir(relay_root);
    fs::create_dir_all(&drop_dir).map_err(|_| RelayTransportError::Io)?;
    let dest = drop_dir.join(format!("{package_id}.json"));
    if dest.exists() {
        return Err(RelayTransportError::AlreadyExists);
    }
    write_json_atomic(&dest, &pkg)?;

    // Receipt in sent/
    fs::create_dir_all(sent_dir(project_path)).map_err(|_| RelayTransportError::Io)?;
    let receipt = sent_dir(project_path).join(format!("{package_id}.json"));
    write_json_atomic(&receipt, &pkg)?;

    let audit = make_audit_event(
        format!("audit-sent-{package_id}"),
        package_id,
        RelayAuditKind::Sent,
        sent_at,
        format!("sent to relay root {}", relay_root.display()),
        actor_label.map(str::to_string),
        Some(pkg.sensitivity_max),
    );
    append_audit_event(project_path, &audit)?;
    Ok(pkg)
}

/// Receive a package from relay root drop into project received/.
pub fn receive_package_from_relay_root(
    project_path: &str,
    package_id: &str,
    relay_root: &Path,
    received_at: &str,
    actor_label: Option<&str>,
) -> Result<RelayPackage, RelayTransportError> {
    let _ = load_project(project_path)?;
    validate_package_id_for_storage(package_id)
        .map_err(|e| RelayTransportError::Validation(e.to_string()))?;
    let src = relay_root_drop_dir(relay_root).join(format!("{package_id}.json"));
    if !src.exists() {
        return Err(RelayTransportError::NotFound);
    }
    let raw = fs::read_to_string(&src).map_err(|_| RelayTransportError::Io)?;
    let pkg: RelayPackage = serde_json::from_str(&raw).map_err(|_| RelayTransportError::Io)?;
    validate_relay_package(&pkg).map_err(|e| RelayTransportError::Validation(e.to_string()))?;

    fs::create_dir_all(received_dir(project_path)).map_err(|_| RelayTransportError::Io)?;
    let dest = received_dir(project_path).join(format!("{package_id}.json"));
    if dest.exists() {
        return Err(RelayTransportError::AlreadyExists);
    }
    write_json_atomic(&dest, &pkg)?;

    let audit = make_audit_event(
        format!("audit-received-{package_id}"),
        package_id,
        RelayAuditKind::Received,
        received_at,
        format!("received from relay root {}", relay_root.display()),
        actor_label.map(str::to_string),
        Some(pkg.sensitivity_max),
    );
    append_audit_event(project_path, &audit)?;
    Ok(pkg)
}

pub fn read_received_package(
    project_path: &str,
    package_id: &str,
) -> Result<RelayPackage, RelayTransportError> {
    let _ = load_project(project_path)?;
    let path = received_dir(project_path).join(format!("{package_id}.json"));
    if !path.exists() {
        return Err(RelayTransportError::NotFound);
    }
    let raw = fs::read_to_string(&path).map_err(|_| RelayTransportError::Io)?;
    let pkg: RelayPackage = serde_json::from_str(&raw).map_err(|_| RelayTransportError::Io)?;
    validate_relay_package(&pkg).map_err(|e| RelayTransportError::Validation(e.to_string()))?;
    Ok(pkg)
}

/// Quarantine an already-decoded package into `relay/received/` (LAN / alternate transports).
pub fn receive_package_payload(
    project_path: &str,
    pkg: &RelayPackage,
    received_at: &str,
    actor_label: Option<&str>,
    source_detail: &str,
) -> Result<RelayPackage, RelayTransportError> {
    let _ = load_project(project_path)?;
    validate_relay_package(pkg).map_err(|e| RelayTransportError::Validation(e.to_string()))?;
    validate_package_id_for_storage(&pkg.package_id)
        .map_err(|e| RelayTransportError::Validation(e.to_string()))?;

    fs::create_dir_all(received_dir(project_path)).map_err(|_| RelayTransportError::Io)?;
    let dest = received_dir(project_path).join(format!("{}.json", pkg.package_id));
    if dest.exists() {
        return Err(RelayTransportError::AlreadyExists);
    }
    write_json_atomic(&dest, pkg)?;

    let detail = if source_detail.len() > 480 {
        format!("{}…", &source_detail[..480])
    } else {
        source_detail.to_string()
    };
    let audit = make_audit_event(
        format!("audit-received-{}", pkg.package_id),
        &pkg.package_id,
        RelayAuditKind::Received,
        received_at,
        detail,
        actor_label.map(str::to_string),
        Some(pkg.sensitivity_max),
    );
    append_audit_event(project_path, &audit)?;
    Ok(pkg.clone())
}

fn load_project(project_path: &str) -> Result<Project, RelayTransportError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(RelayTransportError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), RelayTransportError> {
    let parent = path.parent().ok_or(RelayTransportError::Io)?;
    fs::create_dir_all(parent).map_err(|_| RelayTransportError::Io)?;
    let temp = path.with_extension(TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| RelayTransportError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| RelayTransportError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| RelayTransportError::Io)?;
        file.sync_all().map_err(|_| RelayTransportError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| RelayTransportError::Io)
}

// silence unused import of approved_package_path if not used
#[allow(dead_code)]
fn _approved_path(project_path: &str, id: &str) -> PathBuf {
    approved_package_path(project_path, id)
}
