//! Dev Track 0.1.10 Checkpoint B — peer registry storage tests.

use openmesh_core::mesh::{
    add_peer, list_peer_ids, list_peers, peer_id_from_label, peer_path, peers_dir, read_peer,
    validate_peer_id_for_storage, MeshPeerError, MeshPeerRecord, MESH_PEER_RECORD_PROTOCOL_VERSION,
};
use openmesh_core::storage::init_project;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-mesh-peers-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    init_project(&path).expect("init");
    path
}

fn sample_peer(peer_id: &str, label: &str) -> MeshPeerRecord {
    MeshPeerRecord {
        protocol_version: MESH_PEER_RECORD_PROTOCOL_VERSION.into(),
        peer_id: peer_id.into(),
        label: label.into(),
        proxy_profile_id: Some("profile-yo".into()),
        remote_workspace_id: Some("ws-yo".into()),
        notes: Some("local teammate".into()),
        created_at: "2026-08-02T15:00:00Z".into(),
        updated_at: "2026-08-02T15:00:00Z".into(),
    }
}

#[test]
fn peer_id_from_label_is_stable_slug() {
    assert_eq!(peer_id_from_label("Yo Partner"), "yo-partner");
    assert_eq!(peer_id_from_label("  "), "peer");
}

#[test]
fn rejects_unsafe_peer_id() {
    assert!(validate_peer_id_for_storage("../x").is_err());
    assert!(validate_peer_id_for_storage("a/b").is_err());
}

#[test]
fn add_list_show_peer_roundtrip() {
    let project = temp_project("roundtrip");
    let peer = sample_peer("yo", "Yo");
    add_peer(&project, &peer).expect("add");
    assert!(peer_path(&project, "yo").exists());
    assert!(peers_dir(&project)
        .to_string_lossy()
        .contains("mesh/peers"));

    let ids = list_peer_ids(&project).expect("ids");
    assert_eq!(ids, vec!["yo".to_string()]);

    let loaded = read_peer(&project, "yo").expect("read");
    assert_eq!(loaded, peer);

    let all = list_peers(&project).expect("list");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].label, "Yo");
}

#[test]
fn add_duplicate_peer_fails() {
    let project = temp_project("dup");
    let peer = sample_peer("yo", "Yo");
    add_peer(&project, &peer).expect("first");
    let err = add_peer(&project, &peer).unwrap_err();
    assert!(matches!(err, MeshPeerError::AlreadyExists(_)));
}

#[test]
fn read_missing_peer_is_not_found() {
    let project = temp_project("missing");
    let err = read_peer(&project, "nope").unwrap_err();
    assert_eq!(err, MeshPeerError::NotFound);
}

#[test]
fn list_empty_registry() {
    let project = temp_project("empty");
    assert!(list_peer_ids(&project).unwrap().is_empty());
    assert!(list_peers(&project).unwrap().is_empty());
}
