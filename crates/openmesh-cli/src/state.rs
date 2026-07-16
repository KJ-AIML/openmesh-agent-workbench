// ============================================================================
// State command — Dev Track 0.1.3.7 Checkpoint E
// ============================================================================

use clap::Args;
use openmesh_core::continuity::{
    current_state_projection_path, read_current_state_projection, rebuild_current_state_projection,
    ContinuityError,
};
use openmesh_core::domain::{validate_current_state_projection, CurrentStateProjection};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::{resolve_project, ResolvedProject};

#[derive(Args, Debug, Clone)]
pub struct StateArgs {
    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,

    /// Force a deterministic rebuild and overwrite the cached projection.
    #[arg(long)]
    pub rebuild: bool,
}

pub fn run_state(args: &StateArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match load_current_state_projection(&project_path, args.rebuild) {
        Ok(projection) => {
            if let Err(err) = validate_projection_for_output(&projection) {
                return print_continuity_error(&err, args.json);
            }
            print_state_success(&projection, &resolved, args.json);
            0
        }
        Err(err) => print_continuity_error(&err, args.json),
    }
}

pub(crate) fn load_current_state_projection(
    project_path: &str,
    rebuild: bool,
) -> Result<CurrentStateProjection, ContinuityError> {
    if rebuild {
        return rebuild_current_state_projection(project_path);
    }
    if current_state_projection_path(project_path).exists() {
        read_current_state_projection(project_path)
    } else {
        rebuild_current_state_projection(project_path)
    }
}

fn print_state_success(
    projection: &CurrentStateProjection,
    resolved: &ResolvedProject,
    json_mode: bool,
) {
    let projection_path = current_state_projection_path(&resolved.path.to_string_lossy());
    if json_mode {
        if let Ok(payload) = serde_json::to_value(projection) {
            println!("{payload}");
        }
        return;
    }

    let sections = &projection.sections;
    println!("workspace_id={}", projection.workspace_id);
    println!("project={}", resolved.path.display());
    println!("generated_at={}", projection.generated_at);
    println!(
        "sections: completed={} in_progress={} blocked={} decisions={} needs_attention={} still_open={}",
        sections.completed.len(),
        sections.in_progress.len(),
        sections.blocked.len(),
        sections.decisions.len(),
        sections.needs_attention.len(),
        sections.still_open.len()
    );
    println!("pending_attention={}", projection.pending_attention.len());
    println!("limitations={}", projection.limitations.len());
    if let Ok(counts_val) = serde_json::to_value(&projection.source_counts) {
        println!(
            "source_counts: work_events={} processed_signals={} pending_signals={} audit_records={} quarantine_signals={} duplicate_signals={}",
            counts_val["workEvents"].as_u64().unwrap_or(0),
            counts_val["processedSignals"].as_u64().unwrap_or(0),
            counts_val["pendingSignals"].as_u64().unwrap_or(0),
            counts_val["promotionAuditRecords"].as_u64().unwrap_or(0),
            counts_val["quarantineSignals"].as_u64().unwrap_or(0),
            counts_val["duplicateSignals"].as_u64().unwrap_or(0),
        );
    }
    println!("projection_path={}", projection_path.display());
}

pub(crate) fn print_continuity_error(err: &ContinuityError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_continuity_error(err);
    let message = err.to_string();
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

pub(crate) fn exit_code_for_continuity_error(err: &ContinuityError) -> (i32, &'static str) {
    match err {
        ContinuityError::Validation(_) => (3, "validation"),
        ContinuityError::ProjectionNotFound => (3, "validation"),
        ContinuityError::Reader(_) | ContinuityError::Io(_) | ContinuityError::Json(_) => {
            (4, "read-failed")
        }
    }
}

pub(crate) fn validate_projection_for_output(
    projection: &CurrentStateProjection,
) -> Result<(), ContinuityError> {
    validate_current_state_projection(projection).map_err(ContinuityError::Validation)
}
