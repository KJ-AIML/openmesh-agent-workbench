// ============================================================================
// Output and exit contract — Checkpoint C (approved plan §9, Correction 1/5).
// ============================================================================
// Exit codes:
//   0 -> success
//   1 -> project unavailable / project-resolution failure
//        (including SignalError::ProjectNotInitialized surfacing from
//        write_signal itself, e.g. a TOCTOU race after CLI resolution)
//   2 -> clap usage / parse failure (owned entirely by clap, not this module)
//   3 -> signal rejected (WorkspaceMismatch, InvalidSemantics, RecordTooLarge)
//   4 -> write/system failure (Io, NameReservationFailed, Json)
//
// The mapper below is an exhaustive `match` over all seven `SignalError`
// variants with NO wildcard arm — a future variant added upstream fails this
// crate's own compilation, forcing an explicit exit-code decision rather
// than silently falling through a catch-all.
// ============================================================================

use openmesh_core::domain::{WorkSignal, WorkSignalKind};
use openmesh_core::signals::SignalError;
use serde_json::json;

/// Human-readable kebab-case form, matching the protocol's own wire casing
/// (`#[serde(rename_all = "kebab-case")]` on `WorkSignalKind`). A display
/// convenience only — the JSON success payload serializes `signal.kind`
/// directly via serde, which already produces the same strings.
fn kind_str(kind: WorkSignalKind) -> &'static str {
    match kind {
        WorkSignalKind::Progress => "progress",
        WorkSignalKind::Decision => "decision",
        WorkSignalKind::Blocker => "blocker",
        WorkSignalKind::BlockerResolved => "blocker-resolved",
        WorkSignalKind::ScopeChange => "scope-change",
        WorkSignalKind::Milestone => "milestone",
        WorkSignalKind::ReviewRequired => "review-required",
        WorkSignalKind::UnresolvedQuestion => "unresolved-question",
        WorkSignalKind::Handoff => "handoff",
        WorkSignalKind::SessionEnd => "session-end",
        WorkSignalKind::AgentSwitch => "agent-switch",
    }
}

/// Exhaustive `SignalError` -> exit-code mapping. No wildcard arm (Correction 1).
pub fn exit_code_for_signal_error(err: &SignalError) -> i32 {
    match err {
        SignalError::ProjectNotInitialized(_) => 1,
        SignalError::WorkspaceMismatch => 3,
        SignalError::InvalidSemantics(_) => 3,
        SignalError::RecordTooLarge { .. } => 3,
        SignalError::NameReservationFailed(_) => 4,
        SignalError::Io(_) => 4,
        SignalError::Json(_) => 4,
    }
}

fn category_for_exit_code(code: i32) -> &'static str {
    match code {
        1 => "project-resolution",
        3 => "invalid-signal",
        4 => "write-failed",
        _ => unreachable!("exit_code_for_signal_error only ever returns 1, 3, or 4"),
    }
}

/// Prints the success payload (human or `--json`) and always exposes at
/// least `status`/`signal_id`/`kind`/`project`/`workspace_id` in JSON mode,
/// and `signal_id`/`kind`/`project` in human mode.
pub fn print_success(signal: &WorkSignal, project_path: &str, json_mode: bool) {
    if json_mode {
        let payload = json!({
            "status": "ok",
            "signal_id": signal.signal_id,
            "kind": signal.kind,
            "project": project_path,
            "workspace_id": signal.workspace_id,
        });
        println!("{payload}");
    } else {
        println!(
            "OK  signal_id={}  kind={}  project={}",
            signal.signal_id,
            kind_str(signal.kind),
            project_path
        );
    }
}

/// Prints a project-resolution failure and returns its exit code (always 1).
/// Post-parse failure path — respects `--json` (§9).
pub fn print_project_resolution_error(message: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "category": "project-resolution", "message": message})
        );
    } else {
        eprintln!("ERROR project-resolution: {message}");
    }
    1
}

/// Prints a `write_signal` failure and returns its mapped exit code.
/// Post-parse failure path — respects `--json` (§9).
pub fn print_signal_error(err: &SignalError, json_mode: bool) -> i32 {
    let code = exit_code_for_signal_error(err);
    let category = category_for_exit_code(code);
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
    fn every_signal_error_variant_maps_to_its_frozen_exit_code() {
        assert_eq!(
            exit_code_for_signal_error(&SignalError::ProjectNotInitialized("p".into())),
            1
        );
        assert_eq!(
            exit_code_for_signal_error(&SignalError::WorkspaceMismatch),
            3
        );
        assert_eq!(
            exit_code_for_signal_error(&SignalError::InvalidSemantics("x".into())),
            3
        );
        assert_eq!(
            exit_code_for_signal_error(&SignalError::RecordTooLarge {
                actual: 300_000,
                max: 262_144
            }),
            3
        );
        assert_eq!(
            exit_code_for_signal_error(&SignalError::NameReservationFailed(5)),
            4
        );
        assert_eq!(
            exit_code_for_signal_error(&SignalError::Io(std::io::Error::other("boom"))),
            4
        );
        let bad_json = serde_json::from_str::<serde_json::Value>("{ not json").unwrap_err();
        assert_eq!(exit_code_for_signal_error(&SignalError::Json(bad_json)), 4);
    }

    #[test]
    fn kind_str_matches_the_wire_form_for_every_variant() {
        assert_eq!(kind_str(WorkSignalKind::Progress), "progress");
        assert_eq!(
            kind_str(WorkSignalKind::BlockerResolved),
            "blocker-resolved"
        );
        assert_eq!(kind_str(WorkSignalKind::ScopeChange), "scope-change");
        assert_eq!(
            kind_str(WorkSignalKind::UnresolvedQuestion),
            "unresolved-question"
        );
        assert_eq!(kind_str(WorkSignalKind::SessionEnd), "session-end");
        assert_eq!(kind_str(WorkSignalKind::AgentSwitch), "agent-switch");
    }
}
