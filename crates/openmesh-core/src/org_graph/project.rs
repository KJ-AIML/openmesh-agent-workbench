//! Build org graph from local team / trust / connector evidence only.

use crate::connectors::list_connectors;
use crate::org_graph::contract::{
    validate_org_graph, OrgEdge, OrgEdgeKind, OrgGraph, OrgNode, OrgNodeKind,
    ORG_GRAPH_PROTOCOL_VERSION,
};
use crate::team::read_team_workspace;
use crate::trust_admin::read_trust_policy;
use chrono::Utc;

#[derive(Debug, thiserror::Error)]
pub enum OrgGraphError {
    #[error("team workspace required (run team init)")]
    TeamRequired,
    #[error("validation: {0}")]
    Validation(String),
}

/// Project an inspectable org graph from local evidence only.
///
/// Sources: team workspace registry, optional trust-admin policy, optional connectors.
/// Does not invent org structure without those sources.
pub fn build_org_graph(project_path: &str) -> Result<OrgGraph, OrgGraphError> {
    let team = read_team_workspace(project_path).map_err(|_| OrgGraphError::TeamRequired)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    let team_node_id = format!("team:{}", team.team_id);
    nodes.push(OrgNode {
        id: team_node_id.clone(),
        kind: OrgNodeKind::Team,
        label: team.display_name.clone(),
        evidence: ".openmesh/team/workspace.json".into(),
    });

    let ws_id = format!("workspace:{}", team.host_workspace_id);
    nodes.push(OrgNode {
        id: ws_id.clone(),
        kind: OrgNodeKind::Workspace,
        label: team.host_workspace_id.clone(),
        evidence: "team.host_workspace_id".into(),
    });
    edges.push(OrgEdge {
        from: ws_id,
        to: team_node_id.clone(),
        kind: OrgEdgeKind::Hosts,
        evidence: "team.workspace.json".into(),
    });

    for m in &team.members {
        let mid = format!("member:{}", m.member_id);
        nodes.push(OrgNode {
            id: mid.clone(),
            kind: OrgNodeKind::Member,
            label: format!("{} ({:?})", m.label, m.role),
            evidence: format!("team.member.{}", m.member_id),
        });
        edges.push(OrgEdge {
            from: mid.clone(),
            to: team_node_id.clone(),
            kind: OrgEdgeKind::MemberOf,
            evidence: "team.members".into(),
        });
        if let Some(peer) = &m.mesh_peer_id {
            let pid = format!("peer:{}", peer);
            if !nodes.iter().any(|n| n.id == pid) {
                nodes.push(OrgNode {
                    id: pid.clone(),
                    kind: OrgNodeKind::MeshPeer,
                    label: peer.clone(),
                    evidence: format!("member.{}.mesh_peer_id", m.member_id),
                });
            }
            edges.push(OrgEdge {
                from: mid,
                to: pid,
                kind: OrgEdgeKind::LinkedPeer,
                evidence: "team.member.mesh_peer_id".into(),
            });
        }
    }

    // Trust policy presence is evidence of admin surface (no extra nodes if missing).
    let mut limitations = vec![
        "org graph preview — local evidence only".into(),
        "does not assert org structure without team/connector sources".into(),
    ];
    if read_trust_policy(project_path).is_ok() {
        limitations.push("trust-admin policy present (query/sync controls)".into());
    }

    if let Ok(connectors) = list_connectors(project_path) {
        for c in connectors {
            let cid = format!("connector:{}", c.connector_id);
            nodes.push(OrgNode {
                id: cid.clone(),
                kind: OrgNodeKind::Connector,
                label: c.display_name.clone(),
                evidence: ".openmesh/connectors/registry.json".into(),
            });
            edges.push(OrgEdge {
                from: cid,
                to: team_node_id.clone(),
                kind: OrgEdgeKind::ProducesEvidenceFor,
                evidence: "connector.role=evidence-producer-only".into(),
            });
        }
    }

    let graph = OrgGraph {
        protocol_version: ORG_GRAPH_PROTOCOL_VERSION.into(),
        team_id: team.team_id,
        host_workspace_id: team.host_workspace_id,
        generated_at: now,
        nodes,
        edges,
        limitations,
    };
    validate_org_graph(&graph).map_err(|e| OrgGraphError::Validation(e.to_string()))?;
    Ok(graph)
}
