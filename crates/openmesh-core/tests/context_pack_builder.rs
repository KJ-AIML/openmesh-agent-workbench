//! Dev Track 0.1.5 Checkpoint C — Proxy Context Pack builder tests.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack::{
    build_proxy_context_pack, compose_proxy_context_pack, compute_build_inputs_hash,
    ContextPackBuildError, ProxyContextPackBuildOptions, ProxyContextPackComposeInputs,
};
use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, current_state_projection_path,
    load_continuity_input_snapshot, projections_dir, rebuild_current_state_projection,
    ContinuityInputSnapshot,
};
use openmesh_core::domain::{
    default_work_proxy_profile, deterministic_context_pack_id, validate_proxy_context_pack,
    ActorRef, CatchUpWindow, ContextPackItemProvenance, EvidenceAttachment, EvidenceRef,
    ProducerRef, ProxyContextPack, SourceCounts, WorkEvent, WorkProxyProfile, WorkSignal,
    WorkSignalKind, CONTEXT_PACK_EXECUTION_BOUNDARY, MAX_CONTEXT_PACK_DIAGNOSTICS,
    MAX_CONTEXT_PACK_LIMITATIONS, MAX_CONTEXT_PACK_UNRESOLVED_ITEMS,
    PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::{
    profile_dir, read_work_proxy_profile, work_proxy_profile_path, write_work_proxy_profile,
    ProfileError,
};
use openmesh_core::promotion::{
    promotion_decisions_dir, write_decision_record, PromotionDecision, PromotionDecisionRecord,
    PromotionKey, PromotionReasonCode,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::{get_project_dir, init_project};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const EVENT_TS: &str = "2026-07-17T01:00:00Z";
const CATCH_UP_SINCE: &str = "2026-07-15T00:00:00Z";
const CATCH_UP_UNTIL: &str = "2026-07-18T00:00:00Z";
const GENERATED_AT: &str = "2026-07-18T04:00:00Z";

fn fixed_window() -> CatchUpWindow {
    CatchUpWindow {
        since: CATCH_UP_SINCE.into(),
        until: CATCH_UP_UNTIL.into(),
    }
}

fn default_options() -> ProxyContextPackBuildOptions {
    ProxyContextPackBuildOptions {
        generated_at: GENERATED_AT.into(),
        selection: Default::default(),
    }
}

fn temp_project(label: &str) -> (PathBuf, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-core-context-pack-builder-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let project_path = dir.to_string_lossy().to_string();
    init_project(&project_path).expect("init");
    let project_id = fs::read_to_string(dir.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("project id");
    (dir, project_id)
}

fn sample_event(event_id: &str, workspace_id: &str, summary: &str) -> WorkEvent {
    WorkEvent::new(
        event_id,
        workspace_id,
        "work.completed",
        summary,
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        EVENT_TS,
    )
}

fn correction_event(
    event_id: &str,
    workspace_id: &str,
    target_id: &str,
    summary: &str,
    timestamp: &str,
) -> WorkEvent {
    let mut event = WorkEvent::new(
        event_id,
        workspace_id,
        "work.completed",
        summary,
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        timestamp,
    );
    event.corrects_event_id = Some(target_id.into());
    event
}

fn sample_signal(signal_id: &str, workspace_id: &str, sensitivity: Sensitivity) -> WorkSignal {
    WorkSignal {
        signal_id: signal_id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("builder-test".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: format!("signal summary for {signal_id}"),
        timestamp: EVENT_TS.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity,
        protocol_version: "1.0".into(),
    }
}

fn write_valid_profile(project_path: &str, workspace_id: &str) -> WorkProxyProfile {
    let profile = default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Pack Owner",
        "Pack Role",
        "2026-07-17T08:00:00Z",
    );
    write_work_proxy_profile(project_path, &profile).expect("write profile");
    profile
}

fn seed_minimal_continuity(project_path: &str, workspace_id: &str) {
    append_event(
        project_path,
        &sample_event("evt-builder-seed", workspace_id, "Builder seed event"),
    )
    .unwrap();
    write_signal(
        project_path,
        &sample_signal("sig-builder-seed", workspace_id, Sensitivity::Private),
    )
    .unwrap();
}

fn seed_rich_continuity(project_path: &str, workspace_id: &str) {
    append_event(
        project_path,
        &sample_event("evt-visible", workspace_id, "Visible continuity event"),
    )
    .unwrap();

    let mut secret_event = sample_event(
        "evt-secret",
        workspace_id,
        "vault contains super-secret-api-key-ROTATE-ME",
    );
    secret_event.sensitivity = Sensitivity::Secret;
    secret_event.evidence = vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/restricted-note.md".into()),
        observed_at: None,
    }];
    append_event(project_path, &secret_event).unwrap();

    append_event(
        project_path,
        &sample_event(
            "evt-original",
            workspace_id,
            "Original summary before correction",
        ),
    )
    .unwrap();
    append_event(
        project_path,
        &correction_event(
            "evt-correct-1",
            workspace_id,
            "evt-original",
            "Corrected summary after review",
            "2026-07-17T02:00:00Z",
        ),
    )
    .unwrap();

    write_signal(
        project_path,
        &sample_signal("sig-pending", workspace_id, Sensitivity::Private),
    )
    .unwrap();
    write_signal(
        project_path,
        &sample_signal("sig-processed", workspace_id, Sensitivity::Private),
    )
    .unwrap();
}

fn build_pack(project_path: &str) -> ProxyContextPack {
    build_proxy_context_pack(project_path, fixed_window(), default_options()).expect("build pack")
}

fn compose_from_project(project_path: &str, generated_at: &str) -> ProxyContextPack {
    let profile = read_work_proxy_profile(project_path).expect("profile");
    let snapshot = load_continuity_input_snapshot(project_path).expect("snapshot");
    let current_state = build_current_state_projection(&snapshot).expect("current state");
    let catch_up =
        build_catch_up_view(&snapshot, &current_state, &fixed_window()).expect("catch-up");
    compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window: fixed_window(),
            generated_at: generated_at.into(),
        },
        &default_options(),
    )
    .expect("compose")
}

fn empty_source_counts() -> SourceCounts {
    SourceCounts {
        work_events: 0,
        processed_signals: 0,
        pending_signals: 0,
        promotion_audit_records: 0,
        quarantine_signals: 0,
        duplicate_signals: 0,
        reporter_signals: 0,
        git_signals: 0,
        heli_signals: 0,
        unknown_producer_signals: 0,
        other_producer_signals: 0,
    }
}

fn compose_from_parts(
    profile: WorkProxyProfile,
    snapshot: ContinuityInputSnapshot,
    window: CatchUpWindow,
    generated_at: &str,
) -> ProxyContextPack {
    let current_state = build_current_state_projection(&snapshot).expect("current state");
    let catch_up = build_catch_up_view(&snapshot, &current_state, &window).expect("catch-up");
    compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window,
            generated_at: generated_at.into(),
        },
        &default_options(),
    )
    .expect("compose")
}

fn compose_with_projection_timestamps(
    profile: WorkProxyProfile,
    snapshot: ContinuityInputSnapshot,
    window: CatchUpWindow,
    current_state_generated_at: &str,
    catch_up_generated_at: &str,
    pack_generated_at: &str,
) -> ProxyContextPack {
    let mut current_state = build_current_state_projection(&snapshot).expect("current state");
    current_state.generated_at = current_state_generated_at.into();
    let mut catch_up = build_catch_up_view(&snapshot, &current_state, &window).expect("catch-up");
    catch_up.generated_at = catch_up_generated_at.into();
    compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window,
            generated_at: pack_generated_at.into(),
        },
        &default_options(),
    )
    .expect("compose")
}

fn profile_for_workspace(workspace_id: &str) -> WorkProxyProfile {
    default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Pack Owner",
        "Pack Role",
        "2026-07-17T08:00:00Z",
    )
}

fn hash_for_parts(profile: &WorkProxyProfile, snapshot: &ContinuityInputSnapshot) -> String {
    compute_build_inputs_hash(profile, snapshot, &fixed_window(), &default_options()).expect("hash")
}

fn secret_event(
    event_id: &str,
    workspace_id: &str,
    summary: &str,
    evidence_path: &str,
) -> WorkEvent {
    let mut event = sample_event(event_id, workspace_id, summary);
    event.sensitivity = Sensitivity::Secret;
    event.evidence = vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath(evidence_path.into()),
        observed_at: None,
    }];
    event
}

fn snapshot_with_events(
    workspace_id: &str,
    work_events: Vec<WorkEvent>,
) -> ContinuityInputSnapshot {
    let mut source_counts = empty_source_counts();
    source_counts.work_events = work_events.len() as u32;
    ContinuityInputSnapshot {
        workspace_id: workspace_id.into(),
        loaded_at: "2026-07-17T03:00:00Z".into(),
        pending_signals: Vec::new(),
        processed_signals: Vec::new(),
        quarantine_signals: Vec::new(),
        duplicate_signals: Vec::new(),
        work_events,
        promotion_audit_records: Vec::new(),
        diagnostics: Vec::new(),
        source_counts,
    }
}

fn bucket_snapshot(project_path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let root = get_project_dir(project_path).join("signals");
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = root.join(bucket);
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_file() {
                let key = format!("{bucket}/{}", path.file_name().unwrap().to_string_lossy());
                out.insert(key, fs::read_to_string(&path).unwrap());
            }
        }
    }
    out
}

fn ledger_file_count(project_path: &str) -> usize {
    let dir = ledger_dir(project_path);
    if !dir.exists() {
        return 0;
    }
    fs::read_dir(dir)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0)
}

fn promotion_audit_snapshot(project_path: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let dir = promotion_decisions_dir(project_path);
    if !dir.exists() {
        return out;
    }
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            out.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&path).unwrap(),
            );
        }
    }
    out
}

fn pack_json_lower(pack: &ProxyContextPack) -> String {
    serde_json::to_string(pack)
        .expect("serialize pack")
        .to_ascii_lowercase()
}

fn all_pack_continuity_summaries(pack: &ProxyContextPack) -> Vec<String> {
    let mut summaries = Vec::new();
    let sections = &pack.current_state.sections;
    for item in sections
        .completed
        .iter()
        .chain(sections.in_progress.iter())
        .chain(sections.blocked.iter())
        .chain(sections.decisions.iter())
        .chain(sections.needs_attention.iter())
        .chain(sections.still_open.iter())
    {
        summaries.push(item.summary.clone());
    }
    let catch_up = &pack.catch_up.sections;
    for item in catch_up
        .completed
        .iter()
        .chain(catch_up.changed.iter())
        .chain(catch_up.blocked.iter())
        .chain(catch_up.decided.iter())
        .chain(catch_up.needs_attention.iter())
        .chain(catch_up.still_open.iter())
    {
        summaries.push(item.summary.clone());
    }
    for item in pack
        .current_state
        .pending_attention
        .iter()
        .chain(pack.catch_up.next_suggested_attention.iter())
    {
        summaries.push(item.summary.clone());
    }
    summaries
}

#[test]
fn builder_requires_initialized_project() {
    let path = std::env::temp_dir()
        .join(format!(
            "openmesh-uninitialized-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ))
        .to_string_lossy()
        .to_string();
    let err = build_proxy_context_pack(&path, fixed_window(), default_options()).unwrap_err();
    assert!(matches!(
        err,
        ContextPackBuildError::ProjectNotInitialized(_)
    ));
}

#[test]
fn builder_requires_existing_profile() {
    let (_dir, workspace_id) = temp_project("requires-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert_eq!(err, ContextPackBuildError::ProfileMissing);
}

#[test]
fn builder_rejects_invalid_profile() {
    let (_dir, workspace_id) = temp_project("invalid-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(work_proxy_profile_path(&project_path), "{not-valid-json").unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::MalformedJson(_))
    ));
    assert!(matches!(
        build_proxy_context_pack(&project_path, fixed_window(), default_options()),
        Err(ContextPackBuildError::Profile(_))
    ));
}

#[test]
fn builder_rejects_profile_workspace_mismatch() {
    let (_dir, workspace_id) = temp_project("workspace-mismatch");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let mut profile = write_valid_profile(&project_path, &workspace_id);
    profile.workspace_id = "ws-other".into();
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        build_proxy_context_pack(&project_path, fixed_window(), default_options()),
        Err(ContextPackBuildError::Profile(_))
    ));
}

#[test]
fn builder_does_not_synthesize_missing_profile() {
    let (_dir, workspace_id) = temp_project("no-synthesis");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let _ = build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(!work_proxy_profile_path(&project_path).exists());
}

#[test]
fn builder_loads_continuity_snapshot_read_only() {
    let (_dir, workspace_id) = temp_project("snapshot-read-only");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let buckets_before = bucket_snapshot(&project_path);
    let ledger_before = ledger_file_count(&project_path);
    let audit_before = promotion_audit_snapshot(&project_path);
    let _ = build_pack(&project_path);
    assert_eq!(buckets_before, bucket_snapshot(&project_path));
    assert_eq!(ledger_before, ledger_file_count(&project_path));
    assert_eq!(audit_before, promotion_audit_snapshot(&project_path));
}

#[test]
fn builder_builds_current_state_in_memory() {
    let (_dir, workspace_id) = temp_project("current-state-memory");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    assert!(!current_state_projection_path(&project_path).exists());
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    let expected = build_current_state_projection(&snapshot).expect("projection");
    let pack = build_pack(&project_path);
    assert_eq!(pack.current_state.workspace_id, expected.workspace_id);
    assert_eq!(
        pack.source_counts.work_events,
        expected.source_counts.work_events
    );
    let summaries = all_pack_continuity_summaries(&pack);
    assert!(
        summaries
            .iter()
            .any(|summary| summary.contains("Builder seed event")),
        "current state should be built from snapshot without persisted projection"
    );
}

#[test]
fn builder_builds_catch_up_for_explicit_fixed_window() {
    let (_dir, workspace_id) = temp_project("catch-up-window");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(pack.requested_window.since, CATCH_UP_SINCE);
    assert_eq!(pack.requested_window.until, CATCH_UP_UNTIL);
    assert_eq!(pack.catch_up.window.since, CATCH_UP_SINCE);
    assert_eq!(pack.catch_up.window.until, CATCH_UP_UNTIL);
    assert_eq!(pack.freshness.catch_up_since, CATCH_UP_SINCE);
    assert_eq!(pack.freshness.catch_up_until, CATCH_UP_UNTIL);
}

#[test]
fn builder_rejects_since_after_until() {
    let (_dir, workspace_id) = temp_project("invalid-window");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let window = CatchUpWindow {
        since: "2026-07-19T00:00:00Z".into(),
        until: "2026-07-18T00:00:00Z".into(),
    };
    let err = build_proxy_context_pack(&project_path, window, default_options()).unwrap_err();
    assert!(
        matches!(err, ContextPackBuildError::InvalidWindow(_))
            || matches!(err, ContextPackBuildError::CatchUpBuild(_)),
        "unexpected error: {err:?}"
    );
}

#[test]
fn builder_composes_valid_proxy_context_pack() {
    let (_dir, workspace_id) = temp_project("valid-pack");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    validate_proxy_context_pack(&pack).expect("pack validates");
    assert_eq!(pack.protocol_version, PROXY_CONTEXT_PACK_PROTOCOL_VERSION);
}

#[test]
fn builder_populates_owner_identity_from_profile() {
    let (_dir, workspace_id) = temp_project("owner-identity");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let profile = write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(pack.owner_identity.owner_label, profile.owner_label);
    assert_eq!(pack.owner_identity.role_label, profile.role_label);
}

#[test]
fn builder_copies_declarative_authority_metadata_only() {
    let (_dir, workspace_id) = temp_project("authority-metadata");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let profile = write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(
        pack.authority_summary.authority_rules,
        profile.authority_rules
    );
    assert_eq!(
        pack.authority_summary.default_refusal_rules,
        profile.default_refusal_rules
    );
    assert_eq!(
        pack.authority_summary.execution_boundary,
        CONTEXT_PACK_EXECUTION_BOUNDARY
    );
    assert!(!pack.authority_summary.ladder_levels.is_empty());
}

#[test]
fn builder_copies_privacy_and_evidence_policy_metadata() {
    let (_dir, workspace_id) = temp_project("privacy-policy");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let profile = write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(pack.privacy_summary.privacy_rules, profile.privacy_rules);
    assert_eq!(
        pack.privacy_summary.sensitive_topics,
        profile.sensitive_topics
    );
    assert_eq!(pack.evidence_policy, profile.evidence_policy);
}

#[test]
fn builder_uses_sanitized_current_state_representation() {
    let (_dir, workspace_id) = temp_project("sanitize-current");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let json = pack_json_lower(&pack);
    assert!(!json.contains("super-secret-api-key-rotate-me"));
    assert!(pack.redaction_summary.secret_items_omitted > 0);
}

#[test]
fn builder_uses_sanitized_catch_up_representation() {
    let (_dir, workspace_id) = temp_project("sanitize-catch-up");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let catch_up_json = serde_json::to_string(&pack.catch_up).expect("serialize");
    assert!(!catch_up_json.contains("super-secret-api-key"));
}

#[test]
fn builder_omits_secret_content_from_all_pack_surfaces() {
    let (_dir, workspace_id) = temp_project("omit-secrets");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let json = pack_json_lower(&pack);
    for forbidden in ["super-secret-api-key", "vault contains"] {
        assert!(!json.contains(forbidden), "pack leaked {forbidden}");
    }
}

#[test]
fn builder_uses_checkpoint_b_evidence_selection() {
    let (_dir, workspace_id) = temp_project("evidence-selection");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert!(pack.evidence_index.iter().all(|entry| {
        entry.sensitivity != Sensitivity::Secret
            && !entry.label.to_ascii_lowercase().contains("super-secret")
    }));
    for (index, entry) in pack.evidence_index.iter().enumerate() {
        assert_eq!(entry.ref_id, format!("ref-{:03}", index + 1));
    }
}

#[test]
fn builder_preserves_pending_unconfirmed_provenance() {
    let (_dir, workspace_id) = temp_project("pending-provenance");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert!(pack
        .current_state
        .pending_attention
        .iter()
        .chain(pack.catch_up.next_suggested_attention.iter())
        .any(|item| item.provenance == ContextPackItemProvenance::Pending));
}

#[test]
fn builder_does_not_promote_pending_signals() {
    let (_dir, workspace_id) = temp_project("no-promotion");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let before = bucket_snapshot(&project_path);
    let _ = build_pack(&project_path);
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn builder_uses_effective_corrected_presentation() {
    let (_dir, workspace_id) = temp_project("corrected-presentation");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let summaries = all_pack_continuity_summaries(&pack);
    assert!(summaries
        .iter()
        .any(|summary| summary.contains("Corrected summary")));
    assert!(!summaries
        .iter()
        .any(|summary| summary == "Original summary before correction"));
}

#[test]
fn builder_does_not_reintroduce_superseded_raw_presentation() {
    let (_dir, workspace_id) = temp_project("superseded-raw");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let summaries = all_pack_continuity_summaries(&pack);
    assert!(!summaries
        .iter()
        .any(|summary| { summary == "Original summary before correction" }));
}

#[test]
fn builder_preserves_correction_provenance() {
    let (_dir, workspace_id) = temp_project("correction-provenance");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let corrected = pack
        .current_state
        .sections
        .completed
        .iter()
        .chain(pack.current_state.sections.in_progress.iter())
        .chain(pack.current_state.sections.blocked.iter())
        .chain(pack.current_state.sections.decisions.iter())
        .chain(pack.current_state.sections.needs_attention.iter())
        .chain(pack.current_state.sections.still_open.iter())
        .chain(pack.catch_up.sections.changed.iter())
        .chain(pack.catch_up.sections.completed.iter())
        .find(|item| item.summary.contains("Corrected summary"));
    let item = corrected.expect("corrected continuity item");
    let has_correction_ref = item.evidence_refs.iter().any(|evidence| match evidence {
        EvidenceRef::FilePath(path) => path.contains("evt-correct-1"),
        _ => false,
    });
    let has_correction_metadata = item
        .correction
        .as_ref()
        .map(|correction| correction.is_corrected || !correction.correction_event_ids.is_empty())
        .unwrap_or(false);
    assert!(
        has_correction_ref || has_correction_metadata,
        "expected correction provenance on corrected item"
    );
}

#[test]
fn build_inputs_hash_is_stable_for_identical_semantic_inputs() {
    let (_dir, workspace_id) = temp_project("hash-stable");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let first = build_pack(&project_path);
    let second = build_pack(&project_path);
    assert_eq!(first.build_inputs_hash, second.build_inputs_hash);
}

#[test]
fn build_inputs_hash_ignores_generated_at() {
    let (_dir, workspace_id) = temp_project("hash-generated-at");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let earlier = compose_from_project(&project_path, "2026-07-18T01:00:00Z");
    let later = compose_from_project(&project_path, "2026-07-18T08:00:00Z");
    assert_eq!(earlier.build_inputs_hash, later.build_inputs_hash);
    assert_ne!(earlier.generated_at, later.generated_at);
}

#[test]
fn build_inputs_hash_ignores_input_collection_order() {
    let workspace_id = "ws-order";
    let event_a = sample_event("evt-a", workspace_id, "event a");
    let event_b = sample_event("evt-b", workspace_id, "event b");
    let profile = default_work_proxy_profile(
        workspace_id,
        "profile-ws-order",
        "Owner",
        "Role",
        "2026-07-17T08:00:00Z",
    );
    let snap_a = snapshot_with_events(workspace_id, vec![event_a.clone(), event_b.clone()]);
    let snap_b = snapshot_with_events(workspace_id, vec![event_b, event_a]);
    let hash_a =
        compute_build_inputs_hash(&profile, &snap_a, &fixed_window(), &default_options()).unwrap();
    let hash_b =
        compute_build_inputs_hash(&profile, &snap_b, &fixed_window(), &default_options()).unwrap();
    assert_eq!(hash_a, hash_b);
}

#[test]
fn build_inputs_hash_changes_when_profile_changes() {
    let (_dir, workspace_id) = temp_project("hash-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let mut profile = write_valid_profile(&project_path, &workspace_id);
    let snapshot = load_continuity_input_snapshot(&project_path).unwrap();
    let baseline =
        compute_build_inputs_hash(&profile, &snapshot, &fixed_window(), &default_options())
            .unwrap();
    profile.owner_label = "Different Owner".into();
    let changed =
        compute_build_inputs_hash(&profile, &snapshot, &fixed_window(), &default_options())
            .unwrap();
    assert_ne!(baseline, changed);
}

#[test]
fn build_inputs_hash_changes_when_continuity_truth_changes() {
    let (_dir, workspace_id) = temp_project("hash-continuity");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let profile = write_valid_profile(&project_path, &workspace_id);
    let before = load_continuity_input_snapshot(&project_path).unwrap();
    let hash_before =
        compute_build_inputs_hash(&profile, &before, &fixed_window(), &default_options()).unwrap();
    append_event(
        &project_path,
        &sample_event("evt-extra", &workspace_id, "extra event"),
    )
    .unwrap();
    let after = load_continuity_input_snapshot(&project_path).unwrap();
    let hash_after =
        compute_build_inputs_hash(&profile, &after, &fixed_window(), &default_options()).unwrap();
    assert_ne!(hash_before, hash_after);
}

#[test]
fn build_inputs_hash_changes_when_window_changes() {
    let (_dir, workspace_id) = temp_project("hash-window");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let profile = write_valid_profile(&project_path, &workspace_id);
    let snapshot = load_continuity_input_snapshot(&project_path).unwrap();
    let hash_a =
        compute_build_inputs_hash(&profile, &snapshot, &fixed_window(), &default_options())
            .unwrap();
    let other_window = CatchUpWindow {
        since: "2026-07-16T00:00:00Z".into(),
        until: "2026-07-17T00:00:00Z".into(),
    };
    let hash_b =
        compute_build_inputs_hash(&profile, &snapshot, &other_window, &default_options()).unwrap();
    assert_ne!(hash_a, hash_b);
}

#[test]
fn context_pack_id_is_deterministic_from_build_inputs_hash() {
    let (_dir, workspace_id) = temp_project("pack-id");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(
        pack.context_pack_id,
        deterministic_context_pack_id(&pack.build_inputs_hash)
    );
}

#[test]
fn context_pack_id_ignores_generated_at() {
    let (_dir, workspace_id) = temp_project("pack-id-generated-at");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let earlier = compose_from_project(&project_path, "2026-07-18T01:00:00Z");
    let later = compose_from_project(&project_path, "2026-07-18T08:00:00Z");
    assert_eq!(earlier.context_pack_id, later.context_pack_id);
}

#[test]
fn build_hash_changes_when_event_summary_changes_with_same_id() {
    let workspace_id = "ws-hash-summary";
    let profile = profile_for_workspace(workspace_id);
    let event_a = sample_event("evt-same", workspace_id, "visible summary alpha");
    let event_b = sample_event("evt-same", workspace_id, "visible summary beta");
    let snap_a = snapshot_with_events(workspace_id, vec![event_a]);
    let snap_b = snapshot_with_events(workspace_id, vec![event_b]);
    let hash_a = hash_for_parts(&profile, &snap_a);
    let hash_b = hash_for_parts(&profile, &snap_b);
    assert_ne!(hash_a, hash_b);
}

#[test]
fn build_hash_changes_when_event_kind_changes_with_same_id() {
    let workspace_id = "ws-hash-kind";
    let profile = profile_for_workspace(workspace_id);
    let mut event_a = sample_event("evt-same", workspace_id, "same visible summary");
    event_a.kind = "work.completed".into();
    let mut event_b = sample_event("evt-same", workspace_id, "same visible summary");
    event_b.kind = "work.blocked".into();
    let hash_a = hash_for_parts(&profile, &snapshot_with_events(workspace_id, vec![event_a]));
    let hash_b = hash_for_parts(&profile, &snapshot_with_events(workspace_id, vec![event_b]));
    assert_ne!(hash_a, hash_b);
}

#[test]
fn build_hash_changes_when_effective_correction_changes_with_same_ids() {
    let workspace_id = "ws-hash-correction-effective";
    let profile = profile_for_workspace(workspace_id);
    let original = sample_event(
        "evt-target",
        workspace_id,
        "Original summary before correction",
    );
    let correction_a = correction_event(
        "evt-correct",
        workspace_id,
        "evt-target",
        "Corrected summary alpha",
        "2026-07-17T02:00:00Z",
    );
    let correction_b = correction_event(
        "evt-correct",
        workspace_id,
        "evt-target",
        "Corrected summary beta",
        "2026-07-17T02:00:00Z",
    );
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original.clone(), correction_a]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original, correction_b]),
    );
    assert_ne!(hash_a, hash_b);
}

#[test]
fn build_hash_changes_when_correction_provenance_changes() {
    let workspace_id = "ws-hash-correction-prov";
    let profile = profile_for_workspace(workspace_id);
    let original = sample_event(
        "evt-target",
        workspace_id,
        "Original summary before correction",
    );
    let correction_one = correction_event(
        "evt-correct-1",
        workspace_id,
        "evt-target",
        "Corrected summary after review",
        "2026-07-17T02:00:00Z",
    );
    let correction_two = correction_event(
        "evt-correct-2",
        workspace_id,
        "evt-target",
        "Corrected summary after review",
        "2026-07-17T03:00:00Z",
    );
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original.clone(), correction_one]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original, correction_two]),
    );
    assert_ne!(hash_a, hash_b);
}

#[test]
fn build_hash_changes_when_pending_provenance_changes() {
    let workspace_id = "ws-hash-pending-prov";
    let profile = profile_for_workspace(workspace_id);
    let mut snap_pending = snapshot_with_events(workspace_id, Vec::new());
    snap_pending.pending_signals = vec![sample_signal(
        "sig-pending",
        workspace_id,
        Sensitivity::Private,
    )];
    snap_pending.source_counts.pending_signals = 1;

    let mut snap_unconfirmed = snapshot_with_events(
        workspace_id,
        vec![sample_event("evt-visible", workspace_id, "visible event")],
    );
    snap_unconfirmed.processed_signals = vec![sample_signal(
        "sig-processed",
        workspace_id,
        Sensitivity::Private,
    )];
    snap_unconfirmed.source_counts.processed_signals = 1;
    let key = PromotionKey::from_inputs(workspace_id, &["sig-processed".to_string()]).expect("key");
    let decision =
        PromotionDecision::ambiguous(key, vec!["sig-processed".into()], "needs review".into());
    let record = PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        None,
        "2026-07-17T02:00:00Z".into(),
    );
    snap_unconfirmed.promotion_audit_records = vec![record];
    snap_unconfirmed.source_counts.promotion_audit_records = 1;

    let pack_pending =
        compose_from_parts(profile.clone(), snap_pending, fixed_window(), GENERATED_AT);
    let pack_unconfirmed =
        compose_from_parts(profile, snap_unconfirmed, fixed_window(), GENERATED_AT);
    let pending_prov = pack_pending
        .current_state
        .pending_attention
        .iter()
        .find(|item| item.provenance == ContextPackItemProvenance::Pending)
        .expect("pending provenance item");
    let unconfirmed_prov = pack_unconfirmed
        .current_state
        .pending_attention
        .iter()
        .find(|item| item.provenance == ContextPackItemProvenance::Unconfirmed)
        .expect("unconfirmed provenance item");
    assert_ne!(pending_prov.provenance, unconfirmed_prov.provenance);
    assert_ne!(
        pack_pending.build_inputs_hash,
        pack_unconfirmed.build_inputs_hash
    );
}

#[test]
fn build_hash_changes_when_non_secret_evidence_ref_changes() {
    let workspace_id = "ws-hash-evidence-ref";
    let profile = profile_for_workspace(workspace_id);
    let mut event_a = sample_event("evt-visible", workspace_id, "visible event");
    event_a.evidence = vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/alpha.md".into()),
        observed_at: None,
    }];
    let mut event_b = sample_event("evt-visible", workspace_id, "visible event");
    event_b.evidence = vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/beta.md".into()),
        observed_at: None,
    }];
    let hash_a = hash_for_parts(&profile, &snapshot_with_events(workspace_id, vec![event_a]));
    let hash_b = hash_for_parts(&profile, &snapshot_with_events(workspace_id, vec![event_b]));
    assert_ne!(hash_a, hash_b);
}

#[test]
fn build_hash_changes_when_safe_limitation_changes() {
    let workspace_id = "ws-hash-limitation";
    let mut profile = profile_for_workspace(workspace_id);
    let snapshot = snapshot_with_events(
        workspace_id,
        vec![sample_event("evt-visible", workspace_id, "visible event")],
    );
    let baseline = hash_for_parts(&profile, &snapshot);
    profile
        .limitations
        .push("extra safe limitation for hash".into());
    let changed = hash_for_parts(&profile, &snapshot);
    assert_ne!(baseline, changed);
}

#[test]
fn build_hash_changes_when_safe_unresolved_item_changes() {
    let workspace_id = "ws-hash-unresolved";
    let profile = profile_for_workspace(workspace_id);
    let mut snap_a = snapshot_with_events(workspace_id, Vec::new());
    snap_a.pending_signals = vec![sample_signal(
        "sig-pending-a",
        workspace_id,
        Sensitivity::Private,
    )];
    snap_a.source_counts.pending_signals = 1;
    let mut snap_b = snapshot_with_events(workspace_id, Vec::new());
    snap_b.pending_signals = vec![sample_signal(
        "sig-pending-b",
        workspace_id,
        Sensitivity::Private,
    )];
    snap_b.source_counts.pending_signals = 1;
    let pack_a = compose_from_parts(profile.clone(), snap_a, fixed_window(), GENERATED_AT);
    let pack_b = compose_from_parts(profile, snap_b, fixed_window(), GENERATED_AT);
    assert_ne!(pack_a.build_inputs_hash, pack_b.build_inputs_hash);
    assert_ne!(pack_a.unresolved_items, pack_b.unresolved_items);
}

#[test]
fn build_hash_changes_when_redaction_count_changes() {
    let workspace_id = "ws-hash-redaction-count";
    let profile = profile_for_workspace(workspace_id);
    let visible = sample_event("evt-visible", workspace_id, "visible event");
    let one_secret = snapshot_with_events(
        workspace_id,
        vec![
            visible.clone(),
            secret_event(
                "evt-secret-1",
                workspace_id,
                "secret payload one",
                "docs/secret-one.md",
            ),
        ],
    );
    let two_secret = snapshot_with_events(
        workspace_id,
        vec![
            visible,
            secret_event(
                "evt-secret-1",
                workspace_id,
                "secret payload one",
                "docs/secret-one.md",
            ),
            secret_event(
                "evt-secret-2",
                workspace_id,
                "secret payload two",
                "docs/secret-two.md",
            ),
        ],
    );
    let pack_one = compose_from_parts(profile.clone(), one_secret, fixed_window(), GENERATED_AT);
    let pack_two = compose_from_parts(profile, two_secret, fixed_window(), GENERATED_AT);
    assert!(
        pack_two.redaction_summary.secret_items_omitted
            > pack_one.redaction_summary.secret_items_omitted
    );
    assert_ne!(pack_one.build_inputs_hash, pack_two.build_inputs_hash);
}

#[test]
fn build_hash_ignores_raw_secret_content_changes() {
    let workspace_id = "ws-hash-secret-content";
    let profile = profile_for_workspace(workspace_id);
    let visible = sample_event("evt-visible", workspace_id, "visible event");
    let secret_a = secret_event(
        "evt-secret",
        workspace_id,
        "vault contains super-secret-alpha",
        "docs/restricted-note.md",
    );
    let secret_b = secret_event(
        "evt-secret",
        workspace_id,
        "vault contains super-secret-beta",
        "docs/restricted-note.md",
    );
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![visible.clone(), secret_a]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![visible, secret_b]),
    );
    assert_eq!(hash_a, hash_b);
}

#[test]
fn build_hash_ignores_secret_identity_changes() {
    let workspace_id = "ws-hash-secret-identity";
    let profile = profile_for_workspace(workspace_id);
    let visible = sample_event("evt-visible", workspace_id, "visible event");
    let secret_a = secret_event(
        "evt-secret-alpha",
        workspace_id,
        "vault contains super-secret-alpha",
        "docs/restricted-alpha.md",
    );
    let secret_b = secret_event(
        "evt-secret-beta",
        workspace_id,
        "vault contains super-secret-beta",
        "docs/restricted-beta.md",
    );
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![visible.clone(), secret_a]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![visible, secret_b]),
    );
    assert_eq!(hash_a, hash_b);
}

#[test]
fn build_hash_contains_no_raw_secret_material() {
    let workspace_id = "ws-hash-no-secret-material";
    let profile = profile_for_workspace(workspace_id);
    let snapshot = snapshot_with_events(
        workspace_id,
        vec![
            sample_event("evt-visible", workspace_id, "visible event"),
            secret_event(
                "evt-secret",
                workspace_id,
                "vault contains super-secret-api-key-ROTATE-ME",
                "docs/restricted-note.md",
            ),
        ],
    );
    let pack = compose_from_parts(profile, snapshot, fixed_window(), GENERATED_AT);
    let forbidden = "super-secret-api-key-rotate-me";
    assert!(
        !pack
            .build_inputs_hash
            .to_ascii_lowercase()
            .contains(forbidden),
        "hash must not embed raw secret material"
    );
    assert!(
        !all_pack_continuity_summaries(&pack)
            .iter()
            .any(|summary| summary.to_ascii_lowercase().contains(forbidden)),
        "pack surfaces must remain sanitized"
    );
}

#[test]
fn build_hash_ignores_raw_superseded_presentation() {
    let workspace_id = "ws-hash-superseded";
    let profile = profile_for_workspace(workspace_id);
    let original_a = sample_event(
        "evt-target",
        workspace_id,
        "Original summary before correction",
    );
    let original_b = sample_event(
        "evt-target",
        workspace_id,
        "Different superseded raw presentation only",
    );
    let correction = correction_event(
        "evt-correct",
        workspace_id,
        "evt-target",
        "Corrected summary after review",
        "2026-07-17T02:00:00Z",
    );
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original_a, correction.clone()]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![original_b, correction]),
    );
    assert_eq!(hash_a, hash_b);
}

#[test]
fn build_hash_ignores_generated_timestamps() {
    let workspace_id = "ws-hash-generated-ts";
    let profile = profile_for_workspace(workspace_id);
    let snapshot = snapshot_with_events(
        workspace_id,
        vec![sample_event("evt-visible", workspace_id, "visible event")],
    );
    let earlier = compose_with_projection_timestamps(
        profile.clone(),
        snapshot.clone(),
        fixed_window(),
        "2026-07-17T01:00:00Z",
        "2026-07-17T02:00:00Z",
        "2026-07-18T01:00:00Z",
    );
    let later = compose_with_projection_timestamps(
        profile,
        snapshot,
        fixed_window(),
        "2026-07-18T06:00:00Z",
        "2026-07-18T07:00:00Z",
        "2026-07-18T08:00:00Z",
    );
    assert_eq!(earlier.build_inputs_hash, later.build_inputs_hash);
    assert_ne!(earlier.generated_at, later.generated_at);
}

#[test]
fn build_hash_is_stable_across_collection_order() {
    let workspace_id = "ws-hash-order-stable";
    let profile = profile_for_workspace(workspace_id);
    let event_a = sample_event("evt-a", workspace_id, "event a");
    let event_b = sample_event("evt-b", workspace_id, "event b");
    let hash_a = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![event_a.clone(), event_b.clone()]),
    );
    let hash_b = hash_for_parts(
        &profile,
        &snapshot_with_events(workspace_id, vec![event_b, event_a]),
    );
    assert_eq!(hash_a, hash_b);
}

#[test]
fn context_pack_id_tracks_safe_semantic_hash() {
    let workspace_id = "ws-pack-id-semantic";
    let profile = profile_for_workspace(workspace_id);
    let snapshot = snapshot_with_events(
        workspace_id,
        vec![sample_event("evt-visible", workspace_id, "visible event")],
    );
    let pack = compose_from_parts(profile, snapshot, fixed_window(), GENERATED_AT);
    assert_eq!(
        pack.context_pack_id,
        deterministic_context_pack_id(&pack.build_inputs_hash)
    );
    let duplicate = compose_from_parts(
        profile_for_workspace(workspace_id),
        snapshot_with_events(
            workspace_id,
            vec![sample_event("evt-visible", workspace_id, "visible event")],
        ),
        fixed_window(),
        "2026-07-18T09:00:00Z",
    );
    assert_eq!(pack.context_pack_id, duplicate.context_pack_id);
    assert_eq!(pack.build_inputs_hash, duplicate.build_inputs_hash);
}

#[test]
fn context_pack_id_ignores_secret_content_only_changes() {
    let workspace_id = "ws-pack-id-secret";
    let profile = profile_for_workspace(workspace_id);
    let visible = sample_event("evt-visible", workspace_id, "visible event");
    let pack_a = compose_from_parts(
        profile.clone(),
        snapshot_with_events(
            workspace_id,
            vec![
                visible.clone(),
                secret_event(
                    "evt-secret",
                    workspace_id,
                    "vault contains super-secret-alpha",
                    "docs/restricted-note.md",
                ),
            ],
        ),
        fixed_window(),
        GENERATED_AT,
    );
    let pack_b = compose_from_parts(
        profile,
        snapshot_with_events(
            workspace_id,
            vec![
                visible,
                secret_event(
                    "evt-secret",
                    workspace_id,
                    "vault contains super-secret-beta",
                    "docs/restricted-note.md",
                ),
            ],
        ),
        fixed_window(),
        "2026-07-18T09:00:00Z",
    );
    assert_eq!(pack_a.build_inputs_hash, pack_b.build_inputs_hash);
    assert_eq!(pack_a.context_pack_id, pack_b.context_pack_id);
}

#[test]
fn semantic_fingerprint_uses_sanitized_pack_inputs() {
    let workspace_id = "ws-fingerprint-sanitized";
    let profile = profile_for_workspace(workspace_id);
    let visible = sample_event("evt-visible", workspace_id, "visible event");
    let raw_snapshot = snapshot_with_events(
        workspace_id,
        vec![
            visible.clone(),
            secret_event(
                "evt-secret",
                workspace_id,
                "vault contains super-secret-alpha",
                "docs/restricted-note.md",
            ),
        ],
    );
    let pack = compose_from_parts(
        profile.clone(),
        raw_snapshot.clone(),
        fixed_window(),
        GENERATED_AT,
    );
    assert_eq!(
        pack.build_inputs_hash,
        hash_for_parts(&profile, &raw_snapshot)
    );

    let mut mutated = raw_snapshot;
    if let Some(secret) = mutated
        .work_events
        .iter_mut()
        .find(|event| event.event_id == "evt-secret")
    {
        secret.summary = "vault contains super-secret-beta".into();
        secret.evidence = vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/other-secret.md".into()),
            observed_at: None,
        }];
    }
    let hash_after_mutation = hash_for_parts(&profile, &mutated);
    assert_eq!(pack.build_inputs_hash, hash_after_mutation);

    let semantic_snapshot = snapshot_with_events(
        workspace_id,
        vec![
            visible,
            secret_event(
                "evt-secret",
                workspace_id,
                "vault contains super-secret-beta",
                "docs/other-secret.md",
            ),
        ],
    );
    let pack_semantic = compose_from_parts(
        profile.clone(),
        semantic_snapshot,
        fixed_window(),
        GENERATED_AT,
    );
    assert_eq!(pack.build_inputs_hash, pack_semantic.build_inputs_hash);
}

#[test]
fn builder_populates_objective_freshness_metadata() {
    let (_dir, workspace_id) = temp_project("freshness-metadata");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert_eq!(pack.freshness.pack_generated_at, GENERATED_AT);
    assert!(!pack.freshness.snapshot_observed_at.is_empty());
    assert!(!pack.freshness.current_state_generated_at.is_empty());
}

#[test]
fn freshness_age_is_derived_from_explicit_timestamps() {
    let (_dir, workspace_id) = temp_project("freshness-age");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let observed = chrono::DateTime::parse_from_rfc3339(&pack.freshness.snapshot_observed_at)
        .expect("observed timestamp");
    let generated =
        chrono::DateTime::parse_from_rfc3339(&pack.freshness.pack_generated_at).expect("generated");
    let expected = generated
        .signed_duration_since(observed)
        .num_seconds()
        .max(0) as u64;
    assert_eq!(pack.freshness.age_seconds, expected);
}

#[test]
fn freshness_has_no_strict_or_authority_decision() {
    let (_dir, workspace_id) = temp_project("freshness-non-strict");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let json = serde_json::to_string(&pack.freshness)
        .expect("serialize freshness")
        .to_ascii_lowercase();
    for forbidden in [
        "strictfreshness",
        "strict_freshness",
        "authoritydecision",
        "freshnessdecision",
        "rejectstale",
        "mustbefresh",
    ] {
        assert!(!json.contains(forbidden), "freshness leaked {forbidden}");
    }
}

#[test]
fn builder_merges_limitations_deterministically() {
    let (_dir, workspace_id) = temp_project("limitations-merge");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let first = build_pack(&project_path);
    let second = build_pack(&project_path);
    assert_eq!(first.limitations, second.limitations);
    let mut sorted = first.limitations.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(first.limitations, sorted);
    assert!(first
        .limitations
        .iter()
        .any(|entry| entry.contains("context pack metadata only")));
}

#[test]
fn builder_bounds_limitations_diagnostics_and_unresolved_items() {
    let (_dir, workspace_id) = temp_project("bounds");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert!(pack.limitations.len() <= MAX_CONTEXT_PACK_LIMITATIONS);
    assert!(pack.diagnostics.len() <= MAX_CONTEXT_PACK_DIAGNOSTICS);
    assert!(pack.unresolved_items.len() <= MAX_CONTEXT_PACK_UNRESOLVED_ITEMS);
}

#[test]
fn builder_populates_aggregate_redaction_summary() {
    let (_dir, workspace_id) = temp_project("redaction-summary");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    assert!(pack.redaction_summary.secret_items_omitted > 0);
    let json = serde_json::to_string(&pack.redaction_summary).expect("serialize");
    assert!(!json.contains("super-secret"));
}

#[test]
fn builder_emits_safe_non_identifying_diagnostics() {
    let (_dir, workspace_id) = temp_project("safe-diagnostics");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let pack = build_pack(&project_path);
    let json = serde_json::to_string(&pack.diagnostics).expect("serialize");
    assert!(!json.contains("super-secret-api-key"));
}

#[test]
fn builder_does_not_embed_context_document_annex() {
    let (_dir, workspace_id) = temp_project("no-annex");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let json = pack_json_lower(&build_pack(&project_path));
    for forbidden in [
        "contextdocument",
        "documentannex",
        "contextindex",
        "agentcontextenabled",
    ] {
        assert!(
            !json.contains(forbidden),
            "pack leaked annex field {forbidden}"
        );
    }
}

#[test]
fn successful_ephemeral_build_creates_no_projection_files() {
    let (_dir, workspace_id) = temp_project("success-no-projection");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let _ = build_pack(&project_path);
    assert!(!projections_dir(&project_path).exists());
}

#[test]
fn failed_ephemeral_build_creates_no_projection_files() {
    let (_dir, workspace_id) = temp_project("failed-no-projection");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let _ = build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(!projections_dir(&project_path).exists());
}

#[test]
fn builder_does_not_modify_existing_current_state_projection() {
    let (_dir, workspace_id) = temp_project("projection-immutable");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    rebuild_current_state_projection(&project_path).expect("rebuild");
    let before = fs::read(current_state_projection_path(&project_path)).unwrap();
    let _ = build_pack(&project_path);
    let after = fs::read(current_state_projection_path(&project_path)).unwrap();
    assert_eq!(before, after);
}

#[test]
fn builder_does_not_modify_profile_bytes_or_timestamp() {
    let (_dir, workspace_id) = temp_project("profile-immutable");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let path = work_proxy_profile_path(&project_path);
    let before_bytes = fs::read(&path).unwrap();
    let before_profile: WorkProxyProfile = serde_json::from_slice(&before_bytes).unwrap();
    let _ = build_pack(&project_path);
    let after_bytes = fs::read(&path).unwrap();
    let after_profile: WorkProxyProfile = serde_json::from_slice(&after_bytes).unwrap();
    assert_eq!(before_bytes, after_bytes);
    assert_eq!(
        before_profile.last_updated_at,
        after_profile.last_updated_at
    );
}

#[test]
fn builder_does_not_mutate_signal_inboxes() {
    let (_dir, workspace_id) = temp_project("signals-immutable");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let before = bucket_snapshot(&project_path);
    let _ = build_pack(&project_path);
    assert_eq!(before, bucket_snapshot(&project_path));
}

#[test]
fn builder_does_not_create_work_signals_or_events() {
    let (_dir, workspace_id) = temp_project("no-new-events");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let ledger_before = ledger_file_count(&project_path);
    let buckets_before = bucket_snapshot(&project_path);
    let _ = build_pack(&project_path);
    assert_eq!(ledger_before, ledger_file_count(&project_path));
    assert_eq!(buckets_before, bucket_snapshot(&project_path));
}

#[test]
fn builder_does_not_mutate_promotion_audit() {
    let (_dir, workspace_id) = temp_project("audit-immutable");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let key = PromotionKey::from_inputs(&workspace_id, &["sig-builder-seed".to_string()]).unwrap();
    let decision = PromotionDecision::defer(
        key.clone(),
        vec!["sig-builder-seed".into()],
        PromotionReasonCode::MissingEvidence,
    );
    let record = PromotionDecisionRecord::from_decision(
        workspace_id.clone(),
        decision,
        None,
        "2026-07-17T02:00:00Z".into(),
    );
    write_decision_record(&project_path, &record).expect("write audit");
    let before = promotion_audit_snapshot(&project_path);
    let _ = build_pack(&project_path);
    assert_eq!(before, promotion_audit_snapshot(&project_path));
}

#[test]
fn builder_creates_no_catch_up_persistence() {
    let (_dir, workspace_id) = temp_project("no-catch-up-persist");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let _ = build_pack(&project_path);
    let projections = projections_dir(&project_path);
    if projections.exists() {
        let names: Vec<_> = fs::read_dir(projections)
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(!names.iter().any(|name| name.contains("catch")));
    }
}

#[test]
fn builder_performs_no_network_or_remote_access() {
    let module =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack.rs"))
            .expect("read context_pack.rs");
    for forbidden in [
        "reqwest",
        "ureq",
        "hyper::",
        "TcpStream",
        "UdpSocket",
        "std::net",
        "tokio::net",
        "http::",
        "remote_access",
    ] {
        assert!(
            !module.contains(forbidden),
            "context_pack must not use network via {forbidden}"
        );
    }
}

#[test]
fn builder_contains_no_answer_generation_or_authority_execution() {
    let module =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack.rs"))
            .expect("read context_pack.rs");
    for forbidden in [
        "resolve_profile_authority",
        "generate_answer",
        "ask_my_proxy",
        "ask-my-proxy",
        "ProxyPolicyResult",
        "answer_text",
        "response_body",
    ] {
        assert!(!module.contains(forbidden), "forbidden {forbidden}");
    }
}

#[test]
fn checkpoint_c_does_not_invoke_profile_authority_resolution() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let module = fs::read_to_string(root.join("context_pack.rs")).expect("read context_pack.rs");
    assert!(!module.contains("resolve_profile_authority"));
}

#[test]
fn checkpoint_c_does_not_start_ask_my_proxy() {
    let module =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack.rs"))
            .expect("read context_pack.rs");
    let lowered = module.to_ascii_lowercase();
    assert!(!lowered.contains("ask-my-proxy"));
    assert!(!lowered.contains("ask my proxy"));
    assert!(!lowered.contains("askmyproxy"));
}

#[test]
fn checkpoint_c_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    if tauri_lib.exists() {
        let content = fs::read_to_string(&tauri_lib).expect("read tauri lib");
        assert!(!content.contains("context_pack"));
        assert!(!content.contains("ProxyContextPack"));
        assert_eq!(
            content.matches("#[tauri::command]").count(),
            52,
            "Tauri command count must remain 52"
        );
    }
}

#[test]
fn complete_snapshot_failure_is_builder_error() {
    let (_dir, workspace_id) = temp_project("snapshot-failure");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let ledger = ledger_dir(&project_path);
    fs::remove_dir_all(&ledger).unwrap();
    fs::write(&ledger, "not-a-directory").unwrap();
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(matches!(err, ContextPackBuildError::ContinuitySnapshot(_)));
}

#[test]
fn builder_maps_missing_profile_without_synthesis() {
    let (_dir, workspace_id) = temp_project("maps-missing-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert_eq!(err, ContextPackBuildError::ProfileMissing);
    assert!(!work_proxy_profile_path(&project_path).exists());
}

#[test]
fn builder_maps_invalid_profile_safely() {
    let (_dir, workspace_id) = temp_project("maps-invalid-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(work_proxy_profile_path(&project_path), "{not-valid-json").unwrap();
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(matches!(err, ContextPackBuildError::Profile(_)));
    let text = err.to_string().to_ascii_lowercase();
    assert!(!text.contains("password="));
    assert!(!text.contains("api_key="));
}

#[test]
fn builder_maps_workspace_mismatch_safely() {
    let (_dir, workspace_id) = temp_project("maps-workspace-mismatch");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let mut profile = write_valid_profile(&project_path, &workspace_id);
    profile.workspace_id = "ws-other".into();
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(matches!(err, ContextPackBuildError::Profile(_)));
    let text = err.to_string().to_ascii_lowercase();
    assert!(!text.contains("password="));
    assert!(!text.contains("api_key="));
}

#[test]
fn builder_maps_selection_failure_without_sensitive_details() {
    let workspace_id = "ws-selection-failure-safe";
    let profile = profile_for_workspace(workspace_id);
    let snapshot = snapshot_with_events(
        workspace_id,
        vec![sample_event(
            "evt-visible",
            workspace_id,
            "vault contains api_key=super-secret-token-value",
        )],
    );
    let mut current_state = build_current_state_projection(&snapshot).expect("current state");
    for item in current_state
        .sections
        .in_progress
        .iter_mut()
        .chain(current_state.sections.completed.iter_mut())
    {
        item.timestamp = "not-a-valid-timestamp".into();
    }
    let catch_up =
        build_catch_up_view(&snapshot, &current_state, &fixed_window()).expect("catch-up");
    let err = compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window: fixed_window(),
            generated_at: GENERATED_AT.into(),
        },
        &default_options(),
    )
    .unwrap_err();
    assert!(matches!(err, ContextPackBuildError::Selection(_)));
    let text = err.to_string().to_ascii_lowercase();
    assert!(!text.contains("super-secret-token-value"));
    assert!(!text.contains("api_key=super-secret-token-value"));
}

#[test]
fn builder_runs_complete_validation_before_return() {
    let module =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/context_pack.rs"))
            .expect("read context_pack.rs");
    assert!(module.contains("validate_proxy_context_pack_complete"));
    let compose_start = module
        .find("pub fn compose_proxy_context_pack")
        .expect("compose function");
    let compose_body = &module[compose_start..];
    let validate_pos = compose_body
        .find("validate_proxy_context_pack_complete")
        .expect("compose must call complete validation");
    let return_pos = compose_body
        .find("Ok(pack)")
        .expect("compose must return pack");
    assert!(
        validate_pos < return_pos,
        "compose must validate before returning"
    );
}

#[test]
fn builder_returns_validation_failure_for_invalid_finished_pack() {
    let (_dir, workspace_id) = temp_project("validation-failure");
    let project_path = _dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let mut profile = write_valid_profile(&project_path, &workspace_id);
    profile.authority_rules[0].description = Some("generated answer for the owner".into());
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    let err =
        build_proxy_context_pack(&project_path, fixed_window(), default_options()).unwrap_err();
    assert!(matches!(err, ContextPackBuildError::PackValidation(_)));
    assert_eq!(
        err.to_string(),
        "pack validation failed: forbidden_runtime_surface"
    );
}
