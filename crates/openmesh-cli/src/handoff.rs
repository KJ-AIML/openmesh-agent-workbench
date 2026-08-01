// ============================================================================
// Handoff commands — Dev Track 0.1.8 Checkpoint F
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::continuity::load_continuity_input_snapshot;
use openmesh_core::handoff::{
    approve_handoff_note, build_handoff_note, build_handoff_recipient, link_handoff_work_event,
    read_handoff_note, render_handoff_markdown, resolve_handoff_window, write_handoff_note,
    BuildHandoffRequest, HandoffBuildError, HandoffNote, HandoffStatus, HandoffStorageError,
};
use openmesh_core::storage::{read_project, Project};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;
use crate::state::load_current_state_projection;

#[derive(Subcommand, Debug)]
pub enum HandoffCommand {
    /// Build and persist a draft handoff note for a recipient.
    Create(HandoffCreateArgs),
    /// Show a persisted handoff note (read-only).
    Show(HandoffShowArgs),
    /// Approve a persisted handoff note.
    Approve(HandoffApproveArgs),
    /// Export a handoff note as markdown to stdout.
    Export(HandoffExportArgs),
}

#[derive(Args, Debug, Clone)]
pub struct HandoffCreateArgs {
    /// Recipient label (required).
    #[arg(long)]
    pub recipient: String,

    /// Optional recipient role label.
    #[arg(long)]
    pub role: Option<String>,

    /// RFC 3339 UTC window start. With --until, overrides the default 7-day window.
    #[arg(long)]
    pub since: Option<String>,

    /// RFC 3339 UTC window end. With --since, overrides the default 7-day window.
    #[arg(long)]
    pub until: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,

    /// Append a `work.handoff` WorkEvent and link it on the note.
    #[arg(long = "link-event")]
    pub link_event: bool,
}

#[derive(Args, Debug, Clone)]
pub struct HandoffShowArgs {
    #[arg(long = "id")]
    pub handoff_id: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct HandoffApproveArgs {
    #[arg(long = "id")]
    pub handoff_id: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,

    #[arg(long = "link-event")]
    pub link_event: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum HandoffExportFormat {
    #[default]
    Markdown,
}

#[derive(Args, Debug, Clone)]
pub struct HandoffExportArgs {
    #[arg(long = "id")]
    pub handoff_id: String,

    #[arg(long, value_enum, default_value_t = HandoffExportFormat::Markdown)]
    pub format: HandoffExportFormat,

    #[arg(long)]
    pub project: Option<String>,
}

pub fn run_handoff(cmd: HandoffCommand, cwd: &Path) -> i32 {
    match cmd {
        HandoffCommand::Create(args) => run_handoff_create(&args, cwd),
        HandoffCommand::Show(args) => run_handoff_show(&args, cwd),
        HandoffCommand::Approve(args) => run_handoff_approve(&args, cwd),
        HandoffCommand::Export(args) => run_handoff_export(&args, cwd),
    }
}

pub fn run_handoff_create(args: &HandoffCreateArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    if args.recipient.trim().is_empty() {
        return print_handoff_error(
            "recipient label is empty after trim",
            "validation",
            3,
            args.json,
        );
    }

    let project_path = resolved.path.to_string_lossy().to_string();
    let project = match read_project::<Project>(&project_path, "project.json") {
        Some(project) => project,
        None => {
            return print_handoff_storage_error(
                &HandoffStorageError::ProjectNotInitialized,
                args.json,
            )
        }
    };

    let recipient = match build_handoff_recipient(&args.recipient, args.role.as_deref()) {
        Ok(recipient) => recipient,
        Err(err) => {
            return print_handoff_error(&err.to_string(), "validation", 3, args.json);
        }
    };

    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let window = match resolve_handoff_window(args.since.as_deref(), args.until.as_deref(), &now) {
        Ok(window) => window,
        Err(err) => return print_handoff_error(&err.to_string(), "invalid-window", 3, args.json),
    };

    let snapshot = match load_continuity_input_snapshot(&project_path) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return print_handoff_error(&err.to_string(), "continuity-snapshot", 3, args.json);
        }
    };

    let current_state = match load_current_state_projection(&project_path, false) {
        Ok(projection) => projection,
        Err(err) => return crate::state::print_continuity_error(&err, args.json),
    };

    let request = BuildHandoffRequest {
        workspace_id: project.id,
        recipient,
        window,
        now_rfc3339: now,
    };

    let mut note = match build_handoff_note(&snapshot, &current_state, &request) {
        Ok(note) => note,
        Err(err) => return print_handoff_build_error(&err, args.json),
    };

    if let Err(err) = write_handoff_note(&project_path, &note) {
        return print_handoff_storage_error(&err, args.json);
    }

    if args.link_event {
        note = match link_handoff_work_event(&project_path, note) {
            Ok(note) => note,
            Err(err) => return print_handoff_storage_error(&err, args.json),
        };
    }

    print_handoff_success(&note, &project_path, args.json);
    0
}

pub fn run_handoff_show(args: &HandoffShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_handoff_note(&project_path, &args.handoff_id) {
        Ok(note) => {
            print_handoff_success(&note, &project_path, args.json);
            0
        }
        Err(err) => print_handoff_storage_error(&err, args.json),
    }
}

pub fn run_handoff_approve(args: &HandoffApproveArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let mut note = match approve_handoff_note(&project_path, &args.handoff_id, &now) {
        Ok(note) => note,
        Err(err) => return print_handoff_storage_error(&err, args.json),
    };

    if args.link_event {
        note = match link_handoff_work_event(&project_path, note) {
            Ok(note) => note,
            Err(err) => return print_handoff_storage_error(&err, args.json),
        };
    }

    print_handoff_success(&note, &project_path, args.json);
    0
}

pub fn run_handoff_export(args: &HandoffExportArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => {
            return output::print_project_resolution_error(&err.describe(), false);
        }
    };

    if !matches!(args.format, HandoffExportFormat::Markdown) {
        return print_handoff_error(
            "unsupported export format (only markdown is supported)",
            "validation",
            3,
            false,
        );
    }

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_handoff_note(&project_path, &args.handoff_id) {
        Ok(note) => {
            print!("{}", render_handoff_markdown(&note));
            0
        }
        Err(err) => print_handoff_storage_error(&err, false),
    }
}

fn print_handoff_success(note: &HandoffNote, project_path: &str, json_mode: bool) {
    if json_mode {
        let payload = json!({
            "status": "ok",
            "project": project_path,
            "handoffId": note.handoff_id,
            "handoffStatus": status_wire(note.status),
            "recipient": note.recipient,
            "window": note.window,
            "limitationsCount": note.limitations.len(),
            "workEventId": note.work_event_id,
            "note": note,
        });
        println!(
            "{}",
            serde_json::to_string(&payload).expect("serialize json")
        );
        return;
    }

    println!("handoff_id={}", note.handoff_id);
    println!("status={}", status_wire(note.status));
    println!("recipient={}", note.recipient.label);
    if let Some(role) = &note.recipient.role_label {
        println!("role={role}");
    }
    println!(
        "window_since={} window_until={}",
        note.window.since, note.window.until
    );
    println!("limitations_count={}", note.limitations.len());
    println!("section_items_total={}", section_items_total(note));
    if let Some(work_event_id) = &note.work_event_id {
        println!("work_event_id={work_event_id}");
    }
    println!("project={project_path}");
    println!("note=local handoff package only; not a remote share or authority action");
}

fn section_items_total(note: &HandoffNote) -> usize {
    note.what_changed.items.len()
        + note.what_is_complete.items.len()
        + note.what_is_blocked.items.len()
        + note.what_needs_review.items.len()
        + note.open_questions.items.len()
        + note.safe_to_answer_context.items.len()
        + note.next_suggested_step.items.len()
}

fn status_wire(status: HandoffStatus) -> &'static str {
    match status {
        HandoffStatus::Draft => "draft",
        HandoffStatus::Approved => "approved",
    }
}

pub fn print_handoff_error(message: &str, category: &str, code: i32, json_mode: bool) -> i32 {
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

pub fn print_handoff_build_error(err: &HandoffBuildError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_handoff_build_error(err);
    let message = safe_build_error_message(err);
    print_handoff_error(&message, category, code, json_mode)
}

pub fn print_handoff_storage_error(err: &HandoffStorageError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_handoff_storage_error(err);
    let message = safe_storage_error_message(err);
    print_handoff_error(&message, category, code, json_mode)
}

fn safe_build_error_message(err: &HandoffBuildError) -> String {
    match err {
        HandoffBuildError::WorkspaceMismatch => {
            "workspace_id does not match continuity snapshot".into()
        }
        HandoffBuildError::InvalidTimestamp(_) => "invalid timestamp".into(),
        HandoffBuildError::Continuity(_) => "continuity build failed".into(),
        HandoffBuildError::Validation(_) => "handoff validation failed".into(),
    }
}

fn safe_storage_error_message(err: &HandoffStorageError) -> String {
    match err {
        HandoffStorageError::ValidationFailed { .. } => "handoff validation failed".into(),
        HandoffStorageError::AlreadyLinked(id) => {
            format!("handoff is already linked to work event {id}")
        }
        other => other.to_string(),
    }
}

pub fn exit_code_for_handoff_build_error(err: &HandoffBuildError) -> (i32, &'static str) {
    match err {
        HandoffBuildError::WorkspaceMismatch
        | HandoffBuildError::InvalidTimestamp(_)
        | HandoffBuildError::Continuity(_)
        | HandoffBuildError::Validation(_) => (3, "handoff-build-failed"),
    }
}

pub fn exit_code_for_handoff_storage_error(err: &HandoffStorageError) -> (i32, &'static str) {
    match err {
        HandoffStorageError::ProjectNotInitialized => (1, "project-not-initialized"),
        HandoffStorageError::NotFound => (3, "handoff-not-found"),
        HandoffStorageError::MalformedJson
        | HandoffStorageError::ValidationFailed { .. }
        | HandoffStorageError::WorkspaceMismatch
        | HandoffStorageError::AlreadyLinked(_) => (3, "invalid-handoff"),
        HandoffStorageError::Ledger(_) => (3, "ledger-error"),
        HandoffStorageError::ReadFailed
        | HandoffStorageError::WriteFailed
        | HandoffStorageError::AtomicReplaceFailed => (4, "write-failed"),
    }
}
