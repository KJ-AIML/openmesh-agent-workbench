//! Dev Track 0.1.11 Checkpoint A — relay contract tests.

use openmesh_core::mesh::{
    MeshEnvelope, MeshEvidenceItem, MeshEvidenceSourceKind, MeshPeerRef, MeshSensitivityMax,
    MESH_ENVELOPE_PROTOCOL_VERSION,
};
use openmesh_core::relay::{
    is_package_approved, validate_relay_package, RelayPackage, RelayPolicySnapshot,
    RELAY_PACKAGE_PROTOCOL_VERSION,
};
use openmesh_core::domain::{CatchUpWindow, EvidenceRef};

fn sample_envelope() -> MeshEnvelope {
    MeshEnvelope {
        protocol_version: MESH_ENVELOPE_PROTOCOL_VERSION.into(),
        envelope_id: "env-1".into(),
        from_peer: MeshPeerRef {
            label: "Ter".into(),
            proxy_profile_id: None,
            workspace_id: Some("ws-a".into()),
        },
        to_peer: None,
        generated_at: "2026-08-02T18:00:00Z".into(),
        window: Some(CatchUpWindow {
            since: "2026-08-01T00:00:00Z".into(),
            until: "2026-08-02T18:00:00Z".into(),
        }),
        evidence_items: vec![MeshEvidenceItem {
            summary: "item".into(),
            evidence_refs: vec![EvidenceRef::FilePath("a.md".into())],
            source_kind: MeshEvidenceSourceKind::Other,
            source_id: "s1".into(),
        }],
        handoff_ids: vec![],
        limitations: vec![],
        sensitivity_max: MeshSensitivityMax::Team,
    }
}

fn sample_package() -> RelayPackage {
    RelayPackage {
        protocol_version: RELAY_PACKAGE_PROTOCOL_VERSION.into(),
        package_id: "pkg-1".into(),
        workspace_id: "ws-a".into(),
        generated_at: "2026-08-02T18:00:00Z".into(),
        sensitivity_max: MeshSensitivityMax::Private,
        envelopes: vec![sample_envelope()],
        handoff_ids: vec![],
        policy: RelayPolicySnapshot {
            approved_paths: vec![".openmesh/mesh/outbox".into()],
            denied_classes: vec!["secret".into()],
            selection_notes: vec!["test".into()],
        },
        limitations: vec![],
        content_hash: Some("abc".into()),
        approved_at: None,
        approved_by: None,
    }
}

#[test]
fn valid_package_passes() {
    validate_relay_package(&sample_package()).expect("valid");
    assert!(!is_package_approved(&sample_package()));
}

#[test]
fn empty_without_limitations_fails() {
    let mut p = sample_package();
    p.envelopes.clear();
    p.handoff_ids.clear();
    p.limitations.clear();
    assert!(validate_relay_package(&p).is_err());
}

#[test]
fn envelope_sensitivity_must_not_exceed_package() {
    let mut p = sample_package();
    p.sensitivity_max = MeshSensitivityMax::Public;
    assert!(validate_relay_package(&p).is_err());
}

#[test]
fn unsafe_package_id_fails() {
    let mut p = sample_package();
    p.package_id = "../x".into();
    assert!(validate_relay_package(&p).is_err());
}

#[test]
fn serde_roundtrip() {
    let p = sample_package();
    let j = serde_json::to_string(&p).unwrap();
    let back: RelayPackage = serde_json::from_str(&j).unwrap();
    assert_eq!(back, p);
}
