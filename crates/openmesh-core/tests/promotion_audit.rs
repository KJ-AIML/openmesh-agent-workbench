//! Dev Track 0.1.3.5 Checkpoint B — promotion audit storage tests.

use openmesh_core::domain::{
    EvidenceAttachment, EvidenceRef, WorkEvent, WORK_EVENT_PROTOCOL_VERSION,
};
use openmesh_core::events::{append_event, ledger_dir, list_events};
use openmesh_core::promotion::{
    classify_decision_record, get_decision_record, list_decision_records, promotion_decisions_dir,
    write_decision_record, EvidenceRelationship, PromotionDecision, PromotionDecisionRecord,
    PromotionEvidence, PromotionKey, PromotionOutcome, PromotionReasonCode, WriteDecisionOutcome,
    PROMOTION_AUDIT_PROTOCOL_VERSION,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

fn create_test_project(name: &str) -> (PathBuf, String, String) {
    let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-promotion-audit-{name}-{}-{unique}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("myproject");
    fs::create_dir_all(&project_dir).unwrap();
    let om = project_dir.join(".openmesh");
    fs::create_dir_all(&om).unwrap();

    let project_id = format!("proj-{name}-{unique}");
    let now = "2026-07-08T00:00:00.000Z";
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
    fs::write(
        om.join("project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();

    let project_path = project_dir.to_string_lossy().into_owned();
    (dir, project_path, project_id)
}

fn sample_record(
    workspace_id: &str,
    signal_ids: &[&str],
    outcome: PromotionOutcome,
    recorded_at: &str,
) -> PromotionDecisionRecord {
    let ids: Vec<String> = signal_ids.iter().map(|s| (*s).to_string()).collect();
    let key = PromotionKey::from_inputs(workspace_id, &ids).unwrap();
    let decision = match outcome {
        PromotionOutcome::Suppress => {
            PromotionDecision::suppress(key.clone(), ids.clone(), PromotionReasonCode::ActivitySpam)
        }
        PromotionOutcome::Defer => PromotionDecision::defer(
            key.clone(),
            ids.clone(),
            PromotionReasonCode::MissingEvidence,
        ),
        PromotionOutcome::Ambiguous => {
            PromotionDecision::ambiguous(key.clone(), ids.clone(), "needs seam".into())
        }
        PromotionOutcome::Promote => PromotionDecision::ambiguous(
            key.clone(),
            ids.clone(),
            "placeholder promote path".into(),
        ),
    };
    PromotionDecisionRecord::from_decision(
        workspace_id.to_string(),
        decision,
        Some(PromotionEvidence {
            signal_refs: ids.clone(),
            relationship: EvidenceRelationship::IndependentCorroboration,
            producer_signal_attachments: ids,
        }),
        recorded_at.to_string(),
    )
}

fn signals_file_count(project_path: &str) -> usize {
    let signals_root = openmesh_core::storage::get_project_dir(project_path).join("signals");
    if !signals_root.exists() {
        return 0;
    }
    let mut count = 0usize;
    for bucket in ["pending", "processed", "quarantine", "duplicate"] {
        let dir = signals_root.join(bucket);
        if dir.exists() {
            count += fs::read_dir(dir).map(|rd| rd.count()).unwrap_or(0);
        }
    }
    count
}

#[test]
fn promotion_decision_writes_one_audit_file() {
    let (_dir, project_path, project_id) = create_test_project("write-one");
    let record = sample_record(
        &project_id,
        &["sig-a"],
        PromotionOutcome::Suppress,
        "2026-07-15T14:00:00Z",
    );

    let outcome = write_decision_record(&project_path, &record).unwrap();
    assert!(matches!(outcome, WriteDecisionOutcome::Created(_)));

    let decisions = promotion_decisions_dir(&project_path);
    assert!(decisions.exists());
    let files: Vec<_> = fs::read_dir(&decisions)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].file_name().unwrap().to_str().unwrap(),
        format!("{}.json", record.promotion_key.as_str())
    );
}

#[test]
fn promotion_key_is_idempotent() {
    let (_dir, project_path, project_id) = create_test_project("idempotent");
    let record = sample_record(
        &project_id,
        &["sig-a", "sig-b"],
        PromotionOutcome::Defer,
        "2026-07-15T14:01:00Z",
    );

    let first = write_decision_record(&project_path, &record).unwrap();
    let second = write_decision_record(&project_path, &record).unwrap();

    assert!(matches!(first, WriteDecisionOutcome::Created(_)));
    assert!(matches!(second, WriteDecisionOutcome::Existing(_)));

    let listed = list_decision_records(&project_path).unwrap();
    assert_eq!(listed.len(), 1);
}

#[test]
fn duplicate_promotion_key_does_not_overwrite() {
    let (_dir, project_path, project_id) = create_test_project("no-overwrite");
    let record = sample_record(
        &project_id,
        &["sig-dup"],
        PromotionOutcome::Suppress,
        "2026-07-15T14:02:00Z",
    );
    let path = promotion_decisions_dir(&project_path)
        .join(format!("{}.json", record.promotion_key.as_str()));

    write_decision_record(&project_path, &record).unwrap();
    let bytes_before = fs::read(&path).unwrap();

    let mut altered = record.clone();
    altered.reason_detail = Some("must not replace on disk".into());
    let second = write_decision_record(&project_path, &altered).unwrap();
    assert!(
        matches!(second, WriteDecisionOutcome::Existing(existing) if existing.reason_detail.is_none())
    );

    let bytes_after = fs::read(&path).unwrap();
    assert_eq!(bytes_before, bytes_after);
}

#[test]
fn different_promotion_keys_create_separate_records() {
    let (_dir, project_path, project_id) = create_test_project("separate-keys");
    let first = sample_record(
        &project_id,
        &["sig-1"],
        PromotionOutcome::Suppress,
        "2026-07-15T14:03:00Z",
    );
    let second = sample_record(
        &project_id,
        &["sig-2"],
        PromotionOutcome::Defer,
        "2026-07-15T14:04:00Z",
    );

    write_decision_record(&project_path, &first).unwrap();
    write_decision_record(&project_path, &second).unwrap();

    let listed = list_decision_records(&project_path).unwrap();
    assert_eq!(listed.len(), 2);
    assert_ne!(first.promotion_key, second.promotion_key);
}

#[test]
fn promotion_audit_is_project_isolated() {
    let (_dir_a, path_a, id_a) = create_test_project("iso-a");
    let (_dir_b, path_b, id_b) = create_test_project("iso-b");

    write_decision_record(
        &path_a,
        &sample_record(
            &id_a,
            &["sig-a"],
            PromotionOutcome::Suppress,
            "2026-07-15T14:05:00Z",
        ),
    )
    .unwrap();
    write_decision_record(
        &path_b,
        &sample_record(
            &id_b,
            &["sig-b"],
            PromotionOutcome::Defer,
            "2026-07-15T14:06:00Z",
        ),
    )
    .unwrap();

    assert_eq!(list_decision_records(&path_a).unwrap().len(), 1);
    assert_eq!(list_decision_records(&path_b).unwrap().len(), 1);
    assert_ne!(
        list_decision_records(&path_a).unwrap()[0].promotion_key,
        list_decision_records(&path_b).unwrap()[0].promotion_key
    );
}

#[test]
fn atomic_write_survives_reload() {
    let (_dir, project_path, project_id) = create_test_project("reload");
    let record = sample_record(
        &project_id,
        &["sig-reload"],
        PromotionOutcome::Ambiguous,
        "2026-07-15T14:07:00Z",
    );
    write_decision_record(&project_path, &record).unwrap();

    let reloaded = get_decision_record(&project_path, &record.promotion_key)
        .unwrap()
        .expect("present");
    assert_eq!(reloaded, record);
    assert_eq!(
        reloaded.audit_protocol_version,
        PROMOTION_AUDIT_PROTOCOL_VERSION
    );
}

#[test]
fn corrupted_audit_record_is_not_silently_accepted() {
    let (_dir, project_path, project_id) = create_test_project("corrupt");
    let record = sample_record(
        &project_id,
        &["sig-bad"],
        PromotionOutcome::Suppress,
        "2026-07-15T14:08:00Z",
    );
    write_decision_record(&project_path, &record).unwrap();

    let path = promotion_decisions_dir(&project_path)
        .join(format!("{}.json", record.promotion_key.as_str()));
    fs::write(&path, "{ not valid promotion audit json").unwrap();

    let err = get_decision_record(&project_path, &record.promotion_key).unwrap_err();
    assert!(err.to_string().contains("json") || err.to_string().contains("validation"));

    let project = openmesh_core::storage::read_project::<openmesh_core::storage::Project>(
        &project_path,
        "project.json",
    )
    .unwrap();
    let classify_err = classify_decision_record(&path, &project).unwrap_err();
    assert!(!classify_err.to_string().is_empty());
}

#[test]
fn promotion_audit_does_not_touch_signal_inbox() {
    let (_dir, project_path, project_id) = create_test_project("signals-untouched");
    fs::create_dir_all(
        openmesh_core::storage::get_project_dir(&project_path).join("signals/processed"),
    )
    .unwrap();
    fs::write(
        openmesh_core::storage::get_project_dir(&project_path)
            .join("signals/processed/existing.json"),
        "{}",
    )
    .unwrap();
    let before = signals_file_count(&project_path);

    write_decision_record(
        &project_path,
        &sample_record(
            &project_id,
            &["sig-x"],
            PromotionOutcome::Suppress,
            "2026-07-15T14:09:00Z",
        ),
    )
    .unwrap();

    assert_eq!(signals_file_count(&project_path), before);
}

#[test]
fn promotion_audit_does_not_create_work_events() {
    let (_dir, project_path, project_id) = create_test_project("no-work-events");
    write_decision_record(
        &project_path,
        &sample_record(
            &project_id,
            &["sig-y"],
            PromotionOutcome::Defer,
            "2026-07-15T14:10:00Z",
        ),
    )
    .unwrap();

    assert!(!ledger_dir(&project_path).exists());
    assert!(list_events(&project_path).unwrap().is_empty());

    // Guard: append_event still works independently and audit did not pre-create ledger.
    let event = WorkEvent::new(
        "evt-independent",
        &project_id,
        "work.completed",
        "unaffected",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/x.md".into()),
            observed_at: None,
        }],
        "2026-07-15T14:11:00Z",
    );
    append_event(&project_path, &event).unwrap();
    assert_eq!(list_events(&project_path).unwrap().len(), 1);
    assert_eq!(list_decision_records(&project_path).unwrap().len(), 1);
    assert_eq!(event.protocol_version, WORK_EVENT_PROTOCOL_VERSION);
}

#[test]
fn restart_read_returns_same_audit_state() {
    let (_dir, project_path, project_id) = create_test_project("restart");
    let record = sample_record(
        &project_id,
        &["sig-z"],
        PromotionOutcome::Suppress,
        "2026-07-15T14:12:00Z",
    );
    write_decision_record(&project_path, &record).unwrap();

    let listed_first = list_decision_records(&project_path).unwrap();
    let listed_second = list_decision_records(&project_path).unwrap();
    assert_eq!(listed_first, listed_second);
    assert_eq!(listed_first[0], record);
}
