// ============================================================================
// Event commands — evidence correction inspect/correct CLI
// ============================================================================

use clap::Args;
use openmesh_core::domain::ContinuityConfidence;
use openmesh_core::{
    append_event_correction, inspect_event, AppendCorrectionResult, EventCorrectionRequest,
    EventError, EventInspection,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Args, Debug, Clone)]
pub struct EventInspectArgs {
    /// WorkEvent id to inspect in the project ledger.
    pub event_id: String,

    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct EventCorrectArgs {
    /// WorkEvent id to correct in the project ledger.
    pub event_id: String,

    /// Corrected event kind (becomes effective kind for the target).
    #[arg(long)]
    pub kind: String,

    /// Corrected human-readable summary (becomes effective summary for the target).
    #[arg(long)]
    pub summary: String,

    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Actor label recorded on the correction event. Defaults to `cli-operator`.
    #[arg(long)]
    pub actor_label: Option<String>,

    /// Explicit RFC 3339 UTC timestamp override. Defaults to now.
    #[arg(long)]
    pub timestamp: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,
}

pub fn run_event_inspect(args: &EventInspectArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match inspect_event(&project_path, &args.event_id) {
        Ok(inspection) => {
            print_inspect_success(&inspection, &project_path, args.json);
            0
        }
        Err(err) => print_event_error(&err, args.json),
    }
}

pub fn run_event_correct(args: &EventCorrectArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let kind = match parse_work_event_kind(&args.kind) {
        Ok(kind) => kind,
        Err(message) => return print_invalid_event_request(&message, args.json),
    };
    let summary = args.summary.trim().to_string();
    if summary.is_empty() {
        return print_invalid_event_request("summary is empty after trim", args.json);
    }

    let project_path = resolved.path.to_string_lossy().to_string();
    let request = EventCorrectionRequest {
        corrected_kind: kind,
        corrected_summary: summary,
        actor_label: args.actor_label.clone(),
        timestamp: args.timestamp.clone(),
    };

    match append_event_correction(&project_path, &args.event_id, &request) {
        Ok(result) => {
            print_correct_success(&result, &project_path, args.json);
            0
        }
        Err(err) => print_event_error(&err, args.json),
    }
}

pub fn parse_work_event_kind(kind: &str) -> Result<String, String> {
    let trimmed = kind.trim();
    if trimmed.is_empty() {
        return Err("kind is empty after trim".into());
    }
    Ok(trimmed.to_string())
}

fn confidence_label(confidence: ContinuityConfidence) -> &'static str {
    match confidence {
        ContinuityConfidence::High => "high",
        ContinuityConfidence::Medium => "medium",
        ContinuityConfidence::Low => "low",
        ContinuityConfidence::Ambiguous => "ambiguous",
    }
}

fn print_inspect_success(inspection: &EventInspection, project_path: &str, json_mode: bool) {
    let presentation = &inspection.effective_presentation;
    if json_mode {
        let payload = json!({
            "status": "ok",
            "project": project_path,
            "eventId": inspection.event_id,
            "original": inspection.original,
            "effectiveKind": presentation.kind_text(),
            "effectiveSummary": presentation.summary_text(),
            "originalKind": presentation.original_kind_text(),
            "originalSummary": presentation.original_summary_text(),
            "isCorrected": presentation.is_corrected,
            "isSupersededOriginal": presentation.is_superseded_original,
            "confidence": confidence_label(presentation.confidence),
            "correctionEventIds": presentation.correction_event_ids,
            "correctionEvents": inspection.correction_events,
            "supersededByEventId": presentation.superseded_by_event_id,
            "diagnostics": presentation.diagnostics,
        });
        println!("{payload}");
        return;
    }

    println!("event_id={}", inspection.event_id);
    println!("project={project_path}");
    println!("original_kind={}", presentation.original_kind_text());
    println!("original_summary={}", presentation.original_summary_text());
    println!("effective_kind={}", presentation.kind_text());
    println!("effective-summary={}", presentation.summary_text());
    println!("confidence={}", confidence_label(presentation.confidence));
    println!(
        "is_superseded_original={}",
        presentation.is_superseded_original
    );
    if presentation.correction_event_ids.is_empty() {
        println!("correction_chain=(none)");
    } else {
        println!(
            "correction_chain={}",
            presentation.correction_event_ids.join(", ")
        );
    }
    if !presentation.diagnostics.is_empty() {
        println!("diagnostics={}", presentation.diagnostics.len());
        for diagnostic in &presentation.diagnostics {
            println!("  - {diagnostic:?}");
        }
    }
    if presentation.is_corrected {
        println!("note=corrected presentation is capped at medium confidence");
    }
}

fn print_correct_success(result: &AppendCorrectionResult, project_path: &str, json_mode: bool) {
    let presentation = &result.effective_presentation;
    if json_mode {
        let payload = json!({
            "status": "ok",
            "project": project_path,
            "targetEventId": result.target_event_id,
            "correctionEvent": result.correction_event,
            "effectiveKind": presentation.kind_text(),
            "effectiveSummary": presentation.summary_text(),
            "confidence": confidence_label(presentation.confidence),
            "isCorrected": presentation.is_corrected,
            "correctionEventIds": presentation.correction_event_ids,
        });
        println!("{payload}");
        return;
    }

    println!(
        "OK  correction_event_id={}  target_event_id={}  project={}",
        result.correction_event.event_id, result.target_event_id, project_path
    );
    println!("effective_kind={}", presentation.kind_text());
    println!("effective-summary={}", presentation.summary_text());
    println!("confidence={}", confidence_label(presentation.confidence));
}

fn print_invalid_event_request(message: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "category": "invalid-event", "message": message})
        );
    } else {
        eprintln!("ERROR invalid-event: {message}");
    }
    3
}

pub fn exit_code_for_event_error(err: &EventError) -> i32 {
    match err {
        EventError::ProjectNotInitialized(_) => 1,
        EventError::NotFound(_) | EventError::CorrectionTargetNotFound(_) => 3,
        EventError::WorkspaceMismatch
        | EventError::InvalidSemantics(_)
        | EventError::UnsafeEventId(_)
        | EventError::SelfCorrectionNotAllowed
        | EventError::CorrectionCycle(_)
        | EventError::DuplicateEventId(_)
        | EventError::RecordTooLarge { .. } => 3,
        EventError::Io(_) | EventError::Json(_) => 4,
    }
}

fn category_for_event_exit_code(code: i32) -> &'static str {
    match code {
        1 => "project-resolution",
        3 => "invalid-event",
        4 => "write-failed",
        _ => "invalid-event",
    }
}

pub fn print_event_error(err: &EventError, json_mode: bool) -> i32 {
    let code = exit_code_for_event_error(err);
    let category = category_for_event_exit_code(code);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_work_event_kind_rejects_empty() {
        assert!(parse_work_event_kind("").is_err());
        assert!(parse_work_event_kind("   ").is_err());
    }

    #[test]
    fn parse_work_event_kind_trims_and_accepts() {
        assert_eq!(
            parse_work_event_kind("  work.blocked  ").unwrap(),
            "work.blocked"
        );
    }
}
