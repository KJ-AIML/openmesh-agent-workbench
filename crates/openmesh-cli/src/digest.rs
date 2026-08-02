// ============================================================================
// Return digest command — Dev Track 0.1.9
// ============================================================================

use clap::Args;
use openmesh_core::continuity::load_continuity_input_snapshot;
use openmesh_core::return_digest::{
    build_return_digest, PendingQuestionSourceKind, ReturnDigest, ReturnDigestError,
};
use std::path::Path;

use crate::catch_up::build_catch_up_window;
use crate::output;
use crate::project::resolve_project;
use crate::state::{load_current_state_projection, print_continuity_error};

#[derive(Args, Debug, Clone)]
pub struct DigestArgs {
    /// Explicit project path. If omitted, resolved by upward directory search.
    #[arg(long)]
    pub project: Option<String>,

    /// Emit machine-readable JSON output.
    #[arg(long)]
    pub json: bool,

    /// RFC 3339 UTC window start for "what I missed". Defaults to now UTC minus 24 hours.
    #[arg(long)]
    pub since: Option<String>,
}

pub fn run_digest(args: &DigestArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let window = match build_catch_up_window(args.since.as_deref()) {
        Ok(window) => window,
        Err(message) => return print_invalid_since(&message, args.json),
    };

    let snapshot = match load_continuity_input_snapshot(&project_path) {
        Ok(snapshot) => snapshot,
        Err(err) => return print_continuity_error(&err.into(), args.json),
    };
    let current_state = match load_current_state_projection(&project_path, false) {
        Ok(state) => state,
        Err(err) => return print_continuity_error(&err, args.json),
    };

    match build_return_digest(&project_path, &snapshot, &current_state, &window) {
        Ok(digest) => {
            print_digest_success(&digest, args.json);
            0
        }
        Err(err) => print_digest_error(&err, args.json),
    }
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

fn print_digest_success(digest: &ReturnDigest, json_mode: bool) {
    if json_mode {
        if let Ok(payload) = serde_json::to_value(digest) {
            println!("{payload}");
        }
        return;
    }

    println!("workspace_id={}", digest.workspace_id);
    println!("generated_at={}", digest.generated_at);
    println!("window={} .. {}", digest.window.since, digest.window.until);
    println!("summary={}", digest.summary);
    println!();
    println!("--- what needs me ({}) ---", digest.needs_me.len());
    if digest.needs_me.is_empty() {
        println!("(nothing pending)");
    } else {
        for item in &digest.needs_me {
            let source = match item.source {
                PendingQuestionSourceKind::ProxyPending => "proxy",
                PendingQuestionSourceKind::ContinuityAttention => "attention",
                PendingQuestionSourceKind::UnresolvedSignal => "signal",
            };
            println!(
                "[{}] {} | {} | {}",
                item.severity, source, item.id, item.summary
            );
        }
    }
    println!();
    let missed = &digest.what_i_missed;
    println!("--- what I missed ---");
    println!("catch_up_summary={}", digest.catch_up_summary);
    println!(
        "completed={} changed={} blocked={} decided={} needs_attention={} still_open={}",
        missed.completed.len(),
        missed.changed.len(),
        missed.blocked.len(),
        missed.decided.len(),
        missed.needs_attention.len(),
        missed.still_open.len()
    );
    println!();
    println!("--- handoffs ({}) ---", digest.handoffs.len());
    if digest.handoffs.is_empty() {
        println!("(no handoff notes)");
    } else {
        for handoff in &digest.handoffs {
            println!(
                "[{}] {} → {} (updated {})",
                handoff.status, handoff.handoff_id, handoff.recipient_label, handoff.updated_at
            );
        }
    }
    for limitation in &digest.limitations {
        println!("limitation: {limitation}");
    }
}

fn print_digest_error(err: &ReturnDigestError, json_mode: bool) -> i32 {
    match err {
        ReturnDigestError::Continuity(c) => print_continuity_error(c, json_mode),
        ReturnDigestError::Pending(p) => match p {
            openmesh_core::return_digest::PendingQuestionsError::Continuity(c) => {
                print_continuity_error(c, json_mode)
            }
            other => {
                let message = other.to_string();
                if json_mode {
                    println!(
                        "{}",
                        serde_json::json!({
                            "status": "error",
                            "category": "pending",
                            "message": message,
                        })
                    );
                } else {
                    eprintln!("ERROR pending: {message}");
                }
                3
            }
        },
        ReturnDigestError::Validation(v) => {
            let message = v.to_string();
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
    }
}
