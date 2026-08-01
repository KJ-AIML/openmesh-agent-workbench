//! Dev Track 0.1.7.7 — `proxy verify` CLI (read-only claim verification).

use clap::Args;
use openmesh_core::authority_freshness::evaluate_evidence_freshness;
use openmesh_core::context_pack_storage::read_proxy_context_pack;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::ProxyDraft;
use openmesh_core::proxy_citations::{build_citations, unsupported_claim_texts};
use openmesh_core::proxy_claims::{
    claims_meet_coverage_threshold, extract_claims_from_draft, verify_claims_against_pack,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Args, Debug, Clone)]
pub struct ProxyVerifyArgs {
    /// Draft text to verify (or use with --draft-file).
    #[arg(long, conflicts_with = "draft_file")]
    pub draft_text: Option<String>,

    /// Path to JSON ProxyDraft file.
    #[arg(long, conflicts_with = "draft_text")]
    pub draft_file: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_proxy_verify(args: &ProxyVerifyArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();

    let draft_text = match load_draft_text(args) {
        Ok(text) => text,
        Err(code) => return code,
    };

    let pack = match read_proxy_context_pack(&project_path) {
        Ok(pack) => pack,
        Err(err) => return crate::context::print_context_storage_error(&err, args.json),
    };
    if validate_proxy_context_pack_complete(&pack).is_err() {
        return print_verify_error(
            "context pack failed validation",
            "invalid-context-pack",
            args.json,
        );
    }

    let claims = extract_claims_from_draft(&draft_text);
    let verified = verify_claims_against_pack(&claims, &pack);
    let citations = build_citations(&verified);
    let coverage_ok = claims_meet_coverage_threshold(&verified);
    let freshness = evaluate_evidence_freshness(
        &pack,
        openmesh_core::authority_policy::map_risk_to_freshness_tier(
            openmesh_core::authority_policy::QuestionRiskCategory::Unknown,
        ),
        chrono::Utc::now(),
    );
    let unsupported = unsupported_claim_texts(&verified);

    if args.json {
        println!(
            "{}",
            json!({
                "coverageOk": coverage_ok,
                "claimCount": claims.len(),
                "citations": citations,
                "freshness": freshness,
                "unsupportedClaims": unsupported,
            })
        );
        return if coverage_ok && freshness.is_sufficient {
            0
        } else {
            2
        };
    }

    println!("Proxy verify — read-only claim/evidence alignment");
    println!("Claims: {}", claims.len());
    println!("Coverage OK: {coverage_ok}");
    println!("Freshness sufficient: {}", freshness.is_sufficient);
    if !unsupported.is_empty() {
        println!("Unsupported claims:");
        for text in &unsupported {
            println!("  - {text}");
        }
    }
    if coverage_ok && freshness.is_sufficient {
        0
    } else {
        2
    }
}

fn load_draft_text(args: &ProxyVerifyArgs) -> Result<String, i32> {
    if let Some(text) = &args.draft_text {
        return Ok(text.clone());
    }
    if let Some(path) = &args.draft_file {
        let content = std::fs::read_to_string(path).map_err(|_| {
            print_verify_error("failed to read draft file", "read-failed", args.json)
        })?;
        if path.ends_with(".json") {
            let draft: ProxyDraft = serde_json::from_str(&content).map_err(|_| {
                print_verify_error("malformed proxy draft JSON", "malformed-json", args.json)
            })?;
            return Ok(draft.draft_text);
        }
        return Ok(content);
    }
    Err(print_verify_error(
        "provide --draft-text or --draft-file",
        "invalid-request",
        args.json,
    ))
}

fn print_verify_error(message: &str, category: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!("{}", json!({ "error": message, "category": category }));
    } else {
        eprintln!("Error: {message}");
    }
    3
}
