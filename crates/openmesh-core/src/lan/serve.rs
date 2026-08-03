//! Combined LAN serve: beacon advertiser + HTTP server lifecycle.

use crate::lan::beacon::spawn_beacon_advertiser;
use crate::lan::contract::{
    LanBeacon, LanServeStatus, DEFAULT_UDP_PORT, LAN_PROTOCOL,
};
use crate::lan::server::{bind_http_listener, spawn_http_server, LanHttpIdentity};
use crate::profile::read_work_proxy_profile;
use crate::storage::{get_project_dir, read_project, Project};
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::Duration;
use thiserror::Error;

const LAN_STATUS_REL: &str = "lan/serve-status.json";

#[derive(Debug, Error)]
pub enum LanServeError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("already running")]
    AlreadyRunning,
    #[error("not running")]
    NotRunning,
    #[error("bind failed: {0}")]
    Bind(String),
    #[error("io: {0}")]
    Io(String),
}

struct ActiveServe {
    stop: Arc<AtomicBool>,
    http_join: Option<JoinHandle<()>>,
    beacon_join: Option<JoinHandle<()>>,
    status: LanServeStatus,
}

static ACTIVE: OnceLock<Mutex<Option<ActiveServe>>> = OnceLock::new();

fn active_slot() -> &'static Mutex<Option<ActiveServe>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

/// Handle returned to CLI/Desktop callers (status snapshot at start).
#[derive(Debug, Clone)]
pub struct LanServeHandle {
    pub status: LanServeStatus,
}

pub fn start_lan_serve(
    project_path: &str,
    http_host: &str,
    preferred_http_port: u16,
    udp_port: u16,
    owner_label_override: Option<&str>,
) -> Result<LanServeHandle, LanServeError> {
    let project: Project = read_project(project_path, "project.json")
        .ok_or(LanServeError::ProjectNotInitialized)?;

    {
        let guard = active_slot()
            .lock()
            .map_err(|e| LanServeError::Io(e.to_string()))?;
        if guard.as_ref().is_some_and(|a| a.status.running) {
            return Err(LanServeError::AlreadyRunning);
        }
    }
    // Disk may still say running after a crash; clear before binding.
    reconcile_persisted_status(project_path);

    let owner_label = owner_label_override
        .map(|s| s.to_string())
        .or_else(|| {
            read_work_proxy_profile(project_path)
                .ok()
                .map(|p| p.owner_label)
        })
        .unwrap_or_else(|| "local-operator".into());
    let peer_id = format!("lan-{}", project.id);
    let started_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    // preferred_http_port == 0 → bind ephemeral immediately; else try preferred then fallback.
    let (listener, http_port) =
        bind_http_listener(http_host, preferred_http_port)
            .map_err(|e| LanServeError::Bind(e.to_string()))?;

    let beacon = LanBeacon {
        protocol: LAN_PROTOCOL.into(),
        project_id: project.id.clone(),
        owner_label: owner_label.clone(),
        peer_id: peer_id.clone(),
        http_port,
        started_at: started_at.clone(),
    };

    let stop = Arc::new(AtomicBool::new(false));
    let identity = LanHttpIdentity {
        project_path: project_path.to_string(),
        beacon: beacon.clone(),
    };
    let http_join = spawn_http_server(listener, identity, stop.clone())
        .map_err(|e| LanServeError::Io(e.to_string()))?;

    let udp = if udp_port == 0 {
        DEFAULT_UDP_PORT
    } else {
        udp_port
    };
    let beacon_join = spawn_beacon_advertiser(beacon, udp, stop.clone())
        .map_err(|e| LanServeError::Io(e.to_string()))?;

    let status = LanServeStatus {
        running: true,
        protocol: LAN_PROTOCOL.into(),
        project_path: Some(project_path.to_string()),
        peer_id: Some(peer_id),
        owner_label: Some(owner_label),
        project_id: Some(project.id),
        http_host: Some(http_host.to_string()),
        http_port: Some(http_port),
        udp_port: Some(udp),
        started_at: Some(started_at),
        note: Some(
            "Trusted-LAN alpha. macOS may prompt for firewall on first bind. UDP broadcast may fail on some VPN interfaces — use --to host:port.".into(),
        ),
    };

    persist_status(project_path, &status)?;

    let mut guard = active_slot()
        .lock()
        .map_err(|e| LanServeError::Io(e.to_string()))?;
    *guard = Some(ActiveServe {
        stop,
        http_join: Some(http_join),
        beacon_join: Some(beacon_join),
        status: status.clone(),
    });

    Ok(LanServeHandle { status })
}

pub fn stop_lan_serve() -> Result<LanServeStatus, LanServeError> {
    let mut guard = active_slot()
        .lock()
        .map_err(|e| LanServeError::Io(e.to_string()))?;
    let Some(mut active) = guard.take() else {
        return Err(LanServeError::NotRunning);
    };
    active.stop.store(true, Ordering::SeqCst);
    if let Some(j) = active.http_join.take() {
        let _ = j.join();
    }
    if let Some(j) = active.beacon_join.take() {
        let _ = j.join();
    }
    let mut status = active.status;
    status.running = false;
    status.note = Some("stopped".into());
    if let Some(path) = status.project_path.clone() {
        let _ = persist_status(&path, &status);
    }
    Ok(status)
}

pub fn current_lan_serve_status() -> LanServeStatus {
    if let Ok(g) = active_slot().lock() {
        if let Some(a) = g.as_ref() {
            if a.status.running {
                return a.status.clone();
            }
        }
    }
    idle_status()
}

/// Read serve status for a project (process-local first, then on-disk snapshot).
///
/// `running: true` on disk is only trusted when this process holds an active serve
/// for the project. After crash/exit, stale disk status is cleared on read.
pub fn lan_serve_status_for_project(project_path: &str) -> LanServeStatus {
    if is_live_serve_for_project(project_path) {
        if let Ok(g) = active_slot().lock() {
            if let Some(a) = g.as_ref() {
                return a.status.clone();
            }
        }
    }
    reconcile_persisted_status(project_path).unwrap_or_else(idle_status)
}

fn status_path(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(LAN_STATUS_REL)
}

fn is_live_serve_for_project(project_path: &str) -> bool {
    active_slot()
        .lock()
        .ok()
        .and_then(|g| {
            g.as_ref().map(|a| {
                a.status.running
                    && a.status
                        .project_path
                        .as_deref()
                        .is_some_and(|p| p == project_path)
            })
        })
        .unwrap_or(false)
}

/// If disk claims `running` but nothing is listening on the recorded port, clear it.
///
/// Cross-process observers (CLI `lan status` while `lan serve` runs elsewhere) still
/// see `running: true` when the health probe succeeds.
fn reconcile_persisted_status(project_path: &str) -> Option<LanServeStatus> {
    let status = read_persisted_status(project_path)?;
    if !status.running || is_live_serve_for_project(project_path) {
        return Some(status);
    }
    if is_persisted_serve_alive(&status) {
        return Some(status);
    }
    let mut cleared = status;
    cleared.running = false;
    cleared.note = Some(
        "Stale serve status cleared (no listener on recorded host:port)".into(),
    );
    let _ = persist_status(project_path, &cleared);
    Some(cleared)
}

fn probe_host_for_liveness(host: &str) -> &str {
    match host {
        "0.0.0.0" | "::" | "[::]" => "127.0.0.1",
        other => other,
    }
}

fn is_persisted_serve_alive(status: &LanServeStatus) -> bool {
    let Some(host) = status.http_host.as_deref() else {
        return false;
    };
    let Some(port) = status.http_port else {
        return false;
    };
    if port == 0 {
        return false;
    }
    let probe = probe_host_for_liveness(host);
    let addr = format!("{probe}:{port}");
    TcpStream::connect_timeout(
        &match addr.parse() {
            Ok(a) => a,
            Err(_) => return false,
        },
        Duration::from_millis(350),
    )
    .is_ok()
}

fn persist_status(project_path: &str, status: &LanServeStatus) -> Result<(), LanServeError> {
    let path = status_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| LanServeError::Io(e.to_string()))?;
    }
    let mut json = serde_json::to_string_pretty(status).map_err(|e| LanServeError::Io(e.to_string()))?;
    json.push('\n');
    let temp = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|e| LanServeError::Io(e.to_string()))?;
        file.write_all(json.as_bytes())
            .map_err(|e| LanServeError::Io(e.to_string()))?;
        file.sync_all()
            .map_err(|e| LanServeError::Io(e.to_string()))?;
    }
    fs::rename(&temp, &path).map_err(|e| LanServeError::Io(e.to_string()))?;
    Ok(())
}

fn read_persisted_status(project_path: &str) -> Option<LanServeStatus> {
    let path = status_path(project_path);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn idle_status() -> LanServeStatus {
    LanServeStatus {
        running: false,
        protocol: LAN_PROTOCOL.into(),
        project_path: None,
        peer_id: None,
        owner_label: None,
        project_id: None,
        http_host: None,
        http_port: None,
        udp_port: None,
        started_at: None,
        note: Some("LAN serve is not running".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_project;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> PathBuf {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-lan-serve-status-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).expect("init");
        dir
    }

    #[test]
    fn stale_persisted_running_cleared_on_status_read() {
        let dir = temp_project();
        let path = dir.to_string_lossy().to_string();
        // Bind-then-drop so we know the port is closed when we probe.
        let free_port = {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind free");
            listener.local_addr().expect("addr").port()
        };
        let stale = LanServeStatus {
            running: true,
            protocol: LAN_PROTOCOL.into(),
            project_path: Some(path.clone()),
            peer_id: Some("lan-stale".into()),
            owner_label: Some("ghost".into()),
            project_id: Some("proj".into()),
            http_host: Some("127.0.0.1".into()),
            http_port: Some(free_port),
            udp_port: Some(41777),
            started_at: Some("2026-08-03T00:00:00Z".into()),
            note: Some("crashed mid-serve".into()),
        };
        persist_status(&path, &stale).expect("persist");

        let status = lan_serve_status_for_project(&path);
        assert!(!status.running, "status read must not trust dead process");
        assert!(
            status
                .note
                .as_deref()
                .unwrap_or("")
                .contains("Stale serve status cleared"),
            "note={:?}",
            status.note
        );

        let disk = read_persisted_status(&path).expect("disk");
        assert!(!disk.running, "disk must be rewritten stopped");
        let _ = fs::remove_dir_all(&dir);
    }
}
