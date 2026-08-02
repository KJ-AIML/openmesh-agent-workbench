//! Offline GitHub-shaped evidence producer (stub — no live API).

use crate::connectors::contract::{
    validate_connector_run, ConnectorDescriptor, ConnectorKind, ConnectorRun, EvidenceItemKind,
    ExternalEvidenceItem, CONNECTOR_PROTOCOL_VERSION,
};
use chrono::Utc;

#[derive(Debug, thiserror::Error)]
pub enum GithubStubError {
    #[error("connector is not github-stub")]
    WrongKind,
    #[error("connector disabled")]
    Disabled,
    #[error("validation: {0}")]
    Validation(String),
}

/// Collect stub GitHub-shaped evidence for a registered connector.
///
/// Produces deterministic offline items derived from `external_ref` / connector id.
/// Does **not** call GitHub network APIs and does **not** mutate any SoR.
pub fn collect_github_stub(descriptor: &ConnectorDescriptor) -> Result<ConnectorRun, GithubStubError> {
    if !matches!(descriptor.kind, ConnectorKind::GithubStub) {
        return Err(GithubStubError::WrongKind);
    }
    if !descriptor.enabled {
        return Err(GithubStubError::Disabled);
    }
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let repo = descriptor
        .external_ref
        .clone()
        .unwrap_or_else(|| format!("local/{}", descriptor.connector_id));
    let base = format!("https://github.com/{repo}");

    let items = vec![
        ExternalEvidenceItem {
            external_id: format!("{repo}#1"),
            title: format!("[{repo}] Open coordination issue (stub)"),
            kind: EvidenceItemKind::Issue,
            url: Some(format!("{base}/issues/1")),
            summary: "Stub issue evidence for offline GitHub-shaped producer. Not live API.".into(),
            observed_at: now.clone(),
        },
        ExternalEvidenceItem {
            external_id: format!("{repo}#pr-1"),
            title: format!("[{repo}] Sample pull request (stub)"),
            kind: EvidenceItemKind::PullRequest,
            url: Some(format!("{base}/pull/1")),
            summary: "Stub PR evidence — evidence producer only; GitHub remains SoR.".into(),
            observed_at: now.clone(),
        },
        ExternalEvidenceItem {
            external_id: format!("{repo}#status-main"),
            title: format!("[{repo}] CI status snapshot (stub)"),
            kind: EvidenceItemKind::Status,
            url: Some(format!("{base}/actions")),
            summary: "Stub status evidence for authority/freshness pipelines.".into(),
            observed_at: now.clone(),
        },
    ];

    let run = ConnectorRun {
        protocol_version: CONNECTOR_PROTOCOL_VERSION.into(),
        run_id: format!("cr-{}", now.replace(':', "")),
        connector_id: descriptor.connector_id.clone(),
        kind: ConnectorKind::GithubStub,
        collected_at: now,
        evidence_only: true,
        source: "github-stub-offline".into(),
        items,
        note: "Evidence producer only — does not replace GitHub as system of record".into(),
    };
    validate_connector_run(&run).map_err(|e| GithubStubError::Validation(e.to_string()))?;
    Ok(run)
}
