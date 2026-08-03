//! Live ask answered from the host project's local evidence (read-only).

use crate::authority_freshness::ConfidenceLabel;
use crate::authority_policy::FreshnessTier;
use crate::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use crate::domain::CatchUpWindow;
use crate::mesh::query::{
    validate_mesh_remote_query_answer, MeshRemoteQueryAnswer, MESH_QUERY_PROTOCOL_VERSION,
};
use crate::online_proxy::ask::{ask_online_proxy, OnlineProxyAskRequest};
use crate::online_proxy::contract::{
    EvidenceFreshnessStatement, OnlineProxyConfig, OnlineProxyMode, ONLINE_PROXY_PROTOCOL_VERSION,
};
use crate::online_proxy::storage::{read_config, OnlineProxyStorageError};
use crate::profile::read_work_proxy_profile;
use crate::storage::{read_project, Project};
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct LanAskRequest {
    pub question: String,
    pub tier: FreshnessTier,
    pub query_id: String,
}

#[derive(Debug, Error)]
pub enum LanAskError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("empty question")]
    EmptyQuestion,
    #[error("validation: {0}")]
    Validation(String),
    #[error("ask failed: {0}")]
    Ask(String),
    #[error("io failed: {0}")]
    Io(String),
}

/// Answer a live LAN ask using local proxy evidence (read-only draft).
pub fn answer_live_ask(
    project_path: &str,
    request: &LanAskRequest,
) -> Result<MeshRemoteQueryAnswer, LanAskError> {
    if request.question.trim().is_empty() {
        return Err(LanAskError::EmptyQuestion);
    }
    let project: Project = read_project(project_path, "project.json")
        .ok_or(LanAskError::ProjectNotInitialized)?;

    let owner_label = read_work_proxy_profile(project_path)
        .map(|p| p.owner_label)
        .unwrap_or_else(|_| "local-operator".into());
    let peer_id = format!("lan-{}", project.id);

    let cfg = match read_config(project_path) {
        Ok(c) => c,
        Err(OnlineProxyStorageError::ConfigMissing) => OnlineProxyConfig {
            protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
            proxy_id: format!("online-{}", project.id),
            workspace_id: project.id.clone(),
            owner_label: owner_label.clone(),
            mode: OnlineProxyMode::LocalScaffold,
            default_freshness_tier: FreshnessTier::Standard,
            use_relay_received: true,
            created_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            updated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        },
        Err(e) => return Err(LanAskError::Io(e.to_string())),
    };

    let now = Utc::now();
    let until = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since = (now - Duration::hours(24)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let options = ProxyContextPackBuildOptions {
        generated_at: until.clone(),
        ..ProxyContextPackBuildOptions::default()
    };
    let window = CatchUpWindow { since, until };
    let pack = match build_proxy_context_pack(project_path, window, options) {
        Ok(p) => p,
        Err(e) => {
            return Ok(scaffold_refused_answer(
                &request.query_id,
                &peer_id,
                &owner_label,
                &request.question,
                request.tier,
                &now,
                &format!("cannot build local context pack: {e}"),
            ));
        }
    };

    let answer_id = request.query_id.clone();
    let online_req = OnlineProxyAskRequest {
        question: request.question.clone(),
        now,
        answer_id: answer_id.clone(),
        freshness_tier: Some(request.tier),
    };
    // Do not persist online-proxy answers for LAN asks (ephemeral read-only).
    let online = match ask_online_proxy(project_path, &cfg, &pack, &online_req, false) {
        Ok(a) => a,
        Err(e) => {
            return Ok(scaffold_refused_answer(
                &request.query_id,
                &peer_id,
                &owner_label,
                &request.question,
                request.tier,
                &now,
                &format!("local proxy ask failed: {e}"),
            ));
        }
    };

    let mut limitations = vec![
        "lan live ask is read-only by default".into(),
        "trusted-LAN alpha: no end-to-end encryption beyond local network trust".into(),
        "foreign evidence is not auto-merged into the local WorkEvent ledger".into(),
    ];
    if online.refused {
        limitations.push("answer refused due to freshness policy".into());
    }

    let evidence_summaries: Vec<String> = online
        .freshness
        .evidence_source_ids
        .iter()
        .take(16)
        .cloned()
        .collect();
    let answer = MeshRemoteQueryAnswer {
        protocol_version: MESH_QUERY_PROTOCOL_VERSION.into(),
        query_id: request.query_id.clone(),
        peer_id,
        peer_label: owner_label,
        question: request.question.clone(),
        answer_text: format!(
            "LAN live ask (read-only) against {}'s local Work Proxy.\n\n{}",
            cfg.owner_label, online.answer_text
        ),
        generated_at: online.generated_at,
        read_only: true,
        freshness: online.freshness,
        refused: online.refused,
        envelope_ids: vec![],
        evidence_summaries,
        limitations,
    };

    validate_mesh_remote_query_answer(&answer)
        .map_err(|e| LanAskError::Validation(e.to_string()))?;
    Ok(answer)
}

fn scaffold_refused_answer(
    query_id: &str,
    peer_id: &str,
    peer_label: &str,
    question: &str,
    tier: FreshnessTier,
    now: &DateTime<Utc>,
    reason: &str,
) -> MeshRemoteQueryAnswer {
    let generated_at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let freshness = EvidenceFreshnessStatement {
        statement: format!(
            "Evidence freshness: insufficient for tier {:?} — {reason}. LAN live ask will not invent evidence.",
            tier
        ),
        evaluated_at: generated_at.clone(),
        tier,
        is_sufficient: false,
        confidence_label: ConfidenceLabel::Insufficient,
        oldest_evidence_age_seconds: 0,
        stale_warnings: vec![reason.to_string()],
        evidence_source_ids: vec![],
    };
    MeshRemoteQueryAnswer {
        protocol_version: MESH_QUERY_PROTOCOL_VERSION.into(),
        query_id: query_id.to_string(),
        peer_id: peer_id.to_string(),
        peer_label: peer_label.to_string(),
        question: question.to_string(),
        answer_text: format!(
            "Cannot answer LAN live ask (read-only): {reason}\n\nQuestion: {question}\n\nInitialize a Work Proxy Profile and local evidence, then retry."
        ),
        generated_at,
        read_only: true,
        freshness,
        refused: true,
        envelope_ids: vec![],
        evidence_summaries: vec![],
        limitations: vec![
            "lan live ask is read-only by default".into(),
            "trusted-LAN alpha: no end-to-end encryption beyond local network trust".into(),
            reason.to_string(),
        ],
    }
}
