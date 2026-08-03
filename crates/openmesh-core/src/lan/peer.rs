//! In-memory peer table for discover / serve status.

use crate::lan::contract::{LanBeacon, LanPeerInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Default, Clone)]
pub struct PeerTable {
    inner: Arc<Mutex<HashMap<String, LanPeerInfo>>>,
}

impl PeerTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&self, peer: LanPeerInfo) {
        let key = peer_key(&peer.peer_id, &peer.host, peer.http_port);
        if let Ok(mut g) = self.inner.lock() {
            g.insert(key, peer);
        }
    }

    pub fn snapshot(&self) -> Vec<LanPeerInfo> {
        let Ok(g) = self.inner.lock() else {
            return Vec::new();
        };
        let mut rows: Vec<_> = g.values().cloned().collect();
        rows.sort_by(|a, b| {
            a.owner_label
                .cmp(&b.owner_label)
                .then(a.peer_id.cmp(&b.peer_id))
                .then(a.address.cmp(&b.address))
        });
        rows
    }

    pub fn clear(&self) {
        if let Ok(mut g) = self.inner.lock() {
            g.clear();
        }
    }
}

pub fn merge_peer(table: &PeerTable, beacon: &LanBeacon, host: &str, last_seen_at: &str) {
    table.upsert(LanPeerInfo::from_beacon(beacon, host, last_seen_at));
}

pub fn peer_table_snapshot(table: &PeerTable) -> Vec<LanPeerInfo> {
    table.snapshot()
}

fn peer_key(peer_id: &str, host: &str, port: u16) -> String {
    format!("{peer_id}@{host}:{port}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::contract::LAN_PROTOCOL;

    #[test]
    fn peer_table_upserts_by_id_host_port() {
        let t = PeerTable::new();
        let b = LanBeacon {
            protocol: LAN_PROTOCOL.into(),
            project_id: "p".into(),
            owner_label: "Yo".into(),
            peer_id: "yo".into(),
            http_port: 41778,
            started_at: "2026-08-03T00:00:00Z".into(),
        };
        merge_peer(&t, &b, "127.0.0.1", "2026-08-03T00:00:01Z");
        merge_peer(&t, &b, "127.0.0.1", "2026-08-03T00:00:02Z");
        let snap = t.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].last_seen_at, "2026-08-03T00:00:02Z");
    }
}
