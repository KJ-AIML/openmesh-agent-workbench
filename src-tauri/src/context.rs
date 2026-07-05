// ============================================================================
// OpenMesh Context Domain — Rust Mirror
// ============================================================================
// Dev Track 0.1.2.2 — Rust serde contract mirroring the TypeScript domain.
//
// This module provides serde-compatible types for ContextSource and
// ContextDocument so that Dev Track 0.1.2.3 (Derived Local Index) can
// deserialize shared JSON fixtures in Rust.
//
// No Tauri commands. No persistence. No index code.
// ============================================================================

use serde::{Deserialize, Serialize};

/// Current context schema version. Mirrors the TypeScript constant.
pub const CONTEXT_SCHEMA_VERSION: &str = "1.0.0";

/// Source kinds.
/// Current kinds have mappers; reserved kinds are placeholders for later tracks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ContextSourceKind {
    Doc,
    Note,
    Snapshot,
    Task,
    /// Transitional: bridges current RecentItem data until 0.1.3 WorkEvent.
    Recent,
    AgentSession,
    /// Reserved for OpenMesh 0.1.3
    WorkEvent,
    /// Reserved for OpenMesh 0.1.3
    Git,
    /// Reserved for future connectors.
    Connector,
}

/// Sensitivity classification.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Sensitivity {
    Public,
    Team,
    #[default]
    Private,
    Secret,
}

/// Freshness state for ContextDocument.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FreshnessState {
    Fresh,
    Aging,
    Stale,
    Unknown,
}

/// Structured freshness metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Freshness {
    pub state: FreshnessState,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_updated_at: Option<String>,
}

/// Versioned context source.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSource {
    pub id: String,
    pub schema_version: String,
    pub kind: ContextSourceKind,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_person_id: Option<String>,
    pub canonical_ref: String,
    pub title: String,
    pub sensitivity: Sensitivity,
    pub agent_context_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

/// Normalized context document with freshness metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextDocument {
    pub id: String,
    pub schema_version: String,
    pub source_id: String,
    pub kind: ContextSourceKind,
    pub project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_person_id: Option<String>,
    pub canonical_ref: String,
    pub title: String,
    pub text: String,
    pub sensitivity: Sensitivity,
    pub agent_context_enabled: bool,
    pub observed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_updated_at: Option<String>,
    pub freshness: Freshness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_SOURCE_JSON: &str = r#"
    {
      "id": "abc123",
      "schemaVersion": "1.0.0",
      "kind": "doc",
      "projectId": "proj-1",
      "ownerPersonId": "person-1",
      "canonicalRef": "openmesh://project/proj-1/doc/architecture/overview.md",
      "title": "Architecture Overview",
      "sensitivity": "private",
      "agentContextEnabled": false,
      "createdAt": "2026-07-01T10:00:00.000Z",
      "updatedAt": "2026-07-05T14:30:00.000Z",
      "indexedAt": null,
      "contentHash": "sha256:abc"
    }
    "#;

    #[test]
    fn deserialize_valid_context_source() {
        let source: ContextSource = serde_json::from_str(VALID_SOURCE_JSON).expect("deserialize");
        assert_eq!(source.schema_version, CONTEXT_SCHEMA_VERSION);
        assert_eq!(source.kind, ContextSourceKind::Doc);
        assert_eq!(source.sensitivity, Sensitivity::Private);
    }

    #[test]
    fn deserialize_reserved_kind() {
        let json = r#"{
            "id": "x",
            "schemaVersion": "1.0.0",
            "kind": "work-event",
            "projectId": "p",
            "canonicalRef": "openmesh://project/p/work-event/1",
            "title": "t",
            "sensitivity": "private",
            "agentContextEnabled": false,
            "createdAt": "2026-01-01T00:00:00.000Z",
            "updatedAt": "2026-01-01T00:00:00.000Z"
        }"#;
        let source: ContextSource = serde_json::from_str(json).expect("deserialize reserved kind");
        assert_eq!(source.kind, ContextSourceKind::WorkEvent);
    }

    #[test]
    fn deserialize_invalid_kind_fails() {
        let json = r#"{
            "id": "x",
            "schemaVersion": "1.0.0",
            "kind": "nonexistent-kind",
            "projectId": "p",
            "canonicalRef": "openmesh://project/p/nonexistent-kind/1",
            "title": "t",
            "sensitivity": "private",
            "agentContextEnabled": false,
            "createdAt": "2026-01-01T00:00:00.000Z",
            "updatedAt": "2026-01-01T00:00:00.000Z"
        }"#;
        let result: Result<ContextSource, _> = serde_json::from_str(json);
        assert!(result.is_err(), "unknown kind should fail deserialization");
    }
}
