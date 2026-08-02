// ============================================================================
// Always-online proxy CLI — Dev Track 0.1.12
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::authority_policy::FreshnessTier;
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::domain::CatchUpWindow;
use openmesh_core::online_proxy::{
    ask_online_proxy, read_answer, read_config, write_config, OnlineProxyAskRequest,
    OnlineProxyConfig, OnlineProxyMode, ONLINE_PROXY_PROTOCOL_VERSION,
};
use openmesh_core::profile::read_work_proxy_profile;
use openmesh_core::storage::{read_project, Project};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum OnlineProxyCommand {
    /// Initialize always-online proxy config for this project.
    Init(OnlineProxyInitArgs),
    /// Show online-proxy config.
    Status(OnlineProxyStatusArgs),
    /// Ask the always-online proxy (mandatory freshness disclosure).
    Ask(OnlineProxyAskArgs),
    /// Show a stored answer by id.
    Show(OnlineProxyShowArgs),
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum OnlineModeArg {
    #[default]
    LocalScaffold,
    CloudScaffold,
}

impl From<OnlineModeArg> for OnlineProxyMode {
    fn from(v: OnlineModeArg) -> Self {
        match v {
            OnlineModeArg::LocalScaffold => OnlineProxyMode::LocalScaffold,
            OnlineModeArg::CloudScaffold => OnlineProxyMode::CloudScaffold,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum FreshnessTierArg {
    LowImpact,
    #[default]
    Standard,
    Critical,
}

impl From<FreshnessTierArg> for FreshnessTier {
    fn from(v: FreshnessTierArg) -> Self {
        match v {
            FreshnessTierArg::LowImpact => FreshnessTier::LowImpact,
            FreshnessTierArg::Standard => FreshnessTier::Standard,
            FreshnessTierArg::Critical => FreshnessTier::Critical,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct OnlineProxyInitArgs {
    #[arg(long, default_value = "local-operator")]
    pub owner_label: String,

    #[arg(long, value_enum, default_value_t = OnlineModeArg::LocalScaffold)]
    pub mode: OnlineModeArg,

    #[arg(long = "no-relay-received")]
    pub no_relay_received: bool,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct OnlineProxyStatusArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct OnlineProxyAskArgs {
    #[arg(long)]
    pub question: String,

    #[arg(long, value_enum, default_value_t = FreshnessTierArg::Standard)]
    pub tier: FreshnessTierArg,

    #[arg(long = "answer-id")]
    pub answer_id: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct OnlineProxyShowArgs {
    #[arg(long = "id")]
    pub answer_id: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_online_proxy(command: OnlineProxyCommand, cwd: &Path) -> i32 {
    match command {
        OnlineProxyCommand::Init(a) => run_init(&a, cwd),
        OnlineProxyCommand::Status(a) => run_status(&a, cwd),
        OnlineProxyCommand::Ask(a) => run_ask(&a, cwd),
        OnlineProxyCommand::Show(a) => run_show(&a, cwd),
    }
}

fn run_init(args: &OnlineProxyInitArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let project: Project = match read_project(&project_path, "project.json") {
        Some(p) => p,
        None => return err(args.json, 1, "project", "not initialized"),
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let owner = read_work_proxy_profile(&project_path)
        .map(|p| p.owner_label)
        .unwrap_or_else(|_| args.owner_label.clone());
    let cfg = OnlineProxyConfig {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        proxy_id: format!("online-{}", project.id),
        workspace_id: project.id,
        owner_label: owner,
        mode: args.mode.into(),
        default_freshness_tier: FreshnessTier::Standard,
        use_relay_received: !args.no_relay_received,
        created_at: now.clone(),
        updated_at: now,
    };
    match write_config(&project_path, &cfg) {
        Ok(()) => {
            if args.json {
                println!("{}", serde_json::to_value(&cfg).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("proxy_id={}", cfg.proxy_id);
                println!("mode={:?}", cfg.mode);
                println!("path=.openmesh/online-proxy/config.json");
            }
            0
        }
        Err(e) => err(args.json, 3, "init", &e.to_string()),
    }
}

fn run_status(args: &OnlineProxyStatusArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_config(&project_path) {
        Ok(cfg) => {
            if args.json {
                println!("{}", serde_json::to_value(&cfg).unwrap_or(json!({})));
            } else {
                println!("proxy_id={}", cfg.proxy_id);
                println!("owner_label={}", cfg.owner_label);
                println!("mode={:?}", cfg.mode);
                println!("use_relay_received={}", cfg.use_relay_received);
            }
            0
        }
        Err(e) => err(args.json, 3, "status", &e.to_string()),
    }
}

fn run_ask(args: &OnlineProxyAskArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let cfg = match read_config(&project_path) {
        Ok(c) => c,
        Err(e) => return err(args.json, 3, "config", &e.to_string()),
    };
    let now = Utc::now();
    let until = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since = (now - chrono::Duration::hours(24))
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let options = ProxyContextPackBuildOptions {
        generated_at: until.clone(),
        ..ProxyContextPackBuildOptions::default()
    };
    let window = CatchUpWindow { since, until };
    let pack = match build_proxy_context_pack(&project_path, window, options) {
        Ok(p) => p,
        Err(e) => return err(args.json, 4, "context", &e.to_string()),
    };
    let answer_id = args
        .answer_id
        .clone()
        .unwrap_or_else(|| format!("ans-{}", now.format("%Y%m%dT%H%M%SZ")));
    let req = OnlineProxyAskRequest {
        question: args.question.clone(),
        now,
        answer_id,
        freshness_tier: Some(args.tier.into()),
    };
    match ask_online_proxy(&project_path, &cfg, &pack, &req, true) {
        Ok(ans) => {
            if args.json {
                println!("{}", serde_json::to_value(&ans).unwrap_or(json!({})));
            } else {
                println!("answer_id={}", ans.answer_id);
                println!("refused={}", ans.refused);
                println!("freshness={}", ans.freshness.statement);
                println!("---");
                println!("{}", ans.answer_text);
            }
            0
        }
        Err(e) => err(args.json, 3, "ask", &e.to_string()),
    }
}

fn run_show(args: &OnlineProxyShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_answer(&project_path, &args.answer_id) {
        Ok(ans) => {
            if args.json {
                println!("{}", serde_json::to_value(&ans).unwrap_or(json!({})));
            } else {
                println!("answer_id={}", ans.answer_id);
                println!("refused={}", ans.refused);
                println!("{}", ans.freshness.statement);
                println!("{}", ans.answer_text);
            }
            0
        }
        Err(e) => err(args.json, 3, "show", &e.to_string()),
    }
}

fn err(json_mode: bool, code: i32, category: &str, message: &str) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status":"error","category":category,"message":message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    code
}
