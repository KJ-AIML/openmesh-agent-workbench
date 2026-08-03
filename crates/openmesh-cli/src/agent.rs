//! OpenMesh Agent Engine CLI (0.1.23).

use clap::{Args, Subcommand};
use openmesh_core::agent_engine::{
    probe_provider, resolve_provider_kind, run_agent_turn, AgentDefinition, AgentSecretStore,
    AgentSession, CascadingSecretStore, OpenAiCompatibleProvider, ProviderConfig,
};
use openmesh_core::context_service;
use openmesh_core::mesh::peers::list_peers;
use openmesh_core::pilot::build_pilot_pack;
use openmesh_core::rc::build_rc_pack;
use openmesh_core::storage::{read_project, Project};
use openmesh_core::agent_engine::ToolExecutor;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let executor = CliToolExecutor {
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

struct CliToolExecutor {
    project_path: String,
}

impl ToolExecutor for CliToolExecutor {
    fn execute(&self, tool_name: &str, arguments_json: &str) -> Result<String, String> {
        match tool_name {
            "project_info" => {
                let project: Option<Project> = read_project(&self.project_path, "project.json");
                Ok(serde_json::to_string_pretty(&json!({
                    "path": self.project_path,
                    "project": project,
                }))
                .unwrap_or_else(|_| "{}".into()))
            }
            "list_docs" => list_dir_names(&self.project_path, "docs"),
            "list_notes" => list_dir_names(&self.project_path, "notes"),
            "list_mesh_peers" => {
                let peers = list_peers(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&peers).unwrap_or_else(|_| "[]".into()))
            }
            "pilot_status" => {
                let pack = build_pilot_pack(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&pack).unwrap_or_else(|_| "{}".into()))
            }
            "rc_status" => {
                let pack = build_rc_pack(&self.project_path).map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&pack).unwrap_or_else(|_| "{}".into()))
            }
            "git_status" => {
                let output = Command::new("git")
                    .args(["-C", &self.project_path, "status", "--porcelain=v1", "-b"])
                    .output()
                    .map_err(|e| e.to_string())?;
                Ok(String::from_utf8_lossy(&output.stdout).to_string())
            }
            "search_context" => {
                let args: serde_json::Value =
                    serde_json::from_str(arguments_json).unwrap_or(json!({}));
                let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let hits = context_service::search_project_context(
                    &self.project_path,
                    query,
                    None,
                    Some(12),
                )
                .map_err(|e| e.to_string())?;
                Ok(serde_json::to_string_pretty(&hits).unwrap_or_else(|_| "[]".into()))
            }
            "continuity_summary" => Ok(json!({
                "note": "use Desktop Continuity or pilot/rc tools for full summary",
                "peers": list_peers(&self.project_path).map(|p| p.len()).unwrap_or(0),
            })
            .to_string()),
            other => Err(format!("unknown tool: {other}")),
        }
    }
}

fn list_dir_names(project_path: &str, folder: &str) -> Result<String, String> {
    let dir = PathBuf::from(project_path).join(folder);
    if !dir.is_dir() {
        return Ok(format!("(no {folder}/ directory)"));
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        names.push(entry.map_err(|e| e.to_string())?.file_name().to_string_lossy().to_string());
    }
    names.sort();
    Ok(serde_json::to_string_pretty(&names).unwrap_or_else(|_| "[]".into()))
}
