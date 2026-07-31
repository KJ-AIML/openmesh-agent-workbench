//! Dev Track 0.1.7.4 — Freshness and confidence evaluation.

use crate::authority_policy::FreshnessTier;
use crate::domain::ProxyContextPack;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfidenceLabel {
    High,
    Medium,
    Low,
    Insufficient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessResult {
    pub is_sufficient: bool,
    pub stale_warnings: Vec<String>,
    pub oldest_evidence_age_seconds: u64,
    pub confidence_label: ConfidenceLabel,
    pub tier: FreshnessTier,
}

/// Max evidence age per freshness tier.
pub fn max_age_for_tier(tier: FreshnessTier) -> Duration {
    match tier {
        FreshnessTier::LowImpact => Duration::days(90),
        FreshnessTier::Standard => Duration::days(7),
        FreshnessTier::Critical => Duration::hours(4),
    }
}

/// Evaluate whether pack evidence is fresh enough for the tier.
pub fn evaluate_evidence_freshness(
    pack: &ProxyContextPack,
    tier: FreshnessTier,
    now: DateTime<Utc>,
) -> FreshnessResult {
    let max_age = max_age_for_tier(tier);
    let mut stale_warnings = Vec::new();
    let mut oldest_age = pack.freshness.age_seconds;

    for entry in &pack.evidence_index {
        if let Some(ts) = &entry.timestamp {
            if let Ok(parsed) = DateTime::parse_from_rfc3339(ts) {
                let age = now.signed_duration_since(parsed.with_timezone(&Utc));
                let age_secs = age.num_seconds().max(0) as u64;
                oldest_age = oldest_age.max(age_secs);
                if age > max_age {
                    stale_warnings.push(format!(
                        "evidence {} is older than tier {:?} allows",
                        entry.ref_id, tier
                    ));
                }
            }
        }
    }

    if pack.freshness.age_seconds > max_age.num_seconds().max(0) as u64 {
        stale_warnings.push("pack snapshot exceeds freshness tier".to_string());
    }

    let is_sufficient = stale_warnings.is_empty();
    let confidence_label = if !is_sufficient {
        ConfidenceLabel::Insufficient
    } else if oldest_age < max_age.num_seconds().max(0) as u64 / 4 {
        ConfidenceLabel::High
    } else if oldest_age < max_age.num_seconds().max(0) as u64 / 2 {
        ConfidenceLabel::Medium
    } else {
        ConfidenceLabel::Low
    };

    FreshnessResult {
        is_sufficient,
        stale_warnings,
        oldest_evidence_age_seconds: oldest_age,
        confidence_label,
        tier,
    }
}
