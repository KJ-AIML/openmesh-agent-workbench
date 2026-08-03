//! UDP broadcast beacon advertise / listen.

use crate::lan::contract::{validate_lan_beacon, LanBeacon, DEFAULT_UDP_PORT};
use crate::lan::peer::{merge_peer, PeerTable};
use chrono::Utc;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BeaconListenError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("decode: {0}")]
    Decode(String),
}

pub fn encode_beacon(beacon: &LanBeacon) -> Result<Vec<u8>, String> {
    validate_lan_beacon(beacon).map_err(|e| e.to_string())?;
    serde_json::to_vec(beacon).map_err(|e| e.to_string())
}

pub fn decode_beacon(bytes: &[u8]) -> Result<LanBeacon, String> {
    let beacon: LanBeacon = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    validate_lan_beacon(&beacon).map_err(|e| e.to_string())?;
    Ok(beacon)
}

/// Spawn a thread that broadcasts `beacon` every ~2s until `stop` is set.
pub fn spawn_beacon_advertiser(
    beacon: LanBeacon,
    udp_port: u16,
    stop: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, io::Error> {
    let port = if udp_port == 0 {
        DEFAULT_UDP_PORT
    } else {
        udp_port
    };
    let sock = UdpSocket::bind("0.0.0.0:0")?;
    sock.set_broadcast(true)?;
    sock.set_write_timeout(Some(Duration::from_secs(2)))?;
    let payload = encode_beacon(&beacon).map_err(io::Error::other)?;
    let broadcast_addr: SocketAddr = format!("255.255.255.255:{port}")
        .parse()
        .map_err(io::Error::other)?;
    // Also try subnet-local broadcast on loopback-friendly 127.255.255.255 (ignored if fails).
    let loopback_bcast: SocketAddr = format!("127.255.255.255:{port}")
        .parse()
        .map_err(io::Error::other)?;

    Ok(thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            let _ = sock.send_to(&payload, broadcast_addr);
            let _ = sock.send_to(&payload, loopback_bcast);
            // short sleep slices so stop is responsive
            for _ in 0..20 {
                if stop.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    }))
}

/// Listen for beacons for `seconds`, merging into `table`. Returns peers seen.
pub fn listen_beacons(
    table: &PeerTable,
    udp_port: u16,
    seconds: u64,
    ignore_peer_id: Option<&str>,
) -> Result<Vec<crate::lan::contract::LanPeerInfo>, BeaconListenError> {
    let port = if udp_port == 0 {
        DEFAULT_UDP_PORT
    } else {
        udp_port
    };
    let sock = UdpSocket::bind(("0.0.0.0", port))?;
    sock.set_broadcast(true)?;
    sock.set_read_timeout(Some(Duration::from_millis(250)))?;

    let deadline = std::time::Instant::now() + Duration::from_secs(seconds.max(1));
    let mut buf = [0u8; 4096];
    while std::time::Instant::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                if let Ok(beacon) = decode_beacon(&buf[..n]) {
                    if ignore_peer_id.is_some_and(|id| id == beacon.peer_id) {
                        continue;
                    }
                    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
                    let host = addr.ip().to_string();
                    merge_peer(table, &beacon, &host, &now);
                }
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut => {
                continue;
            }
            Err(e) => return Err(BeaconListenError::Io(e)),
        }
    }
    Ok(table.snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::contract::LAN_PROTOCOL;

    #[test]
    fn beacon_encode_decode() {
        let b = LanBeacon {
            protocol: LAN_PROTOCOL.into(),
            project_id: "p1".into(),
            owner_label: "Ter".into(),
            peer_id: "ter".into(),
            http_port: 41778,
            started_at: "2026-08-03T12:00:00Z".into(),
        };
        let bytes = encode_beacon(&b).unwrap();
        let back = decode_beacon(&bytes).unwrap();
        assert_eq!(b, back);
    }
}
