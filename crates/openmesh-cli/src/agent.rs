//! OpenMesh Agent Engine CLI (0.1.23).

use clap::{Args, Subcommand};
use openmesh_core::agent_engine::{
    probe_provider, resolve_provider_kind, run_agent_turn, AgentDefinition, AgentSecretStore,
    AgentSession, CascadingSecretStore, OpenAiCompatibleProvider, ProviderConfig,
    WorkspaceToolExecutor,
};
use serde_json::json;
use std::path::Path;

use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum AgentCommand {
    /// Run one Agent Engine turn (LLM + tools).
    Ask(AgentAskArgs),
    /// Show whether an API key is configured in the user secret store / env.
    SecretStatus(AgentSecretStatusArgs),
    /// Probe provider connectivity (tiny chat completion, no tools).
    Test(AgentTestArgs),
}

#[derive(Args, Debug, Clone)]
pub struct AgentAskArgs {
    #[arg(long)]
    pub question: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long, default_value = "openai")]
    pub provider: String,

    #[arg(long)]
    pub model: Option<String>,

    #[arg(long = "base-url")]
    pub base_url: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AgentSecretStatusArgs {
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AgentTestArgs {
    #[arg(long, default_value = "openai")]
    pub provider: String,

    #[arg(long)]
    pub model: Option<String>,

    #[arg(long = "base-url")]
    pub base_url: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_agent(cmd: AgentCommand, cwd: &Path) -> i32 {
    match cmd {
        AgentCommand::Ask(args) => run_ask(&args, cwd),
        AgentCommand::SecretStatus(args) => run_secret_status(&args),
        AgentCommand::Test(args) => run_test(&args),
    }
}

fn run_test(args: &AgentTestArgs) -> i32 {
    let store = CascadingSecretStore::default();
    let api_key = match store.get_api_key() {
        Ok(Some(k)) if !k.trim().is_empty() => k,
        _ => {
            eprintln!("error: API key not configured");
            return 2;
        }
    };
    let model = args.model.clone().unwrap_or_else(|| "gpt-4o-mini".into());
    match probe_provider(
        &api_key,
        &model,
        Some(&args.provider),
        args.base_url.as_deref(),
    ) {
        Ok(result) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".into())
                );
            } else if result.ok {
                println!(
                    "ok=true model={} base={} latency_ms={} reply={}",
                    result.model,
                    result.base_url,
                    result.latency_ms,
                    result.reply_preview.unwrap_or_default()
                );
            } else {
                println!(
                    "ok=false model={} base={} latency_ms={} error={}",
                    result.model,
                    result.base_url,
                    result.latency_ms,
                    result.error.unwrap_or_default()
                );
            }
            if result.ok {
                0
            } else {
                1
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}

fn run_secret_status(args: &AgentSecretStatusArgs) -> i32 {
    let store = CascadingSecretStore::default();
    let configured = store.is_configured().unwrap_or(false);
    let path = store.file.path().display().to_string();
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({ "configured": configured, "store": path }))
                .unwrap_or_else(|_| "{}".into())
        );
    } else {
        println!("configured={configured}");
        println!("store={path}");
        println!("env_fallback=OPENMESH_AGENT_API_KEY|OPENAI_API_KEY|DEEPSEEK_API_KEY");
    }
    0
}

fn run_ask(args: &AgentAskArgs, cwd: &Path) -> i32 {
    let project = match resolve_project(args.project.as_deref(), cwd) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e.describe());
            return 1;
        }
    };
    let project_path = project.path.to_string_lossy().to_string();

    let store = CascadingSecretStore::default();
    let api_key = match store.get_api_key() {
        Ok(Some(k)) if !k.trim().is_empty() => k,
        Ok(_) => {
            eprintln!("error: API key not configured. Set OPENMESH_AGENT_API_KEY or save via Desktop Settings.");
            return 2;
        }
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let model = args
        .model
        .clone()
        .unwrap_or_else(|| "gpt-4o-mini".into());
    let (provider, base_url) =
        resolve_provider_kind(Some(&args.provider), args.base_url.as_deref());
    let mut def = AgentDefinition::default_workspace_agent(&model);
    def.provider = provider;
    def.base_url = base_url;

    let cfg = match ProviderConfig::from_definition(&def, &api_key) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let client = match OpenAiCompatibleProvider::new(cfg) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };
    let executor = WorkspaceToolExecutor {
        project_path: project_path.clone(),
    };
    let mut session = AgentSession::default();
    match run_agent_turn(&def, &mut session, &args.question, &client, &executor) {
        Ok(result) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".into())
                );
            } else {
                for step in &result.tool_steps {
                    println!(
                        "tool={} ok={} {}",
                        step.tool_name,
                        step.ok,
                        step.summary.lines().next().unwrap_or("")
                    );
                }
                println!("---");
                println!("{}", result.assistant_text);
            }
            0
        }
        Err(e) => {
            eprintln!("error: {e}");
            1
        }
    }
}
