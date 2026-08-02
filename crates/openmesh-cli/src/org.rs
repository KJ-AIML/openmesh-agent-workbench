//! Organization Graph CLI — Dev Track 0.1.19.

use clap::{Args, Subcommand};
use openmesh_core::org_graph::build_org_graph;
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum OrgCommand {
    /// Show evidence-backed org graph projection.
    #[command(subcommand)]
    Graph(OrgGraphCommand),
}

#[derive(Subcommand, Debug)]
pub enum OrgGraphCommand {
    Show(OrgGraphShowArgs),
}

#[derive(Args, Debug, Clone)]
pub struct OrgGraphShowArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_org(command: OrgCommand, cwd: &Path) -> i32 {
    match command {
        OrgCommand::Graph(OrgGraphCommand::Show(a)) => run_show(&a, cwd),
    }
}

fn run_show(args: &OrgGraphShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match build_org_graph(&project_path) {
        Ok(g) => {
            if args.json {
                println!("{}", serde_json::to_value(&g).unwrap_or(json!({})));
            } else {
                println!("team_id={}", g.team_id);
                println!("nodes={}", g.nodes.len());
                println!("edges={}", g.edges.len());
                for n in &g.nodes {
                    println!("  node {:?} {} | {}", n.kind, n.id, n.label);
                }
                for e in &g.edges {
                    println!("  edge {} -[{:?}]-> {}", e.from, e.kind, e.to);
                }
                for l in &g.limitations {
                    println!("  limit={l}");
                }
            }
            0
        }
        Err(e) => {
            if args.json {
                println!(
                    "{}",
                    json!({ "error": { "category": "org_graph", "message": e.to_string() } })
                );
            } else {
                eprintln!("ERROR org_graph: {e}");
            }
            1
        }
    }
}
