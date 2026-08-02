//! Dev Track 0.1.13 — Desktop Continuity Surfaces.
//!
//! Tauri IPC peers over `openmesh-core` for pending/digest/mesh/relay/online-proxy.
//! Read-first; limited write for online-proxy init/ask only.

use chrono::{Duration, Utc};
use openmesh_core::authority_policy::FreshnessTier;
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::continuity::{
    current_state_projection_path, load_continuity_input_snapshot, read_current_state_projection,
    rebuild_current_state_projection,
};
use openmesh_core::domain::{CatchUpWindow, CurrentStateProjection};
use openmesh_core::mesh::peers::{list_peers, MeshPeerRecord};
use openmesh_core::mesh::query::{
    query_remote_peer_proxy, MeshRemoteQueryAnswer, MeshRemoteQueryRequest,
};
use openmesh_core::mesh::view::{list_envelope_summaries, MeshEnvelopeSummary, MeshMailbox};
use openmesh_core::online_proxy::{
    ask_online_proxy, read_config, write_config, OnlineProxyAnswer, OnlineProxyAskRequest,
    OnlineProxyConfig, OnlineProxyMode, OnlineProxyStorageError, ONLINE_PROXY_PROTOCOL_VERSION,
};
use openmesh_core::profile::read_work_proxy_profile;
use openmesh_core::relay::audit::list_audit_events;
use openmesh_core::relay::RelayAuditEvent;
use openmesh_core::team::{list_team_members, read_team_workspace, TeamMember, TeamWorkspace};
use openmesh_core::team_cloud::{
    build_sync_scaffold, read_team_cloud, TeamCloudConfig, TeamCloudSyncPlan,
};
use openmesh_core::trust_admin::{
    list_audit_events as list_trust_audit_events, read_trust_policy, AdminAuditEvent,
    TeamTrustPolicy,
};
use openmesh_core::return_digest::{
    build_pending_questions_view, build_return_digest, PendingQuestionsView, ReturnDigest,
};
use openmesh_core::storage::{read_project, Project};
use serde::{Deserialize, Serialize};

fn load_current_state(project_path: &str) -> Result<CurrentStateProjection, String> {
    if current_state_projection_path(project_path).exists() {
        read_current_state_projection(project_path).map_err(|e| e.to_string())
    } else {
        rebuild_current_state_projection(project_path).map_err(|e| e.to_string())
    }
}

fn default_window(since_hours: Option<u64>) -> CatchUpWindow {
    let now = Utc::now();
    let hours = since_hours.unwrap_or(24).max(1) as i64;
    let until = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since = (now - Duration::hours(hours)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    CatchUpWindow { since, until }
}

/// Unified pending questions ("what needs me").
#[tauri::command]
pub fn continuity_pending(project_path: String) -> Result<PendingQuestionsView, String> {
    let snapshot = load_continuity_input_snapshot(&project_path).map_err(|e| e.to_string())?;
    let current_state = load_current_state(&project_path)?;
    build_pending_questions_view(&project_path, &snapshot, &current_state)
        .map_err(|e| e.to_string())
}

/// Return digest for an absence window (default last 24h).
#[tauri::command]
pub fn continuity_digest(
    project_path: String,
    since_hours: Option<u64>,
) -> Result<ReturnDigest, String> {
    let snapshot = load_continuity_input_snapshot(&project_path).map_err(|e| e.to_string())?;
    let current_state = load_current_state(&project_path)?;
    let window = default_window(since_hours);
    build_return_digest(&project_path, &snapshot, &current_state, &window)
        .map_err(|e| e.to_string())
}

/// List mesh peer registry entries.
#[tauri::command]
pub fn mesh_list_peers(project_path: String) -> Result<Vec<MeshPeerRecord>, String> {
    list_peers(&project_path).map_err(|e| e.to_string())
}

/// List mesh envelope summaries (inbox + outbox by default).
#[tauri::command]
pub fn mesh_list_envelopes(
    project_path: String,
    mailbox: Option<String>,
) -> Result<Vec<MeshEnvelopeSummary>, String> {
    let mb = match mailbox.as_deref() {
        None | Some("") | Some("all") => None,
        Some("inbox") => Some(MeshMailbox::Inbox),
        Some("outbox") => Some(MeshMailbox::Outbox),
        Some(other) => return Err(format!("unknown mailbox: {other}")),
    };
    list_envelope_summaries(&project_path, mb).map_err(|e| e.to_string())
}

/// List relay audit events (newest-first already from core).
#[tauri::command]
pub fn relay_list_audit(project_path: String) -> Result<Vec<RelayAuditEvent>, String> {
    list_audit_events(&project_path).map_err(|e| e.to_string())
}

/// Online-proxy config status (None when not initialized).
#[tauri::command]
pub fn online_proxy_status(project_path: String) -> Result<Option<OnlineProxyConfig>, String> {
    match read_config(&project_path) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(OnlineProxyStorageError::ConfigMissing) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineProxyInitRequest {
    pub owner_label: Option<String>,
    pub mode: Option<String>,
    pub use_relay_received: Option<bool>,
}

/// Initialize always-online proxy config for the project.
#[tauri::command]
pub fn online_proxy_init(
    project_path: String,
    request: OnlineProxyInitRequest,
) -> Result<OnlineProxyConfig, String> {
    let project: Project = read_project(&project_path, "project.json")
        .ok_or_else(|| "project not initialized".to_string())?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let owner = read_work_proxy_profile(&project_path)
        .map(|p| p.owner_label)
        .unwrap_or_else(|_| {
            request
                .owner_label
                .clone()
                .unwrap_or_else(|| "local-operator".into())
        });
    let mode = match request.mode.as_deref() {
        None | Some("local-scaffold") | Some("LocalScaffold") => OnlineProxyMode::LocalScaffold,
        Some("cloud-scaffold") | Some("CloudScaffold") => OnlineProxyMode::CloudScaffold,
        Some(other) => return Err(format!("unknown mode: {other}")),
    };
    let cfg = OnlineProxyConfig {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        proxy_id: format!("online-{}", project.id),
        workspace_id: project.id,
        owner_label: owner,
        mode,
        default_freshness_tier: FreshnessTier::Standard,
        use_relay_received: request.use_relay_received.unwrap_or(true),
        created_at: now.clone(),
        updated_at: now,
    };
    write_config(&project_path, &cfg).map_err(|e| e.to_string())?;
    Ok(cfg)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnlineProxyAskUiRequest {
    pub question: String,
    pub tier: Option<String>,
    pub answer_id: Option<String>,
}

/// Ask always-online proxy with mandatory freshness disclosure.
#[tauri::command]
pub fn online_proxy_ask(
    project_path: String,
    request: OnlineProxyAskUiRequest,
) -> Result<OnlineProxyAnswer, String> {
    let cfg = read_config(&project_path).map_err(|e| e.to_string())?;
    let now = Utc::now();
    let until = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since = (now - Duration::hours(24)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let options = ProxyContextPackBuildOptions {
        generated_at: until.clone(),
        ..ProxyContextPackBuildOptions::default()
    };
    let window = CatchUpWindow { since, until };
    let pack = build_proxy_context_pack(&project_path, window, options).map_err(|e| e.to_string())?;
    let tier = match request.tier.as_deref() {
        None | Some("standard") | Some("Standard") => FreshnessTier::Standard,
        Some("low-impact") | Some("LowImpact") => FreshnessTier::LowImpact,
        Some("critical") | Some("Critical") => FreshnessTier::Critical,
        Some(other) => return Err(format!("unknown tier: {other}")),
    };
    let answer_id = request
        .answer_id
        .clone()
        .unwrap_or_else(|| format!("ans-{}", now.format("%Y%m%dT%H%M%SZ")));
    let req = OnlineProxyAskRequest {
        question: request.question,
        now,
        answer_id,
        freshness_tier: Some(tier),
    };
    ask_online_proxy(&project_path, &cfg, &pack, &req, true).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshQueryUiRequest {
    pub peer: String,
    pub question: String,
    pub tier: Option<String>,
    pub query_id: Option<String>,
    pub include_relay_received: Option<bool>,
}

/// Ask a teammate's offline Work Proxy (read-only; 0.1.14 Ter×Yo).
#[tauri::command]
pub fn mesh_query_peer(
    project_path: String,
    request: MeshQueryUiRequest,
) -> Result<MeshRemoteQueryAnswer, String> {
    let now = Utc::now();
    let tier = match request.tier.as_deref() {
        None | Some("standard") | Some("Standard") => FreshnessTier::Standard,
        Some("low-impact") | Some("LowImpact") => FreshnessTier::LowImpact,
        Some("critical") | Some("Critical") => FreshnessTier::Critical,
        Some(other) => return Err(format!("unknown tier: {other}")),
    };
    let query_id = request
        .query_id
        .clone()
        .unwrap_or_else(|| format!("mq-{}", now.format("%Y%m%dT%H%M%SZ")));
    let req = MeshRemoteQueryRequest {
        peer: request.peer,
        question: request.question,
        query_id,
        now,
        freshness_tier: tier,
        include_relay_received: request.include_relay_received.unwrap_or(true),
    };
    query_remote_peer_proxy(&project_path, &req, true).map_err(|e| e.to_string())
}

/// Lightweight hub summary for the Continuity page header.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityHubSummary {
    pub open_pending_count: u32,
    pub peer_count: usize,
    pub envelope_count: usize,
    pub audit_event_count: usize,
    pub online_proxy_initialized: bool,
}

/// Team workspace show (0.1.15) — None when not initialized.
#[tauri::command]
pub fn team_workspace_status(project_path: String) -> Result<Option<TeamWorkspace>, String> {
    match read_team_workspace(&project_path) {
        Ok(ws) => Ok(Some(ws)),
        Err(openmesh_core::team::TeamStorageError::Missing) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn team_list_members(project_path: String) -> Result<Vec<TeamMember>, String> {
    list_team_members(&project_path).map_err(|e| e.to_string())
}

/// Team Cloud Beta (0.1.16) — None when not initialized.
#[tauri::command]
pub fn team_cloud_status(project_path: String) -> Result<Option<TeamCloudConfig>, String> {
    match read_team_cloud(&project_path) {
        Ok(cfg) => Ok(Some(cfg)),
        Err(openmesh_core::team_cloud::TeamCloudStorageError::Missing) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Dry-run selective sync scaffold (no network upload).
#[tauri::command]
pub fn team_cloud_sync_scaffold(project_path: String) -> Result<TeamCloudSyncPlan, String> {
    build_sync_scaffold(&project_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn continuity_hub_summary(project_path: String) -> Result<ContinuityHubSummary, String> {
    let open_pending_count = continuity_pending(project_path.clone())
        .map(|v| v.open_count)
        .unwrap_or(0);
    let peer_count = mesh_list_peers(project_path.clone())
        .map(|p| p.len())
        .unwrap_or(0);
    let envelope_count = mesh_list_envelopes(project_path.clone(), None)
        .map(|e| e.len())
        .unwrap_or(0);
    let audit_event_count = relay_list_audit(project_path.clone())
        .map(|a| a.len())
        .unwrap_or(0);
    let online_proxy_initialized = online_proxy_status(project_path)?.is_some();
    Ok(ContinuityHubSummary {
        open_pending_count,
        peer_count,
        envelope_count,
        audit_event_count,
        online_proxy_initialized,
    })
}

/// Trust Admin Beta (0.1.17) — None when not initialized.
#[tauri::command]
pub fn team_trust_policy_status(project_path: String) -> Result<Option<TeamTrustPolicy>, String> {
    match read_trust_policy(&project_path) {
        Ok(p) => Ok(Some(p)),
        Err(openmesh_core::trust_admin::TrustAdminStorageError::Missing) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn team_trust_audit_list(
    project_path: String,
    limit: Option<usize>,
) -> Result<Vec<AdminAuditEvent>, String> {
    list_trust_audit_events(&project_path, limit).map_err(|e| e.to_string())
}

