//! Dev Track 0.1.15 — team workspace contract tests.

use openmesh_core::team::{
    validate_team_member, validate_team_workspace, TeamMember, TeamMemberRole, TeamWorkspace,
    TEAM_PROTOCOL_VERSION,
};

fn sample_member(id: &str, role: TeamMemberRole) -> TeamMember {
    TeamMember {
        member_id: id.into(),
        label: id.into(),
        role,
        mesh_peer_id: Some("yo".into()),
        proxy_profile_id: None,
        remote_workspace_id: Some("ws-yo".into()),
        joined_at: "2026-08-02T17:00:00Z".into(),
    }
}

fn sample_team() -> TeamWorkspace {
    TeamWorkspace {
        protocol_version: TEAM_PROTOCOL_VERSION.into(),
        team_id: "team-1".into(),
        display_name: "Ter × Yo Lab".into(),
        host_workspace_id: "ws-ter".into(),
        members: vec![
            sample_member("owner-local", TeamMemberRole::Owner),
            sample_member("m-yo", TeamMemberRole::Member),
        ],
        created_at: "2026-08-02T17:00:00Z".into(),
        updated_at: "2026-08-02T17:00:00Z".into(),
        limitations: vec![],
    }
}

#[test]
fn valid_team_passes() {
    assert!(validate_team_workspace(&sample_team()).is_ok());
}

#[test]
fn member_requires_label() {
    let mut m = sample_member("x", TeamMemberRole::Member);
    m.label = "  ".into();
    assert!(validate_team_member(&m).is_err());
}

#[test]
fn team_without_owner_fails_when_members_present() {
    let mut t = sample_team();
    t.members = vec![sample_member("only-member", TeamMemberRole::Member)];
    assert!(validate_team_workspace(&t).is_err());
}

#[test]
fn serde_roundtrip() {
    let t = sample_team();
    let json = serde_json::to_string(&t).unwrap();
    let back: TeamWorkspace = serde_json::from_str(&json).unwrap();
    assert_eq!(t, back);
}
