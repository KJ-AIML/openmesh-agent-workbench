//! Dev Track 0.1.10 Checkpoint C — mesh export / outbox tests.

use openmesh_core::continuity::{
    build_current_state_projection, load_continuity_input_snapshot,
};
use openmesh_core::domain::{CatchUpWindow, EvidenceAttachment, EvidenceRef, WorkEvent};
use openmesh_core::events::append_event;
use openmesh_core::mesh::{
    add_peer, export_mesh_envelope_to_outbox, outbox_envelope_path, read_outbox_envelope,
    BuildMeshExportRequest, MeshExportError, MeshPeerRecord, MeshPeerRef, MeshSensitivityMax,
    MESH_PEER_RECORD_PROTOCOL_VERSION,
};
use openmesh_core::storage::init_project;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-mesh-export-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    init_project(&path).expect("init");
    path
}

fn workspace_id(project: &str) -> String {
    let raw = std::fs::read_to_string(
        openmesh_core::storage::get_project_dir(project).join("project.json"),
    )
    .unwrap();
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
    v["id"].as_str().unwrap().to_string()
}

fn register_peer(project: &str) {
    let now = "2026-08-02T16:00:00Z";
    add_peer(
        project,
        &MeshPeerRecord {
            protocol_version: MESH_PEER_RECORD_PROTOCOL_VERSION.into(),
            peer_id: "yo".into(),
            label: "Yo".into(),
            proxy_profile_id: None,
            remote_workspace_id: Some("ws-yo".into()),
            notes: None,
            created_at: now.into(),
            updated_at: now.into(),
        },
    )
    .expect("peer");
}

#[test]
fn export_writes_outbox_envelope_with_continuity_items() {
    let project = temp_project("export");
    let ws = workspace_id(&project);
    register_peer(&project);

    append_event(
        &project,
        &WorkEvent::new(
            "evt-mesh-1",
            &ws,
            "work.completed",
            "Finished peer registry",
            vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("crates/openmesh-core/src/mesh/peers.rs".into()),
                observed_at: None,
            }],
            "2026-08-02T12:00:00Z",
        ),
    )
    .expect("event");

    let snapshot = load_continuity_input_snapshot(&project).expect("snap");
    let state = build_current_state_projection(&snapshot).expect("state");
    let request = BuildMeshExportRequest {
        workspace_id: ws.clone(),
        from_peer: MeshPeerRef {
            label: "Ter".into(),
            proxy_profile_id: None,
            workspace_id: Some(ws),
        },
        to_peer: Some(MeshPeerRef {
            label: "Yo".into(),
            proxy_profile_id: None,
            workspace_id: Some("ws-yo".into()),
        }),
        window: Some(CatchUpWindow {
            since: "2026-08-01T00:00:00Z".into(),
            until: "2026-08-03T00:00:00Z".into(),
        }),
        now_rfc3339: "2026-08-02T16:00:00Z".into(),
        envelope_id: "env-test-1".into(),
        sensitivity_max: MeshSensitivityMax::Private,
        include_handoff_ids: true,
    };

    let envelope =
        export_mesh_envelope_to_outbox(&project, &snapshot, &state, &request).expect("export");
    assert_eq!(envelope.envelope_id, "env-test-1");
    assert!(outbox_envelope_path(&project, "env-test-1").exists());
    assert!(
        !envelope.evidence_items.is_empty() || !envelope.limitations.is_empty(),
        "expected items or limitations"
    );

    let loaded = read_outbox_envelope(&project, "env-test-1").expect("read");
    assert_eq!(loaded.envelope_id, envelope.envelope_id);
}

#[test]
fn export_duplicate_envelope_id_conflicts() {
    let project = temp_project("dup-env");
    let ws = workspace_id(&project);
    register_peer(&project);
    let snapshot = load_continuity_input_snapshot(&project).expect("snap");
    let state = build_current_state_projection(&snapshot).expect("state");
    let request = BuildMeshExportRequest {
        workspace_id: ws.clone(),
        from_peer: MeshPeerRef {
            label: "Ter".into(),
            proxy_profile_id: None,
            workspace_id: Some(ws),
        },
        to_peer: None,
        window: None,
        now_rfc3339: "2026-08-02T16:00:00Z".into(),
        envelope_id: "env-dup".into(),
        sensitivity_max: MeshSensitivityMax::Team,
        include_handoff_ids: false,
    };
    export_mesh_envelope_to_outbox(&project, &snapshot, &state, &request).expect("first");
    let err = export_mesh_envelope_to_outbox(&project, &snapshot, &state, &request).unwrap_err();
    assert!(matches!(err, MeshExportError::AlreadyExists(_)));
}
