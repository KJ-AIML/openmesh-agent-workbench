// ============================================================================
// Collect commands — Dev Track 0.1.3.6 Checkpoint E
// ============================================================================

use clap::Args;
use openmesh_core::producers::{collect_git_signal, collect_heli_signal, CollectSignalOutcome};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::{resolve_project, ResolvedProject};

#[derive(Args, Debug, Clone)]
pub struct CollectArgs {
    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Optional correlation hint shared across producers.
    #[arg(long = "correlation-hint")]
    pub correlation_hint: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,
}

pub fn run_collect_git(args: &CollectArgs, cwd: &Path) -> i32 {
    run_collect(args, cwd, CollectTarget::Git)
}

pub fn run_collect_heli(args: &CollectArgs, cwd: &Path) -> i32 {
    run_collect(args, cwd, CollectTarget::Heli)
}

enum CollectTarget {
    Git,
    Heli,
}

fn run_collect(args: &CollectArgs, cwd: &Path, target: CollectTarget) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    match target {
        CollectTarget::Git => execute_git_collect(args, &resolved),
        CollectTarget::Heli => execute_heli_collect(args, &resolved),
    }
}

fn execute_git_collect(args: &CollectArgs, resolved: &ResolvedProject) -> i32 {
    let project_path = resolved.path.to_string_lossy().to_string();
    match collect_git_signal(
        &resolved.path,
        &resolved.project.id,
        args.correlation_hint.clone(),
    ) {
        Ok(CollectSignalOutcome::Written { signal_id }) => {
            print_collect_success(
                "git",
                &signal_id,
                &project_path,
                &resolved.project.id,
                args.json,
            );
            0
        }
        Ok(CollectSignalOutcome::Skipped { reason }) => {
            print_collect_skip("git", &format!("{reason:?}"), args.json);
            0
        }
        Err(err) => print_collect_error("git", &err.to_string(), args.json),
    }
}

fn execute_heli_collect(args: &CollectArgs, resolved: &ResolvedProject) -> i32 {
    let project_path = resolved.path.to_string_lossy().to_string();
    match collect_heli_signal(
        &resolved.path,
        &resolved.project.id,
        args.correlation_hint.clone(),
    ) {
        Ok(CollectSignalOutcome::Written { signal_id }) => {
            print_collect_success(
                "heli",
                &signal_id,
                &project_path,
                &resolved.project.id,
                args.json,
            );
            0
        }
        Ok(CollectSignalOutcome::Skipped { reason }) => {
            if matches!(
                reason,
                openmesh_core::domain::ProducerSkipReason::HeliAbsent
            ) {
                print_collect_heli_absent(args.json);
                return 0;
            }
            print_collect_skip("heli", &format!("{reason:?}"), args.json);
            0
        }
        Err(err) => print_collect_error("heli", &err.to_string(), args.json),
    }
}

fn print_collect_success(
    producer: &str,
    signal_id: &str,
    project_path: &str,
    workspace_id: &str,
    json_mode: bool,
) {
    if json_mode {
        let payload = json!({
            "status": "ok",
            "producer": producer,
            "signalId": signal_id,
            "project": project_path,
            "workspaceId": workspace_id,
        });
        println!("{payload}");
    } else {
        println!("OK  collect {producer}  signal_id={signal_id}  project={project_path}");
    }
}

fn print_collect_heli_absent(json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            json!({"status": "skipped", "producer": "heli", "message": "heli: absent (no signal written)"})
        );
    } else {
        println!("heli: absent (no signal written)");
    }
}

fn print_collect_skip(producer: &str, reason: &str, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            json!({"status": "skipped", "producer": producer, "message": reason})
        );
    } else {
        println!("collect {producer}: skipped ({reason})");
    }
}

fn print_collect_error(producer: &str, message: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "producer": producer, "message": message})
        );
    } else {
        eprintln!("ERROR collect {producer}: {message}");
    }
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_ref_wire_names_match_collect_targets() {
        use openmesh_core::domain::ProducerRef;
        let git = ProducerRef::Git;
        let heli = ProducerRef::Heli;
        let git_json = serde_json::to_string(&git).unwrap();
        let heli_json = serde_json::to_string(&heli).unwrap();
        assert!(git_json.contains("git"));
        assert!(heli_json.contains("heli"));
    }
}
