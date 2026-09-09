//! v0.2 A4.1 — LAN pairing / bind / fail-closed Agent Engine access.

use openmesh_core::agent_engine::{authorize_agent_turn, AgentOrigin, AgentRequestContext};
use openmesh_core::lan::client::{ask_peer, health_check, LanClientAuth};
use openmesh_core::lan::contract::{LanBeacon, LAN_PROTOCOL};
use openmesh_core::lan::pairing::{LanCapability, LanRegistry, MemoryLanRegistry};
use openmesh_core::lan::server::{bind_http_listener, spawn_http_server, LanHttpIdentity};
use openmesh_core::lan::{start_lan_serve, stop_lan_serve, LanServeOptions, DEFAULT_LAN_HOST};
use openmesh_core::storage::init_project;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn temp_project(label: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "openmesh-lan-trust-{}-{}-{}",
        label,
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    init_project(&path).unwrap();
    path
}

fn spawn_with_registry(
    project: &str,
    registry: MemoryLanRegistry,
) -> (u16, Arc<AtomicBool>, thread::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let (listener, port) = bind_http_listener("127.0.0.1", 0).unwrap();
    let beacon = LanBeacon {
        protocol: LAN_PROTOCOL.into(),
        project_id: "proj-trust".into(),
        owner_label: "Host".into(),
        peer_id: "lan-host".into(),
        http_port: port,
        started_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };
    let identity = LanHttpIdentity::new(project, beacon, LanRegistry::memory(registry));
    let handle = spawn_http_server(listener, identity, stop.clone()).unwrap();
    thread::sleep(Duration::from_millis(80));
    (port, stop, handle)
}

#[test]
fn default_serve_does_not_bind_wildcard() {
    let project = temp_project("default-bind");
    let handle = start_lan_serve(
        &project,
        LanServeOptions {
            http_port: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(handle.status.http_host.as_deref(), Some(DEFAULT_LAN_HOST));
    assert!(!handle.status.expose_lan);
    let _ = stop_lan_serve();
}

#[test]
fn lan_exposure_requires_explicit_flag() {
    let project = temp_project("expose");
    let err = start_lan_serve(
        &project,
        LanServeOptions {
            http_host: "0.0.0.0".into(),
            expose_lan: false,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(err.to_string().contains("expose-lan"));
}

#[test]
fn unauthenticated_live_ask_is_rejected_before_engine() {
    let project = temp_project("unauth");
    let registry = MemoryLanRegistry::new();
    let (port, stop, handle) = spawn_with_registry(&project, registry);
    let err = ask_peer("127.0.0.1", port, "hello", None, None).unwrap_err();
    match err {
        openmesh_core::lan::LanClientError::Peer { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("missing_credential"));
            assert!(!body.contains("missing_api_key"));
        }
        other => panic!("{other}"),
    }
    health_check("127.0.0.1", port).unwrap();
    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn invalid_and_unknown_tokens_are_rejected() {
    let project = temp_project("bad-token");
    let registry = MemoryLanRegistry::new();
    let (port, stop, handle) = spawn_with_registry(&project, registry);
    let malformed = LanClientAuth::bearer("not-hex");
    let err = ask_peer("127.0.0.1", port, "hello", None, Some(&malformed)).unwrap_err();
    match err {
        openmesh_core::lan::LanClientError::Peer { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("malformed_credential") || body.contains("unknown_peer"));
            assert!(!body.contains("missing_api_key"));
        }
        other => panic!("{other}"),
    }
    let unknown = LanClientAuth::bearer("0123456789abcdef0123456789abcdef");
    let err = ask_peer("127.0.0.1", port, "hello", None, Some(&unknown)).unwrap_err();
    match err {
        openmesh_core::lan::LanClientError::Peer { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("unknown_peer"));
            assert!(!body.contains("missing_api_key"));
        }
        other => panic!("{other}"),
    }
    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn revoked_peer_is_rejected() {
    let project = temp_project("revoke");
    let registry = MemoryLanRegistry::new();
    let issued = registry
        .issue("alice", vec![LanCapability::LiveAsk])
        .unwrap();
    registry.revoke(&issued.peer.peer_id).unwrap();
    let (port, stop, handle) = spawn_with_registry(&project, registry);
    let auth = LanClientAuth::bearer(issued.token);
    let err = ask_peer("127.0.0.1", port, "hello", None, Some(&auth)).unwrap_err();
    match err {
        openmesh_core::lan::LanClientError::Peer { status, body } => {
            assert_eq!(status, 401);
            assert!(body.contains("revoked_peer"));
            assert!(!body.contains("missing_api_key"));
        }
        other => panic!("{other}"),
    }
    stop.store(true, Ordering::SeqCst);
    let _ = handle.join();
}

#[test]
fn valid_peer_reaches_lan_peer_policy_without_act_tools() {
    let project = temp_project("valid");
    let registry = MemoryLanRegistry::new();
    let issued = registry
        .issue("alice", vec![LanCapability::LiveAsk])
        .unwrap();
    registry
        .authenticate(
            Some(&format!("Bearer {}", issued.token)),
            LanCapability::LiveAsk,
        )
        .expect("paired token must authenticate");
    let policy = authorize_agent_turn(&AgentRequestContext::live_ask(
        AgentOrigin::LanPeer,
        &project,
    ))
    .unwrap();
    assert_eq!(policy.origin(), AgentOrigin::LanPeer);
    assert!(!policy.allows_mutating_tools());
    assert!(!policy.tool_allowlist().iter().any(|t| t == "propose_patch"));
}

#[test]
fn local_chat_and_continuity_origins_are_unchanged() {
    let chat =
        authorize_agent_turn(&AgentRequestContext::local_chat("/tmp/ws", Some("act"))).unwrap();
    assert_eq!(chat.origin(), AgentOrigin::LocalChat);
    assert!(chat.allows_mutating_tools());
    let continuity = authorize_agent_turn(&AgentRequestContext::live_ask(
        AgentOrigin::ContinuityQuery,
        "/tmp/ws",
    ))
    .unwrap();
    assert_eq!(continuity.origin(), AgentOrigin::ContinuityQuery);
    assert!(!continuity.allows_mutating_tools());
}
