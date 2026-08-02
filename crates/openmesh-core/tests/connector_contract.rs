//! Dev Track 0.1.18 — connector contract tests.

use openmesh_core::connectors::{
    collect_github_stub, validate_connector_descriptor, validate_connector_run, ConnectorDescriptor,
    ConnectorKind, ConnectorRole, ConnectorRun, CONNECTOR_PROTOCOL_VERSION,
};

fn sample_descriptor() -> ConnectorDescriptor {
    ConnectorDescriptor {
        protocol_version: CONNECTOR_PROTOCOL_VERSION.into(),
        connector_id: "gh-lab".into(),
        kind: ConnectorKind::GithubStub,
        display_name: "Lab GitHub stub".into(),
        role: ConnectorRole::EvidenceProducerOnly,
        enabled: true,
        external_ref: Some("acme/lab".into()),
        limitations: vec!["evidence only".into()],
        created_at: "2026-08-03T00:00:00Z".into(),
        updated_at: "2026-08-03T00:00:00Z".into(),
    }
}

#[test]
fn valid_descriptor_passes() {
    assert!(validate_connector_descriptor(&sample_descriptor()).is_ok());
}

#[test]
fn path_traversal_external_ref_rejected() {
    let mut d = sample_descriptor();
    d.external_ref = Some("../etc/passwd".into());
    assert!(validate_connector_descriptor(&d).is_err());
}

#[test]
fn github_stub_collect_evidence_only() {
    let run = collect_github_stub(&sample_descriptor()).unwrap();
    assert!(run.evidence_only);
    assert!(!run.items.is_empty());
    assert!(validate_connector_run(&run).is_ok());
}

#[test]
fn disabled_connector_fails_collect() {
    let mut d = sample_descriptor();
    d.enabled = false;
    assert!(collect_github_stub(&d).is_err());
}

#[test]
fn evidence_only_required_on_run() {
    let mut run = ConnectorRun {
        protocol_version: CONNECTOR_PROTOCOL_VERSION.into(),
        run_id: "r1".into(),
        connector_id: "gh-lab".into(),
        kind: ConnectorKind::GithubStub,
        collected_at: "2026-08-03T00:00:00Z".into(),
        evidence_only: false,
        source: "x".into(),
        items: vec![],
        note: "bad".into(),
    };
    assert!(validate_connector_run(&run).is_err());
    run.evidence_only = true;
    assert!(validate_connector_run(&run).is_ok());
}

#[test]
fn serde_roundtrip() {
    let d = sample_descriptor();
    let json = serde_json::to_string(&d).unwrap();
    let back: ConnectorDescriptor = serde_json::from_str(&json).unwrap();
    assert_eq!(d, back);
}
