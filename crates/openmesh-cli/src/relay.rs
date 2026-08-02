// ============================================================================
// Relay commands — Dev Track 0.1.11 Private Relay Alpha
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::mesh::MeshSensitivityMax;
use openmesh_core::relay::{
    approve_relay_package, list_audit_events, pack_to_staging, read_approved_package,
    read_received_package, read_staging_package, receive_package_from_relay_root,
    send_package_to_relay_root, BuildRelayPackRequest,
};
use openmesh_core::storage::{read_project, Project};
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum RelayCommand {
    /// Build a staging relay package from mesh outbox envelopes.
    Pack(RelayPackArgs),
    /// Show a staged or approved package.
    Show(RelayShowArgs),
    /// Approve a staged package for egress.
    Approve(RelayApproveArgs),
    /// Send an approved package to a filesystem relay root.
    Send(RelaySendArgs),
    /// Receive a package from a filesystem relay root into received/.
    Receive(RelayReceiveArgs),
    /// List relay audit events.
    Audit(RelayAuditArgs),
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum RelaySensitivityArg {
    Public,
    Team,
    #[default]
    Private,
}

impl From<RelaySensitivityArg> for MeshSensitivityMax {
    fn from(v: RelaySensitivityArg) -> Self {
        match v {
            RelaySensitivityArg::Public => MeshSensitivityMax::Public,
            RelaySensitivityArg::Team => MeshSensitivityMax::Team,
            RelaySensitivityArg::Private => MeshSensitivityMax::Private,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct RelayPackArgs {
    /// Repeatable mesh outbox envelope id to include.
    #[arg(long = "envelope-id")]
    pub envelope_id: Vec<String>,

    /// Optional package id (default: pkg-<timestamp>).
    #[arg(long = "package-id")]
    pub package_id: Option<String>,

    #[arg(long, value_enum, default_value_t = RelaySensitivityArg::Private)]
    pub sensitivity: RelaySensitivityArg,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RelayShowArgs {
    #[arg(long = "id")]
    pub package_id: String,

    /// Prefer approved copy when present.
    #[arg(long)]
    pub approved: bool,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RelayApproveArgs {
    #[arg(long = "id")]
    pub package_id: String,

    #[arg(long = "by", default_value = "local-operator")]
    pub approved_by: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RelaySendArgs {
    #[arg(long = "id")]
    pub package_id: String,

    /// Filesystem relay root directory.
    #[arg(long = "relay-root")]
    pub relay_root: PathBuf,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RelayReceiveArgs {
    #[arg(long = "id")]
    pub package_id: String,

    #[arg(long = "relay-root")]
    pub relay_root: PathBuf,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RelayAuditArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_relay(command: RelayCommand, cwd: &Path) -> i32 {
    match command {
        RelayCommand::Pack(a) => run_pack(&a, cwd),
        RelayCommand::Show(a) => run_show(&a, cwd),
        RelayCommand::Approve(a) => run_approve(&a, cwd),
        RelayCommand::Send(a) => run_send(&a, cwd),
        RelayCommand::Receive(a) => run_receive(&a, cwd),
        RelayCommand::Audit(a) => run_audit(&a, cwd),
    }
}

fn run_pack(args: &RelayPackArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let project: Project = match read_project(&project_path, "project.json") {
        Some(p) => p,
        None => return err_json(args.json, 1, "project", "project not initialized"),
    };
    let now = Utc::now();
    let package_id = args
        .package_id
        .clone()
        .unwrap_or_else(|| format!("pkg-{}", now.format("%Y%m%dT%H%M%SZ")));
    let req = BuildRelayPackRequest {
        package_id,
        workspace_id: project.id,
        generated_at: now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        sensitivity_max: args.sensitivity.into(),
        envelope_ids: args.envelope_id.clone(),
        handoff_ids: vec![],
        selection_notes: vec!["cli relay pack".into()],
    };
    match pack_to_staging(&project_path, &req) {
        Ok(pkg) => {
            if args.json {
                println!("{}", serde_json::to_value(&pkg).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("package_id={}", pkg.package_id);
                println!("path=.openmesh/relay/staging/{}.json", pkg.package_id);
                println!("envelopes={}", pkg.envelopes.len());
                println!(
                    "content_hash={}",
                    pkg.content_hash.as_deref().unwrap_or("-")
                );
            }
            0
        }
        Err(e) => err_json(args.json, 3, "pack", &e.to_string()),
    }
}

fn run_show(args: &RelayShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pkg = if args.approved {
        read_approved_package(&project_path, &args.package_id).map_err(|e| e.to_string())
    } else if let Ok(p) = read_staging_package(&project_path, &args.package_id) {
        Ok(p)
    } else if let Ok(p) = read_approved_package(&project_path, &args.package_id) {
        Ok(p)
    } else {
        read_received_package(&project_path, &args.package_id).map_err(|e| e.to_string())
    };
    match pkg {
        Ok(p) => {
            if args.json {
                println!("{}", serde_json::to_value(&p).unwrap_or(json!({})));
            } else {
                println!("package_id={}", p.package_id);
                println!("sensitivity_max={}", p.sensitivity_max.as_str());
                println!("envelopes={}", p.envelopes.len());
                println!(
                    "approved={}",
                    p.approved_at.as_deref().unwrap_or("no")
                );
                println!(
                    "content_hash={}",
                    p.content_hash.as_deref().unwrap_or("-")
                );
            }
            0
        }
        Err(e) => err_json(args.json, 3, "show", &e.to_string()),
    }
}

fn run_approve(args: &RelayApproveArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    match approve_relay_package(
        &project_path,
        &args.package_id,
        &now,
        &args.approved_by,
    ) {
        Ok(pkg) => {
            if args.json {
                println!("{}", serde_json::to_value(&pkg).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("package_id={}", pkg.package_id);
                println!("approved_at={}", pkg.approved_at.as_deref().unwrap_or("-"));
                println!("approved_by={}", pkg.approved_by.as_deref().unwrap_or("-"));
            }
            0
        }
        Err(e) => err_json(args.json, 3, "approve", &e.to_string()),
    }
}

fn run_send(args: &RelaySendArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let root = if args.relay_root.is_absolute() {
        args.relay_root.clone()
    } else {
        cwd.join(&args.relay_root)
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    match send_package_to_relay_root(&project_path, &args.package_id, &root, &now, Some("cli")) {
        Ok(pkg) => {
            if args.json {
                println!("{}", serde_json::to_value(&pkg).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("package_id={}", pkg.package_id);
                println!("relay_root={}", root.display());
            }
            0
        }
        Err(e) => err_json(args.json, 3, "send", &e.to_string()),
    }
}

fn run_receive(args: &RelayReceiveArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let root = if args.relay_root.is_absolute() {
        args.relay_root.clone()
    } else {
        cwd.join(&args.relay_root)
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    match receive_package_from_relay_root(
        &project_path,
        &args.package_id,
        &root,
        &now,
        Some("cli"),
    ) {
        Ok(pkg) => {
            if args.json {
                println!("{}", serde_json::to_value(&pkg).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("package_id={}", pkg.package_id);
                println!("path=.openmesh/relay/received/{}.json", pkg.package_id);
            }
            0
        }
        Err(e) => err_json(args.json, 3, "receive", &e.to_string()),
    }
}

fn run_audit(args: &RelayAuditArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match list_audit_events(&project_path) {
        Ok(events) => {
            if args.json {
                println!("{}", serde_json::to_value(&events).unwrap_or(json!([])));
            } else if events.is_empty() {
                println!("(no relay audit events)");
            } else {
                for ev in events {
                    println!(
                        "{} | {} | {} | {}",
                        ev.at,
                        ev.kind.as_str(),
                        ev.package_id,
                        ev.detail
                    );
                }
            }
            0
        }
        Err(e) => err_json(args.json, 4, "audit", &e.to_string()),
    }
}

fn err_json(json_mode: bool, code: i32, category: &str, message: &str) -> i32 {
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
