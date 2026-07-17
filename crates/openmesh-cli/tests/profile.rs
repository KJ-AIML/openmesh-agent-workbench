//! Dev Track 0.1.4 Checkpoint D — profile CLI command tests.

use openmesh_core::domain::{
    default_work_proxy_profile, validate_work_proxy_profile, ProxyAuthorityLevel,
    UnsupportedClaimBehavior, WorkProxyProfile, WORK_PROXY_PROFILE_VERSION,
};
use openmesh_core::profile::{profile_dir, read_work_proxy_profile, work_proxy_profile_path};
use openmesh_core::profile_validation::validate_profile_policy;
use openmesh_core::storage::{init_project, read_project, Project};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-profile-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    dir
}

fn project_id(project: &Path) -> String {
    read_project::<Project>(&project.to_string_lossy(), "project.json")
        .expect("project")
        .id
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn run(args: &[&str], project: &Path) -> Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--project").arg(project);
    cmd.output().expect("spawn cli")
}

fn run_json(args: &[&str], project: &Path) -> Value {
    let mut cmd_args = args.to_vec();
    cmd_args.push("--json");
    let output = run(&cmd_args, project);
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("json stdout")
}

fn init_profile(project: &Path, owner: &str, role: &str) {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            owner,
            "--role-label",
            role,
        ],
        project,
    );
    assert!(
        output.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn profile_init_creates_safe_local_profile() {
    let project = temp_project("init-safe");
    init_profile(&project, "Owner One", "Lead");
    let path = work_proxy_profile_path(&project.to_string_lossy());
    assert!(path.exists());
    let profile = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    validate_work_proxy_profile(&profile).unwrap();
    validate_profile_policy(&profile).unwrap();
}

#[test]
fn profile_init_uses_project_workspace_id() {
    let project = temp_project("init-ws");
    let id = project_id(&project);
    init_profile(&project, "Owner", "Role");
    let profile = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(profile.workspace_id, id);
}

#[test]
fn profile_init_uses_conservative_authority_defaults() {
    let project = temp_project("init-authority");
    init_profile(&project, "Owner", "Role");
    let profile = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert!(profile
        .authority_rules
        .iter()
        .all(|rule| rule.authority != ProxyAuthorityLevel::CanAnswer));
    assert!(profile
        .authority_rules
        .iter()
        .any(|rule| { rule.scope == "*" && rule.authority == ProxyAuthorityLevel::MustAskHuman }));
    assert!(profile.evidence_policy.require_evidence_for_claims);
    assert!(!profile.evidence_policy.answer_without_evidence);
    assert_eq!(
        profile.evidence_policy.unsupported_claim_behavior,
        UnsupportedClaimBehavior::SayUnknown
    );
}

#[test]
fn profile_init_includes_no_impersonation_refusal() {
    let project = temp_project("init-refusal");
    init_profile(&project, "Owner", "Role");
    let profile = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert!(profile
        .default_refusal_rules
        .iter()
        .any(|rule| rule.statement.contains("cannot impersonate owner")));
}

#[test]
fn profile_init_rejects_existing_profile() {
    let project = temp_project("init-dup");
    init_profile(&project, "Owner", "Role");
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Other",
            "--role-label",
            "Other",
        ],
        &project,
    );
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("profile-already-exists") || stderr.contains("already exists"));
}

#[test]
fn profile_init_json_outputs_valid_profile() {
    let project = temp_project("init-json");
    let payload = run_json(
        &[
            "profile",
            "init",
            "--owner-label",
            "Owner",
            "--role-label",
            "Role",
        ],
        &project,
    );
    assert_eq!(payload["status"], "ok");
    let profile: WorkProxyProfile =
        serde_json::from_value(payload["profile"].clone()).expect("profile json");
    assert_eq!(profile.profile_version, WORK_PROXY_PROFILE_VERSION);
    validate_profile_policy(&profile).unwrap();
}

#[test]
fn profile_show_human_displays_policy_summary() {
    let project = temp_project("show-human");
    init_profile(&project, "Owner", "Role");
    let output = run(&["profile", "show"], &project);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("profile_id="));
    assert!(stdout.contains("authority_rules="));
    assert!(stdout.contains("privacy_rules="));
    assert!(stdout.contains("default_refusal_rules="));
    assert!(stdout.contains("evidence_policy="));
    assert!(stdout.contains("not the human owner"));
    assert!(stdout.contains("profile_path="));
}

#[test]
fn profile_show_json_round_trips_complete_profile() {
    let project = temp_project("show-json");
    init_profile(&project, "Owner", "Role");
    let stored = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    let payload = run_json(&["profile", "show"], &project);
    let shown: WorkProxyProfile = serde_json::from_value(payload).expect("profile json");
    assert_eq!(shown, stored);
}

#[test]
fn profile_show_rejects_missing_profile() {
    let project = temp_project("show-missing");
    let output = run(&["profile", "show"], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn profile_validate_accepts_valid_profile_without_write() {
    let project = temp_project("validate-ok");
    init_profile(&project, "Owner", "Role");
    let before = fs::read_to_string(work_proxy_profile_path(&project.to_string_lossy())).unwrap();
    let output = run(&["profile", "validate"], &project);
    assert!(output.status.success());
    let after = fs::read_to_string(work_proxy_profile_path(&project.to_string_lossy())).unwrap();
    assert_eq!(before, after);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid=true"));
}

#[test]
fn profile_validate_rejects_malformed_profile() {
    let project = temp_project("validate-malformed");
    fs::create_dir_all(profile_dir(&project.to_string_lossy())).unwrap();
    fs::write(
        work_proxy_profile_path(&project.to_string_lossy()),
        "{not-json",
    )
    .unwrap();
    let output = run(&["profile", "validate"], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn profile_validate_rejects_conflicting_policy() {
    let project = temp_project("validate-conflict");
    let id = project_id(&project);
    let mut profile = default_work_proxy_profile(
        &id,
        format!("profile-{id}"),
        "Owner",
        "Role",
        "2026-07-17T08:00:00Z",
    );
    profile
        .authority_rules
        .push(openmesh_core::domain::AuthorityRule {
            rule_id: "rule-b".into(),
            scope: "*".into(),
            authority: ProxyAuthorityLevel::CannotAnswer,
            description: None,
            conditions: vec![],
            evidence_required: true,
            human_confirmation_required: true,
            limitations: vec![],
        });
    fs::create_dir_all(profile_dir(&project.to_string_lossy())).unwrap();
    fs::write(
        work_proxy_profile_path(&project.to_string_lossy()),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    let output = run(&["profile", "validate"], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn profile_update_changes_only_supplied_fields() {
    let project = temp_project("update-fields");
    init_profile(&project, "Owner", "Role");
    let output = run(
        &[
            "profile",
            "update",
            "--owner-label",
            "Updated Owner",
            "--working-style",
            "async-first",
        ],
        &project,
    );
    assert!(output.status.success());
    let profile = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(profile.owner_label, "Updated Owner");
    assert_eq!(profile.working_style, "async-first");
    assert_eq!(profile.role_label, "Role");
}

#[test]
fn profile_update_preserves_profile_id_workspace_and_created_at() {
    let project = temp_project("update-preserve-ids");
    init_profile(&project, "Owner", "Role");
    let before = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    let output = run(
        &["profile", "update", "--role-label", "Updated Role"],
        &project,
    );
    assert!(output.status.success());
    let after = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(after.profile_id, before.profile_id);
    assert_eq!(after.workspace_id, before.workspace_id);
    assert_eq!(after.created_at, before.created_at);
    assert_eq!(after.profile_version, before.profile_version);
}

#[test]
fn profile_update_preserves_authority_privacy_and_evidence_policy() {
    let project = temp_project("update-preserve-policy");
    init_profile(&project, "Owner", "Role");
    let before = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    let output = run(
        &["profile", "update", "--communication-style", "concise"],
        &project,
    );
    assert!(output.status.success());
    let after = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(after.authority_rules, before.authority_rules);
    assert_eq!(after.privacy_rules, before.privacy_rules);
    assert_eq!(after.evidence_policy, before.evidence_policy);
    assert_eq!(after.default_refusal_rules, before.default_refusal_rules);
    assert_eq!(after.sensitive_topics, before.sensitive_topics);
}

#[test]
fn profile_update_updates_last_updated_at() {
    let project = temp_project("update-ts");
    init_profile(&project, "Owner", "Role");
    let before = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    let output = run(
        &["profile", "update", "--role-label", "Updated Role"],
        &project,
    );
    assert!(output.status.success());
    let after = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_ne!(after.last_updated_at, before.last_updated_at);
}

#[test]
fn profile_update_requires_at_least_one_field() {
    let project = temp_project("update-none");
    init_profile(&project, "Owner", "Role");
    let output = run(&["profile", "update"], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn profile_update_rejects_invalid_result_before_write() {
    let project = temp_project("update-invalid");
    init_profile(&project, "Owner", "Role");
    let output = run(&["profile", "update", "--owner-label", "   "], &project);
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn failed_profile_update_preserves_previous_profile() {
    let project = temp_project("update-failed");
    init_profile(&project, "Owner", "Role");
    let before = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    let output = run(&["profile", "update", "--limitations", "   "], &project);
    assert_eq!(output.status.code(), Some(3));
    let after = read_work_proxy_profile(&project.to_string_lossy()).unwrap();
    assert_eq!(after, before);
}
