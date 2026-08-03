//! LAN live ask answered via the peer's Agent Engine (read-only).
//!
//! Requires the peer to have a configured API key / provider. Does not paste
//! LocalScaffold prose. Does not write project files from remote asks.

use crate::agent_engine::{run_live_ask, LiveAskError, LiveAskRequest};
use crate::authority_freshness::ConfidenceLabel;
use crate::authority_policy::FreshnessTier;
use crate::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use crate::domain::CatchUpWindow;
use crate::mesh::query::{
    validate_mesh_remote_query_answer, MeshRemoteQueryAnswer, MESH_QUERY_PROTOCOL_VERSION,
};
use crate::online_proxy::contract::{
    build_freshness_statement_text, EvidenceFreshnessStatement,
};
use crate::authority_freshness::evaluate_evidence_freshness;
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
    #[error("{0}")]
    Live(#[from] LiveAskError),
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed: {0}")]
    Io(String),
}

impl LanAskError {
    pub fn code(&self) -> &'static str {
        match self {
            LanAskError::ProjectNotInitialized => "project_not_initialized",
            LanAskError::EmptyQuestion => "empty_question",
            LanAskError::Live(e) => e.code(),
            LanAskError::Validation(_) => "validation",
            LanAskError::Io(_) => "io",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            LanAskError::Live(LiveAskError::MissingApiKey) => 503,
            LanAskError::EmptyQuestion | LanAskError::ProjectNotInitialized => 400,
            LanAskError::Validation(_) => 400,
            _ => 502,
        }
    }

    pub fn to_json_body(&self) -> String {
        serde_json::json!({
            "error": self.to_string(),
            "code": self.code(),
        })
        .to_string()
    }
}

/// Answer a live LAN ask using the peer's Agent Engine (read-only draft text).
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

    let now = Utc::now();
    let (freshness, context_prefix) = build_optional_freshness_context(project_path, request.tier, now);

    let live_req = LiveAskRequest {
        question: request.question.clone(),
        context_prefix,
        provider_name: None,
        model: None,
        base_url: None,
        system_extra: Some(
            "This question arrived over trusted-LAN HTTP from a peer. Answer from this host's local workspace only. Do not claim WAN mesh or e2e encryption.".into(),
        ),
    };

    let engine = run_live_ask(project_path, &live_req)?;

    let mut limitations = vec![
        "lan live ask is read-only by default".into(),
        "answered via peer Agent Engine (not LocalScaffold)".into(),
        "trusted-LAN alpha: no end-to-end encryption beyond local network trust".into(),
        "foreign evidence is not auto-merged into the local WorkEvent ledger".into(),
        format!("model={}", engine.model),
        format!("provider={}", engine.provider),
    ];
    if !engine.tool_steps.is_empty() {
        limitations.push(format!("tool_steps={}", engine.tool_steps.len()));
    }

    let answer = MeshRemoteQueryAnswer {
        protocol_version: MESH_QUERY_PROTOCOL_VERSION.into(),
        query_id: request.query_id.clone(),
        peer_id,
        peer_label: owner_label,
        question: request.question.clone(),
        answer_text: engine.assistant_text,
        generated_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        read_only: true,
        freshness,
        refused: engine.refused,
        envelope_ids: vec![],
        evidence_summaries: vec![],
        limitations,
    };

    validate_mesh_remote_query_answer(&answer)
        .map_err(|e| LanAskError::Validation(e.to_string()))?;
    Ok(answer)
}

fn build_optional_freshness_context(
    project_path: &str,
    tier: FreshnessTier,
    now: DateTime<Utc>,
) -> (EvidenceFreshnessStatement, Option<String>) {
    let until = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since = (now - Duration::hours(24)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let options = ProxyContextPackBuildOptions {
        generated_at: until.clone(),
        ..ProxyContextPackBuildOptions::default()
    };
    let window = CatchUpWindow { since, until: until.clone() };

    match build_proxy_context_pack(project_path, window, options) {
        Ok(pack) => {
            let eval = evaluate_evidence_freshness(&pack, tier, now);
            let statement = build_freshness_statement_text(&eval, tier);
            let source_ids: Vec<String> = pack
                .evidence_index
                .iter()
                .map(|e| e.ref_id.clone())
                .take(16)
                .collect();
            let freshness = EvidenceFreshnessStatement {
                statement: statement.clone(),
                evaluated_at: until,
                tier,
                is_sufficient: eval.is_sufficient,
                confidence_label: eval.confidence_label,
                oldest_evidence_age_seconds: eval.oldest_evidence_age_seconds,
                stale_warnings: eval.stale_warnings,
                evidence_source_ids: source_ids,
            };
            let prefix = Some(format!(
                "Optional local evidence context (disclosure only; answer via Agent Engine):\n{statement}"
            ));
            (freshness, prefix)
        }
        Err(e) => {
            let statement = format!(
                "Evidence freshness: unavailable for tier {:?} — cannot build local context pack ({e}). Live ask will still use Agent Engine tools when possible.",
                tier
            );
            let freshness = EvidenceFreshnessStatement {
                statement: statement.clone(),
                evaluated_at: until,
                tier,
                is_sufficient: false,
                confidence_label: ConfidenceLabel::Insufficient,
                oldest_evidence_age_seconds: 0,
                stale_warnings: vec![e.to_string()],
                evidence_source_ids: vec![],
            };
            (freshness, Some(statement))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_engine::{
        run_live_ask_with_provider, AgentDefinition, AssistantTurn, ScriptedProvider,
        StubToolExecutor, LIVE_ASK_SYSTEM_PROMPT,
    };
    use crate::storage::init_project;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU64, Ordering};

    static N: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> String {
        let n = N.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-lan-ask-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn lan_ask_error_missing_key_is_structured() {
        let err = LanAskError::Live(LiveAskError::MissingApiKey);
        assert_eq!(err.code(), "missing_api_key");
        assert_eq!(err.http_status(), 503);
        assert!(err.to_json_body().contains("missing_api_key"));
    }

    #[test]
    fn scripted_engine_answer_maps_to_mesh_shape() {
        // Exercise the same composition path as production without network:
        // LiveAskRequest → scripted provider → MeshRemoteQueryAnswer fields.
        let project = temp_project();
        let provider = ScriptedProvider::new(vec![AssistantTurn {
            content: "Peer says: sprint is on track.".into(),
            tool_calls: vec![],
        }]);
        let executor = StubToolExecutor {
            responses: BTreeMap::new(),
        };
        let mut def = AgentDefinition::default_workspace_agent("stub");
        def.system_prompt = LIVE_ASK_SYSTEM_PROMPT.into();
        def.tool_allowlist = vec!["__none__".into()];
        let live = LiveAskRequest {
            question: "Status?".into(),
            context_prefix: Some("Evidence freshness: fresh enough for tier LowImpact.".into()),
            provider_name: None,
            model: Some("stub".into()),
            base_url: None,
            system_extra: None,
        };
        let engine = run_live_ask_with_provider(&def, &live, &provider, &executor).unwrap();
        assert!(engine.assistant_text.contains("on track"));

        let now = Utc::now();
        let answer = MeshRemoteQueryAnswer {
            protocol_version: MESH_QUERY_PROTOCOL_VERSION.into(),
            query_id: "lan-ask-test".into(),
            peer_id: "lan-test".into(),
            peer_label: "Tester".into(),
            question: "Status?".into(),
            answer_text: engine.assistant_text,
            generated_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            read_only: true,
            freshness: EvidenceFreshnessStatement {
                statement: "Evidence freshness: fresh enough for tier LowImpact (age 0s).".into(),
                evaluated_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                tier: FreshnessTier::LowImpact,
                is_sufficient: true,
                confidence_label: ConfidenceLabel::High,
                oldest_evidence_age_seconds: 0,
                stale_warnings: vec![],
                evidence_source_ids: vec![],
            },
            refused: false,
            envelope_ids: vec![],
            evidence_summaries: vec![],
            limitations: vec!["answered via peer Agent Engine (not LocalScaffold)".into()],
        };
        validate_mesh_remote_query_answer(&answer).unwrap();
        assert!(!answer.answer_text.contains("local-scaffold"));
        let _ = std::fs::remove_dir_all(&project);
    }
}
