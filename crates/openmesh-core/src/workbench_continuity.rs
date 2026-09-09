//! Workbench Continuity Bridge (v0.2 A6).
//!
//! Maps structured Agent Workbench boundaries onto existing `WorkSignal`
//! records. Does not append WorkEvents, apply promotion, or change domain
//! kinds. Remote Agent Engine origins cannot write.

use crate::agent_engine::policy::AgentOrigin;
use crate::context::Sensitivity;
use crate::continuity::{
    list_duplicate_signals, list_pending_signals, list_processed_signals, list_quarantine_signals,
    rebuild_current_state_projection,
};
use crate::domain::{
    ActorRef, EvidenceRef, ProducerRef, WorkSignal, WorkSignalKind, WORK_SIGNAL_PROTOCOL_VERSION,
};
use crate::session_readers::redact_secrets;
use crate::signals::{write_signal, SignalError, MAX_SIGNAL_ID_BYTES, MAX_SUMMARY_BYTES};
use crate::storage::{read_project, Project};

const PRODUCER_PROXY_ACTOR: &str = "agent-engine";
const HOST_DEVICE_ACTOR: &str = "openmesh-desktop";
const FALLBACK_SUMMARY: &str = "Workbench work boundary.";

/// Who is asking the bridge to record. Distinct from Continuity `ActorRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundarySource {
    /// Agent Engine origin (Chat, delegate, CLI, LAN, Continuity query).
    Origin(AgentOrigin),
    /// Host IPC (patch apply/reject, recipe run) authorized outside the engine.
    HostAuthorized,
}

impl BoundarySource {
    pub fn local_chat() -> Self {
        Self::Origin(AgentOrigin::LocalChat)
    }

    pub fn local_delegate() -> Self {
        Self::Origin(AgentOrigin::LocalDelegate)
    }

    fn skip_reason(self) -> Option<SkipReason> {
        match self {
            Self::HostAuthorized => None,
            Self::Origin(origin) if origin.is_remote() => Some(SkipReason::RemoteOrigin),
            Self::Origin(AgentOrigin::LocalCLI) => Some(SkipReason::LocalCliOutOfScope),
            Self::Origin(AgentOrigin::LocalChat | AgentOrigin::LocalDelegate) => None,
            Self::Origin(AgentOrigin::LanPeer | AgentOrigin::ContinuityQuery) => {
                Some(SkipReason::RemoteOrigin)
            }
        }
    }
}

/// Why a boundary was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    RemoteOrigin,
    LocalCliOutOfScope,
}

/// Structured workbench boundary — application facts, not model prose.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkBoundary {
    PatchProposed {
        patch_id: String,
        summary: String,
        created_at: String,
        file_count: usize,
    },
    PatchApplied {
        patch_id: String,
        summary: String,
        applied_at: String,
        file_count: usize,
    },
    PatchRejected {
        patch_id: String,
        summary: String,
        rejected_at: String,
    },
    VerifyCompleted {
        run_id: String,
        recipe_id: String,
        ok: bool,
        exit_code: Option<i32>,
        timed_out: bool,
        cancelled: bool,
        created_at: String,
        patch_id: Option<String>,
    },
    DelegateBrief {
        brief_id: String,
        tool: String,
        created_at: String,
    },
    HandoffDraft {
        handoff_id: String,
        created_at: String,
    },
    SessionImported {
        source: String,
        source_id: String,
        chat_session_id: String,
        created_at: String,
    },
}

/// Result of attempting to persist a boundary as a WorkSignal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordOutcome {
    Written { signal_id: String },
    AlreadyRecorded { signal_id: String },
    Skipped { reason: SkipReason },
    IdentityConflict { signal_id: String },
}

#[derive(Debug, thiserror::Error)]
pub enum WorkbenchContinuityError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("signal write: {0}")]
    Signal(#[from] SignalError),
    #[error("invalid workbench boundary: {0}")]
    InvalidBoundary(String),
}

/// Record one workbench boundary into the existing signal inbox.
///
/// Never calls `append_event` or `apply_promotion_decision`.
pub fn record_boundary(
    project_path: &str,
    source: BoundarySource,
    boundary: &WorkBoundary,
) -> Result<RecordOutcome, WorkbenchContinuityError> {
    if let Some(reason) = source.skip_reason() {
        return Ok(RecordOutcome::Skipped { reason });
    }

    let project: Project = read_project(project_path, "project.json")
        .ok_or_else(|| WorkbenchContinuityError::ProjectNotInitialized(project_path.to_string()))?;

    let signal = compose_work_signal(&project.id, boundary)?;
    if let Some(existing) = find_existing_signal(project_path, &signal.signal_id) {
        let incoming = canonical_payload(&signal)?;
        let stored = canonical_payload(&existing)?;
        if incoming == stored {
            return Ok(RecordOutcome::AlreadyRecorded {
                signal_id: signal.signal_id,
            });
        }
        return Ok(RecordOutcome::IdentityConflict {
            signal_id: signal.signal_id,
        });
    }

    write_signal(project_path, &signal)?;
    let _ = rebuild_current_state_projection(project_path);
    Ok(RecordOutcome::Written {
        signal_id: signal.signal_id,
    })
}

/// Compose the WorkSignal the inbox would receive (no I/O).
pub fn compose_work_signal(
    workspace_id: &str,
    boundary: &WorkBoundary,
) -> Result<WorkSignal, WorkbenchContinuityError> {
    let mapping = map_boundary(boundary)?;
    Ok(WorkSignal {
        signal_id: mapping.signal_id,
        workspace_id: workspace_id.to_string(),
        producer: ProducerRef::Native,
        actor: mapping.actor,
        kind: mapping.kind,
        summary: mapping.summary,
        timestamp: mapping.timestamp,
        evidence_refs: mapping.evidence_refs,
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
    })
}

pub fn signal_id_for(boundary: &WorkBoundary) -> Result<String, WorkbenchContinuityError> {
    Ok(map_boundary(boundary)?.signal_id)
}

struct MappedBoundary {
    signal_id: String,
    kind: WorkSignalKind,
    actor: ActorRef,
    summary: String,
    timestamp: String,
    evidence_refs: Vec<EvidenceRef>,
}

fn map_boundary(boundary: &WorkBoundary) -> Result<MappedBoundary, WorkbenchContinuityError> {
    match boundary {
        WorkBoundary::PatchProposed {
            patch_id,
            summary,
            created_at,
            file_count,
        } => {
            let id = require_component("patch_id", patch_id)?;
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "patch", "proposed", &id]),
                kind: WorkSignalKind::ReviewRequired,
                actor: ActorRef::Proxy(PRODUCER_PROXY_ACTOR.into()),
                summary: bound_summary(format!(
                    "Agent proposed patch {id} ({file_count} file{}): {summary}",
                    if *file_count == 1 { "" } else { "s" }
                )),
                timestamp: created_at.clone(),
                evidence_refs: vec![EvidenceRef::FilePath(patch_evidence_path(&id))],
            })
        }
        WorkBoundary::PatchApplied {
            patch_id,
            summary,
            applied_at,
            file_count,
        } => {
            let id = require_component("patch_id", patch_id)?;
            let proposal_id = stable_id(&["wb", "patch", "proposed", &id]);
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "patch", "applied", &id]),
                kind: WorkSignalKind::Progress,
                actor: ActorRef::Device(HOST_DEVICE_ACTOR.into()),
                summary: bound_summary(format!(
                    "Host applied patch {id} ({file_count} file{}): {summary}",
                    if *file_count == 1 { "" } else { "s" }
                )),
                timestamp: applied_at.clone(),
                evidence_refs: vec![
                    EvidenceRef::FilePath(patch_evidence_path(&id)),
                    EvidenceRef::ProducerSignal(proposal_id),
                ],
            })
        }
        WorkBoundary::PatchRejected {
            patch_id,
            summary,
            rejected_at,
        } => {
            let id = require_component("patch_id", patch_id)?;
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "patch", "rejected", &id]),
                kind: WorkSignalKind::Decision,
                actor: ActorRef::Device(HOST_DEVICE_ACTOR.into()),
                summary: bound_summary(format!("Host rejected patch {id}: {summary}")),
                timestamp: rejected_at.clone(),
                evidence_refs: vec![EvidenceRef::FilePath(patch_evidence_path(&id))],
            })
        }
        WorkBoundary::VerifyCompleted {
            run_id,
            recipe_id,
            ok,
            exit_code,
            timed_out,
            cancelled,
            created_at,
            patch_id,
        } => {
            let run = require_component("run_id", run_id)?;
            let recipe = sanitize_component(recipe_id);
            let (kind, outcome) = if *ok && !*timed_out && !*cancelled {
                (WorkSignalKind::Milestone, "succeeded")
            } else if *cancelled {
                (WorkSignalKind::Blocker, "cancelled")
            } else if *timed_out {
                (WorkSignalKind::Blocker, "timed out")
            } else {
                (WorkSignalKind::Blocker, "failed")
            };
            let exit = exit_code.map(|c| format!(" exit={c}")).unwrap_or_default();
            let patch_note = patch_id
                .as_deref()
                .filter(|p| !p.trim().is_empty())
                .map(|p| format!(" patch={}", sanitize_component(p)))
                .unwrap_or_default();
            let mut evidence = vec![EvidenceRef::FilePath(run_evidence_path(&run))];
            if let Some(pid) = patch_id
                .as_deref()
                .filter(|p| !p.trim().is_empty())
                .map(sanitize_component)
            {
                evidence.push(EvidenceRef::FilePath(patch_evidence_path(&pid)));
                evidence.push(EvidenceRef::ProducerSignal(stable_id(&[
                    "wb", "patch", "applied", &pid,
                ])));
            }
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "verify", &run]),
                kind,
                actor: ActorRef::Device(HOST_DEVICE_ACTOR.into()),
                summary: bound_summary(format!(
                    "Verification recipe {recipe} {outcome}{exit}{patch_note}."
                )),
                timestamp: created_at.clone(),
                evidence_refs: evidence,
            })
        }
        WorkBoundary::DelegateBrief {
            brief_id,
            tool,
            created_at,
        } => {
            let id = require_component("brief_id", brief_id)?;
            let tool = sanitize_component(tool);
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "delegate", &id]),
                kind: WorkSignalKind::Progress,
                actor: ActorRef::Proxy(PRODUCER_PROXY_ACTOR.into()),
                summary: bound_summary(format!("Delegate brief {id} written for {tool}.")),
                timestamp: created_at.clone(),
                evidence_refs: vec![EvidenceRef::FilePath(brief_evidence_path(&id))],
            })
        }
        WorkBoundary::HandoffDraft {
            handoff_id,
            created_at,
        } => {
            let id = require_component("handoff_id", handoff_id)?;
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "handoff", &id]),
                kind: WorkSignalKind::Handoff,
                actor: ActorRef::Device(HOST_DEVICE_ACTOR.into()),
                summary: bound_summary(format!("Handoff draft {id} created from workbench.")),
                timestamp: created_at.clone(),
                evidence_refs: vec![EvidenceRef::FilePath(handoff_evidence_path(&id))],
            })
        }
        WorkBoundary::SessionImported {
            source,
            source_id,
            chat_session_id,
            created_at,
        } => {
            let source = sanitize_component(source);
            let source_id = require_component("source_id", source_id)?;
            let chat = sanitize_component(chat_session_id);
            Ok(MappedBoundary {
                signal_id: stable_id(&["wb", "import", &source, &source_id]),
                kind: WorkSignalKind::AgentSwitch,
                actor: ActorRef::Device(HOST_DEVICE_ACTOR.into()),
                summary: bound_summary(format!(
                    "Imported {source} session {source_id} into OpenMesh chat {chat}."
                )),
                timestamp: created_at.clone(),
                evidence_refs: vec![EvidenceRef::FilePath(
                    ".openmesh/agent/session-links.json".into(),
                )],
            })
        }
    }
}

fn require_component(field: &str, value: &str) -> Result<String, WorkbenchContinuityError> {
    let sanitized = sanitize_component(value);
    if sanitized == "unknown" {
        return Err(WorkbenchContinuityError::InvalidBoundary(format!(
            "{field} is empty after sanitization"
        )));
    }
    Ok(sanitized)
}

fn sanitize_component(raw: &str) -> String {
    let mut out: String = raw
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .take(80)
        .collect();
    if out.is_empty() {
        out = "unknown".into();
    }
    out
}

fn stable_id(parts: &[&str]) -> String {
    let mut id = parts.join("-");
    if id.len() > MAX_SIGNAL_ID_BYTES {
        id.truncate(MAX_SIGNAL_ID_BYTES);
    }
    id
}

fn bound_summary(raw: String) -> String {
    let redacted = redact_secrets(&raw);
    let trimmed = redacted.trim();
    if trimmed.is_empty() {
        return FALLBACK_SUMMARY.to_string();
    }
    if trimmed.len() > MAX_SUMMARY_BYTES {
        let mut cut = MAX_SUMMARY_BYTES.saturating_sub(1);
        while cut > 0 && !trimmed.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}…", &trimmed[..cut])
    } else {
        trimmed.to_string()
    }
}

fn patch_evidence_path(patch_id: &str) -> String {
    format!(".openmesh/agent/patches/{patch_id}.json")
}

fn run_evidence_path(run_id: &str) -> String {
    format!(".openmesh/agent/runs/{run_id}.json")
}

fn brief_evidence_path(brief_id: &str) -> String {
    format!(".openmesh/agent/briefs/{brief_id}.md")
}

fn handoff_evidence_path(handoff_id: &str) -> String {
    format!(".openmesh/handoff/{handoff_id}.json")
}

fn canonical_payload(signal: &WorkSignal) -> Result<String, WorkbenchContinuityError> {
    serde_json::to_string_pretty(signal)
        .map_err(|e| WorkbenchContinuityError::Signal(SignalError::Json(e)))
}

fn find_existing_signal(project_path: &str, signal_id: &str) -> Option<WorkSignal> {
    let loaders = [
        list_pending_signals,
        list_processed_signals,
        list_duplicate_signals,
        list_quarantine_signals,
    ];
    for loader in loaders {
        if let Ok(bucket) = loader(project_path) {
            if let Some(found) = bucket
                .signals
                .into_iter()
                .find(|s| s.signal_id == signal_id)
            {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_and_apply_use_distinct_ids_and_kinds() {
        let proposed = WorkBoundary::PatchProposed {
            patch_id: "patch-abc".into(),
            summary: "fix login".into(),
            created_at: "2026-09-09T00:00:00Z".into(),
            file_count: 1,
        };
        let applied = WorkBoundary::PatchApplied {
            patch_id: "patch-abc".into(),
            summary: "fix login".into(),
            applied_at: "2026-09-09T00:01:00Z".into(),
            file_count: 1,
        };
        let a = compose_work_signal("ws", &proposed).unwrap();
        let b = compose_work_signal("ws", &applied).unwrap();
        assert_eq!(a.kind, WorkSignalKind::ReviewRequired);
        assert_eq!(b.kind, WorkSignalKind::Progress);
        assert_ne!(a.signal_id, b.signal_id);
        assert!(a.signal_id.contains("proposed"));
        assert!(b.signal_id.contains("applied"));
        assert!(matches!(a.actor, ActorRef::Proxy(_)));
        assert!(matches!(b.actor, ActorRef::Device(_)));
        assert_eq!(a.producer, ProducerRef::Native);
        assert!(a.correlation_hint.is_none());
        assert!(b.correlation_hint.is_none());
    }

    #[test]
    fn failed_verify_is_blocker_not_milestone() {
        let failed = WorkBoundary::VerifyCompleted {
            run_id: "run-1".into(),
            recipe_id: "npm-test".into(),
            ok: false,
            exit_code: Some(1),
            timed_out: false,
            cancelled: false,
            created_at: "2026-09-09T00:00:00Z".into(),
            patch_id: None,
        };
        let ok = WorkBoundary::VerifyCompleted {
            run_id: "run-2".into(),
            recipe_id: "npm-test".into(),
            ok: true,
            exit_code: Some(0),
            timed_out: false,
            cancelled: false,
            created_at: "2026-09-09T00:00:00Z".into(),
            patch_id: None,
        };
        assert_eq!(
            compose_work_signal("ws", &failed).unwrap().kind,
            WorkSignalKind::Blocker
        );
        assert_eq!(
            compose_work_signal("ws", &ok).unwrap().kind,
            WorkSignalKind::Milestone
        );
    }

    #[test]
    fn import_is_agent_switch_and_names_source() {
        let boundary = WorkBoundary::SessionImported {
            source: "cursor".into(),
            source_id: "sess-99".into(),
            chat_session_id: "chat-1".into(),
            created_at: "2026-09-09T00:00:00Z".into(),
        };
        let signal = compose_work_signal("ws", &boundary).unwrap();
        assert_eq!(signal.kind, WorkSignalKind::AgentSwitch);
        assert!(signal.summary.contains("cursor"));
        assert!(signal.summary.contains("sess-99"));
        assert!(!signal.summary.contains("sessions.json"));
    }

    #[test]
    fn summaries_redact_provider_tokens() {
        let boundary = WorkBoundary::PatchProposed {
            patch_id: "patch-x".into(),
            summary: "key sk-ant-abcdefghijklmnop leaked".into(),
            created_at: "2026-09-09T00:00:00Z".into(),
            file_count: 1,
        };
        let signal = compose_work_signal("ws", &boundary).unwrap();
        assert!(
            !signal.summary.contains("sk-ant-abcdefghijklmnop"),
            "summary={}",
            signal.summary
        );
        assert!(signal.summary.contains("[REDACTED]"));
    }

    #[test]
    fn remote_and_cli_sources_skip_without_compose_error() {
        assert_eq!(
            BoundarySource::Origin(AgentOrigin::LanPeer).skip_reason(),
            Some(SkipReason::RemoteOrigin)
        );
        assert_eq!(
            BoundarySource::Origin(AgentOrigin::ContinuityQuery).skip_reason(),
            Some(SkipReason::RemoteOrigin)
        );
        assert_eq!(
            BoundarySource::Origin(AgentOrigin::LocalCLI).skip_reason(),
            Some(SkipReason::LocalCliOutOfScope)
        );
        assert_eq!(BoundarySource::local_chat().skip_reason(), None);
        assert_eq!(BoundarySource::HostAuthorized.skip_reason(), None);
    }
}
