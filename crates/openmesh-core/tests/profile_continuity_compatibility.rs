//! Dev Track 0.1.4 Checkpoint F — core profile / continuity orthogonality proofs.

use openmesh_core::context::Sensitivity;
use openmesh_core::continuity::{
    build_catch_up_view, load_continuity_input_snapshot, rebuild_current_state_projection,
};
use openmesh_core::domain::{
    default_work_proxy_profile, validate_work_proxy_profile, ActorRef, CatchUpWindow,
    CurrentStateSections, EvidenceAttachment, EvidenceRef, ProducerRef, WorkEvent,
    WorkProxyProfile, WorkSignal, WorkSignalKind, WORK_PROXY_PROFILE_VERSION,
};
use openmesh_core::events::append_event;
use openmesh_core::profile::{
    profile_dir, read_work_proxy_profile, work_proxy_profile_path, write_work_proxy_profile,
    ProfileError,
};
use openmesh_core::signals::write_signal;
use openmesh_core::storage::init_project;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const EVENT_TS: &str = "2026-07-17T01:00:00Z";
const CATCH_UP_SINCE: &str = "2026-07-15T00:00:00Z";
const CATCH_UP_UNTIL: &str = "2026-07-18T00:00:00Z";

fn temp_project(label: &str) -> (PathBuf, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-core-profile-continuity-{label}-{}-{n}",
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

fn sample_event(event_id: &str, workspace_id: &str) -> WorkEvent {
    WorkEvent::new(
        event_id,
        workspace_id,
        "work.completed",
        "Compatibility seed event",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        EVENT_TS,
    )
}

fn sample_signal(signal_id: &str, workspace_id: &str) -> WorkSignal {
    WorkSignal {
        signal_id: signal_id.into(),
        workspace_id: workspace_id.into(),
        producer: ProducerRef::Reporter("compat-test".into()),
        actor: ActorRef::Unknown,
        kind: WorkSignalKind::Progress,
        summary: format!("signal summary for {signal_id}"),
        timestamp: EVENT_TS.into(),
        evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
        correlation_hint: None,
        sensitivity: Sensitivity::Private,
        protocol_version: "1.0".into(),
    }
}

fn seed_minimal_continuity(project_path: &str, workspace_id: &str) {
    append_event(project_path, &sample_event("evt-compat-seed", workspace_id)).unwrap();
    write_signal(
        project_path,
        &sample_signal("sig-compat-seed", workspace_id),
    )
    .unwrap();
}

fn current_state_sections(project_path: &str) -> CurrentStateSections {
    rebuild_current_state_projection(project_path)
        .expect("rebuild current state")
        .sections
}

fn catch_up_sections(project_path: &str) -> openmesh_core::domain::CatchUpSections {
    let snapshot = load_continuity_input_snapshot(project_path).expect("snapshot");
    let state = rebuild_current_state_projection(project_path).expect("state");
    let window = CatchUpWindow {
        since: CATCH_UP_SINCE.into(),
        until: CATCH_UP_UNTIL.into(),
    };
    build_catch_up_view(&snapshot, &state, &window)
        .expect("catch-up")
        .sections
}

fn write_valid_profile(project_path: &str, workspace_id: &str) -> WorkProxyProfile {
    let profile = default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Compat Owner",
        "Compat Role",
        "2026-07-17T08:00:00Z",
    );
    write_work_proxy_profile(project_path, &profile).expect("write profile");
    profile
}

#[test]
fn current_state_builds_without_profile() {
    let (dir, workspace_id) = temp_project("state-no-profile");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    assert!(!work_proxy_profile_path(&project_path).exists());
    let projection = rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(
        !projection.sections.completed.is_empty()
            || !projection.sections.in_progress.is_empty()
            || projection.source_counts.work_events >= 1
    );
}

#[test]
fn catch_up_builds_without_profile() {
    let (dir, workspace_id) = temp_project("catchup-no-profile");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let sections = catch_up_sections(&project_path);
    assert!(sections.changed.len() >= 1 || sections.completed.len() >= 1);
}

#[test]
fn current_state_builds_with_valid_profile() {
    let (dir, workspace_id) = temp_project("state-with-profile");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let projection = rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(projection.source_counts.work_events >= 1);
}

#[test]
fn catch_up_builds_with_valid_profile() {
    let (dir, workspace_id) = temp_project("catchup-with-profile");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    write_valid_profile(&project_path, &workspace_id);
    let sections = catch_up_sections(&project_path);
    assert!(sections.changed.len() >= 1 || sections.completed.len() >= 1);
}

#[test]
fn profile_presence_does_not_change_current_state_semantics() {
    let (dir, workspace_id) = temp_project("state-semantic");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let before = current_state_sections(&project_path);
    write_valid_profile(&project_path, &workspace_id);
    let after = current_state_sections(&project_path);
    assert_eq!(before, after);
}

#[test]
fn profile_presence_does_not_change_catch_up_semantics() {
    let (dir, workspace_id) = temp_project("catchup-semantic");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let before = catch_up_sections(&project_path);
    write_valid_profile(&project_path, &workspace_id);
    let after = catch_up_sections(&project_path);
    assert_eq!(before, after);
}

#[test]
fn malformed_profile_does_not_block_current_state() {
    let (dir, workspace_id) = temp_project("malformed-state");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(work_proxy_profile_path(&project_path), "{not-valid-json").unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::MalformedJson(_))
    ));
    let projection = rebuild_current_state_projection(&project_path).expect("rebuild");
    assert!(projection.source_counts.work_events >= 1);
}

#[test]
fn malformed_profile_does_not_block_catch_up() {
    let (dir, workspace_id) = temp_project("malformed-catchup");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(work_proxy_profile_path(&project_path), "{bad").unwrap();
    let sections = catch_up_sections(&project_path);
    assert!(sections.changed.len() >= 1 || sections.completed.len() >= 1);
}

#[test]
fn profile_workspace_mismatch_is_isolated_from_continuity() {
    let (dir, workspace_id) = temp_project("workspace-mismatch");
    let project_path = dir.to_string_lossy().to_string();
    seed_minimal_continuity(&project_path, &workspace_id);
    let mut profile = write_valid_profile(&project_path, &workspace_id);
    profile.workspace_id = "ws-other".into();
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::WorkspaceMismatch { .. })
    ));
    let projection = rebuild_current_state_projection(&project_path).expect("rebuild");
    assert_eq!(projection.workspace_id, workspace_id);
}

#[test]
fn profile_exposes_stable_future_context_metadata_without_building_context_pack() {
    let profile = default_work_proxy_profile(
        "ws-future",
        "profile-ws-future",
        "Owner",
        "Role",
        "2026-07-17T08:00:00Z",
    );
    validate_work_proxy_profile(&profile).expect("valid profile");
    assert_eq!(profile.profile_version, WORK_PROXY_PROFILE_VERSION);
    assert!(!profile.owner_label.is_empty());
    assert!(!profile.role_label.is_empty());
    assert!(!profile.authority_rules.is_empty());
    assert!(!profile.privacy_rules.is_empty());
    assert!(!profile.default_refusal_rules.is_empty());
    assert!(profile.evidence_policy.require_evidence_for_claims);
    assert!(!profile.limitations.is_empty());

    let serialized = serde_json::to_string(&profile).expect("serialize");
    let lowered = serialized.to_ascii_lowercase();
    for forbidden in [
        "contextpack",
        "proxycontextpack",
        "askmyproxy",
        "answer_text",
        "response_body",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "profile metadata must not contain {forbidden}"
        );
    }
}

#[test]
fn checkpoint_f_does_not_start_0_1_5_or_0_1_6() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for rel in ["profile.rs", "profile_validation.rs"] {
        let content = fs::read_to_string(root.join(rel)).expect("read source");
        let lowered = content.to_ascii_lowercase();
        for forbidden in [
            "proxycontextpack",
            "contextpack",
            "askmyproxy",
            "0.1.5",
            "0.1.6",
        ] {
            assert!(
                !lowered.contains(forbidden),
                "{rel} must not start {forbidden}"
            );
        }
    }
}
