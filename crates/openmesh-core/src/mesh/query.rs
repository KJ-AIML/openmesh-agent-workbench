//! Dev Track 0.1.14 — Ter × Yo remote peer query (read-only by default).
//!
//! "Ask my teammate's Work Proxy while they are offline" using imported mesh
//! inbox envelopes (and optional relay-received packages). Never promotes
//! foreign evidence into the local WorkEvent ledger.

use crate::authority_freshness::ConfidenceLabel;
use crate::authority_policy::FreshnessTier;
use crate::domain::validate_utc_timestamp;
use crate::mesh::contract::{MeshEnvelope, MESH_DIR};
use crate::mesh::import::{list_inbox_envelope_ids, read_inbox_envelope};
use crate::mesh::peers::{list_peers, read_peer, MeshPeerError, MeshPeerRecord};
use crate::online_proxy::contract::EvidenceFreshnessStatement;
use crate::relay::transport::{read_received_package, received_dir};
use crate::storage::{get_project_dir, read_project, Project};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MESH_QUERY_PROTOCOL_VERSION: &str = "1.0";
pub const MESH_QUERIES_DIR: &str = "mesh/queries";
const QUERY_TEMP: &str = "mesh-query-tmp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MeshRemoteQueryAnswer {
    pub protocol_version: String,
    pub query_id: String,
    pub peer_id: String,
    pub peer_label: String,
    pub question: String,
    pub answer_text: String,
    pub generated_at: String,
    /// Always true for remote peer query (Runtime Architecture §21).
    pub read_only: bool,
    pub freshness: EvidenceFreshnessStatement,
    pub refused: bool,
    #[serde(default)]
    pub envelope_ids: Vec<String>,
    #[serde(default)]
    pub evidence_summaries: Vec<String>,
    #[serde(default)]
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MeshRemoteQueryRequest {
    pub peer: String,
    pub question: String,
    pub query_id: String,
    pub now: DateTime<Utc>,
    pub freshness_tier: FreshnessTier,
    /// When true, also scan relay-received packages for peer envelopes.
    pub include_relay_received: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum MeshQueryError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("peer not found: {0}")]
    PeerNotFound(String),
    #[error("empty question")]
    EmptyQuestion,
    #[error("validation: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("peer registry: {0}")]
    Peer(#[from] MeshPeerError),
}

pub fn queries_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(MESH_QUERIES_DIR)
}

pub fn query_answer_path(project_path: &str, query_id: &str) -> PathBuf {
    queries_dir(project_path).join(format!("{query_id}.json"))
}

/// Resolve a registered peer by id or case-insensitive label.
pub fn resolve_peer(project_path: &str, peer: &str) -> Result<MeshPeerRecord, MeshQueryError> {
    let _ = load_project(project_path)?;
    let key = peer.trim();
    if key.is_empty() {
        return Err(MeshQueryError::PeerNotFound(peer.into()));
    }
    if let Ok(rec) = read_peer(project_path, key) {
        return Ok(rec);
    }
    let peers = list_peers(project_path)?;
    let lower = key.to_ascii_lowercase();
    peers
        .into_iter()
        .find(|p| p.label.to_ascii_lowercase() == lower || p.peer_id.to_ascii_lowercase() == lower)
        .ok_or_else(|| MeshQueryError::PeerNotFound(peer.into()))
}

fn load_project(project_path: &str) -> Result<Project, MeshQueryError> {
    read_project(project_path, "project.json").ok_or(MeshQueryError::ProjectNotInitialized)
}

fn envelope_matches_peer(env: &MeshEnvelope, peer: &MeshPeerRecord) -> bool {
    let from = &env.from_peer;
    if let Some(ws) = &peer.remote_workspace_id {
        if from.workspace_id.as_deref() == Some(ws.as_str()) {
            return true;
        }
    }
    if let Some(pid) = &peer.proxy_profile_id {
        if from.proxy_profile_id.as_deref() == Some(pid.as_str()) {
            return true;
        }
    }
    from.label.eq_ignore_ascii_case(&peer.label)
}

fn collect_peer_envelopes(
    project_path: &str,
    peer: &MeshPeerRecord,
    include_relay: bool,
) -> Result<(Vec<MeshEnvelope>, Vec<String>), MeshQueryError> {
    let mut envelopes = Vec::new();
    let mut limitations = Vec::new();

    for id in list_inbox_envelope_ids(project_path).unwrap_or_default() {
        match read_inbox_envelope(project_path, &id) {
            Ok(env) if envelope_matches_peer(&env, peer) => envelopes.push(env),
            Ok(_) => {}
            Err(_) => limitations.push(format!("failed to read inbox envelope {id}")),
        }
    }

    if include_relay {
        let dir = received_dir(project_path);
        if dir.exists() {
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    let stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or_default();
                    match read_received_package(project_path, stem) {
                        Ok(pkg) => {
                            for env in pkg.envelopes {
                                if envelope_matches_peer(&env, peer)
                                    && !envelopes.iter().any(|e| e.envelope_id == env.envelope_id)
                                {
                                    envelopes.push(env);
                                }
                            }
                        }
                        Err(_) => limitations
                            .push(format!("failed to read relay-received package {stem}")),
                    }
                }
            }
        }
    }

    envelopes.sort_by(|a, b| a.generated_at.cmp(&b.generated_at).then(a.envelope_id.cmp(&b.envelope_id)));
    Ok((envelopes, limitations))
}

fn evaluate_envelope_freshness(
    envelopes: &[MeshEnvelope],
    tier: FreshnessTier,
    now: DateTime<Utc>,
) -> EvidenceFreshnessStatement {
    let mut oldest_age: u64 = 0;
    let mut source_ids = Vec::new();
    for env in envelopes {
        source_ids.push(format!("mesh-envelope:{}", env.envelope_id));
        if let Ok(ts) = DateTime::parse_from_rfc3339(&env.generated_at) {
            let age = (now - ts.with_timezone(&Utc)).num_seconds().max(0) as u64;
            oldest_age = oldest_age.max(age);
        }
    }
    source_ids.sort();
    source_ids.dedup();

    // Simple age thresholds aligned with alpha scaffold intent.
    let max_ok = match tier {
        FreshnessTier::LowImpact => 7 * 24 * 3600,
        FreshnessTier::Standard => 48 * 3600,
        FreshnessTier::Critical => 6 * 3600,
    };
    let is_sufficient = !envelopes.is_empty() && oldest_age <= max_ok;
    let confidence = if envelopes.is_empty() {
        ConfidenceLabel::Insufficient
    } else if oldest_age <= max_ok / 4 {
        ConfidenceLabel::High
    } else if is_sufficient {
        ConfidenceLabel::Medium
    } else {
        ConfidenceLabel::Low
    };
    let mut stale_warnings = Vec::new();
    if envelopes.is_empty() {
        stale_warnings.push("no imported mesh envelopes for peer".into());
    } else if !is_sufficient {
        stale_warnings.push(format!(
            "oldest peer envelope age {oldest_age}s exceeds tier {tier:?} bound {max_ok}s"
        ));
    }
    let statement = if envelopes.is_empty() {
        format!(
            "Evidence freshness: insufficient for tier {:?} — no peer envelopes available. Remote query will not invent evidence.",
            tier
        )
    } else if is_sufficient {
        format!(
            "Evidence freshness: fresh enough for tier {:?} (oldest peer envelope age {}s, confidence {:?}). Remote peer query is read-only.",
            tier, oldest_age, confidence
        )
    } else {
        format!(
            "Evidence freshness: stale for tier {:?} (oldest peer envelope age {}s, confidence {:?}). Remote peer query refuses silent staleness.",
            tier, oldest_age, confidence
        )
    };

    EvidenceFreshnessStatement {
        statement,
        evaluated_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        tier,
        is_sufficient,
        confidence_label: confidence,
        oldest_evidence_age_seconds: oldest_age,
        stale_warnings,
        evidence_source_ids: source_ids,
    }
}

/// Ask a teammate's offline proxy from imported mesh evidence (read-only).
pub fn query_remote_peer_proxy(
    project_path: &str,
    request: &MeshRemoteQueryRequest,
    persist: bool,
) -> Result<MeshRemoteQueryAnswer, MeshQueryError> {
    let _ = load_project(project_path)?;
    if request.question.trim().is_empty() {
        return Err(MeshQueryError::EmptyQuestion);
    }
    let peer = resolve_peer(project_path, &request.peer)?;
    let (envelopes, mut limitations) =
        collect_peer_envelopes(project_path, &peer, request.include_relay_received)?;

    let freshness =
        evaluate_envelope_freshness(&envelopes, request.freshness_tier, request.now);

    let refused = envelopes.is_empty()
        || (!freshness.is_sufficient
            && matches!(
                request.freshness_tier,
                FreshnessTier::Critical | FreshnessTier::Standard
            ));

    let mut evidence_summaries = Vec::new();
    let mut envelope_ids = Vec::new();
    for env in &envelopes {
        envelope_ids.push(env.envelope_id.clone());
        for item in &env.evidence_items {
            if evidence_summaries.len() >= 32 {
                break;
            }
            let line = format!("[{}] {}", env.from_peer.label, item.summary);
            if !evidence_summaries.contains(&line) {
                evidence_summaries.push(line);
            }
        }
        for lim in &env.limitations {
            let line = format!("peer-limitation: {lim}");
            if !limitations.contains(&line) {
                limitations.push(line);
            }
        }
    }
    limitations.push("remote peer query is read-only by default".into());
    limitations.push("foreign evidence is not auto-merged into the local WorkEvent ledger".into());
    limitations.sort();
    limitations.dedup();

    let answer_text = if refused && envelopes.is_empty() {
        format!(
            "Cannot answer for peer '{}' ({}): no imported mesh envelopes (and no matching relay-received packages).\n\n{}\n\nQuestion: {}\n\nImport a mesh envelope or receive a relay package from this peer first.",
            peer.label, peer.peer_id, freshness.statement, request.question
        )
    } else if refused {
        format!(
            "Cannot answer from peer '{}' offline proxy: evidence is not fresh enough.\n\n{}\n\nQuestion: {}",
            peer.label, freshness.statement, request.question
        )
    } else {
        let bullets = if evidence_summaries.is_empty() {
            "(envelopes present but no evidence item summaries)".to_string()
        } else {
            evidence_summaries
                .iter()
                .map(|s| format!("- {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "Remote peer query (read-only) against {}'s offline Work Proxy.\n\nQuestion: {}\n\nAttributed evidence from imported mesh envelopes:\n{}\n\n{}\n\nThis draft does not execute actions and does not write to the local ledger.",
            peer.label, request.question, bullets, freshness.statement
        )
    };

    let answer = MeshRemoteQueryAnswer {
        protocol_version: MESH_QUERY_PROTOCOL_VERSION.into(),
        query_id: request.query_id.clone(),
        peer_id: peer.peer_id.clone(),
        peer_label: peer.label.clone(),
        question: request.question.clone(),
        answer_text,
        generated_at: request
            .now
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        read_only: true,
        freshness,
        refused,
        envelope_ids,
        evidence_summaries,
        limitations,
    };

    validate_mesh_remote_query_answer(&answer)
        .map_err(|e| MeshQueryError::Validation(e.to_string()))?;

    if persist {
        write_query_answer(project_path, &answer)?;
    }
    Ok(answer)
}

pub fn validate_mesh_remote_query_answer(
    a: &MeshRemoteQueryAnswer,
) -> Result<(), MeshQueryError> {
    if a.protocol_version != MESH_QUERY_PROTOCOL_VERSION {
        return Err(MeshQueryError::Validation("protocol".into()));
    }
    if a.query_id.trim().is_empty() || a.query_id.contains("..") || a.query_id.contains('/') {
        return Err(MeshQueryError::Validation("query_id".into()));
    }
    if a.question.trim().is_empty() {
        return Err(MeshQueryError::EmptyQuestion);
    }
    if a.answer_text.trim().is_empty() {
        return Err(MeshQueryError::Validation("answer_text".into()));
    }
    if !a.read_only {
        return Err(MeshQueryError::Validation(
            "remote peer query must be read_only=true".into(),
        ));
    }
    validate_utc_timestamp(&a.generated_at)
        .map_err(|e| MeshQueryError::Validation(e.to_string()))?;
    if a.freshness.statement.trim().is_empty() {
        return Err(MeshQueryError::Validation("freshness statement required".into()));
    }
    Ok(())
}

pub fn write_query_answer(
    project_path: &str,
    answer: &MeshRemoteQueryAnswer,
) -> Result<(), MeshQueryError> {
    let _ = load_project(project_path)?;
    validate_mesh_remote_query_answer(answer)?;
    let dir = queries_dir(project_path);
    fs::create_dir_all(&dir).map_err(|_| MeshQueryError::Io)?;
    let path = query_answer_path(project_path, &answer.query_id);
    write_json_atomic(&path, answer)
}

pub fn read_query_answer(
    project_path: &str,
    query_id: &str,
) -> Result<MeshRemoteQueryAnswer, MeshQueryError> {
    let _ = load_project(project_path)?;
    let path = query_answer_path(project_path, query_id);
    if !path.exists() {
        return Err(MeshQueryError::Validation("query answer missing".into()));
    }
    let raw = fs::read_to_string(&path).map_err(|_| MeshQueryError::Io)?;
    let answer: MeshRemoteQueryAnswer =
        serde_json::from_str(&raw).map_err(|_| MeshQueryError::Validation("malformed".into()))?;
    validate_mesh_remote_query_answer(&answer)?;
    Ok(answer)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), MeshQueryError> {
    let parent = path.parent().ok_or(MeshQueryError::Io)?;
    fs::create_dir_all(parent).map_err(|_| MeshQueryError::Io)?;
    let temp = path.with_extension(QUERY_TEMP);
    let mut json = serde_json::to_string_pretty(value).map_err(|_| MeshQueryError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| MeshQueryError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| MeshQueryError::Io)?;
        file.sync_all().map_err(|_| MeshQueryError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| MeshQueryError::Io)
}

// silence unused MESH_DIR warning if not used
#[allow(dead_code)]
fn _mesh_dir_marker() -> &'static str {
    MESH_DIR
}
