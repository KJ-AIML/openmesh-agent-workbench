use chrono::{TimeZone, Utc};
use openmesh_core::authority_freshness::{evaluate_evidence_freshness, ConfidenceLabel};
use openmesh_core::authority_policy::{map_risk_to_freshness_tier, FreshnessTier};
use openmesh_core::domain::ProxyContextPack;

const FIXTURE: &str = include_str!("fixtures/context/proxy-context-pack-valid.json");

#[test]
fn critical_tier_rejects_month_old_evidence() {
    let mut pack: ProxyContextPack = serde_json::from_str(FIXTURE).expect("fixture");
    pack.freshness.age_seconds = 60 * 60 * 24 * 30;
    for entry in &mut pack.evidence_index {
        entry.timestamp = Some("2020-01-01T00:00:00Z".into());
    }
    let now = Utc.with_ymd_and_hms(2026, 7, 24, 12, 0, 0).unwrap();
    let result = evaluate_evidence_freshness(&pack, FreshnessTier::Critical, now);
    assert!(!result.is_sufficient);
    assert_eq!(result.confidence_label, ConfidenceLabel::Insufficient);
}

#[test]
fn low_impact_accepts_month_old_evidence() {
    let mut pack: ProxyContextPack = serde_json::from_str(FIXTURE).expect("fixture");
    pack.freshness.age_seconds = 60 * 60 * 24 * 30;
    let now = Utc.with_ymd_and_hms(2026, 7, 24, 12, 0, 0).unwrap();
    for entry in &mut pack.evidence_index {
        entry.timestamp = Some("2026-06-24T12:00:00Z".into());
    }
    let result = evaluate_evidence_freshness(&pack, FreshnessTier::LowImpact, now);
    assert!(result.is_sufficient);
}

#[test]
fn commitment_risk_maps_to_critical_tier() {
    assert_eq!(
        map_risk_to_freshness_tier(
            openmesh_core::authority_policy::QuestionRiskCategory::Commitment
        ),
        FreshnessTier::Critical
    );
}
