//! Dev Track 0.1.7.3 — Citation mapping from verified claims.

use crate::proxy_claims::{ClaimCoverage, VerifiedClaim};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyCitation {
    pub claim_id: String,
    pub claim_text: String,
    pub evidence_ref_ids: Vec<String>,
    pub coverage: ClaimCoverage,
}

/// Build citations from verified claims.
pub fn build_citations(verified: &[VerifiedClaim]) -> Vec<ProxyCitation> {
    verified
        .iter()
        .map(|v| ProxyCitation {
            claim_id: v.claim.claim_id.clone(),
            claim_text: v.claim.claim_text.clone(),
            evidence_ref_ids: v.matched_evidence_ids.clone(),
            coverage: v.verification_status,
        })
        .collect()
}

/// Collect unsupported claim texts for downgrade messaging.
pub fn unsupported_claim_texts(verified: &[VerifiedClaim]) -> Vec<String> {
    verified
        .iter()
        .filter(|v| v.verification_status == ClaimCoverage::Unsupported)
        .map(|v| v.claim.claim_text.clone())
        .collect()
}
