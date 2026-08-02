//! 1.0 RC Program CLI — Dev Track 0.1.21.

use clap::{Args, Subcommand};
use openmesh_core::rc::{build_rc_pack, read_rc_pack, write_rc_pack};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum RcCommand {
    /// Evaluate RC pack (exit 0 if rc_ready, 2 otherwise).
    Check(RcCheckArgs),
    /// Show last/saved or freshly evaluated RC pack.
    Show(RcShowArgs),
    /// Print regression matrix rows.
    Matrix(RcMatrixArgs),
    /// Print freeze policy.
    #[command(name = "freeze-policy")]
    FreezePolicy(RcFreezeArgs),
}

#[derive(Args, Debug, Clone)]
pub struct RcCheckArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RcShowArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub refresh: bool,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RcMatrixArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct RcFreezeArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_rc(command: RcCommand, cwd: &Path) -> i32 {
    match command {
        RcCommand::Check(a) => run_check(&a, cwd),
        RcCommand::Show(a) => run_show(&a, cwd),
        RcCommand::Matrix(a) => run_matrix(&a, cwd),
        RcCommand::FreezePolicy(a) => run_freeze(&a, cwd),
    }
}

fn run_check(args: &RcCheckArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match build_rc_pack(&project_path) {
        Ok(pack) => {
            if let Err(e) = write_rc_pack(&project_path, &pack) {
                return err(args.json, &e.to_string());
            }
            print_pack(&pack, args.json);
            if pack.rc_ready {
                0
            } else {
                2
            }
        }
        Err(e) => err(args.json, &e.to_string()),
    }
}

fn run_show(args: &RcShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = if args.refresh {
        match build_rc_pack(&project_path) {
            Ok(p) => {
                let _ = write_rc_pack(&project_path, &p);
                p
            }
            Err(e) => return err(args.json, &e.to_string()),
        }
    } else {
        match read_rc_pack(&project_path) {
            Ok(p) => p,
            Err(_) => match build_rc_pack(&project_path) {
                Ok(p) => p,
                Err(e) => return err(args.json, &e.to_string()),
            },
        }
    };
    print_pack(&pack, args.json);
    0
}

fn run_matrix(args: &RcMatrixArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = match build_rc_pack(&project_path) {
        Ok(p) => p,
        Err(e) => return err(args.json, &e.to_string()),
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_value(&pack.regression_matrix).unwrap_or(json!([]))
        );
    } else {
        for r in &pack.regression_matrix {
            println!(
                "{:?} | {} | {} | {} | {}",
                r.status, r.id, r.area, r.surface, r.evidence
            );
        }
    }
    0
}

fn run_freeze(args: &RcFreezeArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let pack = match build_rc_pack(&project_path) {
        Ok(p) => p,
        Err(e) => return err(args.json, &e.to_string()),
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_value(&pack.freeze_policy).unwrap_or(json!({}))
        );
    } else {
        let f = &pack.freeze_policy;
        println!("features_frozen={}", f.features_frozen);
        println!("summary={}", f.summary);
        for a in &f.allowed {
            println!("  allowed: {a}");
        }
        for b in &f.forbidden {
            println!("  forbidden: {b}");
        }
    }
    0
}

fn print_pack(pack: &openmesh_core::rc::RcPack, json: bool) {
    if json {
        println!("{}", serde_json::to_value(pack).unwrap_or(json!({})));
        return;
    }
    println!("rc_ready={}", pack.rc_ready);
    println!(
        "p0_fail={} p1_fail={} open={}",
        pack.p0_fail_count, pack.p1_fail_count, pack.open_count
    );
    for c in &pack.checks {
        println!(
            "  [{:?}/{:?}] {} | {} | {}",
            c.severity, c.status, c.id, c.title, c.evidence
        );
        if let Some(d) = &c.detail {
            println!("    {d}");
        }
    }
    println!("freeze: {}", pack.freeze_policy.summary);
    for l in &pack.limitations {
        println!("  limit={l}");
    }
}

fn err(json: bool, message: &str) -> i32 {
    if json {
        println!(
            "{}",
            json!({ "error": { "category": "rc", "message": message } })
        );
    } else {
        eprintln!("ERROR rc: {message}");
    }
    1
}
