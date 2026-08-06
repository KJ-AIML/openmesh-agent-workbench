//! LAN protocol contracts (pure validators).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Wire protocol id for UDP beacons and HTTP compatibility checks.
pub const LAN_PROTOCOL: &str = "openmesh-lan/0.1";
pub const DEFAULT_UDP_PORT: u16 = 41777;
pub const DEFAULT_HTTP_PORT: u16 = 41778;

const MAX_LABEL_BYTES: usize = 128;
const MAX_PEER_ID_BYTES: usize = 128;
const MAX_PROJECT_ID_BYTES: usize = 128;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LanProtocolError {
    #[error("invalid protocol: {0}")]
    Protocol(String),
    #[error("validation: {0}")]
    Validation(String),
}

/// UDP broadcast beacon payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanBeacon {
    pub protocol: String,
    pub project_id: String,
    pub owner_label: String,
    pub peer_id: String,
    pub http_port: u16,
    pub started_at: String,
}

/// Peer row for discover / UI tables.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPeerInfo {
    pub protocol: String,
    pub project_id: String,
    pub owner_label: String,
    pub peer_id: String,
    pub host: String,
    pub http_port: u16,
    pub started_at: String,
    pub last_seen_at: String,
    pub address: String,
}

impl LanPeerInfo {
    pub fn from_beacon(beacon: &LanBeacon, host: &str, last_seen_at: &str) -> Self {
        let address = format!("{host}:{}", beacon.http_port);
        Self {
            protocol: beacon.protocol.clone(),
            project_id: beacon.project_id.clone(),
            owner_label: beacon.owner_label.clone(),
            peer_id: beacon.peer_id.clone(),
            host: host.to_string(),
            http_port: beacon.http_port,
            started_at: beacon.started_at.clone(),
            last_seen_at: last_seen_at.to_string(),
            address,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanAskHttpBody {
    pub question: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanHealthResponse {
    pub ok: bool,
    pub protocol: String,
    pub peer_id: String,
    pub owner_label: String,
    pub project_id: String,
    pub http_port: u16,
}

/// Live / stale / unreachable presence from probing `GET /v1/health`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanPresenceState {
    /// Health probe succeeded — peer HTTP is reachable now.
    Live,
    /// Discovery saw the peer recently, but health probe failed (VPN/firewall gap).
    Stale,
    /// Health probe failed and no recent discovery signal.
    Unreachable,
    /// Not probed yet.
    Unknown,
}

/// Presence probe result for one host:port (UI green-dot row).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanPeerPresence {
    pub address: String,
    pub state: LanPresenceState,
    pub probed_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub health: Option<LanHealthResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Discovery/last-seen hint used to distinguish stale vs unreachable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
}

/// Human team-chat text message over LAN HTTP (trusted-LAN alpha).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LanChatMessage {
    pub protocol: String,
    pub message_id: String,
    pub from_peer_id: String,
    pub from_label: String,
    pub text: String,
    pub sent_at: String,
    /// Optional conversation key (defaults to peer pair address/id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

pub const LAN_CHAT_PROTOCOL: &str = "openmesh-lan-chat/0.1";
pub const MAX_CHAT_TEXT_BYTES: usize = 4000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanServeStatus {
    pub running: bool,
    pub protocol: String,
    pub project_path: Option<String>,
    pub peer_id: Option<String>,
    pub owner_label: Option<String>,
    pub project_id: Option<String>,
    pub http_host: Option<String>,
    pub http_port: Option<u16>,
    pub udp_port: Option<u16>,
    pub started_at: Option<String>,
    pub note: Option<String>,
}

pub fn validate_lan_beacon(b: &LanBeacon) -> Result<(), LanProtocolError> {
    if b.protocol != LAN_PROTOCOL {
        return Err(LanProtocolError::Protocol(b.protocol.clone()));
    }
    check_nonempty("projectId", &b.project_id, MAX_PROJECT_ID_BYTES)?;
    check_nonempty("ownerLabel", &b.owner_label, MAX_LABEL_BYTES)?;
    check_nonempty("peerId", &b.peer_id, MAX_PEER_ID_BYTES)?;
    if b.http_port == 0 {
        return Err(LanProtocolError::Validation("httpPort must be non-zero".into()));
    }
    if b.started_at.trim().is_empty() {
        return Err(LanProtocolError::Validation("startedAt required".into()));
    }
    Ok(())
}

fn check_nonempty(field: &str, value: &str, max: usize) -> Result<(), LanProtocolError> {
    let t = value.trim();
    if t.is_empty() {
        return Err(LanProtocolError::Validation(format!("{field} required")));
    }
    if t.len() > max {
        return Err(LanProtocolError::Validation(format!("{field} too long")));
    }
    if t.contains("..") || t.contains('/') || t.contains('\\') {
        return Err(LanProtocolError::Validation(format!(
            "{field} has invalid characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lan_beacon_roundtrip_json() {
        let b = LanBeacon {
            protocol: LAN_PROTOCOL.into(),
            project_id: "proj-1".into(),
            owner_label: "Ter".into(),
            peer_id: "peer-ter".into(),
            http_port: 41778,
            started_at: "2026-08-03T00:00:00Z".into(),
        };
        validate_lan_beacon(&b).unwrap();
        let raw = serde_json::to_string(&b).unwrap();
        let back: LanBeacon = serde_json::from_str(&raw).unwrap();
        assert_eq!(b, back);
    }

    #[test]
    fn lan_beacon_rejects_wrong_protocol() {
        let b = LanBeacon {
            protocol: "other/1".into(),
            project_id: "p".into(),
            owner_label: "o".into(),
            peer_id: "id".into(),
            http_port: 1,
            started_at: "t".into(),
        };
        assert!(matches!(
            validate_lan_beacon(&b),
            Err(LanProtocolError::Protocol(_))
        ));
    }
}
