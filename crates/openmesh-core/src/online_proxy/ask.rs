//! Ask the always-online proxy with mandatory freshness disclosure.

use crate::authority_freshness::{evaluate_evidence_freshness, FreshnessResult};
use crate::authority_policy::FreshnessTier;
use crate::domain::ProxyContextPack;
use crate::online_proxy::contract::{
    build_freshness_statement_text, validate_online_proxy_answer, EvidenceFreshnessStatement,
    OnlineProxyAnswer, OnlineProxyConfig, ONLINE_PROXY_PROTOCOL_VERSION,
};
use crate::online_proxy::storage::{write_answer, OnlineProxyStorageError};
use crate::relay::transport::received_dir;
use chrono::{DateTime, Utc};
use std::fs;

#[derive(Debug, Clone)]
pub struct OnlineProxyAskRequest {
    pub question: String,
    pub now: DateTime<Utc>,
    pub answer_id: String,
    /// Optional override tier (else config default).
    pub freshness_tier: Option<FreshnessTier>,
}

#[derive(Debug, thiserror::Error)]
pub enum OnlineProxyAskError {
    #[error("validation: {0}")]
    Validation(String),
    #[error("storage: {0}")]
    Storage(#[from] OnlineProxyStorageError),
    #[error("empty question")]
    EmptyQuestion,
}

/// Produce an always-online answer with explicit freshness; refuse if critical/stale.
pub fn ask_online_proxy(
    project_path: &str,
    config: &OnlineProxyConfig,
    pack: &ProxyContextPack,
    request: &OnlineProxyAskRequest,
    persist: bool,
) -> Result<OnlineProxyAnswer, OnlineProxyAskError> {
    if request.question.trim().is_empty() {
        return Err(OnlineProxyAskError::EmptyQuestion);
    }

    let tier = request
        .freshness_tier
        .unwrap_or(config.default_freshness_tier);
    let freshness_eval = evaluate_evidence_freshness(pack, tier, request.now);

    let mut source_ids: Vec<String> = pack
        .evidence_index
        .iter()
        .map(|e| e.ref_id.clone())
        .collect();

    if config.use_relay_received {
        source_ids.extend(list_received_package_ids(project_path));
    }
    source_ids.sort();
    source_ids.dedup();
    source_ids.truncate(64);

    let statement_text = build_freshness_statement_text(&freshness_eval, tier);
    let freshness = EvidenceFreshnessStatement {
        statement: statement_text,
        evaluated_at: request
            .now
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        tier,
        is_sufficient: freshness_eval.is_sufficient,
        confidence_label: freshness_eval.confidence_label,
        oldest_evidence_age_seconds: freshness_eval.oldest_evidence_age_seconds,
        stale_warnings: freshness_eval.stale_warnings.clone(),
        evidence_source_ids: source_ids,
    };

    let refused = !freshness_eval.is_sufficient
        && matches!(tier, FreshnessTier::Critical | FreshnessTier::Standard);

    let answer_text = if refused {
        format!(
            "Cannot answer from always-online proxy: evidence is not fresh enough.\n\n{}\n\nQuestion was: {}",
            freshness.statement, request.question
        )
    } else {
        // Deterministic alpha scaffold answer (no live model required for gate).
        format!(
            "Always-online proxy ({}) draft answer for: {}\n\nBased on available local{} evidence. This is non-executing draft text.\n\n{}",
            match config.mode {
                crate::online_proxy::contract::OnlineProxyMode::LocalScaffold => "local-scaffold",
                crate::online_proxy::contract::OnlineProxyMode::CloudScaffold => "cloud-scaffold",
            },
            request.question,
            if config.use_relay_received {
                "+relay-received"
            } else {
                ""
            },
            freshness.statement
        )
    };

    let answer = OnlineProxyAnswer {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        answer_id: request.answer_id.clone(),
        proxy_id: config.proxy_id.clone(),
        workspace_id: config.workspace_id.clone(),
        question: request.question.clone(),
        answer_text,
        generated_at: request
            .now
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        freshness,
        refused,
        mode: config.mode,
    };

    validate_online_proxy_answer(&answer)
        .map_err(|e| OnlineProxyAskError::Validation(e.to_string()))?;

    if persist {
        write_answer(project_path, &answer)?;
    }
    Ok(answer)
}

fn list_received_package_ids(project_path: &str) -> Vec<String> {
    let dir = received_dir(project_path);
    if !dir.exists() {
        return Vec::new();
    }
    let mut ids = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    ids.push(format!("relay-received:{stem}"));
                }
            }
        }
    }
    ids.sort();
    ids
}

// re-export for tests
#[allow(dead_code)]
pub fn freshness_result_for_tests(
    pack: &ProxyContextPack,
    tier: FreshnessTier,
    now: DateTime<Utc>,
) -> FreshnessResult {
    evaluate_evidence_freshness(pack, tier, now)
}
