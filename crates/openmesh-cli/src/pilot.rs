//! Enterprise Pilot Readiness CLI — Dev Track 0.1.20.

use clap::{Args, Subcommand};
use openmesh_core::pilot::{build_pilot_pack, read_pilot_pack, write_pilot_pack};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum PilotCommand {
    /// Evaluate and persist the pilot readiness pack.
    Check(PilotCheckArgs),
    /// Show last saved pack (or evaluate if missing with --refresh).
    Show(PilotShowArgs),
    /// Print pilot runbook steps.
    Runbook(PilotRunbookArgs),
    /// Print threat-model notes from the pack.
    Threats(PilotThreatsArgs),
}

#[derive(Args, Debug, Clone)]
pub struct PilotCheckArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PilotShowArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub refresh: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PilotRunbookArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct PilotThreatsArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_pilot(command: PilotCommand, cwd: &Path) -> i32 {
    match command {
        PilotCommand::Check(a) => run_check(&a, cwd),
        PilotCommand::Show(a) => run_show(&a, cwd),
        PilotCommand::Runbook(a) => run_runbook(&a, cwd),
        PilotCommand::Threats(a) => run_threats(&a, cwd),
    }
}

fn run_check(args: &PilotCheckArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match build_pilot_pack(&project_path) {
        Ok(pack) => {
            if let Err(e) = write_pilot_pack(&project_path, &pack) {
                return err(args.json, &e.to_string());
            }
            print_pack(&pack, args.json);
            if pack.pilot_ready {
                0
            } else {
                2
            }
        }
        Err(e) => err(args.json, &e.to_string()),
    }
}

fn run_show(args: &PilotShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = if args.refresh {
        match build_pilot_pack(&project_path) {
            Ok(p) => {
                let _ = write_pilot_pack(&project_path, &p);
                p
            }
            Err(e) => return err(args.json, &e.to_string()),
        }
    } else {
        match read_pilot_pack(&project_path) {
            Ok(p) => p,
            Err(_) => match build_pilot_pack(&project_path) {
                Ok(p) => p,
                Err(e) => return err(args.json, &e.to_string()),
            },
        }
    };
    print_pack(&pack, args.json);
    0
}

fn run_runbook(args: &PilotRunbookArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = match build_pilot_pack(&project_path) {
        Ok(p) => p,
        Err(e) => return err(args.json, &e.to_string()),
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_value(&pack.runbook).unwrap_or(json!([]))
        );
    } else {
        for s in &pack.runbook {
            println!("{} | {}", s.id, s.title);
            println!("  action: {}", s.command_or_action);
            println!("  why: {}", s.purpose);
        }
    }
    0
}

fn run_threats(args: &PilotThreatsArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = match build_pilot_pack(&project_path) {
        Ok(p) => p,
        Err(e) => return err(args.json, &e.to_string()),
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_value(&pack.threat_notes).unwrap_or(json!([]))
        );
    } else {
        for t in &pack.threat_notes {
            println!("{} | {}", t.id, t.title);
            println!("  summary: {}", t.summary);
            println!("  residual: {}", t.residual);
        }
    }
    0
}

fn print_pack(pack: &openmesh_core::pilot::PilotPack, json: bool) {
    if json {
        println!("{}", serde_json::to_value(pack).unwrap_or(json!({})));
        return;
    }
    println!("pilot_ready={}", pack.pilot_ready);
    println!(
        "pass={} warn={} fail={}",
        pack.pass_count, pack.warn_count, pack.fail_count
    );
    for c in &pack.checks {
        println!(
            "  [{:?}] {} | {} | {}",
            c.status, c.id, c.title, c.evidence
        );
        if let Some(d) = &c.detail {
            println!("    {}", d);
        }
    }
    for l in &pack.limitations {
        println!("  limit={l}");
    }
}

fn err(json: bool, message: &str) -> i32 {
    if json {
        println!(
            "{}",
            json!({ "error": { "category": "pilot", "message": message } })
        );
    } else {
        eprintln!("ERROR pilot: {message}");
    }
    1
}
