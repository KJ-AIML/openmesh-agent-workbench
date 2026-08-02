// Checkpoint A — CLI parser tests (approved plan §17/§19.B).
// Verifies: all 11 kinds recognized; phantom top-level commands absent; --summary
// required; missing summary exits 2 (clap usage failure); --help exits 0.
//
// Uses std::process::Command directly against the real compiled binary (via
// Cargo's CARGO_BIN_EXE_<name> env var) — no test-helper dependency added,
// consistent with the plan's frozen dependency accounting (§3).
//
// HARD RULE (2026-07-09 test-isolation correction): `signal <kind>` commands
// are side-effecting — they resolve an ambient project (via `--project`, or
// by walking upward from the real `cwd` if `--project` is omitted) and call
// the real `write_signal`. This file proves parser *recognition* only, and
// must never invoke a `signal <kind>` subcommand with `--summary` set and no
// `--project` override, since that combination reaches real runtime write
// behavior against whatever real ambient project happens to contain this
// process's working directory. Every parser-recognition assertion in this
// file uses `--help` (which clap short-circuits before any project
// resolution or write ever runs) precisely so this file can never write a
// real signal. Any test that needs to prove actual write behavior belongs in
// `tests/e2e.rs`/`tests/json_safety.rs` against an explicitly isolated temp
// project, never here.

use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

const KINDS: &[&str] = &[
    "progress",
    "decision",
    "blocker",
    "blocker-resolved",
    "scope-change",
    "milestone",
    "review-required",
    "unresolved-question",
    "handoff",
    "session-end",
    "agent-switch",
];

#[test]
fn all_eleven_kinds_are_recognized_by_signal_help() {
    let output = cli().args(["signal", "--help"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    for kind in KINDS {
        assert!(
            stdout.contains(kind),
            "signal --help should list `{kind}`, got:\n{stdout}"
        );
    }
}

#[test]
fn no_other_top_level_command_exists() {
    // Frozen gate: forbid legacy/planned command *tokens*, not product prose.
    // 0.1.15 adds `team` whose help text may say "workspace" as English, which
    // must not trip a naive substring check for a top-level `workspace` command.
    let output = cli().arg("--help").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lower = stdout.to_ascii_lowercase();
    let tokens: Vec<&str> = lower
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
        .filter(|t| !t.is_empty())
        .collect();
    for forbidden in ["status", "workspace", "process", "replay"] {
        // Allow descriptive prose ("team workspace foundation") but not a
        // left-column command named exactly `workspace`.
        let as_command = lower.lines().any(|line| {
            let cols: Vec<&str> = line.split_whitespace().collect();
            cols.first().copied() == Some(forbidden)
        });
        assert!(
            !as_command,
            "top-level --help must not expose command `{forbidden}`, got:\n{stdout}"
        );
        // Also block exact token only when it appears as a standalone command id
        // in the clap Commands list style (first word of a help row).
        let _ = tokens; // retained for future exact-token scans
    }
}

#[test]
fn each_kind_subcommand_has_help_and_is_recognized() {
    // Parser-only proof: `--help` short-circuits before project resolution
    // or write_signal ever run, so this can never write a real signal
    // against whatever real project this process's ambient cwd resolves to.
    for kind in KINDS {
        let output = cli().args(["signal", kind, "--help"]).output().unwrap();
        assert!(
            output.status.success(),
            "signal {kind} --help should succeed at the parser level, stderr={}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn missing_summary_is_a_usage_error_exiting_2() {
    let output = cli().args(["signal", "progress"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn top_level_help_exits_0() {
    let output = cli().arg("--help").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn signal_help_exits_0() {
    let output = cli().args(["signal", "--help"]).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn kind_help_exits_0() {
    let output = cli()
        .args(["signal", "progress", "--help"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
}
