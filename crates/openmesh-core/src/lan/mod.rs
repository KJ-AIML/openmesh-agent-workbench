//! Dev Track 0.1.22 — LAN Relay + Live Ask (trusted-LAN alpha).
//!
//! UDP beacon discovery + HTTP package transfer / live ask.
//! Alternate transport into relay quarantine receive and local proxy ask.
//! Does not replace filesystem `relay send --relay-root`.

pub mod ask;
pub mod beacon;
pub mod client;
pub mod contract;
pub mod peer;
pub mod serve;
pub mod server;

pub use ask::{answer_live_ask, LanAskError, LanAskRequest};
pub use beacon::{
    decode_beacon, encode_beacon, listen_beacons, spawn_beacon_advertiser, BeaconListenError,
};
pub use client::{
    ask_peer, health_check, parse_host_port, send_package_to_peer, LanClientError,
};
pub use contract::{
    validate_lan_beacon, LanAskHttpBody, LanBeacon, LanHealthResponse, LanPeerInfo,
    LanProtocolError, LanServeStatus, DEFAULT_HTTP_PORT, DEFAULT_UDP_PORT, LAN_PROTOCOL,
};
pub use peer::{merge_peer, peer_table_snapshot, PeerTable};
pub use serve::{
    current_lan_serve_status, lan_serve_status_for_project, start_lan_serve, stop_lan_serve,
    LanServeError, LanServeHandle,
};
pub use server::{bind_http_listener, spawn_http_server, LanServerError};
