use openmesh_core::context::Sensitivity;
use openmesh_core::proxy_claims::{
    extract_claims_from_draft, verify_claims_against_pack, ClaimCoverage,
};
use openmesh_core::domain::{
    ContextPackEvidenceIndexEntry, ContextPackEvidenceOrigin, EvidenceRef, ProxyContextPack,
};

const FIXTURE: &str = include_str!("fixtures/context/proxy-context-pack-valid.json");

fn fixture_pack_with_label(label: &str) -> ProxyContextPack {
    let mut pack: ProxyContextPack = serde_json::from_str(FIXTURE).expect("fixture");
    pack.evidence_index = vec![ContextPackEvidenceIndexEntry {
        ref_id: "ev-1".into(),
        evidence_ref: EvidenceRef::FilePath("docs/plan.md".into()),
        origin: ContextPackEvidenceOrigin::ContinuityItem,
        sensitivity: Sensitivity::Private,
        label: label.into(),
        timestamp: Some("2026-07-24T10:00:00Z".into()),
    }];
    pack
}

#[test]
fn extracts_sentences_as_claims() {
    let claims = extract_claims_from_draft("First sentence. Second sentence!");
    assert_eq!(claims.len(), 2);
}

#[test]
fn matches_claim_to_evidence_label() {
    let pack = fixture_pack_with_label("deployment plan");
    let claims = extract_claims_from_draft("The deployment plan is ready.");
    let verified = verify_claims_against_pack(&claims, &pack);
    assert_eq!(verified[0].verification_status, ClaimCoverage::Supported);
}

#[test]
fn unsupported_claim_when_no_match() {
    let pack = fixture_pack_with_label("unrelated");
    let claims = extract_claims_from_draft("Completely different topic.");
    let verified = verify_claims_against_pack(&claims, &pack);
    assert_eq!(verified[0].verification_status, ClaimCoverage::Unsupported);
}
