//! v0.2 A6 — Workbench Continuity Bridge: provenance, idempotency, origin gate,
//! and existing promotion rules (no WorkEvent bypass).

use openmesh_core::agent_engine::{
    apply_patch, create_handoff_draft, reject_patch, run_recipe, write_delegate_brief, AgentOrigin,
    Recipe, ToolExecutor, WorkspaceToolExecutor,
};
use openmesh_core::continuity::list_pending_signals;
use openmesh_core::domain::{ActorRef, ProducerRef, WorkSignal, WorkSignalKind};
use openmesh_core::events::list_events;
use openmesh_core::promotion::{
    evaluate_promotion_case, PromotionCase, PromotionOutcome, SignalRef,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::{atomic_write, get_project_dir};
use openmesh_core::storage::{init_project, read_project, Project};
use openmesh_core::workbench_continuity::{
    compose_work_signal, record_boundary, BoundarySource, RecordOutcome, SkipReason, WorkBoundary,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project() -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("openmesh-wb-cont-{}-{}", std::process::id(), n));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.to_string_lossy().to_string();
    init_project(&path).unwrap();
    path
}

fn cleanup(path: &str) {
    let _ = fs::remove_dir_all(path);
}

fn project_id(path: &str) -> String {
    read_project::<Project>(path, "project.json")
        .expect("project.json")
        .id
}

fn pending_signals(path: &str) -> Vec<WorkSignal> {
    list_pending_signals(path).unwrap().signals
}

fn to_ref(signal: &WorkSignal) -> SignalRef {
    SignalRef {
        signal_id: signal.signal_id.clone(),
        kind: signal.kind,
        summary: signal.summary.clone(),
        producer: signal.producer.clone(),
        actor: signal.actor.clone(),
        timestamp: signal.timestamp.clone(),
        correlation_hint: signal.correlation_hint.clone(),
        evidence_refs: signal.evidence_refs.clone(),
    }
}

fn proposed(patch_id: &str) -> WorkBoundary {
    WorkBoundary::PatchProposed {
        patch_id: patch_id.into(),
        summary: "replace login handler".into(),
        created_at: "2026-09-09T12:00:00Z".into(),
        file_count: 2,
    }
}

fn applied(patch_id: &str) -> WorkBoundary {
    WorkBoundary::PatchApplied {
        patch_id: patch_id.into(),
        summary: "replace login handler".into(),
        applied_at: "2026-09-09T12:05:00Z".into(),
        file_count: 2,
    }
}

#[test]
fn local_chat_proposal_writes_review_required_signal() {
    let project = temp_project();
    let outcome = record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-1"))
        .expect("record");
    match outcome {
        RecordOutcome::Written { signal_id } => {
            assert_eq!(signal_id, "wb-patch-proposed-patch-1");
        }
        other => panic!("expected Written, got {other:?}"),
    }
    let signals = pending_signals(&project);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, WorkSignalKind::ReviewRequired);
    assert_eq!(signals[0].producer, ProducerRef::Native);
    assert!(matches!(signals[0].actor, ActorRef::Proxy(_)));
    assert_eq!(signals[0].workspace_id, project_id(&project));
    assert!(signals[0].evidence_refs.iter().any(
        |e| matches!(e, openmesh_core::domain::EvidenceRef::FilePath(p) if p.contains("patch-1"))
    ));
    cleanup(&project);
}

#[test]
fn recording_a_signal_does_not_create_a_work_event() {
    let project = temp_project();
    record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-2")).unwrap();
    let events = list_events(&project).unwrap_or_default();
    assert!(
        events.is_empty(),
        "bridge must not append WorkEvents; got {events:?}"
    );
    cleanup(&project);
}

#[test]
fn duplicate_observation_does_not_duplicate_the_signal() {
    let project = temp_project();
    let first = record_boundary(
        &project,
        BoundarySource::local_chat(),
        &proposed("patch-dup"),
    )
    .unwrap();
    let second = record_boundary(
        &project,
        BoundarySource::local_chat(),
        &proposed("patch-dup"),
    )
    .unwrap();
    assert!(matches!(first, RecordOutcome::Written { .. }));
    assert!(matches!(second, RecordOutcome::AlreadyRecorded { .. }));
    assert_eq!(pending_signals(&project).len(), 1);
    cleanup(&project);
}

#[test]
fn distinct_operations_remain_distinct() {
    let project = temp_project();
    record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-a")).unwrap();
    record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-b")).unwrap();
    let ids: Vec<_> = pending_signals(&project)
        .into_iter()
        .map(|s| s.signal_id)
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"wb-patch-proposed-patch-a".to_string()));
    assert!(ids.contains(&"wb-patch-proposed-patch-b".to_string()));
    cleanup(&project);
}

#[test]
fn proposal_and_apply_are_different_boundaries() {
    let project = temp_project();
    record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-x")).unwrap();
    record_boundary(
        &project,
        BoundarySource::HostAuthorized,
        &applied("patch-x"),
    )
    .unwrap();
    let signals = pending_signals(&project);
    assert_eq!(signals.len(), 2);
    let proposed_s = signals
        .iter()
        .find(|s| s.signal_id.contains("proposed"))
        .unwrap();
    let applied_s = signals
        .iter()
        .find(|s| s.signal_id.contains("applied"))
        .unwrap();
    assert_eq!(proposed_s.kind, WorkSignalKind::ReviewRequired);
    assert_eq!(applied_s.kind, WorkSignalKind::Progress);
    assert_ne!(proposed_s.signal_id, applied_s.signal_id);
    cleanup(&project);
}

#[test]
fn unapplied_patch_cannot_appear_as_applied() {
    let project = temp_project();
    record_boundary(&project, BoundarySource::local_chat(), &proposed("patch-u")).unwrap();
    let signals = pending_signals(&project);
    assert_eq!(signals.len(), 1);
    assert!(!signals[0].signal_id.contains("applied"));
    assert_eq!(signals[0].kind, WorkSignalKind::ReviewRequired);
    assert_ne!(signals[0].kind, WorkSignalKind::Progress);
    cleanup(&project);
}

#[test]
fn verification_preserves_deterministic_provenance() {
    let project = temp_project();
    let boundary = WorkBoundary::VerifyCompleted {
        run_id: "run-ok".into(),
        recipe_id: "npm-test".into(),
        ok: true,
        exit_code: Some(0),
        timed_out: false,
        cancelled: false,
        created_at: "2026-09-09T13:00:00Z".into(),
        patch_id: Some("patch-v".into()),
    };
    record_boundary(&project, BoundarySource::HostAuthorized, &boundary).unwrap();
    let signal = &pending_signals(&project)[0];
    assert_eq!(signal.kind, WorkSignalKind::Milestone);
    assert!(signal.summary.contains("succeeded"));
    assert!(signal.summary.contains("exit=0"));
    assert!(signal.evidence_refs.iter().any(
        |e| matches!(e, openmesh_core::domain::EvidenceRef::FilePath(p) if p.contains("run-ok"))
    ));
    assert!(!signal.summary.contains("stdout"));
    cleanup(&project);
}

#[test]
fn failed_verification_is_not_recorded_as_success() {
    let project = temp_project();
    let boundary = WorkBoundary::VerifyCompleted {
        run_id: "run-fail".into(),
        recipe_id: "npm-test".into(),
        ok: false,
        exit_code: Some(1),
        timed_out: false,
        cancelled: false,
        created_at: "2026-09-09T13:00:00Z".into(),
        patch_id: None,
    };
    record_boundary(&project, BoundarySource::HostAuthorized, &boundary).unwrap();
    let signal = &pending_signals(&project)[0];
    assert_eq!(signal.kind, WorkSignalKind::Blocker);
    assert!(signal.summary.contains("failed"));
    assert!(!signal.summary.contains("succeeded"));
    cleanup(&project);
}

#[test]
fn lan_peer_cannot_mutate_continuity() {
    let project = temp_project();
    let outcome = record_boundary(
        &project,
        BoundarySource::Origin(AgentOrigin::LanPeer),
        &proposed("patch-lan"),
    )
    .unwrap();
    assert_eq!(
        outcome,
        RecordOutcome::Skipped {
            reason: SkipReason::RemoteOrigin
        }
    );
    assert!(pending_signals(&project).is_empty());
    assert!(list_events(&project).unwrap_or_default().is_empty());
    cleanup(&project);
}

#[test]
fn continuity_query_does_not_produce_new_continuity_state() {
    let project = temp_project();
    let outcome = record_boundary(
        &project,
        BoundarySource::Origin(AgentOrigin::ContinuityQuery),
        &proposed("patch-cq"),
    )
    .unwrap();
    assert_eq!(
        outcome,
        RecordOutcome::Skipped {
            reason: SkipReason::RemoteOrigin
        }
    );
    assert!(pending_signals(&project).is_empty());
    cleanup(&project);
}

#[test]
fn remount_replay_is_idempotent() {
    let project = temp_project();
    let import = WorkBoundary::SessionImported {
        source: "cursor".into(),
        source_id: "foreign-1".into(),
        chat_session_id: "chat-1".into(),
        created_at: "2026-09-09T14:00:00Z".into(),
    };
    record_boundary(&project, BoundarySource::HostAuthorized, &import).unwrap();
    let again = record_boundary(&project, BoundarySource::HostAuthorized, &import).unwrap();
    assert!(matches!(again, RecordOutcome::AlreadyRecorded { .. }));
    assert_eq!(pending_signals(&project).len(), 1);
    cleanup(&project);
}

#[test]
fn external_import_preserves_source_agent_provenance() {
    let project = temp_project();
    let import = WorkBoundary::SessionImported {
        source: "claude".into(),
        source_id: "sess-42".into(),
        chat_session_id: "chat-9".into(),
        created_at: "2026-09-09T14:00:00Z".into(),
    };
    record_boundary(&project, BoundarySource::HostAuthorized, &import).unwrap();
    let signal = &pending_signals(&project)[0];
    assert_eq!(signal.kind, WorkSignalKind::AgentSwitch);
    assert!(signal.summary.contains("claude"));
    assert!(signal.summary.contains("sess-42"));
    assert!(!signal
        .summary
        .to_lowercase()
        .contains("authored by openmesh"));
    cleanup(&project);
}

#[test]
fn import_does_not_backfill_canonical_events() {
    let project = temp_project();
    let import = WorkBoundary::SessionImported {
        source: "codex".into(),
        source_id: "old-history".into(),
        chat_session_id: "chat-h".into(),
        created_at: "2026-09-09T14:00:00Z".into(),
    };
    record_boundary(&project, BoundarySource::HostAuthorized, &import).unwrap();
    assert!(list_events(&project).unwrap_or_default().is_empty());
    cleanup(&project);
}

#[test]
fn signals_cannot_cross_project_boundaries() {
    let a = temp_project();
    let b = temp_project();
    record_boundary(&a, BoundarySource::local_chat(), &proposed("patch-xproj")).unwrap();
    assert_eq!(pending_signals(&a).len(), 1);
    assert!(pending_signals(&b).is_empty());
    assert_ne!(project_id(&a), project_id(&b));
    assert_eq!(pending_signals(&a)[0].workspace_id, project_id(&a));
    cleanup(&a);
    cleanup(&b);
}

#[test]
fn credentials_are_not_persisted_in_continuity_records() {
    let project = temp_project();
    let boundary = WorkBoundary::PatchProposed {
        patch_id: "patch-sec".into(),
        summary: "set OPENAI_API_KEY=sk-ant-abcdefghijklmnop and Bearer lan-secret-token-value"
            .into(),
        created_at: "2026-09-09T12:00:00Z".into(),
        file_count: 1,
    };
    record_boundary(&project, BoundarySource::local_chat(), &boundary).unwrap();
    let raw = fs::read_dir(PathBuf::from(&project).join(".openmesh/signals/pending"))
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .expect("pending json");
    assert!(
        !raw.contains("sk-ant-abcdefghijklmnop"),
        "provider token leaked: {raw}"
    );
    assert!(raw.contains("[REDACTED]") || !raw.contains("sk-ant-"));
    cleanup(&project);
}

#[test]
fn local_cli_does_not_write_through_the_bridge() {
    let project = temp_project();
    let outcome = record_boundary(
        &project,
        BoundarySource::Origin(AgentOrigin::LocalCLI),
        &proposed("patch-cli"),
    )
    .unwrap();
    assert_eq!(
        outcome,
        RecordOutcome::Skipped {
            reason: SkipReason::LocalCliOutOfScope
        }
    );
    assert!(pending_signals(&project).is_empty());
    cleanup(&project);
}

#[test]
fn existing_cli_reporter_write_signal_still_works() {
    let project = temp_project();
    let workspace_id = project_id(&project);
    let mut signal = compose_work_signal(&workspace_id, &proposed("patch-mix")).unwrap();
    signal.producer = ProducerRef::Reporter("cli".into());
    signal.signal_id = "sig-cli-compat-1".into();
    write_signal(&project, &signal).expect("CLI-style write_signal must remain valid");
    assert_eq!(pending_signals(&project).len(), 1);
    assert_eq!(
        pending_signals(&project)[0].producer,
        ProducerRef::Reporter("cli".into())
    );
    cleanup(&project);
}

#[test]
fn chat_signals_obey_existing_promotion_and_do_not_bypass_it() {
    let project = temp_project();
    let ws = project_id(&project);
    let proposal = compose_work_signal(&ws, &proposed("patch-promo")).unwrap();
    let apply = compose_work_signal(&ws, &applied("patch-promo")).unwrap();
    let failed_verify = compose_work_signal(
        &ws,
        &WorkBoundary::VerifyCompleted {
            run_id: "run-fv".into(),
            recipe_id: "npm-test".into(),
            ok: false,
            exit_code: Some(1),
            timed_out: false,
            cancelled: false,
            created_at: "2026-09-09T13:00:00Z".into(),
            patch_id: None,
        },
    )
    .unwrap();
    let import = compose_work_signal(
        &ws,
        &WorkBoundary::SessionImported {
            source: "cursor".into(),
            source_id: "s1".into(),
            chat_session_id: "c1".into(),
            created_at: "2026-09-09T14:00:00Z".into(),
        },
    )
    .unwrap();

    let proposal_d = evaluate_promotion_case(&PromotionCase {
        workspace_id: ws.clone(),
        signals: vec![to_ref(&proposal)],
        correlation_hint: None,
    })
    .unwrap();
    assert_eq!(proposal_d.outcome, PromotionOutcome::Promote);
    let kind = proposal_d
        .proposed_composition
        .as_ref()
        .map(|c| c.kind.as_str());
    assert_eq!(kind, Some("work.review-required"));

    let apply_d = evaluate_promotion_case(&PromotionCase {
        workspace_id: ws.clone(),
        signals: vec![to_ref(&apply)],
        correlation_hint: None,
    })
    .unwrap();
    assert_eq!(apply_d.outcome, PromotionOutcome::Promote);
    assert_eq!(
        apply_d
            .proposed_composition
            .as_ref()
            .map(|c| c.kind.as_str()),
        Some("work.progress")
    );

    let fail_d = evaluate_promotion_case(&PromotionCase {
        workspace_id: ws.clone(),
        signals: vec![to_ref(&failed_verify)],
        correlation_hint: None,
    })
    .unwrap();
    assert_eq!(fail_d.outcome, PromotionOutcome::Promote);
    assert_eq!(
        fail_d
            .proposed_composition
            .as_ref()
            .map(|c| c.kind.as_str()),
        Some("work.blocked")
    );

    let import_d = evaluate_promotion_case(&PromotionCase {
        workspace_id: ws,
        signals: vec![to_ref(&import)],
        correlation_hint: None,
    })
    .unwrap();
    assert_eq!(import_d.outcome, PromotionOutcome::Suppress);

    record_boundary(
        &project,
        BoundarySource::local_chat(),
        &proposed("patch-promo"),
    )
    .unwrap();
    assert!(list_events(&project).unwrap_or_default().is_empty());
    cleanup(&project);
}

#[test]
fn local_chat_propose_patch_emits_review_required() {
    let project = temp_project();
    fs::create_dir_all(PathBuf::from(&project).join("src")).unwrap();
    fs::write(PathBuf::from(&project).join("src/a.txt"), "old\n").unwrap();
    let exec = WorkspaceToolExecutor::new(project.clone(), AgentOrigin::LocalChat);
    let out = exec
        .execute(
            "propose_patch",
            r#"{"summary":"update a","files":[{"path":"src/a.txt","newContent":"new\n"}]}"#,
        )
        .unwrap();
    assert!(out.contains("patchId"), "{out}");
    let signals = pending_signals(&project);
    assert_eq!(signals.len(), 1);
    assert_eq!(signals[0].kind, WorkSignalKind::ReviewRequired);
    assert!(signals[0].signal_id.starts_with("wb-patch-proposed-"));
    cleanup(&project);
}

#[test]
fn lan_peer_executor_propose_patch_does_not_write_continuity() {
    let project = temp_project();
    fs::write(PathBuf::from(&project).join("f.txt"), "v1\n").unwrap();
    let exec = WorkspaceToolExecutor::new(project.clone(), AgentOrigin::LanPeer);
    let out = exec
        .execute(
            "propose_patch",
            r#"{"summary":"x","files":[{"path":"f.txt","newContent":"v2\n"}]}"#,
        )
        .unwrap();
    assert!(out.contains("patchId"), "{out}");
    assert!(
        pending_signals(&project).is_empty(),
        "LanPeer must not persist WorkSignals"
    );
    cleanup(&project);
}

#[test]
fn delegate_brief_and_handoff_draft_emit_signals() {
    let project = temp_project();
    write_delegate_brief(&project, "codex", "gap-fill").unwrap();
    create_handoff_draft(&project, r#"{"recipient":"Yo","role":"engineer"}"#).unwrap();
    let signals = pending_signals(&project);
    assert!(
        signals
            .iter()
            .any(|s| s.kind == WorkSignalKind::Progress && s.signal_id.starts_with("wb-delegate-")),
        "missing delegate signal: {signals:?}"
    );
    assert!(
        signals
            .iter()
            .any(|s| s.kind == WorkSignalKind::Handoff && s.signal_id.starts_with("wb-handoff-")),
        "missing handoff signal: {signals:?}"
    );
    cleanup(&project);
}

#[test]
fn host_apply_and_reject_are_distinct_from_proposal() {
    let project = temp_project();
    fs::write(PathBuf::from(&project).join("f.txt"), "v1\n").unwrap();
    let exec = WorkspaceToolExecutor::new(project.clone(), AgentOrigin::LocalChat);
    let out = exec
        .execute(
            "propose_patch",
            r#"{"summary":"x","files":[{"path":"f.txt","newContent":"v2\n"}]}"#,
        )
        .unwrap();
    let id = serde_json::from_str::<serde_json::Value>(&out).unwrap()["patchId"]
        .as_str()
        .unwrap()
        .to_string();
    apply_patch(&project, &id).unwrap();
    let kinds: Vec<_> = pending_signals(&project)
        .into_iter()
        .map(|s| s.kind)
        .collect();
    assert!(kinds.contains(&WorkSignalKind::ReviewRequired));
    assert!(kinds.contains(&WorkSignalKind::Progress));

    let project2 = temp_project();
    fs::write(PathBuf::from(&project2).join("g.txt"), "v1\n").unwrap();
    let exec2 = WorkspaceToolExecutor::new(project2.clone(), AgentOrigin::LocalChat);
    let out2 = exec2
        .execute(
            "propose_patch",
            r#"{"summary":"y","files":[{"path":"g.txt","newContent":"v2\n"}]}"#,
        )
        .unwrap();
    let id2 = serde_json::from_str::<serde_json::Value>(&out2).unwrap()["patchId"]
        .as_str()
        .unwrap()
        .to_string();
    reject_patch(&project2, &id2).unwrap();
    let signals = pending_signals(&project2);
    assert!(signals
        .iter()
        .any(|s| s.kind == WorkSignalKind::ReviewRequired));
    assert!(signals.iter().any(|s| s.kind == WorkSignalKind::Decision));
    assert!(!signals.iter().any(|s| s.signal_id.contains("applied")));
    cleanup(&project);
    cleanup(&project2);
}

fn write_recipe(project: &str, id: &str, argv: Vec<String>) {
    let dir = get_project_dir(project).join("agent").join("recipes");
    fs::create_dir_all(&dir).unwrap();
    let recipe = Recipe {
        id: id.into(),
        title: id.into(),
        argv,
        cwd_rel: String::new(),
        timeout_ms: 5_000,
    };
    atomic_write(
        &dir.join(format!("{id}.json")),
        &serde_json::to_string_pretty(&recipe).unwrap(),
    )
    .unwrap();
}

#[test]
fn recipe_success_and_failure_record_distinct_kinds() {
    let project = temp_project();
    write_recipe(
        &project,
        "echo-hi",
        vec!["echo".into(), "hello-openmesh".into()],
    );
    let ok = run_recipe(&project, "echo-hi", "test-run-ok", None).unwrap();
    assert!(ok.ok, "{ok:?}");
    write_recipe(&project, "fail-now", vec!["false".into()]);
    let failed = run_recipe(&project, "fail-now", "test-run-fail", None).unwrap();
    assert!(!failed.ok, "{failed:?}");

    let signals = pending_signals(&project);
    let success = signals
        .iter()
        .find(|s| s.signal_id == format!("wb-verify-{}", ok.run_id))
        .unwrap();
    let failure = signals
        .iter()
        .find(|s| s.signal_id == format!("wb-verify-{}", failed.run_id))
        .unwrap();
    assert_eq!(success.kind, WorkSignalKind::Milestone);
    assert_eq!(failure.kind, WorkSignalKind::Blocker);
    let raw = fs::read_dir(PathBuf::from(&project).join(".openmesh/signals/pending"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .map(|e| fs::read_to_string(e.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !raw.contains("hello-openmesh"),
        "raw recipe stdout must not enter Continuity records"
    );
    cleanup(&project);
}

#[test]
fn engine_loop_and_live_ask_do_not_call_the_bridge() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine =
        fs::read_to_string(root.join("crates/openmesh-core/src/agent_engine/engine_loop.rs"))
            .unwrap();
    assert!(
        !engine.contains("workbench_continuity") && !engine.contains("record_boundary"),
        "engine loop must not write Continuity on turn completion"
    );
    let live =
        fs::read_to_string(root.join("crates/openmesh-core/src/agent_engine/live_ask.rs")).unwrap();
    assert!(
        !live.contains("record_boundary") && !live.contains("workbench_continuity"),
        "live ask must not record workbench Continuity"
    );
}
