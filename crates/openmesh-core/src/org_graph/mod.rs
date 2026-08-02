//! Dev Track 0.1.19 — Organization Graph Preview.
//!
//! Evidence-backed projection of team structure (members, peers, connectors).
//! No asserted org without evidence.

pub mod contract;
pub mod project;

pub use contract::{
    validate_org_graph, OrgEdge, OrgEdgeKind, OrgGraph, OrgNode, OrgNodeKind,
    ORG_GRAPH_PROTOCOL_VERSION,
};
pub use project::{build_org_graph, OrgGraphError};
