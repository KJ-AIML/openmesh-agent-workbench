//! Dev Track 0.1.5 Checkpoint G — Context Pack / continuity compatibility proofs.

use openmesh_core::context::Sensitivity;
use openmesh_core::context_pack::{
    build_proxy_context_pack, ContextPackBuildError, ProxyContextPackBuildOptions,
};
use openmesh_core::context_pack_storage::{
    proxy_context_pack_path, write_proxy_context_pack, ContextPackStorageError,
};
use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, current_state_projection_path,
    load_continuity_input_snapshot, projections_dir,
};
use openmesh_core::domain::{
    default_work_proxy_profile, validate_proxy_context_pack, CatchUpWindow,
    ContextPackItemProvenance, EvidenceAttachment, EvidenceRef, WorkEvent,
};
use openmesh_core::domain::{ActorRef, ProducerRef, WorkSignal, WorkSignalKind};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::{
    read_work_proxy_profile, work_proxy_profile_path, write_work_proxy_profile,
};
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::signals::write_signal;
use openmesh_core::storage::{get_project_dir, init_project};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const EVENT_TS: &str = "2026-07-17T01:00:00Z";
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";
const WINDOW_UNTIL: &str = "2026-07-18T00:00:00Z";
const GENERATED_AT: &str = "2026-07-18T04:00:00Z";

fn fixed_window() -> CatchUpWindow {
    CatchUpWindow {
        since: WINDOW_SINCE.into(),
        until: WINDOW_UNTIL.into(),
    }
}

fn build_options() -> ProxyContextPackBuildOptions {
    ProxyContextPackBuildOptions {
        generated_at: GENERATED_AT.into(),
        selection: Default::default(),
    }
}

fn temp_project(label: &str) -> (PathBuf, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-core-context-compat-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let project_path = dir.to_string_lossy().to_string();
    init_project(&project_path).expect("init");
    let workspace_id = fs::read_to_string(dir.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    (dir, workspace_id)
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
) -> WorkEvent {
    let mut event = sample_event(event_id, workspace_id, summary);
    event.corrects_event_id = Some(target_id.into());
    event.timestamp = "2026-07-17T02:00:00Z".into();
    event
}

fn sample_signal(signal_id: &str, workspace_id: &str, sensitivity: Sensitivity) -> WorkSignal {
    WorkSignal {
        signal_id: signal_id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("compat-core".into()),
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

fn write_profile(project_path: &str, workspace_id: &str) {
    let profile = default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Compat Owner",
        "Compat Role",
        "2026-07-17T08:00:00Z",
    );
    write_work_proxy_profile(project_path, &profile).expect("profile");
}

fn seed_rich_continuity(project_path: &str, workspace_id: &str) {
    append_event(
        project_path,
        &sample_event("evt-visible", workspace_id, "Visible continuity event"),
    )
    .unwrap();
    let mut secret = sample_event(
        "evt-secret-compat",
        workspace_id,
        "vault contains super-secret-compat-token",
    );
    secret.sensitivity = Sensitivity::Secret;
    secret.evidence = vec![EvidenceAttachment {
        evidence_ref: EvidenceRef::FilePath("docs/restricted-secret.md".into()),
        observed_at: None,
    }];
    append_event(project_path, &secret).unwrap();
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
        ),
    )
    .unwrap();
    write_signal(
        project_path,
        &sample_signal("sig-pending", workspace_id, Sensitivity::Private),
    )
    .unwrap();
}

fn collect_strings(value: &Value, out: &mut BTreeSet<String>) {
    match value {
        Value::String(s) => {
            out.insert(s.clone());
        }
        Value::Array(items) => {
            for item in items {
                collect_strings(item, out);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                collect_strings(child, out);
            }
        }
        _ => {}
    }
}

const SECRET_MARKERS: &[&str] = &[
    "evt-secret-compat",
    "super-secret-compat-token",
    "restricted-secret.md",
    "vault contains",
];

#[test]
fn context_pack_builds_with_valid_profile_and_continuity() {
    let (_dir, workspace_id) = temp_project("builds-with-profile");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack =
        build_proxy_context_pack(&project_path, fixed_window(), build_options()).expect("build");
    validate_proxy_context_pack(&pack).expect("validates");
    assert_eq!(pack.workspace_id, workspace_id);
}

#[test]
fn context_pack_build_fails_closed_without_profile() {
    let (_dir, workspace_id) = temp_project("no-profile");
    let project_path = _dir.to_string_lossy().to_string();
    seed_rich_continuity(&project_path, &workspace_id);
    let err = build_proxy_context_pack(&project_path, fixed_window(), build_options())
        .expect_err("missing profile");
    assert!(matches!(err, ContextPackBuildError::ProfileMissing));
}

#[test]
fn context_pack_build_does_not_create_or_update_profile() {
    let (_dir, workspace_id) = temp_project("no-profile-write");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let before = fs::read(work_proxy_profile_path(&project_path)).unwrap();
    build_proxy_context_pack(&project_path, fixed_window(), build_options()).expect("build");
    let after = fs::read(work_proxy_profile_path(&project_path)).unwrap();
    assert_eq!(before, after);
}

#[test]
fn profile_update_changes_future_context_hash_without_context_mutating_profile() {
    let (_dir, workspace_id) = temp_project("profile-hash");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let before_hash = build_proxy_context_pack(&project_path, fixed_window(), build_options())
        .expect("build")
        .build_inputs_hash;

    let mut profile = read_work_proxy_profile(&project_path).expect("read");
    profile.working_style = "async-first boundary update".into();
    write_work_proxy_profile(&project_path, &profile).expect("update");

    let after_hash = build_proxy_context_pack(&project_path, fixed_window(), build_options())
        .expect("rebuild")
        .build_inputs_hash;
    assert_ne!(before_hash, after_hash);

    let profile_after_build = read_work_proxy_profile(&project_path).expect("profile");
    assert_eq!(
        profile_after_build.working_style,
        "async-first boundary update"
    );
}

#[test]
fn fixed_window_repeated_builds_have_same_safe_hash() {
    let (_dir, workspace_id) = temp_project("repeat-hash");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let a = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let b = build_proxy_context_pack(
        &project_path,
        fixed_window(),
        ProxyContextPackBuildOptions {
            generated_at: "2026-07-18T05:00:00Z".into(),
            selection: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(a.build_inputs_hash, b.build_inputs_hash);
}

#[test]
fn fixed_window_repeated_builds_have_same_context_pack_id() {
    let (_dir, workspace_id) = temp_project("repeat-id");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let a = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let b = build_proxy_context_pack(
        &project_path,
        fixed_window(),
        ProxyContextPackBuildOptions {
            generated_at: "2026-07-18T06:00:00Z".into(),
            selection: Default::default(),
        },
    )
    .unwrap();
    assert_eq!(a.context_pack_id, b.context_pack_id);
    assert_ne!(a.generated_at, b.generated_at);
}

#[test]
fn secret_evidence_is_absent_from_all_pack_surfaces() {
    let (_dir, workspace_id) = temp_project("secret-absent");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let serialized = serde_json::to_string(&pack).expect("serialize");
    for marker in SECRET_MARKERS {
        assert!(
            !serialized.contains(marker),
            "secret marker leaked: {marker}"
        );
    }
    let value = serde_json::to_value(&pack).unwrap();
    let mut strings = BTreeSet::new();
    collect_strings(&value, &mut strings);
    for marker in SECRET_MARKERS {
        assert!(
            strings.iter().all(|s| !s.contains(marker)),
            "secret marker in string field: {marker}"
        );
    }
}

#[test]
fn secret_evidence_appears_only_as_aggregate_redaction_count() {
    let (_dir, workspace_id) = temp_project("secret-redaction");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    assert!(pack.redaction_summary.secret_items_omitted >= 1);
    assert!(pack
        .evidence_index
        .iter()
        .all(|entry| entry.sensitivity != Sensitivity::Secret));
}

#[test]
fn pending_evidence_remains_pending_or_unconfirmed() {
    let (_dir, workspace_id) = temp_project("pending");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    assert!(pack
        .current_state
        .pending_attention
        .iter()
        .chain(pack.catch_up.next_suggested_attention.iter())
        .any(|item| item.provenance == ContextPackItemProvenance::Pending));
}

#[test]
fn corrected_effective_presentation_is_used() {
    let (_dir, workspace_id) = temp_project("correction-effective");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let serialized = serde_json::to_string(&pack).unwrap();
    assert!(serialized.contains("Corrected summary after review"));
}

#[test]
fn superseded_raw_presentation_is_absent() {
    let (_dir, workspace_id) = temp_project("correction-absent");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let serialized = serde_json::to_string(&pack).unwrap();
    assert!(!serialized.contains("Original summary before correction"));
}

#[test]
fn failed_context_write_preserves_previous_valid_pack() {
    let (_dir, workspace_id) = temp_project("failed-write");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let before = fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap();
    let mut invalid = pack.clone();
    invalid.build_inputs_hash = "invalid-hash".into();
    let err = write_proxy_context_pack(&project_path, &invalid).expect_err("reject");
    assert!(matches!(
        err,
        ContextPackStorageError::ValidationFailed { .. }
    ));
    assert_eq!(
        fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap(),
        before
    );
}

#[test]
fn context_build_does_not_modify_continuity_files_via_core_api() {
    let (_dir, workspace_id) = temp_project("continuity-unchanged");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let ledger_before = fs::read_dir(ledger_dir(&project_path))
        .map(|e| e.count())
        .unwrap_or(0);
    let signals_before = fs::read_dir(get_project_dir(&project_path).join("signals/pending"))
        .map(|e| e.count())
        .unwrap_or(0);
    build_proxy_context_pack(&project_path, fixed_window(), build_options()).expect("build");
    assert_eq!(
        fs::read_dir(ledger_dir(&project_path))
            .map(|e| e.count())
            .unwrap_or(0),
        ledger_before
    );
    assert_eq!(
        fs::read_dir(get_project_dir(&project_path).join("signals/pending"))
            .map(|e| e.count())
            .unwrap_or(0),
        signals_before
    );
    assert!(!current_state_projection_path(&project_path).exists());
    assert!(!projections_dir(&project_path).exists());
    assert!(!promotion_decisions_dir(&project_path).exists());
}

#[test]
fn context_pack_and_continuity_views_coexist_without_semantic_coupling() {
    let (_dir, workspace_id) = temp_project("coexist");
    let project_path = _dir.to_string_lossy().to_string();
    write_profile(&project_path, &workspace_id);
    seed_rich_continuity(&project_path, &workspace_id);
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    let state_before = build_current_state_projection(&snapshot).expect("state");
    let catch_up_before =
        build_catch_up_view(&snapshot, &state_before, &fixed_window()).expect("catch-up");
    let pack = build_proxy_context_pack(&project_path, fixed_window(), build_options()).unwrap();
    let snapshot_after = load_continuity_input_snapshot(&project_path).expect("snapshot after");
    let state_after = build_current_state_projection(&snapshot_after).expect("state after");
    let catch_up_after = build_catch_up_view(&snapshot_after, &state_after, &fixed_window())
        .expect("catch-up after");
    assert_eq!(
        serde_json::to_value(&state_before).unwrap(),
        serde_json::to_value(&state_after).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&catch_up_before).unwrap(),
        serde_json::to_value(&catch_up_after).unwrap()
    );
    validate_proxy_context_pack(&pack).expect("pack valid");
}
