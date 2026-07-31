//! Dev Track 0.1.7 — Post-provider claim coverage + freshness fail-closed.

use crate::authority_freshness::{evaluate_evidence_freshness, FreshnessResult};
use crate::authority_policy::{classify_question_risk, map_risk_to_freshness_tier, FreshnessTier};
use crate::domain::{ProxyContextPack, ProxyDraft, MAX_PROXY_DRAFT_LIMITATIONS};
use crate::proxy_citations::{build_citations, unsupported_claim_texts, ProxyCitation};
use crate::proxy_claims::{
    claims_meet_coverage_threshold, extract_claims_from_draft, verify_claims_against_pack,
    VerifiedClaim,
};
use chrono::{DateTime, Utc};

pub const MUST_ASK_DRAFT_PREFIX: &str =
    "Must ask human — draft claims are not sufficiently evidence-backed.";
pub const STALE_FRESHNESS_LIMITATION: &str =
    "evidence freshness is insufficient for this question tier";
pub const UNSUPPORTED_CLAIMS_LIMITATION: &str =
    "unsupported claims were refused or downgraded to must-ask";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostVerifyResult {
    pub coverage_ok: bool,
    pub freshness: FreshnessResult,
    pub verified: Vec<VerifiedClaim>,
    pub citations: Vec<ProxyCitation>,
    pub unsupported: Vec<String>,
    pub downgraded: bool,
    pub must_ask: bool,
    /// Critical-tier failures return non-zero from CLI.
    pub hard_fail: bool,
}

/// Apply claim/freshness gates to a generated draft. Fail closed by rewriting
/// confident unsupported drafts into an explicit must-ask message.
pub fn apply_post_provider_verification(
    draft: &mut ProxyDraft,
    pack: &ProxyContextPack,
    question_text: &str,
    now: DateTime<Utc>,
) -> PostVerifyResult {
    let risk = classify_question_risk(question_text);
    let tier = map_risk_to_freshness_tier(risk);
    let claims = extract_claims_from_draft(&draft.draft_text);
    let verified = verify_claims_against_pack(&claims, pack);
    let citations = build_citations(&verified);
    let unsupported = unsupported_claim_texts(&verified);
    let coverage_ok = claims_meet_coverage_threshold(&verified);
    let freshness = evaluate_evidence_freshness(pack, tier, now);

    let critical = matches!(tier, FreshnessTier::Critical);
    let must_ask = !coverage_ok || !freshness.is_sufficient;
    let hard_fail = critical && must_ask;
    let mut downgraded = false;

    if must_ask {
        let mut message = String::from(MUST_ASK_DRAFT_PREFIX);
        if !coverage_ok {
            message.push_str(" Unsupported claims lack evidence.");
        }
        if !freshness.is_sufficient {
            message.push_str(" Evidence freshness is insufficient.");
        }
        if !unsupported.is_empty() {
            message.push_str(" Examples: ");
            message.push_str(
                &unsupported
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | "),
            );
        }
        draft.draft_text = message;
        push_limitation(draft, UNSUPPORTED_CLAIMS_LIMITATION);
        if !freshness.is_sufficient {
            push_limitation(draft, STALE_FRESHNESS_LIMITATION);
        }
        downgraded = true;
    } else if !unsupported.is_empty() {
        push_limitation(
            draft,
            &format!(
                "some claims are unsupported and must not be treated as verified: {}",
                unsupported
                    .iter()
                    .take(2)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(" | ")
            ),
        );
    }

    PostVerifyResult {
        coverage_ok,
        freshness,
        verified,
        citations,
        unsupported,
        downgraded,
        must_ask,
        hard_fail,
    }
}

fn push_limitation(draft: &mut ProxyDraft, limitation: &str) {
    if draft.limitations.iter().any(|item| item == limitation) {
        return;
    }
    if draft.limitations.len() >= MAX_PROXY_DRAFT_LIMITATIONS {
        return;
    }
    draft.limitations.push(limitation.to_string());
}
