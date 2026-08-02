//! Dev Track 0.1.16 — team cloud contract tests.

use openmesh_core::team_cloud::{
    validate_team_cloud_config, validate_team_cloud_sync_plan, TeamCloudConfig, TeamCloudMode,
    TeamCloudSyncPlan, TEAM_CLOUD_PROTOCOL_VERSION,
};

fn sample_cfg() -> TeamCloudConfig {
    TeamCloudConfig {
        protocol_version: TEAM_CLOUD_PROTOCOL_VERSION.into(),
        team_id: "team-1".into(),
        host_workspace_id: "ws-ter".into(),
        mode: TeamCloudMode::LocalSim,
        online_proxy_id: Some("proxy-1".into()),
        selective_sync: true,
        sync_paths: vec![".openmesh/team".into(), ".openmesh/mesh".into()],
        last_sync_at: None,
        limitations: vec!["scaffold only".into()],
        created_at: "2026-08-03T00:00:00Z".into(),
        updated_at: "2026-08-03T00:00:00Z".into(),
    }
}

#[test]
fn valid_config_passes() {
    assert!(validate_team_cloud_config(&sample_cfg()).is_ok());
}

#[test]
fn selective_sync_required() {
    let mut c = sample_cfg();
    c.selective_sync = false;
    assert!(validate_team_cloud_config(&c).is_err());
}

#[test]
fn path_traversal_rejected() {
    let mut c = sample_cfg();
    c.sync_paths = vec!["../etc/passwd".into()];
    assert!(validate_team_cloud_config(&c).is_err());
}

#[test]
fn sync_plan_must_be_scaffold_only() {
    let plan = TeamCloudSyncPlan {
        protocol_version: TEAM_CLOUD_PROTOCOL_VERSION.into(),
        team_id: "team-1".into(),
        mode: TeamCloudMode::LocalSim,
        generated_at: "2026-08-03T00:00:00Z".into(),
        planned_paths: vec![".openmesh/team".into()],
        scaffold_only: false,
        note: "bad".into(),
    };
    assert!(validate_team_cloud_sync_plan(&plan).is_err());
}

#[test]
fn serde_roundtrip() {
    let c = sample_cfg();
    let json = serde_json::to_string(&c).unwrap();
    let back: TeamCloudConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c, back);
}
