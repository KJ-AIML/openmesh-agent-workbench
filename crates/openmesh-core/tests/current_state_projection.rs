//! Dev Track 0.1.3.7 Checkpoint C — Current State projection persistence smoke tests.

use openmesh_core::continuity::{
    build_current_state_projection, current_state_projection_path, read_current_state_projection,
    write_current_state_projection, ContinuityInputSnapshot,
};
use openmesh_core::domain::{SourceCounts, CURRENT_STATE_PROJECTION_PROTOCOL_VERSION};

#[test]
fn current_state_projection_wire_round_trip() {
    let snapshot = ContinuityInputSnapshot {
        workspace_id: "ws-roundtrip".into(),
        loaded_at: "2026-07-16T10:00:00Z".into(),
        pending_signals: vec![],
        processed_signals: vec![],
        quarantine_signals: vec![],
        duplicate_signals: vec![],
        work_events: vec![],
        promotion_audit_records: vec![],
        diagnostics: vec![],
        source_counts: SourceCounts {
            work_events: 0,
            processed_signals: 0,
            pending_signals: 0,
            promotion_audit_records: 0,
            quarantine_signals: 0,
            duplicate_signals: 0,
            reporter_signals: 0,
            git_signals: 0,
            heli_signals: 0,
            unknown_producer_signals: 0,
            other_producer_signals: 0,
        },
    };
    let projection = build_current_state_projection(&snapshot).expect("build");
    assert_eq!(
        projection.protocol_version,
        CURRENT_STATE_PROJECTION_PROTOCOL_VERSION
    );
    let json = serde_json::to_string_pretty(&projection).expect("serialize");
    let restored: openmesh_core::domain::CurrentStateProjection =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored.workspace_id, projection.workspace_id);
    assert_eq!(restored.rebuild_inputs_hash, projection.rebuild_inputs_hash);

    let dir = std::env::temp_dir().join(format!(
        "openmesh-projection-roundtrip-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir.join("myproject/.openmesh")).unwrap();
    let project_json = serde_json::json!({
        "id": "ws-roundtrip",
        "name": "Test",
        "folderPath": dir.join("myproject").to_str().unwrap(),
        "repoUrl": null,
        "defaultBranch": "main",
        "sprintSource": "none",
        "docsFolder": null,
        "terminalDir": null,
        "defaultAgentCli": null,
        "notes": null,
        "status": "active",
        "createdAt": "2026-07-16T10:00:00Z",
        "updatedAt": "2026-07-16T10:00:00Z",
    });
    std::fs::write(
        dir.join("myproject/.openmesh/project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();
    let project_path = dir.join("myproject").to_string_lossy().into_owned();
    write_current_state_projection(&project_path, &projection).expect("write");
    assert!(current_state_projection_path(&project_path).exists());
    let read = read_current_state_projection(&project_path).expect("read");
    assert_eq!(read.workspace_id, "ws-roundtrip");
}
