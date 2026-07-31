// ============================================================================
// Proxy commands — Dev Track 0.1.6 Checkpoint E + 0.1.7 Authority Gate
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::answer_receipt::{write_answer_receipt, AnswerReceipt};
use openmesh_core::authority_gate::{
    run_pre_provider_authority_gate, AuthorityGateOutcome, AuthorityOutcomeLabel,
};
use openmesh_core::authority_policy::{classify_question_risk, AuthorityPolicyDecision};
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::context_pack_storage::read_proxy_context_pack;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{ProxyContextPack, ProxyDraft, MAX_PROXY_DRAFT_TEXT_BYTES};
use openmesh_core::pending_proxy_question::write_pending_proxy_question;
use openmesh_core::profile::read_work_proxy_profile;
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, ProxyAskError, ProxyAskOptions, ProxyDraftClock, SystemProxyDraftClock,
};
use openmesh_core::proxy_post_verify::apply_post_provider_verification;
use openmesh_core::proxy_question::{
    create_proxy_question, ProcessLocalRequestIdentityProvider, ProxyQuestionConstructionError,
    ProxyRequestIdentityProvider,
};
use openmesh_core::proxy_runtime::ProxyDraftRuntime;
use serde_json::json;
use std::path::Path;

use crate::context::build_fixed_window;
use crate::output;
use crate::project::resolve_project;
use crate::proxy_runtime_factory::{
    resolve_production_proxy_draft_runtime, ProxyRuntimeFactoryError,
};
use crate::proxy_verify::ProxyVerifyArgs;

pub const HUMAN_OUTPUT_HEADER: &str = "Local Work Proxy draft — not the human owner.";
pub const HUMAN_OUTPUT_ACTION_LINE: &str = "No action was performed.";

#[derive(Subcommand, Debug)]
pub enum ProxyCommand {
    /// Ask the local Work Proxy for a draft answer.
    Ask(ProxyAskArgs),
    /// Verify draft claims against persisted context pack evidence (read-only).
    Verify(ProxyVerifyArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ProxyAskArgs {
    /// Question text for the local proxy draft.
    #[arg(long)]
    pub question: String,

    #[arg(long)]
    pub since: Option<String>,

    #[arg(long)]
    pub until: Option<String>,

    #[arg(long, conflicts_with_all = ["since", "until"])]
    pub from_persisted: bool,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,

    #[arg(long, value_parser = parse_positive_timeout_secs)]
    pub timeout_secs: Option<u64>,
}

fn parse_positive_timeout_secs(raw: &str) -> Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|_| "timeout_secs must be a positive integer".to_string())?;
    if value == 0 {
        return Err("timeout_secs must be greater than zero".to_string());
    }
    Ok(value)
}

pub trait ProxyRuntimeResolver: Send + Sync {
    fn resolve(&self) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionProxyRuntimeResolver;

impl ProxyRuntimeResolver for ProductionProxyRuntimeResolver {
    fn resolve(&self) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError> {
        resolve_production_proxy_draft_runtime()
    }
}

/// Injectable execution harness for automated CLI tests.
pub struct ProxyAskHarness<'a> {
    pub runtime_resolver: &'a dyn ProxyRuntimeResolver,
    pub identity_provider: &'a dyn ProxyRequestIdentityProvider,
    pub clock: &'a dyn ProxyDraftClock,
}

pub fn run_proxy(cmd: ProxyCommand, cwd: &Path) -> i32 {
    match cmd {
        ProxyCommand::Ask(args) => run_proxy_ask(&args, cwd),
        ProxyCommand::Verify(args) => crate::proxy_verify::run_proxy_verify(&args, cwd),
    }
}

pub fn run_proxy_ask(args: &ProxyAskArgs, cwd: &Path) -> i32 {
    let harness = ProxyAskHarness {
        runtime_resolver: &ProductionProxyRuntimeResolver,
        identity_provider: &ProcessLocalRequestIdentityProvider::new(),
        clock: &SystemProxyDraftClock,
    };
    run_proxy_ask_with_harness(args, cwd, &harness)
}

pub fn run_proxy_ask_with_harness(
    args: &ProxyAskArgs,
    cwd: &Path,
    harness: &ProxyAskHarness<'_>,
) -> i32 {
    let question_text = args.question.trim();
    if question_text.is_empty() {
        return print_proxy_error(
            "question must not be empty",
            "invalid-question",
            3,
            args.json,
        );
    }

    if args.from_persisted && (args.since.is_some() || args.until.is_some()) {
        return print_proxy_error(
            "--from-persisted cannot be combined with --since or --until",
            "invalid-request",
            3,
            args.json,
        );
    }

    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();

    let profile = match read_work_proxy_profile(&project_path) {
        Ok(profile) => profile,
        Err(_) => {
            return print_proxy_error(
                "work proxy profile is required for authority gate",
                "profile-missing",
                3,
                args.json,
            )
        }
    };

    let gate = run_pre_provider_authority_gate(question_text, &profile, "local-proxy-ask");
    let decision = match &gate {
        AuthorityGateOutcome::Proceed { decision, .. }
        | AuthorityGateOutcome::MustAsk { decision, .. }
        | AuthorityGateOutcome::Denied { decision, .. } => decision.clone(),
    };

    match &gate {
        AuthorityGateOutcome::MustAsk { message, .. } => {
            let _ = write_pending_for_question(&project_path, question_text, &decision);
            return print_proxy_error(message, "must-ask-human", 2, args.json);
        }
        AuthorityGateOutcome::Denied { message, .. } => {
            let _ = write_pending_for_question(&project_path, question_text, &decision);
            return print_proxy_error(message, "authority-denied", 2, args.json);
        }
        AuthorityGateOutcome::Proceed { .. } => {}
    }

    let pack = match load_context_pack(args, &project_path) {
        Ok(pack) => pack,
        Err(code) => return code,
    };

    // Pre-provider freshness gate for critical tiers — fail closed before provider.
    let risk = classify_question_risk(question_text);
    let tier = openmesh_core::authority_policy::map_risk_to_freshness_tier(risk);
    let freshness_pre = openmesh_core::authority_freshness::evaluate_evidence_freshness(
        &pack,
        tier,
        Utc::now(),
    );
    if matches!(tier, openmesh_core::authority_policy::FreshnessTier::Critical)
        && !freshness_pre.is_sufficient
    {
        let _ = write_pending_for_question(&project_path, question_text, &decision);
        return print_proxy_error(
            "critical question requires fresher evidence; must ask human",
            "freshness-insufficient",
            2,
            args.json,
        );
    }

    let question = match create_proxy_question(question_text, harness.identity_provider) {
        Ok(question) => question,
        Err(err) => return print_question_construction_error(&err, args.json),
    };

    let runtime = match harness.runtime_resolver.resolve() {
        Ok(runtime) => runtime,
        Err(err) => return print_factory_error(&err, args.json),
    };

    let timeout_ms = match timeout_ms_from_args(args.timeout_secs) {
        Ok(timeout_ms) => timeout_ms,
        Err(message) => return print_proxy_error(&message, "invalid-request", 3, args.json),
    };
    let options = ProxyAskOptions::new(timeout_ms, MAX_PROXY_DRAFT_TEXT_BYTES as u32);

    match ask_my_proxy_local(&pack, &question, &options, runtime.as_ref(), harness.clock) {
        Ok(mut draft) => {
            let post = apply_post_provider_verification(
                &mut draft,
                &pack,
                &question.text,
                Utc::now(),
            );
            let _ = write_receipt(
                &project_path,
                &question,
                &pack,
                &draft,
                &decision,
                &post,
            );
            let label = match gate {
                AuthorityGateOutcome::Proceed { label, .. } => label,
                _ => AuthorityOutcomeLabel::Proceed,
            };
            print_proxy_draft_success(&draft, &label, &post, args.json);
            if post.hard_fail {
                2
            } else {
                0
            }
        }
        Err(err) => print_proxy_ask_error(&err, args.json),
    }
}

fn write_pending_for_question(
    project_path: &str,
    question_text: &str,
    decision: &AuthorityPolicyDecision,
) -> Result<(), ()> {
    let risk = classify_question_risk(question_text);
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_pending_proxy_question(project_path, question_text, risk, decision, &created_at)
        .map(|_| ())
        .map_err(|_| ())
}

fn write_receipt(
    project_path: &str,
    question: &openmesh_core::domain::ProxyQuestion,
    pack: &ProxyContextPack,
    draft: &ProxyDraft,
    decision: &AuthorityPolicyDecision,
    post: &openmesh_core::proxy_post_verify::PostVerifyResult,
) -> Result<(), ()> {
    let receipt = AnswerReceipt {
        receipt_id: format!("receipt-{}", question.question_id),
        question_id: question.question_id.clone(),
        question_text: question.text.clone(),
        resolved_authority: decision.resolved_authority,
        authority_decision_reason: decision.decision_reason.clone(),
        context_pack_id: pack.context_pack_id.clone(),
        draft_text: draft.draft_text.clone(),
        claims_json: serde_json::to_string(&post.citations).unwrap_or_else(|_| "[]".to_string()),
        freshness_summary: serde_json::to_string(&post.freshness).unwrap_or_default(),
        generated_at: draft.generated_at.clone(),
        correction_of: None,
    };
    write_answer_receipt(project_path, &receipt).map_err(|_| ())
}

fn load_context_pack(args: &ProxyAskArgs, project_path: &str) -> Result<ProxyContextPack, i32> {
    if args.from_persisted {
        let pack = match read_proxy_context_pack(project_path) {
            Ok(pack) => pack,
            Err(err) => return Err(crate::context::print_context_storage_error(&err, args.json)),
        };
        if validate_proxy_context_pack_complete(&pack).is_err() {
            return Err(print_proxy_error(
                "persisted context pack failed validation",
                "invalid-context-pack",
                3,
                args.json,
            ));
        }
        return Ok(pack);
    }

    let window = match build_fixed_window(args.since.as_deref(), args.until.as_deref()) {
        Ok(window) => window,
        Err(message) => return Err(print_proxy_error(&message, "invalid-window", 3, args.json)),
    };

    let options = ProxyContextPackBuildOptions {
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        selection: Default::default(),
    };

    let pack = match build_proxy_context_pack(project_path, window, options) {
        Ok(pack) => pack,
        Err(err) => return Err(crate::context::print_context_build_error(&err, args.json)),
    };
    if validate_proxy_context_pack_complete(&pack).is_err() {
        return Err(print_proxy_error(
            "context pack failed validation",
            "invalid-context-pack",
            3,
            args.json,
        ));
    }
    Ok(pack)
}

pub fn timeout_ms_from_args(timeout_secs: Option<u64>) -> Result<u64, String> {
    let secs = timeout_secs.unwrap_or(60);
    secs.checked_mul(1_000)
        .ok_or_else(|| "timeout_secs overflow".to_string())
}

fn outcome_label_wire(label: &AuthorityOutcomeLabel) -> &'static str {
    match label {
        AuthorityOutcomeLabel::Proceed => "proceed",
        AuthorityOutcomeLabel::MustAskHuman => "must-ask-human",
        AuthorityOutcomeLabel::CannotAnswer => "cannot-answer",
        AuthorityOutcomeLabel::DeniedBeforeProvider => "denied-before-provider",
    }
}

fn print_proxy_draft_success(
    draft: &ProxyDraft,
    label: &AuthorityOutcomeLabel,
    post: &openmesh_core::proxy_post_verify::PostVerifyResult,
    json_mode: bool,
) {
    if json_mode {
        // Keep wire compatible with frozen ProxyDraft JSON (no wrapper keys).
        println!(
            "{}",
            serde_json::to_string(draft).expect("serialize proxy draft")
        );
        return;
    }

    println!("{HUMAN_OUTPUT_HEADER}");
    println!("authority_outcome={}", outcome_label_wire(label));
    println!(
        "coverage_ok={} freshness_ok={} confidence={:?}",
        post.coverage_ok,
        post.freshness.is_sufficient,
        post.freshness.confidence_label
    );
    if post.must_ask {
        println!("outcome=must-ask-human");
    }
    println!("{}", draft.draft_text);
    if !draft.limitations.is_empty() {
        println!("limitations:");
        for limitation in &draft.limitations {
            println!("- {limitation}");
        }
    }
    println!(
        "runtime_kind={} provider_id={} model_id={} network_used={} duration_ms={}",
        draft.runtime.runtime_kind,
        draft.runtime.provider_id,
        draft.runtime.model_id,
        draft.runtime.network_used,
        draft.runtime.duration_ms
    );
    println!("{HUMAN_OUTPUT_ACTION_LINE}");
}

pub fn print_proxy_error(message: &str, category: &str, code: i32, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "category": category, "message": message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    code
}

fn print_factory_error(err: &ProxyRuntimeFactoryError, json_mode: bool) -> i32 {
    let message = format!("{err}");
    print_proxy_error(&message, "runtime-configuration", 3, json_mode)
}

fn print_question_construction_error(err: &ProxyQuestionConstructionError, json_mode: bool) -> i32 {
    let message = match err {
        ProxyQuestionConstructionError::InvalidText(_) => "question text is invalid",
        ProxyQuestionConstructionError::IdentityGenerationFailed(_) => {
            "question identity generation failed"
        }
    };
    print_proxy_error(message, "invalid-question", 3, json_mode)
}

fn print_proxy_ask_error(err: &ProxyAskError, json_mode: bool) -> i32 {
    let (code, category, message) = match err {
        ProxyAskError::InvalidOptions => (3, "invalid-request", "proxy ask options are invalid"),
        ProxyAskError::InvalidQuestion => (3, "invalid-question", "proxy question is invalid"),
        ProxyAskError::InvalidContextPack => (3, "invalid-context-pack", "context pack is invalid"),
        ProxyAskError::PromptCompositionFailed => {
            (3, "prompt-composition-failed", "prompt composition failed")
        }
        ProxyAskError::TraceConstructionFailed => {
            (3, "trace-construction-failed", "trace construction failed")
        }
        ProxyAskError::InvalidRuntimeRequest => {
            (3, "invalid-runtime-request", "runtime request is invalid")
        }
        ProxyAskError::RuntimeNotConfigured => (
            3,
            "runtime-not-configured",
            "proxy draft runtime is not configured",
        ),
        ProxyAskError::RuntimeTimeout => (3, "runtime-timeout", "proxy draft runtime timed out"),
        ProxyAskError::RuntimeUnavailable => (
            3,
            "runtime-unavailable",
            "proxy draft runtime is unavailable",
        ),
        ProxyAskError::ProviderFailure => {
            (3, "provider-failure", "proxy draft runtime provider failed")
        }
        ProxyAskError::InvalidRuntimeOutput => {
            (3, "invalid-runtime-output", "runtime output is invalid")
        }
        ProxyAskError::UnsafeDraft => (
            3,
            "unsafe-draft",
            "generated draft failed safety validation",
        ),
        ProxyAskError::ClockFailure => (4, "clock-failure", "proxy draft clock is unavailable"),
        ProxyAskError::InvalidProxyDraft => {
            (3, "invalid-proxy-draft", "assembled proxy draft is invalid")
        }
    };
    print_proxy_error(message, category, code, json_mode)
}
