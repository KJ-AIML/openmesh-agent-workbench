//! Dev Track 0.1.10 Checkpoint D — mesh import / inbox tests.

use openmesh_core::domain::{CatchUpWindow, EvidenceRef};
use openmesh_core::mesh::{
    import_mesh_envelope, import_mesh_envelope_from_file, inbox_envelope_path,
    list_inbox_envelope_ids, read_inbox_envelope, write_outbox_envelope, ImportMeshOptions,
    MeshEnvelope, MeshEvidenceItem, MeshEvidenceSourceKind, MeshImportError, MeshPeerRef,
    MeshSensitivityMax, MESH_ENVELOPE_PROTOCOL_VERSION,
};
use openmesh_core::storage::init_project;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-mesh-import-{label}-{}-{n}",
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

fn sample_envelope(from_ws: &str, envelope_id: &str) -> MeshEnvelope {
    MeshEnvelope {
        protocol_version: MESH_ENVELOPE_PROTOCOL_VERSION.into(),
        envelope_id: envelope_id.into(),
        from_peer: MeshPeerRef {
            label: "Ter".into(),
            proxy_profile_id: None,
            workspace_id: Some(from_ws.into()),
        },
        to_peer: Some(MeshPeerRef {
            label: "Yo".into(),
            proxy_profile_id: None,
            workspace_id: None,
        }),
        generated_at: "2026-08-02T17:00:00Z".into(),
        window: Some(CatchUpWindow {
            since: "2026-08-01T00:00:00Z".into(),
            until: "2026-08-02T17:00:00Z".into(),
        }),
        evidence_items: vec![MeshEvidenceItem {
            summary: "Shared progress".into(),
            evidence_refs: vec![EvidenceRef::FilePath("docs/plan.md".into())],
            source_kind: MeshEvidenceSourceKind::WorkEvent,
            source_id: "evt-shared".into(),
        }],
        handoff_ids: vec![],
        limitations: vec![],
        sensitivity_max: MeshSensitivityMax::Private,
    }
}

#[test]
fn import_stores_inbox_and_lists() {
    let a = temp_project("exporter");
    let b = temp_project("importer");
    let a_ws = workspace_id(&a);
    let envelope = sample_envelope(&a_ws, "env-import-1");
    write_outbox_envelope(&a, &envelope).expect("outbox");

    let path = openmesh_core::mesh::outbox_envelope_path(&a, "env-import-1");
    let imported = import_mesh_envelope_from_file(
        &b,
        &path,
        &ImportMeshOptions {
            register_from_peer: true,
            allow_self_workspace: false,
        },
    )
    .expect("import");
    assert_eq!(imported.envelope_id, "env-import-1");
    assert!(inbox_envelope_path(&b, "env-import-1").exists());
    assert_eq!(list_inbox_envelope_ids(&b).unwrap(), vec!["env-import-1"]);
    let loaded = read_inbox_envelope(&b, "env-import-1").unwrap();
    assert_eq!(loaded.from_peer.label, "Ter");
}

#[test]
fn refuse_self_workspace_import() {
    let project = temp_project("self");
    let ws = workspace_id(&project);
    let envelope = sample_envelope(&ws, "env-self");
    let err = import_mesh_envelope(&project, &envelope, &ImportMeshOptions::default()).unwrap_err();
    assert!(matches!(err, MeshImportError::SelfWorkspaceImport));
}

#[test]
fn allow_self_with_flag() {
    let project = temp_project("self-ok");
    let ws = workspace_id(&project);
    let envelope = sample_envelope(&ws, "env-self-ok");
    import_mesh_envelope(
        &project,
        &envelope,
        &ImportMeshOptions {
            register_from_peer: false,
            allow_self_workspace: true,
        },
    )
    .expect("allowed");
}

#[test]
fn duplicate_inbox_id_conflicts() {
    let a = temp_project("dup-a");
    let b = temp_project("dup-b");
    let envelope = sample_envelope(&workspace_id(&a), "env-dup");
    import_mesh_envelope(&b, &envelope, &ImportMeshOptions::default()).expect("first");
    let err = import_mesh_envelope(&b, &envelope, &ImportMeshOptions::default()).unwrap_err();
    assert!(matches!(err, MeshImportError::AlreadyExists(_)));
}
