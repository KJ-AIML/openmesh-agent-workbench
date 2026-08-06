//! HTTP client for LAN peer send / ask / health / presence / chat.

use crate::lan::contract::{
    LanAskHttpBody, LanChatMessage, LanHealthResponse, LanPeerPresence, LanPresenceState,
};
use crate::mesh::query::MeshRemoteQueryAnswer;
use crate::relay::contract::RelayPackage;
use chrono::{DateTime, Utc};
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LanClientError {
    #[error("invalid address: {0}")]
    Address(String),
    #[error("http: {0}")]
    Http(String),
    #[error("peer error ({status}): {body}")]
    Peer { status: u16, body: String },
    #[error("decode: {0}")]
    Decode(String),
}

pub fn parse_host_port(to: &str) -> Result<(String, u16), LanClientError> {
    let t = to.trim();
    if let Some((host, port_s)) = t.rsplit_once(':') {
        let host = host.trim().trim_start_matches('[').trim_end_matches(']');
        if host.is_empty() {
            return Err(LanClientError::Address(to.into()));
        }
        let port: u16 = port_s
            .trim()
            .parse()
            .map_err(|_| LanClientError::Address(to.into()))?;
        if port == 0 {
            return Err(LanClientError::Address(to.into()));
        }
        Ok((host.to_string(), port))
    } else {
        Err(LanClientError::Address(format!(
            "expected host:port, got {to}"
        )))
    }
}

fn blocking_client() -> Result<reqwest::blocking::Client, LanClientError> {
    // Live Agent Engine asks can exceed a short HTTP timeout.
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| LanClientError::Http(e.to_string()))
}

fn presence_client() -> Result<reqwest::blocking::Client, LanClientError> {
    // Presence probes must fail fast so the LAN tab stays responsive.
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .connect_timeout(std::time::Duration::from_secs(1))
        .build()
        .map_err(|e| LanClientError::Http(e.to_string()))
}

pub fn health_check(host: &str, port: u16) -> Result<LanHealthResponse, LanClientError> {
    let url = format!("http://{host}:{port}/v1/health");
    let client = blocking_client()?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

/// Fast health probe for UI presence (2s timeout).
pub fn health_check_quick(host: &str, port: u16) -> Result<LanHealthResponse, LanClientError> {
    let url = format!("http://{host}:{port}/v1/health");
    let client = presence_client()?;
    let resp = client
        .get(&url)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

/// Seconds after discovery `lastSeenAt` where a failed health still counts as stale.
pub const PRESENCE_STALE_WINDOW_SECS: i64 = 90;

fn last_seen_is_recent(last_seen_at: Option<&str>, now: DateTime<Utc>) -> bool {
    let Some(raw) = last_seen_at.map(str::trim).filter(|s| !s.is_empty()) else {
        return false;
    };
    let Ok(seen) = DateTime::parse_from_rfc3339(raw) else {
        return false;
    };
    let seen_utc = seen.with_timezone(&Utc);
    (now - seen_utc).num_seconds() <= PRESENCE_STALE_WINDOW_SECS
}

/// Probe one `host:port` for green-dot style presence.
pub fn probe_presence(
    address: &str,
    last_seen_at: Option<&str>,
) -> Result<LanPeerPresence, LanClientError> {
    let (host, port) = parse_host_port(address)?;
    let now = Utc::now();
    let probed_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let started = Instant::now();
    match health_check_quick(&host, port) {
        Ok(health) => Ok(LanPeerPresence {
            address: format!("{host}:{port}"),
            state: LanPresenceState::Live,
            probed_at,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            health: Some(health),
            error: None,
            last_seen_at: last_seen_at.map(|s| s.to_string()),
        }),
        Err(e) => {
            let state = if last_seen_is_recent(last_seen_at, now) {
                LanPresenceState::Stale
            } else {
                LanPresenceState::Unreachable
            };
            Ok(LanPeerPresence {
                address: format!("{host}:{port}"),
                state,
                probed_at,
                latency_ms: Some(started.elapsed().as_millis() as u64),
                health: None,
                error: Some(e.to_string()),
                last_seen_at: last_seen_at.map(|s| s.to_string()),
            })
        }
    }
}

/// Probe many addresses. Failures become unreachable/stale rows (never hard-fail the batch).
pub fn probe_presence_many(
    targets: &[(String, Option<String>)],
) -> Vec<LanPeerPresence> {
    let mut out = Vec::with_capacity(targets.len());
    for (address, last_seen) in targets {
        match probe_presence(address, last_seen.as_deref()) {
            Ok(row) => out.push(row),
            Err(e) => {
                let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                out.push(LanPeerPresence {
                    address: address.clone(),
                    state: LanPresenceState::Unreachable,
                    probed_at: now,
                    latency_ms: None,
                    health: None,
                    error: Some(e.to_string()),
                    last_seen_at: last_seen.clone(),
                });
            }
        }
    }
    out
}

/// Deliver a human chat message to a LAN peer (`POST /v1/chat/message`).
pub fn send_chat_message(
    host: &str,
    port: u16,
    message: &LanChatMessage,
) -> Result<serde_json::Value, LanClientError> {
    let url = format!("http://{host}:{port}/v1/chat/message");
    let client = blocking_client()?;
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(message)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

pub fn send_package_to_peer(
    host: &str,
    port: u16,
    package: &RelayPackage,
) -> Result<serde_json::Value, LanClientError> {
    let url = format!("http://{host}:{port}/v1/relay/package");
    let client = blocking_client()?;
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(package)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

pub fn ask_peer(
    host: &str,
    port: u16,
    question: &str,
    tier: Option<&str>,
) -> Result<MeshRemoteQueryAnswer, LanClientError> {
    let url = format!("http://{host}:{port}/v1/mesh/ask");
    let body = LanAskHttpBody {
        question: question.to_string(),
        tier: tier.map(|s| s.to_string()),
    };
    let client = blocking_client()?;
    let resp = client
        .post(&url)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| LanClientError::Http(e.to_string()))?;
    let status = resp.status().as_u16();
    let text = resp.text().map_err(|e| LanClientError::Http(e.to_string()))?;
    if !(200..300).contains(&status) {
        return Err(LanClientError::Peer {
            status,
            body: text,
        });
    }
    serde_json::from_str(&text).map_err(|e| LanClientError::Decode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::contract::{LanBeacon, LAN_PROTOCOL};
    use crate::lan::server::{bind_http_listener, spawn_http_server, LanHttpIdentity};
    use crate::storage::init_project;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn parse_host_port_ok() {
        let (h, p) = parse_host_port("127.0.0.1:41778").unwrap();
        assert_eq!(h, "127.0.0.1");
        assert_eq!(p, 41778);
    }

    #[test]
    fn parse_host_port_rejects_missing_port() {
        assert!(parse_host_port("127.0.0.1").is_err());
    }

    #[test]
    fn probe_presence_live_on_loopback() {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-presence-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let project = dir.to_string_lossy().to_string();
        init_project(&project).unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let (listener, port) = bind_http_listener("127.0.0.1", 0).unwrap();
        let beacon = LanBeacon {
            protocol: LAN_PROTOCOL.into(),
            project_id: "proj-presence".into(),
            owner_label: "Presence".into(),
            peer_id: "lan-presence".into(),
            http_port: port,
            started_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        };
        let identity = LanHttpIdentity {
            project_path: project,
            beacon,
        };
        let handle = spawn_http_server(listener, identity, stop.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(80));

        let addr = format!("127.0.0.1:{port}");
        let row = probe_presence(&addr, None).unwrap();
        assert_eq!(row.state, LanPresenceState::Live);
        assert!(row.health.is_some());
        assert!(row.error.is_none());

        stop.store(true, Ordering::SeqCst);
        let _ = handle.join();
    }

    #[test]
    fn probe_presence_unreachable_without_listener() {
        // High unused port — should fail fast as unreachable.
        let row = probe_presence("127.0.0.1:1", None).unwrap();
        assert_eq!(row.state, LanPresenceState::Unreachable);
        assert!(row.error.is_some());
    }

    #[test]
    fn probe_presence_stale_when_recent_last_seen() {
        let recent = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let row = probe_presence("127.0.0.1:1", Some(&recent)).unwrap();
        assert_eq!(row.state, LanPresenceState::Stale);
    }
}
