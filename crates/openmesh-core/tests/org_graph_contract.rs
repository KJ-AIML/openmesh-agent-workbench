//! Dev Track 0.1.19 — org graph contract tests.

use openmesh_core::org_graph::{
    validate_org_graph, OrgEdge, OrgEdgeKind, OrgGraph, OrgNode, OrgNodeKind,
    ORG_GRAPH_PROTOCOL_VERSION,
};

fn sample() -> OrgGraph {
    OrgGraph {
        protocol_version: ORG_GRAPH_PROTOCOL_VERSION.into(),
        team_id: "team-1".into(),
        host_workspace_id: "ws-1".into(),
        generated_at: "2026-08-03T00:00:00Z".into(),
        nodes: vec![
            OrgNode {
                id: "team:team-1".into(),
                kind: OrgNodeKind::Team,
                label: "Lab".into(),
                evidence: "team".into(),
            },
            OrgNode {
                id: "member:m1".into(),
                kind: OrgNodeKind::Member,
                label: "Ter".into(),
                evidence: "member".into(),
            },
        ],
        edges: vec![OrgEdge {
            from: "member:m1".into(),
            to: "team:team-1".into(),
            kind: OrgEdgeKind::MemberOf,
            evidence: "team.members".into(),
        }],
        limitations: vec![],
    }
}

#[test]
fn valid_graph_passes() {
    assert!(validate_org_graph(&sample()).is_ok());
}

#[test]
fn dangling_edge_fails() {
    let mut g = sample();
    g.edges[0].to = "missing".into();
    assert!(validate_org_graph(&g).is_err());
}

#[test]
fn serde_roundtrip() {
    let g = sample();
    let json = serde_json::to_string(&g).unwrap();
    let back: OrgGraph = serde_json::from_str(&json).unwrap();
    assert_eq!(g, back);
}
