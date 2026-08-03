//! Hand-rolled HTTP/1.1 server for LAN relay receive + live ask.

use crate::authority_policy::FreshnessTier;
use crate::lan::ask::{answer_live_ask, LanAskRequest};
use crate::lan::contract::{
    LanAskHttpBody, LanBeacon, LanHealthResponse, LAN_PROTOCOL,
};
use crate::relay::contract::RelayPackage;
use crate::relay::transport::receive_package_payload;
use chrono::Utc;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LanServerError {
    #[error("bind failed: {0}")]
    Bind(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct LanHttpIdentity {
    pub project_path: String,
    pub beacon: LanBeacon,
}

/// Bind HTTP listener on `preferred_port`, falling back to ephemeral if busy.
pub fn bind_http_listener(
    host: &str,
    preferred_port: u16,
) -> Result<(TcpListener, u16), LanServerError> {
    if preferred_port == 0 {
        let ephemeral = format!("{host}:0");
        let l = TcpListener::bind(&ephemeral).map_err(|e| LanServerError::Bind(e.to_string()))?;
        let port = l
            .local_addr()
            .map(|a| a.port())
            .map_err(|e| LanServerError::Bind(e.to_string()))?;
        return Ok((l, port));
    }
    let preferred = format!("{host}:{preferred_port}");
    match TcpListener::bind(&preferred) {
        Ok(l) => {
            let port = l.local_addr().map(|a| a.port()).unwrap_or(preferred_port);
            Ok((l, port))
        }
        Err(_) => {
            let ephemeral = format!("{host}:0");
            let l = TcpListener::bind(&ephemeral).map_err(|e| LanServerError::Bind(e.to_string()))?;
            let port = l
                .local_addr()
                .map(|a| a.port())
                .map_err(|e| LanServerError::Bind(e.to_string()))?;
            Ok((l, port))
        }
    }
}

pub fn spawn_http_server(
    listener: TcpListener,
    identity: LanHttpIdentity,
    stop: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, LanServerError> {
    listener.set_nonblocking(true)?;
    Ok(thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, addr)) => {
                    let id = identity.clone();
                    let _ = thread::spawn(move || {
                        let _ = handle_connection(stream, addr, &id);
                    });
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(50));
                }
            }
        }
    }))
}

fn handle_connection(
    mut stream: TcpStream,
    _addr: SocketAddr,
    identity: &LanHttpIdentity,
) -> Result<(), String> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));

    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0usize;
    loop {
        match stream.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                if total >= buf.len() {
                    // grow once for larger relay packages
                    if buf.len() < 4 * 1024 * 1024 {
                        buf.resize(buf.len() * 2, 0);
                    } else {
                        break;
                    }
                }
                if let Some(header_end) = find_header_end(&buf[..total]) {
                    let (method, path, content_length) = parse_request_line_and_length(&buf[..header_end])?;
                    let body_start = header_end;
                    while total < body_start + content_length {
                        if total >= buf.len() {
                            if buf.len() < 4 * 1024 * 1024 {
                                buf.resize(buf.len() * 2, 0);
                            } else {
                                break;
                            }
                        }
                        match stream.read(&mut buf[total..]) {
                            Ok(0) => break,
                            Ok(n) => total += n,
                            Err(_) => break,
                        }
                    }
                    let body = if content_length > 0 {
                        &buf[body_start..body_start + content_length.min(total - body_start)]
                    } else {
                        &[][..]
                    };
                    let (status, resp_body, content_type) =
                        route_request(&method, &path, body, identity);
                    return write_response(&mut stream, status, content_type, &resp_body);
                }
                if total > 64 * 1024 && find_header_end(&buf[..total]).is_none() {
                    return write_response(
                        &mut stream,
                        400,
                        "text/plain",
                        b"bad request headers",
                    );
                }
            }
            Err(_) => break,
        }
    }
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| i + 4)
}

fn parse_request_line_and_length(header_bytes: &[u8]) -> Result<(String, String, usize), String> {
    let text = std::str::from_utf8(header_bytes).map_err(|e| e.to_string())?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("").to_string();
    let mut content_length = 0usize;
    for line in lines {
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse().unwrap_or(0);
        }
    }
    Ok((method, path, content_length))
}

fn route_request(
    method: &str,
    path: &str,
    body: &[u8],
    identity: &LanHttpIdentity,
) -> (u16, Vec<u8>, &'static str) {
    match (method, path) {
        ("GET", "/v1/health") => {
            let resp = LanHealthResponse {
                ok: true,
                protocol: LAN_PROTOCOL.into(),
                peer_id: identity.beacon.peer_id.clone(),
                owner_label: identity.beacon.owner_label.clone(),
                project_id: identity.beacon.project_id.clone(),
                http_port: identity.beacon.http_port,
            };
            json_ok(&resp)
        }
        ("POST", "/v1/relay/package") => handle_relay_package(body, identity),
        ("POST", "/v1/mesh/ask") => handle_mesh_ask(body, identity),
        _ => (404, b"{\"error\":\"not found\"}".to_vec(), "application/json"),
    }
}

fn handle_relay_package(body: &[u8], identity: &LanHttpIdentity) -> (u16, Vec<u8>, &'static str) {
    let pkg: RelayPackage = match serde_json::from_slice(body) {
        Ok(p) => p,
        Err(e) => {
            return (
                400,
                format!(r#"{{"error":"invalid package: {e}"}}"#).into_bytes(),
                "application/json",
            );
        }
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    match receive_package_payload(
        &identity.project_path,
        &pkg,
        &now,
        Some("lan-peer"),
        &format!("received via LAN HTTP from peer {}", identity.beacon.peer_id),
    ) {
        Ok(stored) => json_ok(&serde_json::json!({
            "ok": true,
            "packageId": stored.package_id,
            "quarantine": "relay/received",
        })),
        Err(e) => {
            let code = if e.to_string().contains("already present") {
                409
            } else {
                400
            };
            (
                code,
                format!(r#"{{"error":"{e}"}}"#).into_bytes(),
                "application/json",
            )
        }
    }
}

fn handle_mesh_ask(body: &[u8], identity: &LanHttpIdentity) -> (u16, Vec<u8>, &'static str) {
    let req_body: LanAskHttpBody = match serde_json::from_slice(body) {
        Ok(b) => b,
        Err(e) => {
            return (
                400,
                format!(r#"{{"error":"invalid ask body: {e}"}}"#).into_bytes(),
                "application/json",
            );
        }
    };
    let tier = match req_body.tier.as_deref() {
        None | Some("standard") | Some("Standard") => FreshnessTier::Standard,
        Some("low-impact") | Some("LowImpact") => FreshnessTier::LowImpact,
        Some("critical") | Some("Critical") => FreshnessTier::Critical,
        Some(other) => {
            return (
                400,
                format!(r#"{{"error":"unknown tier: {other}"}}"#).into_bytes(),
                "application/json",
            );
        }
    };
    let now = Utc::now();
    let query_id = format!("lan-ask-{}", now.format("%Y%m%dT%H%M%SZ"));
    let req = LanAskRequest {
        question: req_body.question,
        tier,
        query_id,
    };
    match answer_live_ask(&identity.project_path, &req) {
        Ok(answer) => json_ok(&answer),
        Err(e) => (
            e.http_status(),
            e.to_json_body().into_bytes(),
            "application/json",
        ),
    }
}

fn json_ok<T: serde::Serialize>(value: &T) -> (u16, Vec<u8>, &'static str) {
    match serde_json::to_vec(value) {
        Ok(bytes) => (200, bytes, "application/json"),
        Err(e) => (
            500,
            format!(r#"{{"error":"{e}"}}"#).into_bytes(),
            "application/json",
        ),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(header.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(body).map_err(|e| e.to_string())?;
    let _ = stream.flush();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::client::{ask_peer, health_check, send_package_to_peer};
    use crate::lan::contract::DEFAULT_HTTP_PORT;
    use crate::mesh::{MeshEnvelope, MeshEvidenceItem, MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax};
    use crate::relay::contract::{RelayPackage, RelayPolicySnapshot, RELAY_PACKAGE_PROTOCOL_VERSION};
    use crate::storage::init_project;
    use std::sync::atomic::AtomicBool;

    fn temp_project(label: &str) -> String {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-lan-server-{}-{}-{}",
            label,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn lan_http_health_receive_and_ask_loopback() {
        let project = temp_project("srv");
        let stop = Arc::new(AtomicBool::new(false));
        let (listener, port) = bind_http_listener("127.0.0.1", DEFAULT_HTTP_PORT).unwrap();
        let beacon = LanBeacon {
            protocol: LAN_PROTOCOL.into(),
            project_id: "proj-lan".into(),
            owner_label: "Server".into(),
            peer_id: "server-peer".into(),
            http_port: port,
            started_at: "2026-08-03T00:00:00Z".into(),
        };
        let identity = LanHttpIdentity {
            project_path: project.clone(),
            beacon: beacon.clone(),
        };
        let handle = spawn_http_server(listener, identity, stop.clone()).unwrap();
        // give accept loop a tick
        thread::sleep(Duration::from_millis(80));

        let health = health_check("127.0.0.1", port).unwrap();
        assert!(health.ok);
        assert_eq!(health.peer_id, "server-peer");

        use crate::domain::{CatchUpWindow, EvidenceRef};
        use crate::mesh::MESH_ENVELOPE_PROTOCOL_VERSION;
        let env = MeshEnvelope {
            protocol_version: MESH_ENVELOPE_PROTOCOL_VERSION.into(),
            envelope_id: "env-lan-1".into(),
            from_peer: MeshPeerRef {
                label: "Sender".into(),
                proxy_profile_id: None,
                workspace_id: Some("ws-a".into()),
            },
            to_peer: None,
            generated_at: "2026-08-03T00:00:00Z".into(),
            window: Some(CatchUpWindow {
                since: "2026-08-01T00:00:00Z".into(),
                until: "2026-08-03T00:00:00Z".into(),
            }),
            evidence_items: vec![MeshEvidenceItem {
                summary: "LAN package evidence".into(),
                evidence_refs: vec![EvidenceRef::FilePath("README.md".into())],
                source_kind: MeshEvidenceSourceKind::WorkEvent,
                source_id: "evt-1".into(),
            }],
            handoff_ids: vec![],
            limitations: vec![],
            sensitivity_max: MeshSensitivityMax::Team,
        };
        let pkg = RelayPackage {
            protocol_version: RELAY_PACKAGE_PROTOCOL_VERSION.into(),
            package_id: "pkg-lan-1".into(),
            workspace_id: "ws-a".into(),
            generated_at: "2026-08-03T00:00:00Z".into(),
            sensitivity_max: MeshSensitivityMax::Team,
            envelopes: vec![env],
            handoff_ids: vec![],
            policy: RelayPolicySnapshot::default(),
            limitations: vec![],
            content_hash: Some("hash-lan".into()),
            approved_at: Some("2026-08-03T00:01:00Z".into()),
            approved_by: Some("tester".into()),
        };
        send_package_to_peer("127.0.0.1", port, &pkg).unwrap();
        assert!(std::path::Path::new(&project)
            .join(".openmesh/relay/received/pkg-lan-1.json")
            .exists());

        // Without a configured peer API key, live ask must fail closed (not LocalScaffold).
        let ask_err = ask_peer("127.0.0.1", port, "What is in progress?", Some("low-impact"));
        match ask_err {
            Err(crate::lan::client::LanClientError::Peer { status, body }) => {
                assert_eq!(status, 503);
                assert!(
                    body.contains("missing_api_key") || body.contains("API key"),
                    "body={body}"
                );
            }
            Ok(answer) => {
                // If the host unexpectedly has a key configured, still assert read-only shape.
                assert!(answer.read_only);
                assert!(!answer.answer_text.is_empty());
                assert!(
                    !answer.answer_text.contains("local-scaffold"),
                    "must not return scaffold paste"
                );
            }
            Err(other) => panic!("unexpected ask error: {other}"),
        }

        stop.store(true, Ordering::SeqCst);
        let _ = handle.join();
    }
}
