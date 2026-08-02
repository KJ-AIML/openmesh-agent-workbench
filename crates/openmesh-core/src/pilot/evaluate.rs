//! Evaluate pilot readiness from local OpenMesh evidence.

use crate::connectors::list_connectors;
use crate::online_proxy::read_config as read_online_proxy;
use crate::pilot::contract::{
    validate_pilot_pack, PilotCheckItem, PilotCheckStatus, PilotPack, RunbookStep, ThreatNote,
    PILOT_PROTOCOL_VERSION,
};
use crate::storage::{read_project, Project};
use crate::team::read_team_workspace;
use crate::team_cloud::read_team_cloud;
use crate::trust_admin::read_trust_policy;
use chrono::Utc;

#[derive(Debug, thiserror::Error)]
pub enum PilotEvaluateError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("validation: {0}")]
    Validation(String),
}

fn check(
    id: &str,
    title: &str,
    status: PilotCheckStatus,
    evidence: &str,
    detail: Option<&str>,
) -> PilotCheckItem {
    PilotCheckItem {
        id: id.into(),
        title: title.into(),
        status,
        evidence: evidence.into(),
        detail: detail.map(|s| s.into()),
    }
}

fn static_threats() -> Vec<ThreatNote> {
    vec![
        ThreatNote {
            id: "t-secret-export".into(),
            title: "Secret export / exfil via pack surfaces".into(),
            summary: "Context packs and proxy answers could leak credentials if not fail-closed.".into(),
            residual: "secret_topics_fail_closed + allow_secret_export=false enforced in trust-admin; pack secret scanners exist.".into(),
        },
        ThreatNote {
            id: "t-remote-query".into(),
            title: "Remote teammate query over-disclosure".into(),
            summary: "Mesh/team query could answer beyond intended scope.".into(),
            residual: "Queries are read-only; allowlist modes in trust-admin; freshness gates on evidence.".into(),
        },
        ThreatNote {
            id: "t-full-repo-sync".into(),
            title: "Full-repo upload disguised as sync".into(),
            summary: "Cloud/sync tier might expand beyond selective paths.".into(),
            residual: "team-cloud requires selective_sync=true; sync-scaffold is dry-run only.".into(),
        },
        ThreatNote {
            id: "t-connector-sor".into(),
            title: "Connector treated as SoR / write-back".into(),
            summary: "Operators might assume connectors replace GitHub/Linear.".into(),
            residual: "Connector role fixed to evidence-producer-only; evidence_only required on runs.".into(),
        },
        ThreatNote {
            id: "t-no-sla".into(),
            title: "Pilotaken production SLA expectation".into(),
            summary: "Pilot readiness is not multi-region HA or customer SLA.".into(),
            residual: "Explicit non-goal; pack limitations state no production SLA.".into(),
        },
    ]
}

fn static_runbook() -> Vec<RunbookStep> {
    vec![
        RunbookStep {
            id: "r-init-project".into(),
            title: "Initialize OpenMesh project".into(),
            command_or_action: "openmesh-cli init (or open Desktop project)".into(),
            purpose: "Create .openmesh workspace root".into(),
        },
        RunbookStep {
            id: "r-team".into(),
            title: "Create team registry".into(),
            command_or_action: "team init --name \"Pilot Team\" --owner-label You".into(),
            purpose: "Local multi-member registry for pilot".into(),
        },
        RunbookStep {
            id: "r-trust".into(),
            title: "Initialize trust/privacy policy".into(),
            command_or_action: "trust-admin init && trust-admin set-query-mode --mode allowlist-only".into(),
            purpose: "Fail-closed secrets + controlled remote query".into(),
        },
        RunbookStep {
            id: "r-cloud".into(),
            title: "Optional team cloud scaffold".into(),
            command_or_action: "team cloud init --mode local-sim && team cloud sync-scaffold".into(),
            purpose: "Selective sync dry-run only".into(),
        },
        RunbookStep {
            id: "r-connector".into(),
            title: "Optional evidence connector".into(),
            command_or_action: "connector register --id gh-lab --kind github-stub --ref org/repo".into(),
            purpose: "Evidence producer only — not SoR".into(),
        },
        RunbookStep {
            id: "r-org".into(),
            title: "Inspect org graph".into(),
            command_or_action: "org graph show".into(),
            purpose: "Evidence-backed membership / peer / connector map".into(),
        },
        RunbookStep {
            id: "r-pilot".into(),
            title: "Re-run pilot checklist".into(),
            command_or_action: "pilot check".into(),
            purpose: "Refresh readiness pack with current evidence".into(),
        },
    ]
}

/// Build a pilot readiness pack from local project evidence.
pub fn build_pilot_pack(project_path: &str) -> Result<PilotPack, PilotEvaluateError> {
    let project: Project = read_project(project_path, "project.json")
        .ok_or(PilotEvaluateError::ProjectNotInitialized)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut checks = Vec::new();

    checks.push(check(
        "c-project",
        "Project initialized",
        PilotCheckStatus::Pass,
        ".openmesh/project.json",
        Some(&format!("workspace {}", project.id)),
    ));

    match read_team_workspace(project_path) {
        Ok(ws) => {
            let status = if ws.members.is_empty() {
                PilotCheckStatus::Warn
            } else {
                PilotCheckStatus::Pass
            };
            checks.push(check(
                "c-team",
                "Team workspace present",
                status,
                ".openmesh/team/workspace.json",
                Some(&format!(
                    "team_id={} members={}",
                    ws.team_id,
                    ws.members.len()
                )),
            ));
        }
        Err(_) => checks.push(check(
            "c-team",
            "Team workspace present",
            PilotCheckStatus::Fail,
            "missing",
            Some("run: team init --name \"Pilot Team\" --owner-label You"),
        )),
    }

    match read_trust_policy(project_path) {
        Ok(p) => {
            let secrets_ok = p.secret_topics_fail_closed && !p.allow_secret_export;
            let selective_ok = p.sync_require_selective;
            let status = if secrets_ok && selective_ok {
                PilotCheckStatus::Pass
            } else {
                PilotCheckStatus::Fail
            };
            checks.push(check(
                "c-trust",
                "Trust/privacy policy fail-closed",
                status,
                ".openmesh/trust-admin/policy.json",
                Some(&format!(
                    "secret_fail_closed={} secret_export={} selective={} query_mode={:?}",
                    p.secret_topics_fail_closed,
                    p.allow_secret_export,
                    p.sync_require_selective,
                    p.query_allowlist_mode
                )),
            ));
        }
        Err(_) => checks.push(check(
            "c-trust",
            "Trust/privacy policy fail-closed",
            PilotCheckStatus::Fail,
            "missing",
            Some("run: trust-admin init"),
        )),
    }

    match read_team_cloud(project_path) {
        Ok(c) => {
            let status = if c.selective_sync {
                PilotCheckStatus::Pass
            } else {
                PilotCheckStatus::Fail
            };
            checks.push(check(
                "c-team-cloud",
                "Team cloud selective sync",
                status,
                ".openmesh/team-cloud/config.json",
                Some(&format!("mode={:?} selective={}", c.mode, c.selective_sync)),
            ));
        }
        Err(_) => checks.push(check(
            "c-team-cloud",
            "Team cloud selective sync",
            PilotCheckStatus::NotApplicable,
            "not configured",
            Some("optional: team cloud init --mode local-sim"),
        )),
    }

    match list_connectors(project_path) {
        Ok(list) if list.is_empty() => checks.push(check(
            "c-connectors",
            "Connectors evidence-only",
            PilotCheckStatus::NotApplicable,
            "none registered",
            Some("optional: connector register --kind github-stub"),
        )),
        Ok(list) => {
            let all_ep = list.iter().all(|c| {
                matches!(
                    c.role,
                    crate::connectors::ConnectorRole::EvidenceProducerOnly
                )
            });
            checks.push(check(
                "c-connectors",
                "Connectors evidence-only",
                if all_ep {
                    PilotCheckStatus::Pass
                } else {
                    PilotCheckStatus::Fail
                },
                ".openmesh/connectors/registry.json",
                Some(&format!("count={}", list.len())),
            ));
        }
        Err(_) => checks.push(check(
            "c-connectors",
            "Connectors evidence-only",
            PilotCheckStatus::NotApplicable,
            "unavailable",
            None,
        )),
    }

    match crate::org_graph::build_org_graph(project_path) {
        Ok(g) => checks.push(check(
            "c-org-graph",
            "Org graph projectable",
            PilotCheckStatus::Pass,
            "org graph from team evidence",
            Some(&format!("nodes={} edges={}", g.nodes.len(), g.edges.len())),
        )),
        Err(_) => checks.push(check(
            "c-org-graph",
            "Org graph projectable",
            PilotCheckStatus::Warn,
            "requires team init",
            Some("run team init then org graph show"),
        )),
    }

    match read_online_proxy(project_path) {
        Ok(_) => checks.push(check(
            "c-online-proxy",
            "Online proxy configured",
            PilotCheckStatus::Pass,
            ".openmesh/online-proxy/config.json",
            Some("local/cloud scaffold only"),
        )),
        Err(_) => checks.push(check(
            "c-online-proxy",
            "Online proxy configured",
            PilotCheckStatus::NotApplicable,
            "not configured",
            Some("optional for pilot"),
        )),
    }

    let mut pass_count = 0u32;
    let mut warn_count = 0u32;
    let mut fail_count = 0u32;
    for c in &checks {
        match c.status {
            PilotCheckStatus::Pass => pass_count += 1,
            PilotCheckStatus::Warn => warn_count += 1,
            PilotCheckStatus::Fail => fail_count += 1,
            PilotCheckStatus::NotApplicable => {}
        }
    }
    let pilot_ready = fail_count == 0;

    let pack = PilotPack {
        protocol_version: PILOT_PROTOCOL_VERSION.into(),
        workspace_id: project.id,
        generated_at: now,
        pilot_ready,
        pass_count,
        warn_count,
        fail_count,
        checks,
        threat_notes: static_threats(),
        runbook: static_runbook(),
        limitations: vec![
            "enterprise pilot readiness — not a production SLA".into(),
            "local evidence only; no multi-region HA claim".into(),
            "no customer IdP/SSO in this pack".into(),
        ],
    };
    validate_pilot_pack(&pack).map_err(|e| PilotEvaluateError::Validation(e.to_string()))?;
    Ok(pack)
}
