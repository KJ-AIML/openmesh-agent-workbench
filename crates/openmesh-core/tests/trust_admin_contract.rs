//! Dev Track 0.1.17 — trust admin contract + gate tests.

use openmesh_core::trust_admin::{
    evaluate_remote_query, validate_team_trust_policy, QueryAllowEntry, QueryAllowlistMode,
    QueryPermission, TeamTrustPolicy, TRUST_ADMIN_PROTOCOL_VERSION,
};

fn sample_policy() -> TeamTrustPolicy {
    TeamTrustPolicy {
        protocol_version: TRUST_ADMIN_PROTOCOL_VERSION.into(),
        team_id: "team-1".into(),
        host_workspace_id: "ws-1".into(),
        remote_query_enabled: true,
        query_allowlist_mode: QueryAllowlistMode::AllowAll,
        query_allowlist: vec![],
        secret_topics_fail_closed: true,
        allow_secret_export: false,
        sync_require_selective: true,
        admin_member_ids: vec!["owner-local".into()],
        limitations: vec![],
        created_at: "2026-08-03T00:00:00Z".into(),
        updated_at: "2026-08-03T00:00:00Z".into(),
    }
}

#[test]
fn valid_policy_passes() {
    assert!(validate_team_trust_policy(&sample_policy()).is_ok());
}

#[test]
fn secret_fail_closed_required() {
    let mut p = sample_policy();
    p.secret_topics_fail_closed = false;
    assert!(validate_team_trust_policy(&p).is_err());
}

#[test]
fn secret_export_forbidden() {
    let mut p = sample_policy();
    p.allow_secret_export = true;
    assert!(validate_team_trust_policy(&p).is_err());
}

#[test]
fn selective_sync_required() {
    let mut p = sample_policy();
    p.sync_require_selective = false;
    assert!(validate_team_trust_policy(&p).is_err());
}

#[test]
fn allowlist_only_denies_unknown() {
    let mut p = sample_policy();
    p.query_allowlist_mode = QueryAllowlistMode::AllowlistOnly;
    p.query_allowlist = vec![QueryAllowEntry {
        member_id: Some("m-yo".into()),
        mesh_peer_id: Some("yo".into()),
        note: None,
        added_at: "2026-08-03T00:00:00Z".into(),
    }];
    let d = evaluate_remote_query(&p, Some("m-other"), Some("other"));
    assert_eq!(d.permission, QueryPermission::Denied);
    let d2 = evaluate_remote_query(&p, Some("m-yo"), Some("yo"));
    assert_eq!(d2.permission, QueryPermission::Allowed);
}

#[test]
fn deny_all_blocks() {
    let mut p = sample_policy();
    p.query_allowlist_mode = QueryAllowlistMode::DenyAll;
    let d = evaluate_remote_query(&p, Some("m-yo"), Some("yo"));
    assert_eq!(d.permission, QueryPermission::Denied);
}

#[test]
fn remote_query_disabled() {
    let mut p = sample_policy();
    p.remote_query_enabled = false;
    let d = evaluate_remote_query(&p, Some("m-yo"), None);
    assert_eq!(d.permission, QueryPermission::Denied);
}

#[test]
fn serde_roundtrip() {
    let p = sample_policy();
    let json = serde_json::to_string(&p).unwrap();
    let back: TeamTrustPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}
