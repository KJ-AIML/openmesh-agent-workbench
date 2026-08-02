//! Pure wire contracts for Organization Graph Preview (0.1.19).

use crate::domain::validate_utc_timestamp;
use serde::{Deserialize, Serialize};

pub const ORG_GRAPH_PROTOCOL_VERSION: &str = "1.0";
pub const MAX_NODES: usize = 128;
pub const MAX_EDGES: usize = 256;
pub const MAX_LABEL_BYTES: usize = 256;
pub const MAX_EVIDENCE_BYTES: usize = 256;
pub const MAX_LIMITATIONS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrgNodeKind {
    Team,
    Member,
    Workspace,
    MeshPeer,
    Connector,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrgEdgeKind {
    Hosts,
    MemberOf,
    LinkedPeer,
    ProducesEvidenceFor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrgNode {
    pub id: String,
    pub kind: OrgNodeKind,
    pub label: String,
    /// Short evidence path or source id (never secret content).
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrgEdge {
    pub from: String,
    pub to: String,
    pub kind: OrgEdgeKind,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrgGraph {
    pub protocol_version: String,
    pub team_id: String,
    pub host_workspace_id: String,
    pub generated_at: String,
    #[serde(default)]
    pub nodes: Vec<OrgNode>,
    #[serde(default)]
    pub edges: Vec<OrgEdge>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OrgGraphValidationError {
    #[error("unsupported protocol_version {found}")]
    UnsupportedProtocol { found: String },
    #[error("team_id empty")]
    EmptyTeamId,
    #[error("too many nodes/edges")]
    Bounds,
    #[error("invalid node")]
    InvalidNode,
    #[error("invalid edge")]
    InvalidEdge,
    #[error("edge references unknown node")]
    DanglingEdge,
    #[error("invalid timestamp: {0}")]
    InvalidTimestamp(String),
}

pub fn validate_org_graph(g: &OrgGraph) -> Result<(), OrgGraphValidationError> {
    if g.protocol_version != ORG_GRAPH_PROTOCOL_VERSION {
        return Err(OrgGraphValidationError::UnsupportedProtocol {
            found: g.protocol_version.clone(),
        });
    }
    if g.team_id.trim().is_empty() {
        return Err(OrgGraphValidationError::EmptyTeamId);
    }
    if g.nodes.len() > MAX_NODES || g.edges.len() > MAX_EDGES {
        return Err(OrgGraphValidationError::Bounds);
    }
    if g.limitations.len() > MAX_LIMITATIONS {
        return Err(OrgGraphValidationError::Bounds);
    }
    validate_utc_timestamp(&g.generated_at).map_err(OrgGraphValidationError::InvalidTimestamp)?;
    let mut ids = std::collections::BTreeSet::new();
    for n in &g.nodes {
        if n.id.trim().is_empty()
            || n.label.trim().is_empty()
            || n.label.len() > MAX_LABEL_BYTES
            || n.evidence.len() > MAX_EVIDENCE_BYTES
        {
            return Err(OrgGraphValidationError::InvalidNode);
        }
        if !ids.insert(n.id.clone()) {
            return Err(OrgGraphValidationError::InvalidNode);
        }
    }
    for e in &g.edges {
        if e.evidence.len() > MAX_EVIDENCE_BYTES {
            return Err(OrgGraphValidationError::InvalidEdge);
        }
        if !ids.contains(&e.from) || !ids.contains(&e.to) {
            return Err(OrgGraphValidationError::DanglingEdge);
        }
    }
    Ok(())
}
