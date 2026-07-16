//! WorkSignal composition and inbox write for Git/Heli producers (Checkpoint D).

use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::context::Sensitivity;
use crate::domain::{
    ActorRef, EvidenceRef, GitProducerError, GitProducerResult, GitSnapshot, HeliProducerResult,
    HeliSnapshot, ProducerRef, ProducerSkipReason, WorkSignal, WorkSignalKind,
    WORK_SIGNAL_PROTOCOL_VERSION, WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE,
};
use crate::signals::{write_signal, SignalError};

use super::git::read_git_snapshot;
use super::heli::read_heli_snapshot;

/// Outcome of a producer collect invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollectSignalOutcome {
    Written { signal_id: String },
    Skipped { reason: ProducerSkipReason },
}

/// Collect errors surfaced from producer readers or inbox validation.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CollectSignalError {
    #[error("git producer: {0:?}")]
    Git(GitProducerError),
    #[error("heli producer: {0}")]
    Heli(String),
    #[error("signal write: {0}")]
    Write(String),
}

/// Read Git state and write one WorkSignal to `pending/`.
pub fn collect_git_signal(
    project_path: &Path,
    workspace_id: &str,
    correlation_hint: Option<String>,
) -> Result<CollectSignalOutcome, CollectSignalError> {
    let snapshot_result = read_git_snapshot(project_path);
    let snapshot = match snapshot_result {
        GitProducerResult::Snapshot(s) => s,
        GitProducerResult::Skip(reason) => return Ok(CollectSignalOutcome::Skipped { reason }),
        GitProducerResult::Err(err) => return Err(CollectSignalError::Git(err)),
    };

    let signal = compose_git_signal(workspace_id, &snapshot, correlation_hint);
    write_signal(&project_path.to_string_lossy(), &signal)
        .map_err(|e| CollectSignalError::Write(signal_error_message(&e)))?;

    Ok(CollectSignalOutcome::Written {
        signal_id: signal.signal_id,
    })
}

/// Read Heli state and write one WorkSignal to `pending/` when content exists.
pub fn collect_heli_signal(
    project_path: &Path,
    workspace_id: &str,
    correlation_hint: Option<String>,
) -> Result<CollectSignalOutcome, CollectSignalError> {
    let snapshot_result = read_heli_snapshot(project_path);
    let snapshot = match snapshot_result {
        HeliProducerResult::Snapshot(s) => s,
        HeliProducerResult::Skip(reason) => return Ok(CollectSignalOutcome::Skipped { reason }),
        HeliProducerResult::Err(err) => {
            return Err(CollectSignalError::Heli(format!("{err:?}")));
        }
    };

    if !heli_snapshot_has_emit_content(&snapshot) {
        return Ok(CollectSignalOutcome::Skipped {
            reason: ProducerSkipReason::HeliAbsent,
        });
    }

    let signal = compose_heli_signal(workspace_id, &snapshot, correlation_hint);
    write_signal(&project_path.to_string_lossy(), &signal)
        .map_err(|e| CollectSignalError::Write(signal_error_message(&e)))?;

    Ok(CollectSignalOutcome::Written {
        signal_id: signal.signal_id,
    })
}

pub fn compose_git_signal(
    workspace_id: &str,
    snapshot: &GitSnapshot,
    correlation_hint: Option<String>,
) -> WorkSignal {
    let kind = map_git_snapshot_to_kind(snapshot);
    WorkSignal {
        signal_id: generate_signal_id("git"),
        workspace_id: workspace_id.to_string(),
        producer: ProducerRef::Git,
        actor: ActorRef::Unknown,
        kind,
        summary: git_summary(snapshot),
        timestamp: snapshot.observed_at.clone(),
        evidence_refs: vec![EvidenceRef::GitState(snapshot.clone())],
        correlation_hint,
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION_WITH_GIT_EVIDENCE.to_string(),
    }
}

pub fn compose_heli_signal(
    workspace_id: &str,
    snapshot: &HeliSnapshot,
    correlation_hint: Option<String>,
) -> WorkSignal {
    let kind = map_heli_snapshot_to_kind(snapshot);
    WorkSignal {
        signal_id: generate_signal_id("heli"),
        workspace_id: workspace_id.to_string(),
        producer: ProducerRef::Heli,
        actor: ActorRef::Unknown,
        kind,
        summary: heli_summary(snapshot),
        timestamp: snapshot.observed_at.clone(),
        evidence_refs: heli_evidence_refs(snapshot),
        correlation_hint,
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
    }
}

fn heli_snapshot_has_emit_content(snapshot: &HeliSnapshot) -> bool {
    snapshot.current_task_excerpt.is_some()
        || snapshot.decisions_tail_excerpt.is_some()
        || snapshot.latest_report_path.is_some()
}

fn heli_evidence_refs(snapshot: &HeliSnapshot) -> Vec<EvidenceRef> {
    let mut refs = Vec::new();
    if snapshot.current_task_excerpt.is_some() {
        refs.push(EvidenceRef::FilePath(
            ".heli-harness/state/current-task.md".into(),
        ));
    }
    if snapshot.decisions_tail_excerpt.is_some() {
        refs.push(EvidenceRef::FilePath(
            ".heli-harness/state/decisions.md".into(),
        ));
    }
    if let Some(report) = &snapshot.latest_report_path {
        refs.push(EvidenceRef::FilePath(report.clone()));
    }
    refs
}

/// Frozen kind mapping (execution plan §3.8).
pub fn map_git_snapshot_to_kind(snapshot: &GitSnapshot) -> WorkSignalKind {
    if snapshot.dirty || !snapshot.changed_paths.is_empty() {
        return WorkSignalKind::Progress;
    }
    if snapshot.ahead.unwrap_or(0) > 0 {
        return WorkSignalKind::Handoff;
    }
    if !snapshot.dirty && snapshot.ahead.unwrap_or(0) == 0 {
        return WorkSignalKind::Milestone;
    }
    WorkSignalKind::Progress
}

/// Frozen kind mapping (execution plan §3.8).
pub fn map_heli_snapshot_to_kind(snapshot: &HeliSnapshot) -> WorkSignalKind {
    let task = snapshot.current_task_excerpt.as_deref().unwrap_or("");
    let task_upper = task.to_ascii_uppercase();
    if contains_whole_word(&task_upper, "BLOCKED") {
        return WorkSignalKind::Blocker;
    }
    if task_upper.contains("CLOSED") || task_upper.contains("PUBLISHED — CLOSED") {
        return WorkSignalKind::Milestone;
    }
    let decisions = snapshot.decisions_tail_excerpt.as_deref().unwrap_or("");
    if decisions.contains("## 20") && decisions.to_ascii_uppercase().contains("PASS") {
        return WorkSignalKind::Decision;
    }
    if !task.trim().is_empty() {
        return WorkSignalKind::Progress;
    }
    WorkSignalKind::UnresolvedQuestion
}

fn git_summary(snapshot: &GitSnapshot) -> String {
    let head_short = snapshot.head.chars().take(7).collect::<String>();
    format!(
        "Git: branch={} head={head_short} dirty={} changed={}",
        snapshot.branch,
        snapshot.dirty,
        snapshot.changed_paths.len()
    )
}

fn heli_summary(snapshot: &HeliSnapshot) -> String {
    let first_line = snapshot
        .current_task_excerpt
        .as_deref()
        .and_then(|t| t.lines().find(|l| !l.trim().is_empty()))
        .unwrap_or("absent");
    let decision_count = snapshot
        .decisions_tail_excerpt
        .as_deref()
        .map(|d| d.matches("## ").count())
        .unwrap_or(0);
    format!("Heli: task={first_line} decisions={decision_count}")
}

fn contains_whole_word(haystack: &str, needle: &str) -> bool {
    haystack
        .split(|c: char| !c.is_ascii_alphanumeric())
        .any(|word| word == needle)
}

fn generate_signal_id(prefix: &str) -> String {
    let date = chrono::Utc::now().format("%Y%m%d");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let pid = process::id();
    format!("{prefix}-{date}-{nanos:x}-{pid:x}")
}

fn signal_error_message(err: &SignalError) -> String {
    err.to_string()
}
