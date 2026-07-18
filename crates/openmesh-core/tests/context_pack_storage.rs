//! Dev Track 0.1.5 Checkpoint E — Proxy Context Pack storage tests.

use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::context_pack_storage::{
    context_pack_exists, context_pack_projections_dir, proxy_context_pack_path,
    read_proxy_context_pack, read_proxy_context_pack_file, write_proxy_context_pack,
    ContextPackStorageError, PROXY_CONTEXT_PACK_FILENAME,
};
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{
    default_work_proxy_profile, CatchUpWindow, ProxyContextPack,
    PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::{work_proxy_profile_path, write_work_proxy_profile};
use openmesh_core::promotion::promotion_decisions_dir;
use openmesh_core::storage::{get_project_dir, init_project};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

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

fn temp_project(label: &str) -> (PathBuf, String, String) {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-context-pack-storage-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    let project_path = dir.to_string_lossy().to_string();
    let project_id = fs::read_to_string(dir.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("project id");
    (dir, project_path, project_id)
}

fn seed_profile_and_event(project_path: &str, workspace_id: &str) {
    let profile = default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Storage Owner",
        "Storage Role",
        "2026-07-17T08:00:00Z",
    );
    write_work_proxy_profile(project_path, &profile).expect("profile");
    let event = openmesh_core::domain::WorkEvent::new(
        "evt-storage-seed",
        workspace_id,
        "work.completed",
        "storage seed",
        vec![openmesh_core::domain::EvidenceAttachment {
            evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    );
    append_event(project_path, &event).expect("event");
}

fn build_valid_pack(project_path: &str) -> ProxyContextPack {
    build_proxy_context_pack(project_path, fixed_window(), build_options()).expect("build")
}

fn build_prepared_pack(project_path: &str, workspace_id: &str) -> ProxyContextPack {
    seed_profile_and_event(project_path, workspace_id);
    build_valid_pack(project_path)
}

fn project_id_for(project_path: &str) -> String {
    fs::read_to_string(Path::new(project_path).join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("project id")
}

fn projection_entries(project_path: &str) -> Vec<String> {
    let dir = context_pack_projections_dir(project_path);
    if !dir.exists() {
        return Vec::new();
    }
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

#[test]
fn context_pack_path_is_canonical() {
    let (_dir, project_path, _id) = temp_project("canonical-path");
    let path = proxy_context_pack_path(&project_path);
    assert!(path.ends_with(format!(
        ".openmesh/projections/{PROXY_CONTEXT_PACK_FILENAME}"
    )));
}

#[test]
fn context_pack_exists_is_false_when_missing() {
    let (_dir, project_path, _id) = temp_project("exists-false");
    assert!(!context_pack_exists(&project_path));
}

#[test]
fn read_missing_pack_returns_explicit_not_found() {
    let (_dir, project_path, _id) = temp_project("read-missing");
    let err = read_proxy_context_pack(&project_path).expect_err("missing");
    assert_eq!(err, ContextPackStorageError::PackNotFound);
}

#[test]
fn read_missing_pack_creates_no_directories() {
    let (_dir, project_path, _id) = temp_project("read-no-dir");
    let _ = read_proxy_context_pack(&project_path);
    assert!(!context_pack_projections_dir(&project_path).exists());
}

#[test]
fn write_then_read_pack_round_trips() {
    let (_dir, project_path, workspace_id) = temp_project("round-trip");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let loaded = read_proxy_context_pack(&project_path).expect("read");
    assert_eq!(loaded, pack);
}

#[test]
fn written_pack_uses_pretty_deterministic_json() {
    let (_dir, project_path, workspace_id) = temp_project("pretty-json");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let raw = fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap();
    assert!(raw.contains('\n'));
    assert!(raw.contains("  \"contextPackId\""));
    let reparsed: ProxyContextPack = serde_json::from_str(&raw).unwrap();
    assert_eq!(reparsed, pack);
}

#[test]
fn written_pack_has_trailing_newline() {
    let (_dir, project_path, workspace_id) = temp_project("trailing-newline");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let raw = fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap();
    assert!(raw.ends_with('\n'));
    assert!(!raw.ends_with("\n\n"));
}

#[test]
fn write_creates_projection_directory_only_on_success() {
    let (_dir, project_path, workspace_id) = temp_project("dir-on-success");
    assert!(!context_pack_projections_dir(&project_path).exists());
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    assert!(context_pack_projections_dir(&project_path).is_dir());
}

#[test]
fn write_validates_before_filesystem_mutation() {
    let (_dir, project_path, workspace_id) = temp_project("validate-before-write");
    let mut pack = build_prepared_pack(&project_path, &workspace_id);
    pack.build_inputs_hash = "not-a-valid-hash".into();
    let err = write_proxy_context_pack(&project_path, &pack).expect_err("invalid");
    assert!(matches!(
        err,
        ContextPackStorageError::ValidationFailed { .. }
    ));
    assert!(!context_pack_projections_dir(&project_path).exists());
}

#[test]
fn write_rejects_workspace_mismatch() {
    let (_dir, project_path, workspace_id) = temp_project("write-ws-mismatch");
    let mut pack = build_prepared_pack(&project_path, &workspace_id);
    pack.workspace_id = "other-workspace".into();
    let err = write_proxy_context_pack(&project_path, &pack).expect_err("mismatch");
    assert_eq!(err, ContextPackStorageError::WorkspaceMismatch);
}

#[test]
fn read_rejects_workspace_mismatch() {
    let (_dir_a, path_a, workspace_a) = temp_project("read-ws-a");
    let (_dir_b, path_b, _) = temp_project("read-ws-b");
    let pack = build_prepared_pack(&path_a, &workspace_a);
    write_proxy_context_pack(&path_a, &pack).expect("write a");
    fs::create_dir_all(context_pack_projections_dir(&path_b)).unwrap();
    fs::copy(
        proxy_context_pack_path(&path_a),
        proxy_context_pack_path(&path_b),
    )
    .unwrap();
    let err = read_proxy_context_pack(&path_b).expect_err("mismatch");
    assert_eq!(err, ContextPackStorageError::WorkspaceMismatch);
}

#[test]
fn read_rejects_malformed_json_without_echoing_content() {
    let (_dir, project_path, _id) = temp_project("malformed-json");
    fs::create_dir_all(context_pack_projections_dir(&project_path)).unwrap();
    let secret = "super-secret-token-ABC123";
    fs::write(
        proxy_context_pack_path(&project_path),
        format!("{{not-json:{secret}}}"),
    )
    .unwrap();
    let err = read_proxy_context_pack(&project_path).expect_err("malformed");
    assert_eq!(err, ContextPackStorageError::MalformedJson);
    let message = err.to_string();
    assert!(!message.contains(secret));
    assert!(!message.contains("not-json"));
}

#[test]
fn read_runs_complete_validation() {
    let (_dir, project_path, workspace_id) = temp_project("complete-validation");
    let mut pack = build_prepared_pack(&project_path, &workspace_id);
    pack.build_inputs_hash = "deadbeef".into();
    fs::create_dir_all(context_pack_projections_dir(&project_path)).unwrap();
    fs::write(
        proxy_context_pack_path(&project_path),
        serde_json::to_string_pretty(&pack).unwrap() + "\n",
    )
    .unwrap();
    let err = read_proxy_context_pack(&project_path).expect_err("invalid hash");
    assert!(matches!(
        err,
        ContextPackStorageError::ValidationFailed { .. }
    ));
}

#[test]
fn overwrite_replaces_existing_pack_atomically() {
    let (_dir, project_path, workspace_id) = temp_project("atomic-overwrite");
    let pack_a = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack_a).expect("write a");
    let mut options_b = build_options();
    options_b.generated_at = "2026-07-18T05:00:00Z".into();
    let pack_b =
        build_proxy_context_pack(&project_path, fixed_window(), options_b).expect("build b");
    write_proxy_context_pack(&project_path, &pack_b).expect("write b");
    let loaded = read_proxy_context_pack(&project_path).expect("read");
    assert_eq!(loaded.generated_at, "2026-07-18T05:00:00Z");
}

#[test]
fn failed_overwrite_preserves_previous_valid_pack() {
    let (_dir, project_path, workspace_id) = temp_project("failed-overwrite");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let before = fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap();
    let mut invalid = pack.clone();
    invalid.build_inputs_hash = "invalid".into();
    let err = write_proxy_context_pack(&project_path, &invalid).expect_err("reject");
    assert!(matches!(
        err,
        ContextPackStorageError::ValidationFailed { .. }
    ));
    let after = fs::read_to_string(proxy_context_pack_path(&project_path)).unwrap();
    assert_eq!(before, after);
}

#[test]
fn successful_write_leaves_no_temp_files() {
    let (_dir, project_path, workspace_id) = temp_project("no-temp");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let entries = projection_entries(&project_path);
    assert_eq!(entries, vec![PROXY_CONTEXT_PACK_FILENAME.to_string()]);
}

#[test]
fn packs_are_isolated_between_projects() {
    let (_dir_a, path_a, workspace_a) = temp_project("iso-a");
    let (_dir_b, path_b, _) = temp_project("iso-b");
    let pack_a = build_prepared_pack(&path_a, &workspace_a);
    write_proxy_context_pack(&path_a, &pack_a).expect("write a");
    assert!(context_pack_exists(&path_a));
    assert!(!context_pack_exists(&path_b));
}

#[test]
fn read_proxy_context_pack_file_validates_without_project() {
    let (_dir, project_path, workspace_id) = temp_project("explicit-file");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    write_proxy_context_pack(&project_path, &pack).expect("write");
    let loaded =
        read_proxy_context_pack_file(&proxy_context_pack_path(&project_path)).expect("file read");
    assert_eq!(loaded, pack);
}

#[test]
fn write_does_not_touch_profile_signals_events_or_current_state() {
    let (_dir, project_path, workspace_id) = temp_project("side-effects");
    let root = Path::new(&project_path);
    seed_profile_and_event(&project_path, &workspace_id);
    let profile_before = fs::read_to_string(work_proxy_profile_path(&project_path)).unwrap();
    let signals_root = get_project_dir(&project_path).join("signals");
    let signals_before = signals_root
        .join("pending")
        .exists()
        .then(|| {
            fs::read_dir(signals_root.join("pending"))
                .map(|e| e.count())
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let events_before = fs::read_dir(ledger_dir(&project_path))
        .map(|e| e.count())
        .unwrap_or(0);
    let promotion_before = fs::read_dir(promotion_decisions_dir(&project_path))
        .map(|e| e.count())
        .unwrap_or(0);

    let pack = build_valid_pack(&project_path);
    write_proxy_context_pack(&project_path, &pack).expect("write");

    assert_eq!(
        fs::read_to_string(work_proxy_profile_path(&project_path)).unwrap(),
        profile_before
    );
    assert_eq!(
        signals_root
            .join("pending")
            .exists()
            .then(|| fs::read_dir(signals_root.join("pending"))
                .map(|e| e.count())
                .unwrap_or(0))
            .unwrap_or(0),
        signals_before
    );
    assert_eq!(
        fs::read_dir(ledger_dir(&project_path))
            .map(|e| e.count())
            .unwrap_or(0),
        events_before
    );
    assert_eq!(
        fs::read_dir(promotion_decisions_dir(&project_path))
            .map(|e| e.count())
            .unwrap_or(0),
        promotion_before
    );
    assert!(!current_state_projection_path(&project_path).exists());
    assert!(!root.join(".openmesh/catch-up").exists());
    assert_eq!(workspace_id, project_id_for(&project_path));
    assert!(proxy_context_pack_path(&project_path).exists());
    assert_eq!(
        get_project_dir(&project_path).join("projections").exists(),
        true
    );
}

#[test]
fn read_rejects_unsupported_protocol_version() {
    let (_dir, project_path, workspace_id) = temp_project("unsupported-protocol");
    let mut pack = build_prepared_pack(&project_path, &workspace_id);
    pack.protocol_version = "99.0".into();
    fs::create_dir_all(context_pack_projections_dir(&project_path)).unwrap();
    fs::write(
        proxy_context_pack_path(&project_path),
        serde_json::to_string_pretty(&pack).unwrap() + "\n",
    )
    .unwrap();
    let err = read_proxy_context_pack(&project_path).expect_err("unsupported");
    assert_eq!(err, ContextPackStorageError::UnsupportedProtocolVersion);
    assert!(!err.to_string().contains(&pack.context_pack_id));
}

#[test]
fn serialized_pack_preserves_protocol_constant() {
    let (_dir, project_path, workspace_id) = temp_project("protocol-constant");
    let pack = build_prepared_pack(&project_path, &workspace_id);
    assert_eq!(pack.protocol_version, PROXY_CONTEXT_PACK_PROTOCOL_VERSION);
}
