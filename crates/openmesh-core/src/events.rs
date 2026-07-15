// ============================================================================
// OpenMesh Canonical WorkEvent Ledger — Dev Track 0.1.3.4
// ============================================================================
// Checkpoint B: project-scoped ledger layout + race-safe append.
// Checkpoint C: ledger validation, quarantine recovery, safe enumeration.
// Checkpoint D: append-only correction / supersession helpers.
// Checkpoint E: stable public core API boundary (no CLI/Tauri/Desktop surface).
//
// Public ledger API surface:
// - append_event, get_event, list_events, list_ledger_entries
// - validate_ledger, classify_ledger_record
// - list_corrections_for, effective_summary
// - read_event_file (low-level deserialize; prefer classify_ledger_record)
// - ledger_dir, quarantine_dir, MAX_RECORD_BYTES
// - EventError, LedgerClassification, LedgerValidationReport, ...
// ============================================================================

use crate::domain::{validate_event_semantics, WorkEvent, WORK_EVENT_PROTOCOL_VERSION};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// Frozen bound (approved 0.1.3.4 plan §3.3): canonical record maximum size.
pub const MAX_RECORD_BYTES: usize = 256 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("event workspace_id does not match the project's id")]
    WorkspaceMismatch,
    #[error("event failed semantic validation: {0}")]
    InvalidSemantics(String),
    #[error("event_id is not safe for ledger storage: {0}")]
    UnsafeEventId(String),
    #[error("event_id already exists in the ledger: {0}")]
    DuplicateEventId(String),
    #[error("canonical record exceeds the {max}-byte bound (was {actual} bytes)")]
    RecordTooLarge { actual: usize, max: usize },
    #[error("event not found: {0}")]
    NotFound(String),
    #[error("correction target not found: {0}")]
    CorrectionTargetNotFound(String),
    #[error("event cannot correct itself")]
    SelfCorrectionNotAllowed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Outcome of validating a single on-disk ledger record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerClassification {
    Valid,
    Malformed(String),
    UnsupportedVersion(String),
    InvalidSemantics(String),
    WrongWorkspace { expected: String, found: String },
}

/// A ledger record moved from `ledger/` to `quarantine/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuarantinedRecord {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub classification: LedgerClassification,
}

/// A valid ledger record retained in `ledger/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidLedgerRecord {
    pub path: PathBuf,
    pub event: WorkEvent,
}

/// Result of scanning `ledger/` and quarantining invalid records.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerValidationReport {
    pub valid: Vec<ValidLedgerRecord>,
    pub quarantined: Vec<QuarantinedRecord>,
    pub move_failed: Vec<(PathBuf, LedgerClassification)>,
}

// ============================================================================
// Physical layout
// ============================================================================

fn events_root(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("events")
}

pub fn ledger_dir(project_path: &str) -> PathBuf {
    events_root(project_path).join("ledger")
}

pub fn quarantine_dir(project_path: &str) -> PathBuf {
    events_root(project_path).join("quarantine")
}

fn ensure_ledger_directories(project_path: &str) -> std::io::Result<()> {
    fs::create_dir_all(ledger_dir(project_path))?;
    fs::create_dir_all(quarantine_dir(project_path))?;
    Ok(())
}

fn ledger_file_path(project_path: &str, event_id: &str) -> PathBuf {
    ledger_dir(project_path).join(format!("{event_id}.json"))
}

/// Lists canonical (non-`.tmp`, non-symlink, regular-file) entries in a single
/// directory, sorted by filename.
fn list_canonical_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "tmp").unwrap_or(false) {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn list_ledger_paths(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    list_canonical_files(dir)
}

fn validate_event_id_for_storage(event_id: &str) -> Result<(), EventError> {
    if event_id.contains('/') || event_id.contains('\\') {
        return Err(EventError::UnsafeEventId(
            "event_id must not contain path separators".into(),
        ));
    }
    if event_id.contains("..") {
        return Err(EventError::UnsafeEventId(
            "event_id must not contain '..'".into(),
        ));
    }
    Ok(())
}

fn load_project(project_path: &str) -> Result<Project, EventError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or_else(|| EventError::ProjectNotInitialized(project_path.to_string()))
}

fn write_all_and_flush(mut file: fs::File, content: &str) -> Result<(), EventError> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

// ============================================================================
// Checkpoint C — classification and quarantine recovery
// ============================================================================

/// Validates a single ledger record file against the frozen validation order.
/// Never panics on malformed input.
pub fn classify_ledger_record(
    path: &Path,
    project: &Project,
) -> Result<(String, WorkEvent), LedgerClassification> {
    let metadata = fs::metadata(path)
        .map_err(|e| LedgerClassification::Malformed(format!("cannot stat file: {e}")))?;
    if metadata.len() as usize > MAX_RECORD_BYTES {
        return Err(LedgerClassification::InvalidSemantics(format!(
            "record exceeds the {MAX_RECORD_BYTES}-byte bound"
        )));
    }

    let raw = fs::read_to_string(path)
        .map_err(|e| LedgerClassification::Malformed(format!("cannot read file: {e}")))?;

    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| LedgerClassification::Malformed(format!("invalid JSON: {e}")))?;

    let protocol_version = value
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| LedgerClassification::Malformed("missing protocolVersion".into()))?;
    if protocol_version != WORK_EVENT_PROTOCOL_VERSION {
        return Err(LedgerClassification::UnsupportedVersion(
            protocol_version.to_string(),
        ));
    }

    let event: WorkEvent = serde_json::from_str(&raw)
        .map_err(|e| LedgerClassification::Malformed(format!("strict deserialize failed: {e}")))?;

    validate_event_semantics(&event)
        .map_err(|e| LedgerClassification::InvalidSemantics(e.to_string()))?;

    if event.workspace_id != project.id {
        return Err(LedgerClassification::WrongWorkspace {
            expected: project.id.clone(),
            found: event.workspace_id.clone(),
        });
    }

    Ok((raw, event))
}

/// Moves a ledger record into `quarantine/` without rewriting bytes.
/// Returns `Err` if the destination filename already exists.
fn quarantine_ledger_record(project_path: &str, source: &Path) -> Result<PathBuf, EventError> {
    ensure_ledger_directories(project_path)?;
    let filename = source
        .file_name()
        .ok_or_else(|| EventError::Io(std::io::Error::other("source path has no filename")))?;
    let destination = quarantine_dir(project_path).join(filename);
    if destination.exists() {
        return Err(EventError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!(
                "quarantine destination already exists: {}",
                destination.display()
            ),
        )));
    }
    fs::rename(source, &destination)?;
    Ok(destination)
}

/// Scans `ledger/`, quarantines invalid records, and returns a report.
pub fn validate_ledger(project_path: &str) -> Result<LedgerValidationReport, EventError> {
    let project = load_project(project_path)?;
    ensure_ledger_directories(project_path)?;
    let mut report = LedgerValidationReport::default();

    for path in list_ledger_paths(&ledger_dir(project_path))? {
        match classify_ledger_record(&path, &project) {
            Ok((_, event)) => report.valid.push(ValidLedgerRecord {
                path: path.clone(),
                event,
            }),
            Err(classification) => match quarantine_ledger_record(project_path, &path) {
                Ok(destination) => report.quarantined.push(QuarantinedRecord {
                    source: path,
                    destination,
                    classification,
                }),
                Err(_) => report.move_failed.push((path, classification)),
            },
        }
    }

    Ok(report)
}

// ============================================================================
// Checkpoint D — correction / supersession
// ============================================================================

/// Validates `correctsEventId` against the active ledger before append.
fn validate_correction_target(
    project_path: &str,
    event: &WorkEvent,
    project: &Project,
) -> Result<(), EventError> {
    let Some(target_id) = event.corrects_event_id.as_deref() else {
        return Ok(());
    };
    if target_id == event.event_id {
        return Err(EventError::SelfCorrectionNotAllowed);
    }
    let target_path = ledger_file_path(project_path, target_id);
    if !target_path.exists() {
        return Err(EventError::CorrectionTargetNotFound(target_id.to_string()));
    }
    match classify_ledger_record(&target_path, project) {
        Ok(_) => Ok(()),
        Err(_) => Err(EventError::CorrectionTargetNotFound(target_id.to_string())),
    }
}

/// Picks the winning correction for `effective_summary`.
/// Rule: latest `timestamp` (lexicographic ISO-8601 UTC), then `event_id` tie-break.
fn select_latest_correction(corrections: Vec<WorkEvent>) -> Option<WorkEvent> {
    corrections.into_iter().max_by(|a, b| {
        a.timestamp
            .cmp(&b.timestamp)
            .then_with(|| a.event_id.cmp(&b.event_id))
    })
}

// ============================================================================
// Public ledger API
// ============================================================================

/// Official append API. Persists one valid WorkEvent as
/// `<project>/.openmesh/events/ledger/{eventId}.json`.
pub fn append_event(project_path: &str, event: &WorkEvent) -> Result<(), EventError> {
    let project = load_project(project_path)?;
    if event.workspace_id != project.id {
        return Err(EventError::WorkspaceMismatch);
    }

    validate_event_semantics(event).map_err(|e| EventError::InvalidSemantics(e.to_string()))?;
    validate_event_id_for_storage(&event.event_id)?;
    validate_correction_target(project_path, event, &project)?;

    let final_path = ledger_file_path(project_path, &event.event_id);
    if final_path.exists() {
        return Err(EventError::DuplicateEventId(event.event_id.clone()));
    }

    let payload = serde_json::to_string_pretty(event)?;
    let payload_len = payload.len();
    if payload_len > MAX_RECORD_BYTES {
        return Err(EventError::RecordTooLarge {
            actual: payload_len,
            max: MAX_RECORD_BYTES,
        });
    }

    ensure_ledger_directories(project_path)?;
    let ledger = ledger_dir(project_path);
    let temp_path = ledger.join(format!("{}.tmp", event.event_id));

    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)?;

    write_all_and_flush(file, &payload)?;
    fs::rename(&temp_path, &final_path)?;
    Ok(())
}

/// Reads a single persisted WorkEvent by `event_id`, if present and valid.
/// Invalid on-disk records are quarantined and reported as absent.
pub fn get_event(project_path: &str, event_id: &str) -> Result<Option<WorkEvent>, EventError> {
    let path = ledger_file_path(project_path, event_id);
    if !path.exists() {
        return Ok(None);
    }
    let project = load_project(project_path)?;
    match classify_ledger_record(&path, &project) {
        Ok((_, event)) => Ok(Some(event)),
        Err(_) => {
            let _ = quarantine_ledger_record(project_path, &path);
            Ok(None)
        }
    }
}

/// Lists valid persisted WorkEvents in deterministic filename order.
/// Invalid ledger records are quarantined during this call.
pub fn list_events(project_path: &str) -> Result<Vec<WorkEvent>, EventError> {
    let report = validate_ledger(project_path)?;
    Ok(report
        .valid
        .into_iter()
        .map(|record| record.event)
        .collect())
}

/// Lists ledger file paths in deterministic order (active `ledger/` only).
pub fn list_ledger_entries(project_path: &str) -> Result<Vec<PathBuf>, EventError> {
    Ok(list_ledger_paths(&ledger_dir(project_path))?)
}

/// Lists correction events that directly reference `event_id` via `correctsEventId`.
/// Results are in deterministic ledger filename order (lexicographic `event_id`).
pub fn list_corrections_for(
    project_path: &str,
    event_id: &str,
) -> Result<Vec<WorkEvent>, EventError> {
    Ok(list_events(project_path)?
        .into_iter()
        .filter(|event| event.corrects_event_id.as_deref() == Some(event_id))
        .collect())
}

/// Returns the effective summary for `event_id` at ledger level only.
///
/// When no direct corrections exist, returns the original event summary.
/// When one or more corrections exist, returns the summary of the correction
/// with the latest `timestamp`; ties break on lexicographic `event_id`.
pub fn effective_summary(project_path: &str, event_id: &str) -> Result<Option<String>, EventError> {
    let Some(original) = get_event(project_path, event_id)? else {
        return Ok(None);
    };
    let corrections = list_corrections_for(project_path, event_id)?;
    if corrections.is_empty() {
        return Ok(Some(original.summary));
    }
    let latest = select_latest_correction(corrections).expect("non-empty corrections");
    Ok(Some(latest.summary))
}

/// Deserializes a single on-disk ledger record file without recovery.
/// Prefer `classify_ledger_record` for validation-aware reads.
pub fn read_event_file(path: &Path) -> Result<WorkEvent, EventError> {
    let raw = fs::read_to_string(path)?;
    let event: WorkEvent = serde_json::from_str(&raw)?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Sensitivity;
    use crate::domain::{EvidenceAttachment, EvidenceRef, WORK_EVENT_PROTOCOL_VERSION};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn create_test_project(name: &str) -> (PathBuf, String, String) {
        let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-events-test-{name}-{}-{unique}",
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

    fn sample_event(event_id: &str, workspace_id: &str) -> WorkEvent {
        WorkEvent {
            event_id: event_id.to_string(),
            workspace_id: workspace_id.to_string(),
            kind: "work.completed".into(),
            summary: format!("test event {event_id}"),
            timestamp: "2026-07-15T07:00:00Z".into(),
            evidence: vec![EvidenceAttachment {
                evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
                observed_at: None,
            }],
            corrects_event_id: None,
            sensitivity: Sensitivity::Private,
            protocol_version: WORK_EVENT_PROTOCOL_VERSION.to_string(),
        }
    }

    fn correction_event(
        event_id: &str,
        workspace_id: &str,
        corrects_event_id: &str,
        summary: &str,
        timestamp: &str,
    ) -> WorkEvent {
        let mut event = sample_event(event_id, workspace_id);
        event.corrects_event_id = Some(corrects_event_id.to_string());
        event.summary = summary.to_string();
        event.timestamp = timestamp.to_string();
        event
    }

    fn drop_raw_ledger_file(project_path: &str, filename: &str, content: &str) {
        ensure_ledger_directories(project_path).unwrap();
        fs::write(ledger_dir(project_path).join(filename), content).unwrap();
    }

    fn signals_file_count(project_path: &str) -> usize {
        let signals_root = get_project_dir(project_path).join("signals");
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

    // ------------------------------------------------------------------
    // Checkpoint B tests
    // ------------------------------------------------------------------

    #[test]
    fn append_writes_one_file_under_ledger_directory() {
        let (_dir, project_path, project_id) = create_test_project("append-one");
        let event = sample_event("evt-append-1", &project_id);

        append_event(&project_path, &event).expect("append");

        let ledger = ledger_dir(&project_path);
        assert!(ledger.exists());
        let entries = list_ledger_entries(&project_path).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], ledger.join("evt-append-1.json"));
    }

    #[test]
    fn append_one_json_file_per_event_id() {
        let (_dir, project_path, project_id) = create_test_project("one-per-event");
        append_event(&project_path, &sample_event("evt-a", &project_id)).unwrap();
        append_event(&project_path, &sample_event("evt-b", &project_id)).unwrap();

        let entries = list_ledger_entries(&project_path).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].ends_with("evt-a.json"));
        assert!(entries[1].ends_with("evt-b.json"));
    }

    #[test]
    fn get_event_reloads_after_fresh_call() {
        let (_dir, project_path, project_id) = create_test_project("reload");
        let event = sample_event("evt-reload", &project_id);
        append_event(&project_path, &event).unwrap();

        let restored = get_event(&project_path, "evt-reload")
            .unwrap()
            .expect("event on disk");
        assert_eq!(restored.event_id, "evt-reload");
        assert_eq!(restored.workspace_id, project_id);
        assert_eq!(restored.kind, "work.completed");
    }

    #[test]
    fn list_events_enumerates_deterministically() {
        let (_dir, project_path, project_id) = create_test_project("enumerate");
        append_event(&project_path, &sample_event("evt-c", &project_id)).unwrap();
        append_event(&project_path, &sample_event("evt-a", &project_id)).unwrap();
        append_event(&project_path, &sample_event("evt-b", &project_id)).unwrap();

        let events = list_events(&project_path).unwrap();
        let ids: Vec<&str> = events.iter().map(|e| e.event_id.as_str()).collect();
        assert_eq!(ids, vec!["evt-a", "evt-b", "evt-c"]);
    }

    #[test]
    fn project_isolation_keeps_ledgers_separate() {
        let (_dir_a, project_a, project_a_id) = create_test_project("iso-a");
        let (_dir_b, project_b, project_b_id) = create_test_project("iso-b");

        append_event(&project_a, &sample_event("evt-only-a", &project_a_id)).unwrap();
        append_event(&project_b, &sample_event("evt-only-b", &project_b_id)).unwrap();

        let a_events = list_events(&project_a).unwrap();
        let b_events = list_events(&project_b).unwrap();
        assert_eq!(a_events.len(), 1);
        assert_eq!(b_events.len(), 1);
        assert_eq!(a_events[0].event_id, "evt-only-a");
        assert_eq!(b_events[0].event_id, "evt-only-b");
    }

    #[test]
    fn append_survives_store_recreation() {
        let (_dir, project_path, project_id) = create_test_project("durability");
        append_event(&project_path, &sample_event("evt-durable", &project_id)).unwrap();

        let events = list_events(&project_path).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt-durable");

        let again = get_event(&project_path, "evt-durable")
            .unwrap()
            .expect("still on disk");
        assert_eq!(again.summary, "test event evt-durable");
    }

    #[test]
    fn append_does_not_touch_signal_inbox() {
        let (_dir, project_path, project_id) = create_test_project("no-signals");
        let before = signals_file_count(&project_path);

        append_event(&project_path, &sample_event("evt-no-signals", &project_id)).unwrap();

        assert_eq!(signals_file_count(&project_path), before);
        assert!(
            !get_project_dir(&project_path).join("signals").exists()
                || signals_file_count(&project_path) == 0
        );
    }

    #[test]
    fn duplicate_event_id_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("duplicate");
        let event = sample_event("evt-dup", &project_id);
        append_event(&project_path, &event).unwrap();

        let err = append_event(&project_path, &event).unwrap_err();
        assert!(matches!(err, EventError::DuplicateEventId(ref id) if id == "evt-dup"));
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 1);
    }

    #[test]
    fn append_rejects_uninitialized_project() {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-events-test-uninitialized-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let project_path = dir.join("nope").to_string_lossy().into_owned();
        let event = sample_event("evt-1", "ws-1");

        let err = append_event(&project_path, &event).unwrap_err();
        assert!(matches!(err, EventError::ProjectNotInitialized(_)));
        assert!(!events_root(&project_path).exists());
    }

    #[test]
    fn append_rejects_workspace_mismatch() {
        let (_dir, project_path, _project_id) = create_test_project("mismatch");
        let event = sample_event("evt-mismatch", "wrong-workspace-id");

        let err = append_event(&project_path, &event).unwrap_err();
        assert!(matches!(err, EventError::WorkspaceMismatch));
        assert!(!ledger_dir(&project_path).exists());
    }

    #[test]
    fn append_rejects_invalid_semantics() {
        let (_dir, project_path, project_id) = create_test_project("invalid");
        let mut event = sample_event("evt-invalid", &project_id);
        event.evidence.clear();

        let err = append_event(&project_path, &event).unwrap_err();
        assert!(matches!(err, EventError::InvalidSemantics(_)));
        assert!(!ledger_dir(&project_path).exists());
    }

    // ------------------------------------------------------------------
    // Checkpoint C tests
    // ------------------------------------------------------------------

    #[test]
    fn malformed_json_record_is_quarantined_without_blocking_valid_events() {
        let (_dir, project_path, project_id) = create_test_project("malformed-json");
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        let bad = "{ this is not json";
        drop_raw_ledger_file(&project_path, "evt-bad.json", bad);

        let report = validate_ledger(&project_path).unwrap();
        assert_eq!(report.valid.len(), 1);
        assert_eq!(report.valid[0].event.event_id, "evt-good");
        assert_eq!(report.quarantined.len(), 1);
        assert!(matches!(
            report.quarantined[0].classification,
            LedgerClassification::Malformed(_)
        ));
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 1);
    }

    #[test]
    fn unsupported_protocol_record_is_quarantined_or_safely_classified() {
        let (_dir, project_path, project_id) = create_test_project("unsupported-version");
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        let raw = r#"{
            "eventId": "evt-future",
            "workspaceId": "WRONG",
            "kind": "work.completed",
            "summary": "future version",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [{ "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }],
            "protocolVersion": "999.0",
            "sensitivity": "private"
        }"#;
        drop_raw_ledger_file(&project_path, "evt-future.json", raw);

        let report = validate_ledger(&project_path).unwrap();
        assert_eq!(report.valid.len(), 1);
        assert_eq!(report.quarantined.len(), 1);
        assert_eq!(
            report.quarantined[0].classification,
            LedgerClassification::UnsupportedVersion("999.0".into())
        );
    }

    #[test]
    fn invalid_semantic_record_is_quarantined_or_safely_classified() {
        let (_dir, project_path, project_id) = create_test_project("invalid-semantics");
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        let raw = format!(
            r#"{{
            "eventId": "evt-empty-evidence",
            "workspaceId": "{project_id}",
            "kind": "work.completed",
            "summary": "no evidence",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [],
            "protocolVersion": "1.0",
            "sensitivity": "private"
        }}"#
        );
        drop_raw_ledger_file(&project_path, "evt-empty-evidence.json", &raw);

        let report = validate_ledger(&project_path).unwrap();
        assert_eq!(report.valid.len(), 1);
        assert_eq!(report.quarantined.len(), 1);
        assert!(matches!(
            report.quarantined[0].classification,
            LedgerClassification::InvalidSemantics(_)
        ));
    }

    #[test]
    fn quarantine_preserves_original_bad_record_bytes() {
        let (_dir, project_path, _project_id) = create_test_project("preserve-bytes");
        let original = "{ not-json-preserve-me";
        drop_raw_ledger_file(&project_path, "evt-preserve.json", original);

        let report = validate_ledger(&project_path).unwrap();
        assert_eq!(report.quarantined.len(), 1);
        let preserved = fs::read_to_string(&report.quarantined[0].destination).unwrap();
        assert_eq!(preserved, original);
    }

    #[test]
    fn quarantine_does_not_overwrite_existing_quarantine_record() {
        let (_dir, project_path, _project_id) = create_test_project("no-overwrite");
        ensure_ledger_directories(&project_path).unwrap();
        let existing = "already quarantined bytes";
        fs::write(
            quarantine_dir(&project_path).join("evt-collision.json"),
            existing,
        )
        .unwrap();
        drop_raw_ledger_file(&project_path, "evt-collision.json", "{ bad json");

        let report = validate_ledger(&project_path).unwrap();
        assert_eq!(report.quarantined.len(), 0);
        assert_eq!(report.move_failed.len(), 1);
        assert!(ledger_dir(&project_path)
            .join("evt-collision.json")
            .exists());
        let still =
            fs::read_to_string(quarantine_dir(&project_path).join("evt-collision.json")).unwrap();
        assert_eq!(still, existing);
    }

    #[test]
    fn list_events_ignores_quarantine_directory() {
        let (_dir, project_path, project_id) = create_test_project("ignore-quarantine");
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        ensure_ledger_directories(&project_path).unwrap();
        fs::write(
            quarantine_dir(&project_path).join("phantom.json"),
            r#"{"eventId":"phantom"}"#,
        )
        .unwrap();

        let events = list_events(&project_path).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt-good");
    }

    #[test]
    fn valid_events_still_enumerate_deterministically_after_recovery() {
        let (_dir, project_path, project_id) = create_test_project("recover-order");
        append_event(&project_path, &sample_event("evt-c", &project_id)).unwrap();
        append_event(&project_path, &sample_event("evt-a", &project_id)).unwrap();
        drop_raw_ledger_file(&project_path, "evt-bad.json", "{ bad");
        append_event(&project_path, &sample_event("evt-b", &project_id)).unwrap();

        let events = list_events(&project_path).unwrap();
        let ids: Vec<&str> = events.iter().map(|e| e.event_id.as_str()).collect();
        assert_eq!(ids, vec!["evt-a", "evt-b", "evt-c"]);
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 3);
    }

    #[test]
    fn recovery_does_not_touch_signal_inbox() {
        let (_dir, project_path, project_id) = create_test_project("recovery-no-signals");
        let before = signals_file_count(&project_path);
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        drop_raw_ledger_file(&project_path, "evt-bad.json", "{ bad");

        let _report = validate_ledger(&project_path).unwrap();
        assert_eq!(signals_file_count(&project_path), before);
    }

    #[test]
    fn wrong_workspace_record_is_not_loaded_as_valid_event() {
        let (_dir, project_path, project_id) = create_test_project("wrong-workspace");
        append_event(&project_path, &sample_event("evt-good", &project_id)).unwrap();
        let raw = r#"{
            "eventId": "evt-wrong-ws",
            "workspaceId": "not-the-project-id",
            "kind": "work.completed",
            "summary": "wrong workspace",
            "timestamp": "2026-07-15T07:00:00Z",
            "evidence": [{ "evidenceRef": { "type": "file-path", "value": "docs/a.md" } }],
            "protocolVersion": "1.0",
            "sensitivity": "private"
        }"#;
        drop_raw_ledger_file(&project_path, "evt-wrong-ws.json", raw);

        let events = list_events(&project_path).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, "evt-good");
        assert!(get_event(&project_path, "evt-wrong-ws").unwrap().is_none());
        assert_eq!(
            list_canonical_files(&quarantine_dir(&project_path))
                .unwrap()
                .len(),
            1
        );
    }

    // ------------------------------------------------------------------
    // Checkpoint D tests
    // ------------------------------------------------------------------

    #[test]
    fn correction_event_appends_without_modifying_original() {
        let (_dir, project_path, project_id) = create_test_project("corr-append");
        let original = sample_event("evt-original", &project_id);
        append_event(&project_path, &original).unwrap();
        let original_path = ledger_file_path(&project_path, "evt-original");
        let original_bytes = fs::read(&original_path).unwrap();

        let correction = correction_event(
            "evt-correction",
            &project_id,
            "evt-original",
            "corrected summary",
            "2026-07-15T08:00:00Z",
        );
        append_event(&project_path, &correction).unwrap();

        assert_eq!(fs::read(&original_path).unwrap(), original_bytes);
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 2);
        let restored = get_event(&project_path, "evt-original")
            .unwrap()
            .expect("original still present");
        assert_eq!(restored.summary, "test event evt-original");
    }

    #[test]
    fn correction_requires_existing_target_event() {
        let (_dir, project_path, project_id) = create_test_project("corr-missing-target");
        let correction = correction_event(
            "evt-correction",
            &project_id,
            "evt-missing",
            "corrected",
            "2026-07-15T08:00:00Z",
        );

        let err = append_event(&project_path, &correction).unwrap_err();
        assert!(matches!(
            err,
            EventError::CorrectionTargetNotFound(ref id) if id == "evt-missing"
        ));
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 0);
    }

    #[test]
    fn correction_cannot_correct_itself() {
        let (_dir, project_path, project_id) = create_test_project("corr-self");
        let correction = correction_event(
            "evt-corr",
            &project_id,
            "evt-corr",
            "self correction",
            "2026-07-15T08:00:00Z",
        );

        let err = append_event(&project_path, &correction).unwrap_err();
        assert!(matches!(err, EventError::SelfCorrectionNotAllowed));
        assert_eq!(list_ledger_entries(&project_path).unwrap().len(), 0);
    }

    #[test]
    fn correction_target_is_project_isolated() {
        let (_dir_a, project_a, project_a_id) = create_test_project("corr-iso-a");
        let (_dir_b, project_b, project_b_id) = create_test_project("corr-iso-b");
        append_event(&project_a, &sample_event("evt-only-a", &project_a_id)).unwrap();

        let correction = correction_event(
            "evt-correction-b",
            &project_b_id,
            "evt-only-a",
            "cross-project correction",
            "2026-07-15T08:00:00Z",
        );
        let err = append_event(&project_b, &correction).unwrap_err();
        assert!(matches!(
            err,
            EventError::CorrectionTargetNotFound(ref id) if id == "evt-only-a"
        ));
        assert_eq!(list_ledger_entries(&project_b).unwrap().len(), 0);
    }

    #[test]
    fn list_corrections_for_returns_direct_corrections() {
        let (_dir, project_path, project_id) = create_test_project("corr-list");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(&project_path, &sample_event("evt-unrelated", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-a",
                &project_id,
                "evt-original",
                "first correction",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-b",
                &project_id,
                "evt-original",
                "second correction",
                "2026-07-15T09:00:00Z",
            ),
        )
        .unwrap();

        let corrections = list_corrections_for(&project_path, "evt-original").unwrap();
        assert_eq!(corrections.len(), 2);
        let ids: Vec<&str> = corrections.iter().map(|e| e.event_id.as_str()).collect();
        assert_eq!(ids, vec!["evt-corr-a", "evt-corr-b"]);
        assert!(corrections
            .iter()
            .all(|e| e.corrects_event_id.as_deref() == Some("evt-original")));
    }

    #[test]
    fn list_corrections_for_is_deterministic() {
        let (_dir, project_path, project_id) = create_test_project("corr-deterministic");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-z",
                &project_id,
                "evt-original",
                "z correction",
                "2026-07-15T10:00:00Z",
            ),
        )
        .unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-a",
                &project_id,
                "evt-original",
                "a correction",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();

        let first = list_corrections_for(&project_path, "evt-original").unwrap();
        let second = list_corrections_for(&project_path, "evt-original").unwrap();
        assert_eq!(first, second);
        let ids: Vec<&str> = first.iter().map(|e| e.event_id.as_str()).collect();
        assert_eq!(ids, vec!["evt-corr-a", "evt-corr-z"]);
    }

    #[test]
    fn effective_summary_returns_original_without_correction() {
        let (_dir, project_path, project_id) = create_test_project("eff-original");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();

        let summary = effective_summary(&project_path, "evt-original")
            .unwrap()
            .expect("summary");
        assert_eq!(summary, "test event evt-original");
    }

    #[test]
    fn effective_summary_returns_latest_correction_summary() {
        let (_dir, project_path, project_id) = create_test_project("eff-latest");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-early",
                &project_id,
                "evt-original",
                "earlier correction",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr-late",
                &project_id,
                "evt-original",
                "latest correction",
                "2026-07-15T09:00:00Z",
            ),
        )
        .unwrap();

        let summary = effective_summary(&project_path, "evt-original")
            .unwrap()
            .expect("summary");
        assert_eq!(summary, "latest correction");
    }

    #[test]
    fn correction_helpers_survive_reload() {
        let (_dir, project_path, project_id) = create_test_project("corr-reload");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr",
                &project_id,
                "evt-original",
                "corrected after reload",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();

        let corrections = list_corrections_for(&project_path, "evt-original").unwrap();
        assert_eq!(corrections.len(), 1);
        let summary = effective_summary(&project_path, "evt-original")
            .unwrap()
            .expect("summary");
        assert_eq!(summary, "corrected after reload");
        let original = get_event(&project_path, "evt-original")
            .unwrap()
            .expect("original");
        assert_eq!(original.summary, "test event evt-original");
    }

    #[test]
    fn correction_helpers_ignore_quarantined_or_invalid_records() {
        let (_dir, project_path, project_id) = create_test_project("corr-ignore-bad");
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr",
                &project_id,
                "evt-original",
                "valid correction",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();
        drop_raw_ledger_file(&project_path, "evt-bad.json", "{ bad json");

        let corrections = list_corrections_for(&project_path, "evt-original").unwrap();
        assert_eq!(corrections.len(), 1);
        let summary = effective_summary(&project_path, "evt-original")
            .unwrap()
            .expect("summary");
        assert_eq!(summary, "valid correction");
        assert_eq!(list_events(&project_path).unwrap().len(), 2);
    }

    #[test]
    fn correction_does_not_touch_signal_inbox() {
        let (_dir, project_path, project_id) = create_test_project("corr-no-signals");
        let before = signals_file_count(&project_path);
        append_event(&project_path, &sample_event("evt-original", &project_id)).unwrap();
        append_event(
            &project_path,
            &correction_event(
                "evt-corr",
                &project_id,
                "evt-original",
                "corrected",
                "2026-07-15T08:00:00Z",
            ),
        )
        .unwrap();

        assert_eq!(signals_file_count(&project_path), before);
    }
}
