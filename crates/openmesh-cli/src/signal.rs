// ============================================================================
// Signal construction — Checkpoint B (approved plan §7/§8).
// ============================================================================
// Builds one in-memory `WorkSignal` from parsed CLI arguments plus the
// resolved project. Never calls `write_signal` here (Checkpoint C).
// ============================================================================

use crate::project::ResolvedProject;
use crate::{ActorArg, SensitivityArg, SignalArgs, SignalKindCommand};
use openmesh_core::context::Sensitivity;
use openmesh_core::domain::{
    ActorRef, EvidenceRef, ProducerRef, WorkSignal, WorkSignalKind, WORK_SIGNAL_PROTOCOL_VERSION,
};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn kind_of(cmd: &SignalKindCommand) -> WorkSignalKind {
    match cmd {
        SignalKindCommand::Progress(_) => WorkSignalKind::Progress,
        SignalKindCommand::Decision(_) => WorkSignalKind::Decision,
        SignalKindCommand::Blocker(_) => WorkSignalKind::Blocker,
        SignalKindCommand::BlockerResolved(_) => WorkSignalKind::BlockerResolved,
        SignalKindCommand::ScopeChange(_) => WorkSignalKind::ScopeChange,
        SignalKindCommand::Milestone(_) => WorkSignalKind::Milestone,
        SignalKindCommand::ReviewRequired(_) => WorkSignalKind::ReviewRequired,
        SignalKindCommand::UnresolvedQuestion(_) => WorkSignalKind::UnresolvedQuestion,
        SignalKindCommand::Handoff(_) => WorkSignalKind::Handoff,
        SignalKindCommand::SessionEnd(_) => WorkSignalKind::SessionEnd,
        SignalKindCommand::AgentSwitch(_) => WorkSignalKind::AgentSwitch,
    }
}

/// Frozen format (approved plan §7): `sig-<YYYYMMDD>-<unix_nanos_hex>-<pid_hex>`.
/// Producer-neutral, printable ASCII, no randomness/UUID dependency, no
/// cryptographic-uniqueness claim — collision handling is 0.1.3.2's existing
/// duplicate-identity semantics, not this function's job.
pub fn generate_signal_id() -> String {
    let date = chrono::Utc::now().format("%Y%m%d");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is before UNIX_EPOCH")
        .as_nanos();
    let pid = process::id();
    format!("sig-{date}-{nanos:x}-{pid:x}")
}

fn actor_ref(actor: &ActorArg) -> ActorRef {
    match actor {
        ActorArg::Person(name) => ActorRef::Person(name.clone()),
        ActorArg::Device(name) => ActorRef::Device(name.clone()),
        ActorArg::Proxy(name) => ActorRef::Proxy(name.clone()),
        ActorArg::Unknown => ActorRef::Unknown,
    }
}

fn sensitivity_value(sensitivity: &SensitivityArg) -> Sensitivity {
    match sensitivity {
        SensitivityArg::Public => Sensitivity::Public,
        SensitivityArg::Team => Sensitivity::Team,
        SensitivityArg::Private => Sensitivity::Private,
        SensitivityArg::Secret => Sensitivity::Secret,
    }
}

/// Constructs one `WorkSignal` from parsed arguments and the resolved
/// project. `workspace_id` is always derived from the resolved project's own
/// `id` — the CLI never accepts a caller-supplied `--workspace-id`, so a
/// workspace mismatch cannot be constructed through normal CLI use.
pub fn build_work_signal(cmd: &SignalKindCommand, resolved: &ResolvedProject) -> WorkSignal {
    let args: &SignalArgs = cmd.args();

    let producer_name = args.producer.clone().unwrap_or_else(|| "cli".to_string());

    let evidence_refs = args
        .evidence
        .iter()
        .map(|path| EvidenceRef::FilePath(path.clone()))
        .chain(
            args.evidence_signal
                .iter()
                .map(|signal_id| EvidenceRef::ProducerSignal(signal_id.clone())),
        )
        .collect();

    WorkSignal {
        signal_id: args.signal_id.clone().unwrap_or_else(generate_signal_id),
        workspace_id: resolved.project.id.clone(),
        producer: ProducerRef::Reporter(producer_name),
        actor: actor_ref(&args.actor),
        kind: kind_of(cmd),
        summary: args.summary.clone(),
        timestamp: args
            .timestamp
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339()),
        evidence_refs,
        correlation_hint: args.correlation_hint.clone(),
        sensitivity: sensitivity_value(&args.sensitivity),
        protocol_version: WORK_SIGNAL_PROTOCOL_VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::ResolvedProject;
    use openmesh_core::storage::Project;
    use std::path::PathBuf;

    fn resolved_project(id: &str) -> ResolvedProject {
        ResolvedProject {
            path: PathBuf::from("/fake/project"),
            project: Project {
                id: id.to_string(),
                name: "fake".to_string(),
                folder_path: "/fake/project".to_string(),
                repo_url: None,
                default_branch: "main".to_string(),
                sprint_source: "none".to_string(),
                docs_folder: None,
                terminal_dir: None,
                default_agent_cli: None,
                notes: None,
                status: "active".to_string(),
                created_at: "2026-07-09T00:00:00Z".to_string(),
                updated_at: "2026-07-09T00:00:00Z".to_string(),
            },
        }
    }

    fn minimal_args(overrides: impl FnOnce(&mut SignalArgs)) -> SignalArgs {
        let mut args = SignalArgs {
            summary: "a summary".to_string(),
            project: None,
            producer: None,
            actor: ActorArg::Unknown,
            evidence: Vec::new(),
            evidence_signal: Vec::new(),
            correlation_hint: None,
            sensitivity: SensitivityArg::Private,
            signal_id: None,
            timestamp: None,
            json: false,
        };
        overrides(&mut args);
        args
    }

    #[test]
    fn defaults_are_applied_correctly() {
        let resolved = resolved_project("ws-1");
        let cmd = SignalKindCommand::Progress(minimal_args(|_| {}));
        let signal = build_work_signal(&cmd, &resolved);

        assert_eq!(signal.workspace_id, "ws-1");
        assert_eq!(signal.producer, ProducerRef::Reporter("cli".to_string()));
        assert_eq!(signal.actor, ActorRef::Unknown);
        assert_eq!(signal.sensitivity, Sensitivity::Private);
        assert_eq!(signal.protocol_version, WORK_SIGNAL_PROTOCOL_VERSION);
        assert_eq!(signal.kind, WorkSignalKind::Progress);
        assert!(signal.signal_id.starts_with("sig-"));
        // Default timestamp must itself be valid RFC 3339 UTC.
        let parsed = chrono::DateTime::parse_from_rfc3339(&signal.timestamp)
            .expect("default timestamp must be valid RFC 3339");
        assert_eq!(parsed.offset().local_minus_utc(), 0);
    }

    #[test]
    fn overrides_are_respected() {
        let resolved = resolved_project("ws-2");
        let cmd = SignalKindCommand::Blocker(minimal_args(|a| {
            a.producer = Some("codex".to_string());
            a.actor = ActorArg::Person("ter".to_string());
            a.sensitivity = SensitivityArg::Secret;
            a.signal_id = Some("sig-override".to_string());
            a.timestamp = Some("2026-07-09T09:15:00Z".to_string());
            a.correlation_hint = Some("dogfood-0.1.3.3-abc".to_string());
        }));
        let signal = build_work_signal(&cmd, &resolved);

        assert_eq!(signal.producer, ProducerRef::Reporter("codex".to_string()));
        assert_eq!(signal.actor, ActorRef::Person("ter".to_string()));
        assert_eq!(signal.sensitivity, Sensitivity::Secret);
        assert_eq!(signal.signal_id, "sig-override");
        assert_eq!(signal.timestamp, "2026-07-09T09:15:00Z");
        assert_eq!(
            signal.correlation_hint,
            Some("dogfood-0.1.3.3-abc".to_string())
        );
        // workspace_id always derives from the resolved project, never from
        // any CLI-supplied override — there is no --workspace-id flag at all.
        assert_eq!(signal.workspace_id, "ws-2");
    }

    #[test]
    fn generated_signal_id_matches_the_frozen_format() {
        let id = generate_signal_id();
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(
            parts.len(),
            4,
            "expected sig-<date>-<nanos>-<pid>, got {id}"
        );
        assert_eq!(parts[0], "sig");
        assert_eq!(parts[1].len(), 8, "date component must be YYYYMMDD");
        assert!(parts[1].chars().all(|c| c.is_ascii_digit()));
        assert!(parts[2].chars().all(|c| c.is_ascii_hexdigit()));
        assert!(parts[3].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn generated_signal_id_is_producer_neutral() {
        // generate_signal_id() takes no producer argument at all — its
        // output cannot vary with --producer by construction.
        let a = generate_signal_id();
        let b = generate_signal_id();
        assert!(a.starts_with("sig-"));
        assert!(b.starts_with("sig-"));
    }

    #[test]
    fn workspace_id_always_derives_from_the_resolved_project_across_every_kind() {
        for (cmd_ctor, expected_kind) in [
            (
                (|a| SignalKindCommand::Progress(a)) as fn(SignalArgs) -> SignalKindCommand,
                WorkSignalKind::Progress,
            ),
            (
                (|a| SignalKindCommand::Handoff(a)) as fn(SignalArgs) -> SignalKindCommand,
                WorkSignalKind::Handoff,
            ),
            (
                (|a| SignalKindCommand::SessionEnd(a)) as fn(SignalArgs) -> SignalKindCommand,
                WorkSignalKind::SessionEnd,
            ),
        ] {
            let resolved = resolved_project("ws-fixed");
            let cmd = cmd_ctor(minimal_args(|_| {}));
            let signal = build_work_signal(&cmd, &resolved);
            assert_eq!(signal.workspace_id, "ws-fixed");
            assert_eq!(signal.kind, expected_kind);
        }
    }
}
