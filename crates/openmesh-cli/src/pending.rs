// ============================================================================
// Pending questions command — Dev Track 0.1.9
// ============================================================================

use clap::Args;
use openmesh_core::continuity::load_continuity_input_snapshot;
use openmesh_core::return_digest::{
    build_pending_questions_view, PendingQuestionSourceKind, PendingQuestionsError,
    PendingQuestionsView,
};
use std::path::Path;

use crate::output;
use crate::project::resolve_project;
use crate::state::{load_current_state_projection, print_continuity_error};

#[derive(Args, Debug, Clone)]
pub struct PendingArgs {
    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,
}

pub fn run_pending(args: &PendingArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let snapshot = match load_continuity_input_snapshot(&project_path) {
        Ok(snapshot) => snapshot,
        Err(err) => return print_continuity_error(&err.into(), args.json),
    };
    let current_state = match load_current_state_projection(&project_path, false) {
        Ok(state) => state,
        Err(err) => return print_continuity_error(&err, args.json),
    };

    match build_pending_questions_view(&project_path, &snapshot, &current_state) {
        Ok(view) => {
            print_pending_success(&view, args.json);
            0
        }
        Err(err) => print_pending_error(&err, args.json),
    }
}

fn print_pending_success(view: &PendingQuestionsView, json_mode: bool) {
    if json_mode {
        if let Ok(payload) = serde_json::to_value(view) {
            println!("{payload}");
        }
        return;
    }

    println!("workspace_id={}", view.workspace_id);
    println!("generated_at={}", view.generated_at);
    println!("open_count={}", view.open_count);
    println!(
        "sources: proxy={} attention={} unresolved-signal={}",
        view.source_counts.proxy_pending,
        view.source_counts.continuity_attention,
        view.source_counts.unresolved_signal
    );
    if view.items.is_empty() {
        println!("(no pending questions)");
    } else {
        println!("--- needs me ---");
        for item in &view.items {
            let source = match item.source {
                PendingQuestionSourceKind::ProxyPending => "proxy",
                PendingQuestionSourceKind::ContinuityAttention => "attention",
                PendingQuestionSourceKind::UnresolvedSignal => "signal",
            };
            println!(
                "[{}] {} | {} | {} | {}",
                item.severity, source, item.status, item.id, item.summary
            );
        }
    }
    for limitation in &view.limitations {
        println!("limitation: {limitation}");
    }
}

fn print_pending_error(err: &PendingQuestionsError, json_mode: bool) -> i32 {
    let (category, message) = match err {
        PendingQuestionsError::Continuity(c) => {
            return print_continuity_error(c, json_mode);
        }
        PendingQuestionsError::Validation(v) => ("validation", v.to_string()),
        PendingQuestionsError::ProjectNotInitialized => {
            ("project", "project not initialized".to_string())
        }
        PendingQuestionsError::ProxyPendingRead => {
            ("io", "failed to read proxy pending questions".to_string())
        }
    };
    if json_mode {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "category": category,
                "message": message,
            })
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    3
}
