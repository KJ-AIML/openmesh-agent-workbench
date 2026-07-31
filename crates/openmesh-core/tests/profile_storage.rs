//! Dev Track 0.1.4 Checkpoint C — local Work Proxy Profile storage tests.

use openmesh_core::domain::{
    AuthorityRule, CommunicationPreferences, DecisionPreferences, DefaultRefusalRule,
    EvidencePolicy, EvidenceSourceKind, PrivacyAllowedUse, PrivacyRule, PrivacySensitivity,
    ProfileValidationError, ProxyAuthorityLevel, UnsupportedClaimBehavior, WorkProxyProfile,
    WORK_PROXY_PROFILE_VERSION,
};
use openmesh_core::profile::{
    profile_dir, profile_exists, read_work_proxy_profile, work_proxy_profile_path,
    write_work_proxy_profile, ProfileError, WORK_PROXY_PROFILE_FILENAME,
};
use openmesh_core::storage::get_project_dir;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

const ACTIVE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab";
const WORKTREE_PROJECT_ROOT: &str = r"D:\KJ\repo\open-mesh-lab\repos\openmesh-agent-workbench";

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-profile-storage-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();

    let project_id = format!("proj-profile-{name}-{unique}");
    let now = "2026-07-17T09:00:00.000Z";
    let project_json = serde_json::json!({
        "id": project_id,
        "name": "Test Project",
        "folderPath": project_dir.to_str().unwrap(),
        "repoUrl": null,
        "defaultBranch": "main",
        "sprintSource": "none",
        "docsFolder": null,
        "terminalDir": null,
        "defaultAgentCli": null,
        "notes": null,
        "status": "active",
        "createdAt": now,
        "updatedAt": now,
    });
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();
    fs::write(
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();

    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

fn authority_rule(rule_id: &str, scope: &str, authority: ProxyAuthorityLevel) -> AuthorityRule {
    AuthorityRule {
        rule_id: rule_id.into(),
        scope: scope.into(),
        authority,
        description: Some(format!("rule for {scope}")),
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: matches!(
            authority,
            ProxyAuthorityLevel::MustAskHuman | ProxyAuthorityLevel::CannotAnswer
        ),
        limitations: vec!["rule limitation".into()],
    }
}

fn valid_profile(workspace_id: &str) -> WorkProxyProfile {
    WorkProxyProfile {
        profile_id: format!("profile-{workspace_id}"),
        workspace_id: workspace_id.into(),
        owner_label: "Fixture Owner".into(),
        role_label: "Engineering lead".into(),
        working_style: "async-first".into(),
        communication_style: "concise".into(),
        communication_preferences: CommunicationPreferences {
            tone: "direct".into(),
            detail_level: "medium".into(),
            async_preference: "prefer-async".into(),
            correction_preference: "surface-limitations".into(),
        },
        decision_preferences: DecisionPreferences {
            decision_style: "evidence-first".into(),
            escalation_preference: "ask-human-on-ambiguity".into(),
        },
        authority_rules: vec![
            authority_rule("rule-global", "*", ProxyAuthorityLevel::MustAskHuman),
            authority_rule(
                "rule-factual",
                "work.progress",
                ProxyAuthorityLevel::CanAnswer,
            ),
        ],
        privacy_rules: vec![PrivacyRule {
            rule_id: "privacy-credentials".into(),
            topic: "credentials".into(),
            sensitivity: PrivacySensitivity::Secret,
            allowed_use: PrivacyAllowedUse::ExcludeFromAnswers,
            restriction: "never include in proxy output".into(),
            requires_human_confirmation: true,
        }],
        sensitive_topics: vec!["credentials".into()],
        default_refusal_rules: vec![
            DefaultRefusalRule {
                rule_id: "refusal-no-impersonation".into(),
                statement: "cannot impersonate owner".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-invented-evidence".into(),
                statement: "cannot invent evidence".into(),
            },
            DefaultRefusalRule {
                rule_id: "refusal-no-irreversible-approval".into(),
                statement: "cannot approve irreversible actions".into(),
            },
        ],
        evidence_policy: EvidencePolicy {
            answer_without_evidence: false,
            require_evidence_for_claims: true,
            expose_limitations: true,
            cite_source_kinds: vec![EvidenceSourceKind::FilePath, EvidenceSourceKind::WorkEvent],
            unsupported_claim_behavior: UnsupportedClaimBehavior::AskHuman,
        },
        limitations: vec![
            "proxy profile metadata only".into(),
            "no answering behavior in 0.1.4".into(),
        ],
        created_at: "2026-07-17T08:00:00Z".into(),
        last_updated_at: "2026-07-17T08:30:00Z".into(),
        profile_version: WORK_PROXY_PROFILE_VERSION.to_string(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BucketSnapshot {
    pending: usize,
    processed: usize,
    quarantine: usize,
    duplicate: usize,
}

fn bucket_snapshot(signals_root: &Path) -> BucketSnapshot {
    let count = |bucket: &str| -> usize {
        let dir = signals_root.join(bucket);
        if dir.exists() {
            fs::read_dir(dir)
                .map(|entries| entries.count())
                .unwrap_or(0)
        } else {
            0
        }
    };
    BucketSnapshot {
        pending: count("pending"),
        processed: count("processed"),
        quarantine: count("quarantine"),
        duplicate: count("duplicate"),
    }
}

fn real_inbox_snapshots() -> (BucketSnapshot, BucketSnapshot) {
    let active = bucket_snapshot(&PathBuf::from(ACTIVE_PROJECT_ROOT).join(".openmesh/signals"));
    let worktree = bucket_snapshot(&PathBuf::from(WORKTREE_PROJECT_ROOT).join(".openmesh/signals"));
    (active, worktree)
}

#[test]
fn work_proxy_profile_path_is_canonical() {
    let (_dir, project_path, _) = create_test_project("path");
    let path = work_proxy_profile_path(&project_path);
    let normalized = path.to_string_lossy().replace('\\', "/");
    assert!(normalized.ends_with(".openmesh/profile/work-proxy-profile.json"));
    assert_eq!(
        path.file_name().and_then(|name| name.to_str()),
        Some(WORK_PROXY_PROFILE_FILENAME)
    );
}

#[test]
fn profile_exists_is_false_when_profile_is_missing() {
    let (_dir, project_path, _) = create_test_project("exists");
    assert!(!profile_exists(&project_path).unwrap());
    assert!(!profile_dir(&project_path).exists());
}

#[test]
fn read_missing_profile_returns_explicit_missing_result() {
    let (_dir, project_path, _) = create_test_project("read-missing");
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::ProfileMissing)
    ));
}

#[test]
fn read_missing_profile_does_not_create_profile_directory() {
    let (_dir, project_path, _) = create_test_project("read-no-dir");
    let _ = read_work_proxy_profile(&project_path);
    assert!(!profile_dir(&project_path).exists());
}

#[test]
fn write_then_read_profile_round_trips() {
    let (_dir, project_path, project_id) = create_test_project("roundtrip");
    let profile = valid_profile(&project_id);
    write_work_proxy_profile(&project_path, &profile).unwrap();
    let restored = read_work_proxy_profile(&project_path).unwrap();
    assert_eq!(restored, profile);
}

#[test]
fn written_profile_uses_pretty_deterministic_json() {
    let (_dir, project_path, project_id) = create_test_project("pretty");
    let profile = valid_profile(&project_id);
    write_work_proxy_profile(&project_path, &profile).unwrap();
    let raw = fs::read_to_string(work_proxy_profile_path(&project_path)).unwrap();
    let reparsed: WorkProxyProfile = serde_json::from_str(&raw).unwrap();
    assert_eq!(reparsed, profile);
    assert!(raw.contains("\n  \"profileId\""));
    let second_write = raw.clone();
    write_work_proxy_profile(&project_path, &profile).unwrap();
    let raw_again = fs::read_to_string(work_proxy_profile_path(&project_path)).unwrap();
    assert_eq!(second_write, raw_again);
}

#[test]
fn written_profile_has_trailing_newline() {
    let (_dir, project_path, project_id) = create_test_project("newline");
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    let raw = fs::read_to_string(work_proxy_profile_path(&project_path)).unwrap();
    assert!(raw.ends_with('\n'));
}

#[test]
fn write_creates_profile_directory_only_when_needed() {
    let (_dir, project_path, project_id) = create_test_project("mkdir");
    assert!(!profile_dir(&project_path).exists());
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    assert!(profile_dir(&project_path).is_dir());
}

#[test]
fn write_rejects_invalid_profile_before_filesystem_mutation() {
    let (_dir, project_path, project_id) = create_test_project("reject-invalid");
    let mut profile = valid_profile(&project_id);
    profile.limitations.clear();
    let result = write_work_proxy_profile(&project_path, &profile);
    assert!(matches!(
        result,
        Err(ProfileError::ValidationFailure(
            ProfileValidationError::EmptyLimitations
        ))
    ));
    assert!(!work_proxy_profile_path(&project_path).exists());
}

#[test]
fn write_rejects_unsupported_profile_version() {
    let (_dir, project_path, project_id) = create_test_project("reject-version");
    let mut profile = valid_profile(&project_id);
    profile.profile_version = "99.0".into();
    assert!(matches!(
        write_work_proxy_profile(&project_path, &profile),
        Err(ProfileError::UnsupportedVersion { .. })
    ));
}

#[test]
fn write_rejects_workspace_mismatch() {
    let (_dir, project_path, _) = create_test_project("reject-ws-write");
    let profile = valid_profile("wrong-workspace-id");
    assert!(matches!(
        write_work_proxy_profile(&project_path, &profile),
        Err(ProfileError::WorkspaceMismatch { .. })
    ));
}

#[test]
fn read_rejects_workspace_mismatch() {
    let (_dir, project_path, project_id) = create_test_project("reject-ws-read");
    let profile = valid_profile("wrong-workspace-id");
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::WorkspaceMismatch {
            expected,
            found
        }) if expected == project_id && found == "wrong-workspace-id"
    ));
}

#[test]
fn read_rejects_malformed_json_without_panic() {
    let (_dir, project_path, _) = create_test_project("malformed");
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(work_proxy_profile_path(&project_path), "{not-json").unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::MalformedJson(_))
    ));
}

#[test]
fn read_rejects_invalid_profile_policy() {
    let (_dir, project_path, project_id) = create_test_project("invalid-policy");
    let mut profile = valid_profile(&project_id);
    profile.authority_rules = vec![
        authority_rule("rule-a", "same-scope", ProxyAuthorityLevel::CanAnswer),
        authority_rule("rule-b", "same-scope", ProxyAuthorityLevel::CannotAnswer),
    ];
    fs::create_dir_all(profile_dir(&project_path)).unwrap();
    fs::write(
        work_proxy_profile_path(&project_path),
        serde_json::to_string_pretty(&profile).unwrap(),
    )
    .unwrap();
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::ValidationFailure(
            ProfileValidationError::ConflictingProfilePolicy(_)
        ))
    ));
}

#[test]
fn overwrite_replaces_existing_profile_atomically() {
    let (_dir, project_path, project_id) = create_test_project("overwrite");
    let mut first = valid_profile(&project_id);
    first.owner_label = "First Owner".into();
    write_work_proxy_profile(&project_path, &first).unwrap();

    let mut second = valid_profile(&project_id);
    second.owner_label = "Second Owner".into();
    second.last_updated_at = "2026-07-17T10:00:00Z".into();
    write_work_proxy_profile(&project_path, &second).unwrap();

    let restored = read_work_proxy_profile(&project_path).unwrap();
    assert_eq!(restored.owner_label, "Second Owner");
}

#[test]
fn failed_overwrite_preserves_previous_valid_profile() {
    let (_dir, project_path, project_id) = create_test_project("failed-overwrite");
    let valid = valid_profile(&project_id);
    write_work_proxy_profile(&project_path, &valid).unwrap();

    let mut invalid = valid_profile(&project_id);
    invalid.profile_version = "99.0".into();
    assert!(write_work_proxy_profile(&project_path, &invalid).is_err());

    let restored = read_work_proxy_profile(&project_path).unwrap();
    assert_eq!(restored, valid);
}

#[test]
fn successful_write_leaves_no_temporary_files() {
    let (_dir, project_path, project_id) = create_test_project("no-temp");
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    let entries: Vec<_> = fs::read_dir(profile_dir(&project_path))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec![WORK_PROXY_PROFILE_FILENAME.to_string()]);
}

#[test]
fn profiles_are_isolated_between_projects() {
    let (_dir_a, project_a, id_a) = create_test_project("iso-a");
    let (_dir_b, project_b, id_b) = create_test_project("iso-b");
    let mut profile_a = valid_profile(&id_a);
    profile_a.owner_label = "Project A".into();
    let mut profile_b = valid_profile(&id_b);
    profile_b.owner_label = "Project B".into();

    write_work_proxy_profile(&project_a, &profile_a).unwrap();
    write_work_proxy_profile(&project_b, &profile_b).unwrap();

    assert_eq!(
        read_work_proxy_profile(&project_a).unwrap().owner_label,
        "Project A"
    );
    assert_eq!(
        read_work_proxy_profile(&project_b).unwrap().owner_label,
        "Project B"
    );
}

#[test]
fn storage_preserves_authority_privacy_and_evidence_policy() {
    let (_dir, project_path, project_id) = create_test_project("preserve-policy");
    let profile = valid_profile(&project_id);
    write_work_proxy_profile(&project_path, &profile).unwrap();
    let restored = read_work_proxy_profile(&project_path).unwrap();
    assert_eq!(restored.authority_rules, profile.authority_rules);
    assert_eq!(restored.privacy_rules, profile.privacy_rules);
    assert_eq!(restored.evidence_policy, profile.evidence_policy);
    assert_eq!(
        restored.default_refusal_rules,
        profile.default_refusal_rules
    );
}

#[test]
fn storage_does_not_create_default_profile_implicitly() {
    let (_dir, project_path, _) = create_test_project("no-default");
    assert!(matches!(
        read_work_proxy_profile(&project_path),
        Err(ProfileError::ProfileMissing)
    ));
    assert!(!profile_exists(&project_path).unwrap());
}

#[test]
fn storage_does_not_touch_signal_inboxes() {
    let before = real_inbox_snapshots();
    let (_dir, project_path, project_id) = create_test_project("signals");
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    let _ = read_work_proxy_profile(&project_path);
    let after = real_inbox_snapshots();
    assert_eq!(before, after);
    assert_eq!(
        before.0,
        BucketSnapshot {
            pending: 0,
            processed: 0,
            quarantine: 0,
            duplicate: 0,
        }
    );
    assert_eq!(before.1.pending, 0);
    assert_eq!(before.1.quarantine, 0);
    assert_eq!(before.1.duplicate, 0);
}

#[test]
fn storage_does_not_touch_event_or_promotion_ledgers() {
    let (_dir, project_path, project_id) = create_test_project("ledgers");
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    let om = get_project_dir(&project_path);
    assert!(!om.join("events").exists());
    assert!(!om.join("events/promotion").exists());
}

#[test]
fn storage_does_not_create_current_state_projection() {
    let (_dir, project_path, project_id) = create_test_project("no-projection");
    write_work_proxy_profile(&project_path, &valid_profile(&project_id)).unwrap();
    assert!(!get_project_dir(&project_path).join("projections").exists());
}

#[test]
fn storage_does_not_start_context_pack_or_ask_my_proxy() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let profile_src = fs::read_to_string(root.join("src/profile.rs")).expect("read profile.rs");
    for forbidden in [
        "ask-my-proxy",
        "ask my proxy",
        "context-pack",
        "context pack",
        "ProxyContextPack",
        "generate_answer",
    ] {
        assert!(
            !profile_src.to_ascii_lowercase().contains(forbidden),
            "profile.rs must not reference {forbidden}"
        );
    }
}

#[test]
fn checkpoint_c_does_not_touch_cli_tauri_or_remote_surface() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in [
        "../openmesh-cli/src/main.rs",
        "../../src-tauri/src/lib.rs",
        "src/continuity/current_state.rs",
        "src/events.rs",
        "src/signals.rs",
    ] {
        let path = root.join(rel);
        if path.exists() {
            let content = fs::read_to_string(path).expect("read source");
            assert!(!content.contains("write_work_proxy_profile"));
            assert!(!content.contains("read_work_proxy_profile"));
        }
    }
}
