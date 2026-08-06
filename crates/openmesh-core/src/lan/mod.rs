//! Dev Track 0.1.22 — LAN Relay + Live Ask (trusted-LAN alpha).
//!
//! UDP beacon discovery + HTTP package transfer / live ask.
//! Alternate transport into relay quarantine receive and local proxy ask.
//! Does not replace filesystem `relay send --relay-root`.

pub mod ask;
pub mod beacon;
pub mod chat;
pub mod client;
pub mod contract;
pub mod last_peers;
pub mod peer;
pub mod serve;
pub mod server;

pub use ask::{answer_live_ask, LanAskError, LanAskRequest};
pub use beacon::{
    decode_beacon, encode_beacon, listen_beacons, spawn_beacon_advertiser, BeaconListenError,
};
pub use chat::{
    append_chat_message, list_chat_messages, new_outbound_message, validate_chat_message,
    LanChatDirection, LanChatError, StoredLanChatMessage,
};
pub use client::{
    ask_peer, health_check, health_check_quick, parse_host_port, probe_presence,
    probe_presence_many, send_chat_message, send_package_to_peer, LanClientError,
    PRESENCE_STALE_WINDOW_SECS,
};
pub use contract::{
    validate_lan_beacon, LanAskHttpBody, LanBeacon, LanChatMessage, LanHealthResponse,
    LanPeerInfo, LanPeerPresence, LanPresenceState, LanProtocolError, LanServeStatus,
    DEFAULT_HTTP_PORT, DEFAULT_UDP_PORT, LAN_CHAT_PROTOCOL, LAN_PROTOCOL, MAX_CHAT_TEXT_BYTES,
};
pub use last_peers::{read_last_peers, remember_discovered_peers, write_last_peers};
pub use peer::{merge_peer, peer_table_snapshot, PeerTable};
pub use serve::{
    current_lan_serve_status, lan_serve_status_for_project, start_lan_serve, stop_lan_serve,
    LanServeError, LanServeHandle,
};
pub use server::{bind_http_listener, spawn_http_server, LanServerError};
