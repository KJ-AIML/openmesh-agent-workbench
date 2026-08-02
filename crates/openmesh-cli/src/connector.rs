//! Connector Layer CLI — Dev Track 0.1.18.

use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::connectors::{
    collect_github_stub, init_or_register_connector, list_connectors, read_connector,
    write_connector_run, ConnectorKind, ConnectorStorageError,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum ConnectorCommand {
    /// Register a connector (evidence producer only).
    Register(RegisterArgs),
    /// List registered connectors.
    List(ListArgs),
    /// Show one connector.
    Show(ShowArgs),
    /// Collect evidence from a connector (stub/offline producers).
    Collect(CollectArgs),
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum ConnectorKindArg {
    #[default]
    #[value(name = "github-stub")]
    GithubStub,
}

impl From<ConnectorKindArg> for ConnectorKind {
    fn from(v: ConnectorKindArg) -> Self {
        match v {
            ConnectorKindArg::GithubStub => ConnectorKind::GithubStub,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct RegisterArgs {
    #[arg(long = "id")]
    pub connector_id: String,
    #[arg(long = "name")]
    pub display_name: Option<String>,
    #[arg(long, value_enum, default_value_t = ConnectorKindArg::GithubStub)]
    pub kind: ConnectorKindArg,
    /// e.g. owner/repo for GitHub-shaped stub.
    #[arg(long = "ref")]
    pub external_ref: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ListArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ShowArgs {
    #[arg(long = "id")]
    pub connector_id: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CollectArgs {
    #[arg(long = "id")]
    pub connector_id: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_connector(command: ConnectorCommand, cwd: &Path) -> i32 {
    match command {
        ConnectorCommand::Register(a) => run_register(&a, cwd),
        ConnectorCommand::List(a) => run_list(&a, cwd),
        ConnectorCommand::Show(a) => run_show(&a, cwd),
        ConnectorCommand::Collect(a) => run_collect(&a, cwd),
    }
}

fn run_register(args: &RegisterArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let name = args
        .display_name
        .clone()
        .unwrap_or_else(|| args.connector_id.clone());
    match init_or_register_connector(
        &project_path,
        &args.connector_id,
        &name,
        args.kind.into(),
        args.external_ref.clone(),
    ) {
        Ok(d) => {
            if args.json {
                println!("{}", serde_json::to_value(&d).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("connector_id={}", d.connector_id);
                println!("kind={:?}", d.kind);
                println!("role={:?}", d.role);
                println!("enabled={}", d.enabled);
            }
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_list(args: &ListArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match list_connectors(&project_path) {
        Ok(list) => {
            if args.json {
                println!("{}", serde_json::to_value(&list).unwrap_or(json!([])));
            } else if list.is_empty() {
                println!("(no connectors registered)");
            } else {
                for c in list {
                    println!(
                        "{} | {:?} | enabled={} | ref={}",
                        c.connector_id,
                        c.kind,
                        c.enabled,
                        c.external_ref.as_deref().unwrap_or("-")
                    );
                }
            }
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_show(args: &ShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_connector(&project_path, &args.connector_id) {
        Ok(d) => {
            if args.json {
                println!("{}", serde_json::to_value(&d).unwrap_or(json!({})));
            } else {
                println!("connector_id={}", d.connector_id);
                println!("display_name={}", d.display_name);
                println!("kind={:?}", d.kind);
                println!("role={:?}", d.role);
                println!("enabled={}", d.enabled);
                println!("external_ref={}", d.external_ref.as_deref().unwrap_or("-"));
                for l in &d.limitations {
                    println!("  limit={l}");
                }
            }
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_collect(args: &CollectArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let d = match read_connector(&project_path, &args.connector_id) {
        Ok(d) => d,
        Err(e) => return err_store(e, args.json),
    };
    let run = match collect_github_stub(&d) {
        Ok(r) => r,
        Err(e) => return err_msg(args.json, "collect", &e.to_string()),
    };
    if let Err(e) = write_connector_run(&project_path, &run) {
        return err_store(e, args.json);
    }
    if args.json {
        println!("{}", serde_json::to_value(&run).unwrap_or(json!({})));
    } else {
        println!("status=ok");
        println!("run_id={}", run.run_id);
        println!("evidence_only={}", run.evidence_only);
        println!("items={}", run.items.len());
        println!("note={}", run.note);
        for i in &run.items {
            println!("  {} | {:?} | {}", i.external_id, i.kind, i.title);
        }
    }
    0
}

fn err_store(e: ConnectorStorageError, json: bool) -> i32 {
    err_msg(json, "connector", &e.to_string())
}

fn err_msg(json: bool, category: &str, message: &str) -> i32 {
    if json {
        println!(
            "{}",
            json!({ "error": { "category": category, "message": message } })
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    1
}
