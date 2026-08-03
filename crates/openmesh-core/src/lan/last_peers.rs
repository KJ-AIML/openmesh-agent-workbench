//! Persist last-known LAN peers for dogfood when UDP discovery is flaky.

use crate::lan::contract::LanPeerInfo;
use crate::storage::get_project_dir;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

const LAST_PEERS_REL: &str = "lan/last-peers.json";
const MAX_LAST_PEERS: usize = 32;

fn last_peers_path(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(LAST_PEERS_REL)
}

pub fn read_last_peers(project_path: &str) -> Vec<LanPeerInfo> {
    let path = last_peers_path(project_path);
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

pub fn write_last_peers(project_path: &str, peers: &[LanPeerInfo]) -> Result<(), String> {
    let path = last_peers_path(project_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut trimmed: Vec<LanPeerInfo> = peers.iter().cloned().take(MAX_LAST_PEERS).collect();
    // Prefer newest last_seen_at first for UI.
    trimmed.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
    let mut json = serde_json::to_string_pretty(&trimmed).map_err(|e| e.to_string())?;
    json.push('\n');
    let temp = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&temp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Merge freshly discovered peers into last-known and persist.
pub fn remember_discovered_peers(
    project_path: &str,
    discovered: &[LanPeerInfo],
) -> Result<Vec<LanPeerInfo>, String> {
    if discovered.is_empty() {
        return Ok(read_last_peers(project_path));
    }
    let mut by_key = std::collections::BTreeMap::new();
    for p in read_last_peers(project_path) {
        by_key.insert(format!("{}@{}", p.peer_id, p.address), p);
    }
    for p in discovered {
        by_key.insert(format!("{}@{}", p.peer_id, p.address), p.clone());
    }
    let merged: Vec<_> = by_key.into_values().collect();
    write_last_peers(project_path, &merged)?;
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::contract::LAN_PROTOCOL;
    use crate::storage::init_project;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn remember_merges_and_persists() {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-last-peers-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();

        let peer = LanPeerInfo {
            protocol: LAN_PROTOCOL.into(),
            project_id: "p".into(),
            owner_label: "Yo".into(),
            peer_id: "yo".into(),
            host: "192.168.1.10".into(),
            http_port: 41778,
            started_at: "2026-08-03T00:00:00Z".into(),
            last_seen_at: "2026-08-03T00:01:00Z".into(),
            address: "192.168.1.10:41778".into(),
        };
        remember_discovered_peers(&path, &[peer.clone()]).unwrap();
        let loaded = read_last_peers(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].address, peer.address);
        let _ = fs::remove_dir_all(&dir);
    }
}
