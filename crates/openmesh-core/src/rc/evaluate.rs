//! Build RC pack from pilot + domain surface evidence (no feature expansion).

use crate::connectors::list_connectors;
use crate::pilot::build_pilot_pack;
use crate::rc::contract::{
    validate_rc_pack, RcCheckItem, RcCheckStatus, RcFreezePolicy, RcPack, RcRegressionRow,
    RcSeverity, RC_PROTOCOL_VERSION,
};
use crate::storage::{read_project, Project};
use crate::team::read_team_workspace;
use crate::trust_admin::read_trust_policy;
use chrono::Utc;

#[derive(Debug, thiserror::Error)]
pub enum RcEvaluateError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("validation: {0}")]
    Validation(String),
}

fn chk(
    id: &str,
    title: &str,
    severity: RcSeverity,
    status: RcCheckStatus,
    evidence: &str,
    detail: Option<&str>,
) -> RcCheckItem {
    RcCheckItem {
        id: id.into(),
        title: title.into(),
        severity,
        status,
        evidence: evidence.into(),
        detail: detail.map(|s| s.into()),
    }
}

fn row(id: &str, area: &str, surface: &str, status: RcCheckStatus, evidence: &str) -> RcRegressionRow {
    RcRegressionRow {
        id: id.into(),
        area: area.into(),
        surface: surface.into(),
        status,
        evidence: evidence.into(),
    }
}

fn freeze_policy() -> RcFreezePolicy {
    RcFreezePolicy {
        features_frozen: true,
        allowed: vec![
            "bugfix for P0/P1".into(),
            "docs / CHANGELOG / ledger".into(),
            "test coverage for regressions".into(),
            "RC pack re-evaluation".into(),
        ],
        forbidden: vec![
            "new domain features".into(),
            "scope expansion without Product Bible amendment".into(),
            "breaking protocol without migration plan".into(),
        ],
        summary: "RC window: features frozen; stabilize toward 1.0 with no known P0/P1 fails."
            .into(),
    }
}

/// Evaluate RC readiness for a project workspace.
pub fn build_rc_pack(project_path: &str) -> Result<RcPack, RcEvaluateError> {
    let project: Project = read_project(project_path, "project.json")
        .ok_or(RcEvaluateError::ProjectNotInitialized)?;
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut checks = Vec::new();
    let mut matrix = Vec::new();

    // P0: project root
    checks.push(chk(
        "rc-project",
        "Project initialized",
        RcSeverity::P0,
        RcCheckStatus::Pass,
        ".openmesh/project.json",
        Some(&project.id),
    ));
    matrix.push(row(
        "m-project",
        "core",
        "project.json",
        RcCheckStatus::Pass,
        "present",
    ));

    // P0: pilot pack ready
    match build_pilot_pack(project_path) {
        Ok(p) if p.pilot_ready => {
            checks.push(chk(
                "rc-pilot",
                "Pilot readiness pack has zero fails",
                RcSeverity::P0,
                RcCheckStatus::Pass,
                ".openmesh/pilot (evaluated)",
                Some(&format!(
                    "pass={} warn={} fail={}",
                    p.pass_count, p.warn_count, p.fail_count
                )),
            ));
            matrix.push(row(
                "m-pilot",
                "pilot",
                "pilot check",
                RcCheckStatus::Pass,
                "pilot_ready=true",
            ));
        }
        Ok(p) => {
            checks.push(chk(
                "rc-pilot",
                "Pilot readiness pack has zero fails",
                RcSeverity::P0,
                RcCheckStatus::Fail,
                "pilot not ready",
                Some(&format!(
                    "fail_count={} — run pilot check / team+trust init",
                    p.fail_count
                )),
            ));
            matrix.push(row(
                "m-pilot",
                "pilot",
                "pilot check",
                RcCheckStatus::Fail,
                "pilot_ready=false",
            ));
        }
        Err(e) => {
            checks.push(chk(
                "rc-pilot",
                "Pilot readiness pack has zero fails",
                RcSeverity::P0,
                RcCheckStatus::Fail,
                "pilot evaluate error",
                Some(&e.to_string()),
            ));
            matrix.push(row(
                "m-pilot",
                "pilot",
                "pilot check",
                RcCheckStatus::Fail,
                "error",
            ));
        }
    }

    // P0: team
    match read_team_workspace(project_path) {
        Ok(ws) => {
            checks.push(chk(
                "rc-team",
                "Team workspace foundation",
                RcSeverity::P0,
                RcCheckStatus::Pass,
                ".openmesh/team/workspace.json",
                Some(&format!("members={}", ws.members.len())),
            ));
            matrix.push(row(
                "m-team",
                "team",
                "team show",
                RcCheckStatus::Pass,
                "workspace present",
            ));
        }
        Err(_) => {
            checks.push(chk(
                "rc-team",
                "Team workspace foundation",
                RcSeverity::P0,
                RcCheckStatus::Fail,
                "missing",
                Some("team init required for RC"),
            ));
            matrix.push(row(
                "m-team",
                "team",
                "team show",
                RcCheckStatus::Fail,
                "missing",
            ));
        }
    }

    // P1: trust-admin
    match read_trust_policy(project_path) {
        Ok(p)
            if p.secret_topics_fail_closed && !p.allow_secret_export && p.sync_require_selective =>
        {
            checks.push(chk(
                "rc-trust",
                "Trust/privacy fail-closed invariants",
                RcSeverity::P1,
                RcCheckStatus::Pass,
                ".openmesh/trust-admin/policy.json",
                Some(&format!("query_mode={:?}", p.query_allowlist_mode)),
            ));
            matrix.push(row(
                "m-trust",
                "trust-admin",
                "trust-admin show",
                RcCheckStatus::Pass,
                "invariants hold",
            ));
        }
        Ok(_) => {
            checks.push(chk(
                "rc-trust",
                "Trust/privacy fail-closed invariants",
                RcSeverity::P1,
                RcCheckStatus::Fail,
                "policy violates invariants",
                None,
            ));
            matrix.push(row(
                "m-trust",
                "trust-admin",
                "trust-admin show",
                RcCheckStatus::Fail,
                "invariants broken",
            ));
        }
        Err(_) => {
            checks.push(chk(
                "rc-trust",
                "Trust/privacy fail-closed invariants",
                RcSeverity::P1,
                RcCheckStatus::Fail,
                "missing",
                Some("trust-admin init required"),
            ));
            matrix.push(row(
                "m-trust",
                "trust-admin",
                "trust-admin show",
                RcCheckStatus::Fail,
                "missing",
            ));
        }
    }

    // P1: connectors role if any
    match list_connectors(project_path) {
        Ok(list) if list.is_empty() => {
            checks.push(chk(
                "rc-connectors",
                "Connectors remain evidence-only when present",
                RcSeverity::P1,
                RcCheckStatus::Pass,
                "none registered",
                Some("N/A — empty registry OK for RC"),
            ));
            matrix.push(row(
                "m-connectors",
                "connectors",
                "connector list",
                RcCheckStatus::Pass,
                "empty ok",
            ));
        }
        Ok(list) => {
            let ok = list.iter().all(|c| {
                matches!(
                    c.role,
                    crate::connectors::ConnectorRole::EvidenceProducerOnly
                )
            });
            checks.push(chk(
                "rc-connectors",
                "Connectors remain evidence-only when present",
                RcSeverity::P1,
                if ok {
                    RcCheckStatus::Pass
                } else {
                    RcCheckStatus::Fail
                },
                ".openmesh/connectors/registry.json",
                Some(&format!("count={}", list.len())),
            ));
            matrix.push(row(
                "m-connectors",
                "connectors",
                "connector list",
                if ok {
                    RcCheckStatus::Pass
                } else {
                    RcCheckStatus::Fail
                },
                "role check",
            ));
        }
        Err(_) => {
            checks.push(chk(
                "rc-connectors",
                "Connectors remain evidence-only when present",
                RcSeverity::P2,
                RcCheckStatus::Warn,
                "unavailable",
                None,
            ));
        }
    }

    // P1: org graph
    match crate::org_graph::build_org_graph(project_path) {
        Ok(g) => {
            checks.push(chk(
                "rc-org",
                "Org graph projectable without invented nodes",
                RcSeverity::P1,
                RcCheckStatus::Pass,
                "org graph build",
                Some(&format!("nodes={}", g.nodes.len())),
            ));
            matrix.push(row(
                "m-org",
                "org",
                "org graph show",
                RcCheckStatus::Pass,
                "built",
            ));
        }
        Err(_) => {
            checks.push(chk(
                "rc-org",
                "Org graph projectable without invented nodes",
                RcSeverity::P1,
                RcCheckStatus::Fail,
                "build failed",
                Some("requires team"),
            ));
            matrix.push(row(
                "m-org",
                "org",
                "org graph show",
                RcCheckStatus::Fail,
                "failed",
            ));
        }
    }

    // P2: freeze policy acknowledged in pack
    checks.push(chk(
        "rc-freeze",
        "Feature freeze policy active",
        RcSeverity::P2,
        RcCheckStatus::Pass,
        "rc freeze policy",
        Some("features_frozen=true"),
    ));
    matrix.push(row(
        "m-freeze",
        "rc",
        "freeze policy",
        RcCheckStatus::Pass,
        "frozen",
    ));

    // Optional surfaces as regression rows (warn if missing, not P0/P1 fail)
    for (id, area, surface, present) in [
        (
            "m-cloud",
            "team-cloud",
            "team cloud show",
            crate::team_cloud::read_team_cloud(project_path).is_ok(),
        ),
        (
            "m-online",
            "online-proxy",
            "online-proxy status",
            crate::online_proxy::read_config(project_path).is_ok(),
        ),
    ] {
        matrix.push(row(
            id,
            area,
            surface,
            if present {
                RcCheckStatus::Pass
            } else {
                RcCheckStatus::Warn
            },
            if present { "configured" } else { "optional missing" },
        ));
    }

    let mut p0_fail = 0u32;
    let mut p1_fail = 0u32;
    let mut open = 0u32;
    for c in &checks {
        if matches!(c.status, RcCheckStatus::Open) {
            open += 1;
        }
        if matches!(c.status, RcCheckStatus::Fail) {
            match c.severity {
                RcSeverity::P0 => p0_fail += 1,
                RcSeverity::P1 => p1_fail += 1,
                _ => {}
            }
        }
    }
    let rc_ready = p0_fail == 0 && p1_fail == 0;

    let pack = RcPack {
        protocol_version: RC_PROTOCOL_VERSION.into(),
        workspace_id: project.id,
        generated_at: now,
        rc_ready,
        p0_fail_count: p0_fail,
        p1_fail_count: p1_fail,
        open_count: open,
        checks,
        regression_matrix: matrix,
        freeze_policy: freeze_policy(),
        limitations: vec![
            "RC program — no feature expansion".into(),
            "rc_ready means no known P0/P1 fails on local evidence".into(),
            "1.0.0 still requires full gate verification package".into(),
        ],
    };
    validate_rc_pack(&pack).map_err(|e| RcEvaluateError::Validation(e.to_string()))?;
    Ok(pack)
}
