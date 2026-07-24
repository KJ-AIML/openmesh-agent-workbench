// ============================================================================
// Proxy commands — Dev Track 0.1.6 Checkpoint E
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::context_pack_storage::read_proxy_context_pack;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{ProxyContextPack, ProxyDraft, MAX_PROXY_DRAFT_TEXT_BYTES};
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, ProxyAskError, ProxyAskOptions, ProxyDraftClock, SystemProxyDraftClock,
};
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

pub const HUMAN_OUTPUT_HEADER: &str = "Local Work Proxy draft — not the human owner.";
pub const HUMAN_OUTPUT_ACTION_LINE: &str = "No action was performed.";

#[derive(Subcommand, Debug)]
pub enum ProxyCommand {
    /// Ask the local Work Proxy for a draft answer.
    Ask(ProxyAskArgs),
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
    let question = args.question.trim();
    if question.is_empty() {
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

    let pack = match load_context_pack(args, &project_path) {
        Ok(pack) => pack,
        Err(code) => return code,
    };

    let question = match create_proxy_question(question, harness.identity_provider) {
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
        Ok(draft) => {
            print_proxy_draft_success(&draft, args.json);
            0
        }
        Err(err) => print_proxy_ask_error(&err, args.json),
    }
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

fn print_proxy_draft_success(draft: &ProxyDraft, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(draft).expect("serialize proxy draft")
        );
        return;
    }

    println!("{HUMAN_OUTPUT_HEADER}");
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
