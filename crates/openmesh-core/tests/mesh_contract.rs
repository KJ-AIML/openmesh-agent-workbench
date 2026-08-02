//! Dev Track 0.1.10 Checkpoint A — Mesh envelope contract tests (pure).

use openmesh_core::domain::{CatchUpWindow, EvidenceRef};
use openmesh_core::mesh::{
    validate_envelope_id_for_storage, validate_mesh_envelope, MeshEnvelope, MeshEvidenceItem,
    MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax, MeshValidationError,
    MESH_ENVELOPE_PROTOCOL_VERSION, MESH_DIR, MESH_INBOX_DIR, MESH_OUTBOX_DIR, MESH_PEERS_DIR,
};

fn sample_item() -> MeshEvidenceItem {
    MeshEvidenceItem {
        summary: "Authority gate landed".into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/plan.md".into())],
        source_kind: MeshEvidenceSourceKind::WorkEvent,
        source_id: "evt-1".into(),
    }
}

fn sample_envelope() -> MeshEnvelope {
    MeshEnvelope {
        protocol_version: MESH_ENVELOPE_PROTOCOL_VERSION.into(),
        envelope_id: "env-20260802-ter-to-yo".into(),
        from_peer: MeshPeerRef {
            label: "Ter".into(),
            proxy_profile_id: Some("profile-ter".into()),
            workspace_id: Some("ws-ter".into()),
        },
        to_peer: Some(MeshPeerRef {
            label: "Yo".into(),
            proxy_profile_id: None,
            workspace_id: None,
        }),
        generated_at: "2026-08-02T14:00:00Z".into(),
        window: Some(CatchUpWindow {
            since: "2026-08-01T00:00:00Z".into(),
            until: "2026-08-02T14:00:00Z".into(),
        }),
        evidence_items: vec![sample_item()],
        handoff_ids: vec!["handoff-1".into()],
        limitations: vec![],
        sensitivity_max: MeshSensitivityMax::Private,
    }
}

#[test]
fn storage_dir_constants_are_stable() {
    assert_eq!(MESH_DIR, "mesh");
    assert_eq!(MESH_OUTBOX_DIR, "mesh/outbox");
    assert_eq!(MESH_INBOX_DIR, "mesh/inbox");
    assert_eq!(MESH_PEERS_DIR, "mesh/peers");
}

#[test]
fn valid_envelope_passes() {
    validate_mesh_envelope(&sample_envelope()).expect("valid");
}

#[test]
fn serde_roundtrip_preserves_envelope() {
    let env = sample_envelope();
    let json = serde_json::to_string(&env).expect("ser");
    let back: MeshEnvelope = serde_json::from_str(&json).expect("de");
    assert_eq!(back, env);
}

#[test]
fn deny_unknown_fields() {
    let json = r#"{
      "protocolVersion":"1.0",
      "envelopeId":"env-1",
      "fromPeer":{"label":"Ter","workspaceId":"ws-ter"},
      "generatedAt":"2026-08-02T14:00:00Z",
      "evidenceItems":[{"summary":"x","evidenceRefs":[],"sourceKind":"other","sourceId":"s1"}],
      "handoffIds":[],
      "limitations":[],
      "sensitivityMax":"private",
      "extraField":true
    }"#;
    let err = serde_json::from_str::<MeshEnvelope>(json).unwrap_err();
    assert!(err.to_string().contains("unknown field") || err.is_data());
}

#[test]
fn rejects_unsupported_protocol() {
    let mut env = sample_envelope();
    env.protocol_version = "9.9".into();
    let err = validate_mesh_envelope(&env).unwrap_err();
    assert!(matches!(
        err,
        MeshValidationError::UnsupportedProtocolVersion { .. }
    ));
}

#[test]
fn rejects_empty_envelope_without_limitations() {
    let mut env = sample_envelope();
    env.evidence_items.clear();
    env.handoff_ids.clear();
    env.limitations.clear();
    assert_eq!(
        validate_mesh_envelope(&env),
        Err(MeshValidationError::EmptyEnvelopeWithoutLimitations)
    );
}

#[test]
fn empty_envelope_allowed_with_limitation() {
    let mut env = sample_envelope();
    env.evidence_items.clear();
    env.handoff_ids.clear();
    env.limitations = vec!["no shareable evidence in window".into()];
    validate_mesh_envelope(&env).expect("limitation documents empty body");
}

#[test]
fn rejects_from_peer_without_workspace() {
    let mut env = sample_envelope();
    env.from_peer.workspace_id = None;
    assert_eq!(
        validate_mesh_envelope(&env),
        Err(MeshValidationError::FromPeerWorkspaceRequired)
    );
}

#[test]
fn rejects_unsafe_envelope_id() {
    assert_eq!(
        validate_envelope_id_for_storage("../escape"),
        Err(MeshValidationError::UnsafeEnvelopeId)
    );
    assert_eq!(
        validate_envelope_id_for_storage("a/b"),
        Err(MeshValidationError::UnsafeEnvelopeId)
    );
}

#[test]
fn rejects_inverted_window() {
    let mut env = sample_envelope();
    env.window = Some(CatchUpWindow {
        since: "2026-08-02T14:00:00Z".into(),
        until: "2026-08-01T00:00:00Z".into(),
    });
    assert_eq!(
        validate_mesh_envelope(&env),
        Err(MeshValidationError::WindowInverted)
    );
}

#[test]
fn rejects_empty_evidence_summary() {
    let mut env = sample_envelope();
    env.evidence_items[0].summary = "  ".into();
    assert_eq!(
        validate_mesh_envelope(&env),
        Err(MeshValidationError::EmptyEvidenceSummary)
    );
}

#[test]
fn sensitivity_max_wire_is_kebab_free_lowercase() {
    let env = sample_envelope();
    let json = serde_json::to_value(&env).unwrap();
    assert_eq!(json["sensitivityMax"], "private");
    assert_eq!(json["fromPeer"]["label"], "Ter");
    assert_eq!(json["evidenceItems"][0]["sourceKind"], "work-event");
}
