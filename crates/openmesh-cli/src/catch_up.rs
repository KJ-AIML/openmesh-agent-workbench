// ============================================================================
// Catch-up command — Dev Track 0.1.3.7 Checkpoint E
// ============================================================================

use chrono::{Duration, Utc};
use clap::Args;
use openmesh_core::continuity::{
    build_catch_up_view, load_continuity_input_snapshot, ContinuityError,
};
use openmesh_core::domain::{validate_catch_up_view, CatchUpView, CatchUpWindow};
use std::path::Path;

use crate::output;
use crate::project::resolve_project;
use crate::state::{load_current_state_projection, print_continuity_error};

#[derive(Args, Debug, Clone)]
pub struct CatchUpArgs {
    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,

    /// RFC 3339 UTC window start. Defaults to now UTC minus 24 hours.
    #[arg(long)]
    pub since: Option<String>,
}

pub fn run_catch_up(args: &CatchUpArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let window = match build_catch_up_window(args.since.as_deref()) {
        Ok(window) => window,
        Err(message) => return print_invalid_since(&message, args.json),
    };

    match build_catch_up_for_project(&project_path, &window) {
        Ok(view) => {
            print_catch_up_success(&view, args.json);
            0
        }
        Err(err) => print_continuity_error(&err, args.json),
    }
}

fn build_catch_up_for_project(
    project_path: &str,
    window: &CatchUpWindow,
) -> Result<CatchUpView, ContinuityError> {
    let snapshot = load_continuity_input_snapshot(project_path)?;
    let current_state = load_current_state_projection(project_path, false)?;
    let view = build_catch_up_view(&snapshot, &current_state, window)?;
    validate_catch_up_view(&view).map_err(ContinuityError::Validation)?;
    Ok(view)
}

pub(crate) fn build_catch_up_window(since_override: Option<&str>) -> Result<CatchUpWindow, String> {
    let until = Utc::now();
    let until_str = until.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let since_str = match since_override {
        Some(raw) => {
            validate_since_timestamp(raw)?;
            raw.to_string()
        }
        None => {
            let since = until - Duration::hours(24);
            since.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        }
    };
    Ok(CatchUpWindow {
        since: since_str,
        until: until_str,
    })
}

fn validate_since_timestamp(raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err(format!(
            "invalid --since value `{raw}` (expected RFC 3339 UTC timestamp)"
        ));
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| format!("invalid --since value `{raw}` (expected RFC 3339 UTC timestamp)"))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!(
            "invalid --since value `{raw}` (timestamp must use UTC `Z` offset)"
        ));
    }
    Ok(())
}

fn print_invalid_since(message: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "category": "validation",
                "message": message,
            })
        );
    } else {
        eprintln!("ERROR validation: {message}");
    }
    3
}

fn print_catch_up_success(view: &CatchUpView, json_mode: bool) {
    if json_mode {
        if let Ok(payload) = serde_json::to_value(view) {
            println!("{payload}");
        }
        return;
    }

    let sections = &view.sections;
    println!(
        "window: since={} until={}",
        view.window.since, view.window.until
    );
    println!("summary: {}", view.summary);
    println!(
        "sections: completed={} changed={} blocked={} decided={} needs_attention={} still_open={}",
        sections.completed.len(),
        sections.changed.len(),
        sections.blocked.len(),
        sections.decided.len(),
        sections.needs_attention.len(),
        sections.still_open.len()
    );
    println!(
        "next_suggested_attention={}",
        view.next_suggested_attention.len()
    );
    println!("limitations={}", view.limitations.len());
    println!("evidence_refs={}", view.evidence_refs.len());
}
