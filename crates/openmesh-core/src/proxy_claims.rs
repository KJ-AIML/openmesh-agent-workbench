//! Dev Track 0.1.7.3 — Claim extraction and evidence verification.

use crate::domain::ProxyContextPack;
use serde::{Deserialize, Serialize};

/// Minimum fraction of claims that must be supported or marked inference.
pub const MIN_CLAIM_COVERAGE_RATIO: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind {
    Fact,
    Inference,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimCoverage {
    Supported,
    Unsupported,
    Inference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyClaim {
    pub claim_id: String,
    pub claim_text: String,
    pub claim_kind: ClaimKind,
    pub evidence_refs: Vec<String>,
    pub coverage: ClaimCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedClaim {
    pub claim: ProxyClaim,
    pub matched_evidence_ids: Vec<String>,
    pub verification_status: ClaimCoverage,
}

/// Deterministic sentence-split claim extraction (no LLM).
pub fn extract_claims_from_draft(draft_text: &str) -> Vec<ProxyClaim> {
    let mut claims = Vec::new();
    for (index, sentence) in split_sentences(draft_text).into_iter().enumerate() {
        let trimmed = sentence.trim();
        if trimmed.len() < 8 {
            continue;
        }
        let claim_kind = classify_claim_kind(trimmed);
        claims.push(ProxyClaim {
            claim_id: format!("claim-{}", index + 1),
            claim_text: trimmed.to_string(),
            claim_kind,
            evidence_refs: Vec::new(),
            coverage: ClaimCoverage::Unsupported,
        });
    }
    claims
}

/// Verify claims against context pack evidence index (deterministic alignment).
pub fn verify_claims_against_pack(
    claims: &[ProxyClaim],
    pack: &ProxyContextPack,
) -> Vec<VerifiedClaim> {
    let evidence_labels: Vec<(String, String)> = pack
        .evidence_index
        .iter()
        .map(|entry| (entry.ref_id.clone(), entry.label.to_ascii_lowercase()))
        .collect();

    claims
        .iter()
        .map(|claim| {
            let claim_lower = claim.claim_text.to_ascii_lowercase();
            let claim_tokens = significant_tokens(&claim_lower);
            let matched: Vec<String> = evidence_labels
                .iter()
                .filter(|(_, label)| {
                    if label.is_empty() {
                        return false;
                    }
                    if claim_lower.contains(label.as_str()) || label.contains(&claim_lower) {
                        return true;
                    }
                    let label_tokens = significant_tokens(label);
                    token_overlap(&claim_tokens, &label_tokens) >= 2
                        || (!label_tokens.is_empty()
                            && label_tokens.iter().all(|token| claim_tokens.contains(token)))
                })
                .map(|(ref_id, _)| ref_id.clone())
                .collect();

            let verification_status = if !matched.is_empty() {
                ClaimCoverage::Supported
            } else if claim.claim_kind == ClaimKind::Inference {
                ClaimCoverage::Inference
            } else {
                ClaimCoverage::Unsupported
            };

            VerifiedClaim {
                claim: ProxyClaim {
                    coverage: verification_status,
                    evidence_refs: matched.clone(),
                    ..claim.clone()
                },
                matched_evidence_ids: matched,
                verification_status,
            }
        })
        .collect()
}

fn significant_tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphanumeric())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .filter(|token| {
            !matches!(
                token.as_str(),
                "the" | "and" | "for" | "with" | "from" | "that" | "this" | "are" | "was" | "were"
            )
        })
        .collect()
}

fn token_overlap(left: &[String], right: &[String]) -> usize {
    left.iter().filter(|token| right.contains(token)).count()
}

/// Returns true when coverage meets minimum threshold.
pub fn claims_meet_coverage_threshold(verified: &[VerifiedClaim]) -> bool {
    if verified.is_empty() {
        return true;
    }
    let supported = verified
        .iter()
        .filter(|v| {
            matches!(
                v.verification_status,
                ClaimCoverage::Supported | ClaimCoverage::Inference
            )
        })
        .count();
    (supported as f32 / verified.len() as f32) >= MIN_CLAIM_COVERAGE_RATIO
}

fn classify_claim_kind(sentence: &str) -> ClaimKind {
    let lower = sentence.to_ascii_lowercase();
    if contains_any(&lower, &["maybe", "might", "possibly", "likely", "i think"]) {
        ClaimKind::Inference
    } else if contains_any(&lower, &["unknown", "unclear", "not sure", "cannot determine"]) {
        ClaimKind::Unknown
    } else {
        ClaimKind::Fact
    }
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if ch == '.' || ch == '!' || ch == '?' || ch == '\n' {
            if !current.trim().is_empty() {
                sentences.push(current.trim().to_string());
            }
            current.clear();
        }
    }
    if !current.trim().is_empty() {
        sentences.push(current.trim().to_string());
    }
    sentences
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}
