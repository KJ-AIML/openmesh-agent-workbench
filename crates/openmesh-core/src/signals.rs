// ============================================================================
// OpenMesh File-backed Signal Inbox — Dev Track 0.1.3.2
// ============================================================================
// The official local semantic intake path for WorkSignals. See the approved
// execution plan: .heli-harness/state/reports/openmesh-0.1.3.2-execution-plan.md
//
// Checkpoint B: physical inbox layout + the race-safe write path only.
// Classification (steps 1-8, §8), the project-metadata processing precondition,
// duplicate identity, and replay are Checkpoints C/D — not implemented here.
// ============================================================================

use crate::domain::{
    is_supported_work_signal_protocol, validate_work_signal_semantics, WorkSignal,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Frozen bound (approved plan §3.1): `signal_id` maximum length.
pub const MAX_SIGNAL_ID_BYTES: usize = 256;
/// Frozen bound (approved plan §3.5): `summary` maximum length.
pub const MAX_SUMMARY_BYTES: usize = 4096;
/// Frozen bound (approved plan §3.5/§11): canonical record maximum size.
pub const MAX_RECORD_BYTES: usize = 256 * 1024;

/// Bounded retry count for candidate-filename collisions (approved plan §7).
const MAX_NAME_RESERVATION_ATTEMPTS: u32 = 5;

#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("signal workspace_id does not match the project's id")]
    WorkspaceMismatch,
    #[error("signal failed semantic validation: {0}")]
    InvalidSemantics(String),
    #[error("canonical record exceeds the {max}-byte bound (was {actual} bytes)")]
    RecordTooLarge { actual: usize, max: usize },
    #[error("failed to reserve a unique filename after {0} attempts")]
    NameReservationFailed(u32),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

// ============================================================================
// Physical layout
// ============================================================================

fn signals_root(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("signals")
}

pub(crate) fn pending_dir(project_path: &str) -> PathBuf {
    signals_root(project_path).join("pending")
}

pub(crate) fn processed_dir(project_path: &str) -> PathBuf {
    signals_root(project_path).join("processed")
}

pub(crate) fn quarantine_dir(project_path: &str) -> PathBuf {
    signals_root(project_path).join("quarantine")
}

pub(crate) fn duplicate_dir(project_path: &str) -> PathBuf {
    signals_root(project_path).join("duplicate")
}

fn ensure_lifecycle_directories(project_path: &str) -> std::io::Result<()> {
    for dir in [
        pending_dir(project_path),
        processed_dir(project_path),
        quarantine_dir(project_path),
        duplicate_dir(project_path),
    ] {
        fs::create_dir_all(dir)?;
    }
    Ok(())
}

/// Lists canonical (non-`.tmp`, non-symlink, regular-file) entries in a single
/// lifecycle directory, sorted by filename. Does not classify anything.
pub(crate) fn list_canonical_entries(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "tmp").unwrap_or(false) {
            continue; // interrupted/in-progress write — never a record (§8)
        }
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            continue; // mirrors ingestion.rs's existing symlink-rejection posture
        }
        if metadata.is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

// ============================================================================
// Shared semantic validation (used identically by write_signal here and by
// the classifier in Checkpoint C — approved plan §3.6/§7/§8).
// ============================================================================

pub fn validate_semantics(signal: &WorkSignal) -> Result<(), SignalError> {
    if signal.signal_id.trim().is_empty() {
        return Err(SignalError::InvalidSemantics(
            "signal_id is empty after trim".into(),
        ));
    }
    if signal.signal_id.len() > MAX_SIGNAL_ID_BYTES {
        return Err(SignalError::InvalidSemantics(format!(
            "signal_id exceeds {MAX_SIGNAL_ID_BYTES} bytes"
        )));
    }
    if signal.signal_id.chars().any(|c| c.is_control()) {
        return Err(SignalError::InvalidSemantics(
            "signal_id contains a control character".into(),
        ));
    }

    if signal.summary.trim().is_empty() {
        return Err(SignalError::InvalidSemantics(
            "summary is empty after trim".into(),
        ));
    }
    if signal.summary.len() > MAX_SUMMARY_BYTES {
        return Err(SignalError::InvalidSemantics(format!(
            "summary exceeds {MAX_SUMMARY_BYTES} bytes"
        )));
    }

    validate_timestamp(&signal.timestamp)?;

    validate_work_signal_semantics(signal)
        .map_err(|e| SignalError::InvalidSemantics(e.to_string()))?;

    Ok(())
}

/// `timestamp` must parse as RFC 3339 and represent UTC (`Z` or `+00:00`);
/// the original string is never rewritten (approved plan §3.6).
fn validate_timestamp(timestamp: &str) -> Result<(), SignalError> {
    let parsed = chrono::DateTime::parse_from_rfc3339(timestamp).map_err(|_| {
        SignalError::InvalidSemantics(format!("timestamp is not valid RFC 3339: {timestamp}"))
    })?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(SignalError::InvalidSemantics(format!(
            "timestamp offset must be UTC: {timestamp}"
        )));
    }
    // Closure-audit fix: RFC 3339's "-00:00" designator conventionally means
    // "UTC, but the offset is unknown" — numerically zero seconds, exactly
    // like "+00:00", so `chrono`'s `FixedOffset` cannot distinguish them by
    // value alone. The approved protocol contract accepts only two literal
    // wire forms — `Z` and `+00:00` — and explicitly does not accept
    // `-00:00`; this must be a string-level check, not a numeric one.
    if timestamp.trim_end().ends_with("-00:00") {
        return Err(SignalError::InvalidSemantics(format!(
            "timestamp offset -00:00 is not an approved UTC representation (only Z and +00:00 are): {timestamp}"
        )));
    }
    Ok(())
}

// ============================================================================
// Project-identity precondition (approved plan §3.2/§7 step 1)
// ============================================================================

fn load_project(project_path: &str) -> Result<Project, SignalError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or_else(|| SignalError::ProjectNotInitialized(project_path.to_string()))
}

// ============================================================================
// Race-safe write path (approved plan §7)
// ============================================================================

/// Official write API. Creates nothing under `project_path` unless the
/// project is already initialized (`project.json` loads) and `signal`'s
/// declared `workspace_id` matches it.
pub fn write_signal(project_path: &str, signal: &WorkSignal) -> Result<(), SignalError> {
    write_signal_with_names(project_path, signal, generate_candidate_name)
}

/// Same algorithm as `write_signal`, parameterized over candidate-name
/// generation so tests can force filename collisions deterministically
/// (approved plan §7 Checkpoint B: "forced-collision test via a test-only
/// seam"). Production code always goes through `write_signal` above.
/// Module-private (narrowest visibility that still reaches the child
/// `tests` submodule via `use super::*`) — not `pub(crate)`, since no other
/// module in the crate calls it; never a public production API.
fn write_signal_with_names(
    project_path: &str,
    signal: &WorkSignal,
    mut next_name: impl FnMut() -> String,
) -> Result<(), SignalError> {
    // Step 1: project-identity precondition — nothing is created before this.
    let project = load_project(project_path)?;
    if signal.workspace_id != project.id {
        return Err(SignalError::WorkspaceMismatch);
    }

    // Step 2: shared semantic validation.
    validate_semantics(signal)?;

    // Step 3: serialize once; enforce the size bound before any disk write.
    let payload = serde_json::to_string_pretty(signal)?;
    let payload_len = payload.len();
    if payload_len > MAX_RECORD_BYTES {
        return Err(SignalError::RecordTooLarge {
            actual: payload_len,
            max: MAX_RECORD_BYTES,
        });
    }

    // Step 4: lazily ensure the four lifecycle directories exist.
    ensure_lifecycle_directories(project_path)?;
    let pending = pending_dir(project_path);
    let processed = processed_dir(project_path);
    let quarantine = quarantine_dir(project_path);
    let duplicate = duplicate_dir(project_path);

    for _ in 0..MAX_NAME_RESERVATION_ATTEMPTS {
        let name = next_name();

        // Step 6: exclusive reservation of the temp path.
        let temp_path = pending.join(format!("{name}.tmp"));
        let file = match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(SignalError::Io(e)),
        };

        // Step 7: ordered four-bucket final-name check — pending first, since
        // that closes the writer-vs-single-processor transition case (a
        // record might still be in pending/ or might already be in
        // processed/ by the time this check runs).
        let final_pending = pending.join(format!("{name}.json"));
        let final_processed = processed.join(format!("{name}.json"));
        let final_quarantine = quarantine.join(format!("{name}.json"));
        let final_duplicate = duplicate.join(format!("{name}.json"));
        if final_pending.exists()
            || final_processed.exists()
            || final_quarantine.exists()
            || final_duplicate.exists()
        {
            drop(file);
            let _ = fs::remove_file(&temp_path);
            continue;
        }

        // Step 8/9: write the already-serialized bytes, flush.
        write_all_and_flush(file, &payload)?;

        // Step 10: rename temp -> pending final. Provably free per the proof
        // in the approved plan §7.
        fs::rename(&temp_path, &final_pending)?;
        return Ok(());
    }

    Err(SignalError::NameReservationFailed(
        MAX_NAME_RESERVATION_ATTEMPTS,
    ))
}

fn write_all_and_flush(mut file: fs::File, content: &str) -> Result<(), SignalError> {
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

/// Time-derived, not cryptographically random — matches what
/// `storage::generate_id`/`rand_suffix` already do (approved plan §7).
fn generate_candidate_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{millis}-{suffix:x}")
}

// ============================================================================
// Validation, quarantine, workspace enforcement, duplicate identity, and
// read-only replay (Checkpoints C and D)
// ============================================================================
//
// Implements the full frozen 8-step validation order (approved plan §8).
// Step 1 (filesystem entry filtering) is handled by `list_canonical_entries`
// before `classify_pre_identity` is ever called. Step 8 (duplicate identity)
// is resolved separately (`resolve_identity`), since it needs cross-record
// state `classify_pre_identity` alone doesn't have — both `process_pending`
// and `replay` share the same processed-first, two-phase precedence rule
// (approved plan §9 Part D): Phase 1 establishes accepted identity anchors
// from `processed/` alone; Phase 2 classifies everything else against that
// baseline, never displacing a Phase 1 anchor regardless of filename order.

/// Outcome of validating a single canonical record against the full frozen
/// validation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Classification {
    /// Passed every gate, including duplicate-identity (step 8).
    Valid,
    /// Same `signal_id`, byte-identical canonical payload, as an
    /// already-accepted record (approved plan §9 Part A) — benign.
    Duplicate,
    /// Same `signal_id`, any byte difference, as an already-accepted record
    /// (approved plan §9 Part A) — needs human attention.
    DuplicateConflict,
    /// Failed step 3 (invalid JSON) or step 5 (a current-version record
    /// missing/mistyping a required field, or containing an unrecognized
    /// enum value).
    Malformed(String),
    /// Failed step 4 — `protocolVersion` present and readable, but not one
    /// this build recognizes.
    UnsupportedVersion(String),
    /// Failed step 6 — a bound violation (§3.1/§3.5/§3.6).
    InvalidSemantics(String),
    /// Failed step 7 — `workspace_id` does not match the containing
    /// project's real `Project.id`.
    WrongWorkspace { expected: String, found: String },
}

impl Classification {
    /// The lifecycle bucket this classification routes to — an explicit
    /// family match, never an "anything else" catch-all (approved plan §5).
    fn bucket_name(&self) -> &'static str {
        match self {
            Classification::Valid => "processed",
            Classification::Duplicate | Classification::DuplicateConflict => "duplicate",
            Classification::Malformed(_)
            | Classification::UnsupportedVersion(_)
            | Classification::InvalidSemantics(_)
            | Classification::WrongWorkspace { .. } => "quarantine",
        }
    }
}

/// `signal_id` → canonical raw payload bytes, for records already accepted
/// as identity anchors. Never persisted — rebuilt fresh on every
/// `process_pending`/`replay` call (approved plan §5).
type IdentityMap = std::collections::HashMap<String, String>;

/// Validates a single canonical record file against validation-order steps
/// 2-7 (step 1 is the caller's job via `list_canonical_entries`; step 8 is
/// `resolve_identity`, below, since it needs cross-record state this
/// function doesn't have). Never panics on malformed input — every failure
/// mode is a typed `Classification`, not an `Err`. On success, returns the
/// raw content and parsed signal so the caller can resolve identity (step 8)
/// without re-reading or re-parsing the file.
fn classify_pre_identity(
    path: &Path,
    project: &Project,
) -> Result<(String, WorkSignal), Classification> {
    // Step 2: record-size bound, checked before attempting to parse further.
    let metadata = fs::metadata(path)
        .map_err(|e| Classification::Malformed(format!("cannot stat file: {e}")))?;
    if metadata.len() as usize > MAX_RECORD_BYTES {
        return Err(Classification::InvalidSemantics(format!(
            "record exceeds the {MAX_RECORD_BYTES}-byte bound"
        )));
    }

    let raw = fs::read_to_string(path)
        .map_err(|e| Classification::Malformed(format!("cannot read file: {e}")))?;

    // Step 3: JSON syntax parse.
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| Classification::Malformed(format!("invalid JSON: {e}")))?;

    // Step 4: protocol-version preflight — the one field the loose pass
    // reads (approved plan §8/§10). Missing or non-string is MALFORMED, not
    // UNSUPPORTED_VERSION: there is nothing to preflight against.
    let protocol_version = value
        .get("protocolVersion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Classification::Malformed("missing protocolVersion".into()))?;
    if !is_supported_work_signal_protocol(protocol_version) {
        return Err(Classification::UnsupportedVersion(
            protocol_version.to_string(),
        ));
    }

    // Step 5: strict typed deserialize. An unrecognized enum value under the
    // current, recognized protocol version fails here, as ordinary
    // current-version invalid data (MALFORMED) — never UNSUPPORTED_VERSION.
    let signal: WorkSignal = serde_json::from_str(&raw)
        .map_err(|e| Classification::Malformed(format!("strict deserialize failed: {e}")))?;

    // Step 6: shared semantic validation — the identical function
    // `write_signal` (Checkpoint B) already calls, so the two paths can
    // never disagree about what counts as semantically valid.
    if let Err(SignalError::InvalidSemantics(msg)) = validate_semantics(&signal) {
        return Err(Classification::InvalidSemantics(msg));
    }

    // Step 7: workspace validation.
    if signal.workspace_id != project.id {
        return Err(Classification::WrongWorkspace {
            expected: project.id.clone(),
            found: signal.workspace_id.clone(),
        });
    }

    Ok((raw, signal))
}

/// Step 8 alone: resolves duplicate identity against the accepted-identity
/// map. Does not mutate the map — callers decide when (and whether) to
/// record a new anchor, since a failed move must never claim identity
/// (approved plan §9 Part D).
fn resolve_identity(known: &IdentityMap, signal_id: &str, raw: &str) -> Classification {
    match known.get(signal_id) {
        None => Classification::Valid,
        Some(existing_raw) if existing_raw == raw => Classification::Duplicate,
        Some(_) => Classification::DuplicateConflict,
    }
}

/// Phase 1 of the processed-first two-phase algorithm, shared by
/// `process_pending` and `replay` (approved plan §9 Part D): scans
/// `processed/` alone, in deterministic order, re-validating each record
/// through steps 1-7 before it may seed an identity anchor. A record that
/// fails any gate does not seed identity — it is reported under its real
/// classification (an anomaly; it should never have arrived in `processed/`
/// under normal operation, but this must not crash). Returns the accepted
/// identity map plus a full per-record report of what Phase 1 found.
fn phase1_processed_baseline(
    project_path: &str,
    project: &Project,
) -> std::io::Result<(IdentityMap, Vec<(PathBuf, Classification)>)> {
    let mut known = IdentityMap::new();
    let mut report = Vec::new();

    for path in list_canonical_entries(&processed_dir(project_path))? {
        let classification = match classify_pre_identity(&path, project) {
            Ok((raw, signal)) => {
                let classification = resolve_identity(&known, &signal.signal_id, &raw);
                if classification == Classification::Valid {
                    known.insert(signal.signal_id.clone(), raw);
                }
                classification
            }
            Err(classification) => classification,
        };
        report.push((path, classification));
    }

    Ok((known, report))
}

/// Deterministic ordering key for Phase 2 (approved plan §9 Part B):
/// filename primary key, full relative path secondary tie-break — so two
/// records with the pathological same filename in different buckets still
/// sort deterministically, without an implicit/unstable bucket-priority rule.
fn phase2_sort_key(path: &Path) -> (String, String) {
    let filename = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    let full = path.to_string_lossy().into_owned();
    (filename, full)
}

/// A record whose lifecycle move was refused because the destination already
/// existed — a genuine anomaly (approved plan §7); the source is left
/// untouched in `pending/` for a human to inspect.
#[derive(Debug, Clone)]
pub struct MoveFailure {
    pub source: PathBuf,
    pub classification: Classification,
}

#[derive(Debug, Clone, Default)]
pub struct ProcessSummary {
    /// Records moved to `processed/` this pass (new, post-move location).
    pub valid: Vec<PathBuf>,
    /// Records moved to `duplicate/` this pass (`Duplicate`/`DuplicateConflict`).
    pub duplicates: Vec<(PathBuf, Classification)>,
    /// Records moved to `quarantine/` this pass, with why.
    pub quarantined: Vec<(PathBuf, Classification)>,
    /// Records whose move was refused because the destination already existed.
    pub move_failed: Vec<MoveFailure>,
}

/// Mutating. Phase 1 seeds identity from `processed/` alone; Phase 2 lists
/// `pending/` only, sorted deterministically, and processes one record at a
/// time — when a record classifies `Valid`, the lifecycle move is attempted
/// first, and only *after* it succeeds is the record's identity added to the
/// map (a failed move never claims duplicate identity; this is also what
/// makes same-batch duplicate detection correct, not only cross-run
/// detection). Requires project metadata to load successfully first — if it
/// can't, this is a hard processing error: zero files are moved or
/// classified (approved plan §3.2), never a mass `WRONG_WORKSPACE`
/// misclassification.
pub fn process_pending(project_path: &str) -> Result<ProcessSummary, SignalError> {
    let project = load_project(project_path)?;

    // Phase 1: accepted identity baseline from processed/ alone.
    let (mut known, _phase1_report) = phase1_processed_baseline(project_path, &project)?;

    let pending = pending_dir(project_path);
    let processed = processed_dir(project_path);
    let quarantine = quarantine_dir(project_path);
    let duplicate = duplicate_dir(project_path);

    let mut summary = ProcessSummary::default();

    // Phase 2: pending/ only, one record at a time, deterministic order
    // (list_canonical_entries already sorts by filename, sufficient within
    // a single directory).
    for path in list_canonical_entries(&pending)? {
        let (classification, accepted) = match classify_pre_identity(&path, &project) {
            Ok((raw, signal)) => {
                let classification = resolve_identity(&known, &signal.signal_id, &raw);
                (classification, Some((signal.signal_id, raw)))
            }
            Err(classification) => (classification, None),
        };

        let file_name = path
            .file_name()
            .expect("a listed entry always has a filename")
            .to_owned();
        let destination_dir = match classification.bucket_name() {
            "processed" => &processed,
            "duplicate" => &duplicate,
            "quarantine" => &quarantine,
            other => unreachable!("unknown bucket name: {other}"),
        };
        let destination = destination_dir.join(&file_name);

        if destination.exists() {
            // Non-clobbering move: refuse rather than overwrite an existing
            // canonical record; leave the source untouched (approved plan §7).
            summary.move_failed.push(MoveFailure {
                source: path,
                classification,
            });
            continue;
        }

        fs::create_dir_all(destination_dir)?;
        fs::rename(&path, &destination)?;

        match classification {
            Classification::Valid => {
                // Only after the move succeeds does identity get recorded.
                if let Some((signal_id, raw)) = accepted {
                    known.insert(signal_id, raw);
                }
                summary.valid.push(destination);
            }
            Classification::Duplicate | Classification::DuplicateConflict => {
                summary.duplicates.push((destination, classification));
            }
            other => summary.quarantined.push((destination, other)),
        }
    }

    Ok(summary)
}

/// One record's classification as reconstructed by `replay`.
#[derive(Debug, Clone)]
pub struct ReplayRecord {
    pub path: PathBuf,
    pub classification: Classification,
}

#[derive(Debug, Clone, Default)]
pub struct ReplayReport {
    /// Phase 1 (`processed/`) records first, then Phase 2 records, in the
    /// exact order each phase examined them.
    pub records: Vec<ReplayRecord>,
}

/// Read-only. Reconstructs exactly what `process_pending` already did (or
/// would do) via the identical two-phase, processed-first algorithm — moves
/// no file, mutates no canonical record, creates no sidecar (approved plan
/// §9 Part C/D). Phase 1: `processed/` alone, establishing identity anchors.
/// Phase 2: `pending/` + `quarantine/` + `duplicate/` together, sorted by
/// filename primary / full-path secondary key, classified against the Phase
/// 1 baseline — never displacing an anchor regardless of filename order.
pub fn replay(project_path: &str) -> Result<ReplayReport, SignalError> {
    let project = load_project(project_path)?;

    let (mut known, phase1_report) = phase1_processed_baseline(project_path, &project)?;

    let mut report = ReplayReport::default();
    for (path, classification) in phase1_report {
        report.records.push(ReplayRecord {
            path,
            classification,
        });
    }

    let mut phase2_paths: Vec<PathBuf> = Vec::new();
    for dir in [
        pending_dir(project_path),
        quarantine_dir(project_path),
        duplicate_dir(project_path),
    ] {
        phase2_paths.extend(list_canonical_entries(&dir)?);
    }
    phase2_paths.sort_by_key(|p| phase2_sort_key(p));

    for path in phase2_paths {
        let classification = match classify_pre_identity(&path, &project) {
            Ok((raw, signal)) => {
                let classification = resolve_identity(&known, &signal.signal_id, &raw);
                if classification == Classification::Valid {
                    known.insert(signal.signal_id, raw);
                }
                classification
            }
            Err(classification) => classification,
        };
        report.records.push(ReplayRecord {
            path,
            classification,
        });
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Sensitivity;
    use crate::domain::{ActorRef, EvidenceRef, ProducerRef, WorkSignalKind};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Creates a fresh temp project (with a real `project.json`) and returns
    /// `(temp_dir, project_path, project_id)`. Mirrors the existing
    /// `create_test_project` convention already used by `ingestion.rs`.
    fn create_test_project(name: &str) -> (PathBuf, String, String) {
        let unique = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-signals-test-{name}-{}-{unique}",
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

    fn sample_signal(id: &str, workspace_id: &str) -> WorkSignal {
        WorkSignal {
            signal_id: id.to_string(),
            workspace_id: workspace_id.to_string(),
            producer: ProducerRef::Reporter("codex".into()),
            actor: ActorRef::Unknown,
            kind: WorkSignalKind::Progress,
            summary: format!("test signal {id}"),
            timestamp: "2026-07-08T00:00:00Z".to_string(),
            evidence_refs: vec![EvidenceRef::FilePath("docs/overview.md".into())],
            correlation_hint: None,
            sensitivity: Sensitivity::Private,
            protocol_version: "1.0".to_string(),
        }
    }

    #[test]
    fn write_then_read_raw_round_trip() {
        let (_dir, project_path, project_id) = create_test_project("roundtrip");
        let signal = sample_signal("s-1", &project_id);

        write_signal(&project_path, &signal).expect("write should succeed");

        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(entries.len(), 1);
        let raw = fs::read_to_string(&entries[0]).unwrap();
        let restored: WorkSignal = serde_json::from_str(&raw).unwrap();
        assert_eq!(restored.signal_id, "s-1");
        assert_eq!(restored.workspace_id, project_id);
    }

    #[test]
    fn write_signal_rejects_uninitialized_project() {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-signals-test-uninitialized-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let project_path = dir.to_string_lossy().into_owned();

        let signal = sample_signal("s-1", "whatever");
        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::ProjectNotInitialized(_))));
        assert!(
            !signals_root(&project_path).exists(),
            "no signals/ directory should be created for an uninitialized project"
        );
    }

    #[test]
    fn write_signal_rejects_workspace_mismatch() {
        let (_dir, project_path, project_id) = create_test_project("mismatch");
        let signal = sample_signal("s-1", &format!("{project_id}-WRONG"));

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::WorkspaceMismatch)));
        assert!(
            !pending_dir(&project_path).exists(),
            "no pending/ directory should be created on a rejected write"
        );
    }

    #[test]
    fn write_signal_rejects_oversized_typed_signal_before_any_disk_write() {
        let (_dir, project_path, project_id) = create_test_project("oversized");
        let mut signal = sample_signal("s-1", &project_id);
        // Comfortably exceed 256 KB once serialized — verified by direct
        // measurement, not assumed: each padded evidence ref pretty-prints to
        // well over 100 bytes, so 10,000 of them safely clears the bound.
        signal.summary = "x".repeat(4096);
        signal.evidence_refs = (0..10_000)
            .map(|i| {
                EvidenceRef::FilePath(format!(
                    "docs/some/deeply/nested/padded/path/file-{i:06}-padded-to-be-large.md"
                ))
            })
            .collect();
        let serialized_len = serde_json::to_string_pretty(&signal).unwrap().len();
        assert!(
            serialized_len > MAX_RECORD_BYTES,
            "test fixture must actually exceed the bound; was {serialized_len} bytes"
        );

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::RecordTooLarge { .. })));
        assert!(
            !pending_dir(&project_path).exists(),
            "no pending/ directory should be created for a rejected oversized write"
        );
    }

    #[test]
    fn timestamp_z_form_accepted() {
        let (_dir, project_path, project_id) = create_test_project("ts-z");
        let mut signal = sample_signal("s-1", &project_id);
        signal.timestamp = "2026-07-08T09:15:00Z".to_string();
        write_signal(&project_path, &signal).expect("Z-form timestamp should be accepted");
        // Closure audit: prove acceptance at the classifier path too, not
        // only the official write API.
        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.valid.len(), 1);
    }

    #[test]
    fn timestamp_explicit_utc_offset_accepted() {
        let (_dir, project_path, project_id) = create_test_project("ts-utc-offset");
        let mut signal = sample_signal("s-1", &project_id);
        signal.timestamp = "2026-07-08T09:15:00+00:00".to_string();
        write_signal(&project_path, &signal).expect("+00:00 timestamp should be accepted");
        // Closure audit: prove acceptance at the classifier path too, not
        // only the official write API.
        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.valid.len(), 1);
    }

    #[test]
    fn timestamp_invalid_syntax_rejected() {
        let (_dir, project_path, project_id) = create_test_project("ts-invalid");
        let mut signal = sample_signal("s-1", &project_id);
        signal.timestamp = "not-a-timestamp".to_string();
        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    #[test]
    fn timestamp_non_utc_offset_rejected() {
        let (_dir, project_path, project_id) = create_test_project("ts-non-utc");
        let mut signal = sample_signal("s-1", &project_id);
        signal.timestamp = "2026-07-08T09:15:00-05:00".to_string();
        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    /// Closure-audit addition: `-00:00` is numerically zero (indistinguishable
    /// from `+00:00` via `chrono`'s `FixedOffset` alone), but is not one of
    /// the two approved wire forms (`Z`, `+00:00`) and must be rejected.
    #[test]
    fn timestamp_negative_zero_offset_rejected() {
        let (_dir, project_path, project_id) = create_test_project("ts-negative-zero");
        let mut signal = sample_signal("s-1", &project_id);
        signal.timestamp = "2026-07-08T09:15:00-00:00".to_string();
        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    #[test]
    fn concurrent_writes_produce_distinct_files_with_no_overwrites() {
        let (_dir, project_path, project_id) = create_test_project("concurrent");
        let project_path = std::sync::Arc::new(project_path);
        let project_id = std::sync::Arc::new(project_id);

        let handles: Vec<_> = (0..16)
            .map(|i| {
                let project_path = project_path.clone();
                let project_id = project_id.clone();
                std::thread::spawn(move || {
                    let signal = sample_signal(&format!("s-{i}"), &project_id);
                    write_signal(&project_path, &signal).expect("concurrent write should succeed");
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }

        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(
            entries.len(),
            16,
            "every concurrent write must land as a distinct file"
        );
    }

    #[test]
    fn forced_temp_collision_engages_retry_and_still_lands_distinctly() {
        let (_dir, project_path, project_id) = create_test_project("forced-collision");
        let signal_a = sample_signal("s-a", &project_id);
        let signal_b = sample_signal("s-b", &project_id);

        // First write "wins" a fixed name; the second write's forced sequence
        // deliberately repeats that same name before falling back to a fresh
        // one, proving the create_new-based retry loop actually engages.
        write_signal_with_names(&project_path, &signal_a, || "fixed-name".to_string())
            .expect("first write with the fixed name should succeed");

        let mut forced_names =
            vec!["fixed-name".to_string(), "fixed-name-2".to_string()].into_iter();
        write_signal_with_names(&project_path, &signal_b, move || {
            forced_names.next().unwrap()
        })
        .expect("second write should retry past the collision and succeed");

        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(entries.len(), 2);
        let first_content =
            fs::read_to_string(pending_dir(&project_path).join("fixed-name.json")).unwrap();
        let first: WorkSignal = serde_json::from_str(&first_content).unwrap();
        assert_eq!(
            first.signal_id, "s-a",
            "the original record at the collided name must be untouched"
        );
    }

    #[test]
    fn historical_pending_filename_collision_does_not_overwrite() {
        let (_dir, project_path, project_id) = create_test_project("historical-pending");
        let existing = sample_signal("s-existing", &project_id);
        write_signal_with_names(&project_path, &existing, || "reused-name".to_string())
            .expect("seed write should succeed");

        let newcomer = sample_signal("s-newcomer", &project_id);
        let mut forced_names =
            vec!["reused-name".to_string(), "reused-name-2".to_string()].into_iter();
        write_signal_with_names(&project_path, &newcomer, move || {
            forced_names.next().unwrap()
        })
        .expect("write should retry past the historical pending collision");

        let existing_raw =
            fs::read_to_string(pending_dir(&project_path).join("reused-name.json")).unwrap();
        let existing_restored: WorkSignal = serde_json::from_str(&existing_raw).unwrap();
        assert_eq!(
            existing_restored.signal_id, "s-existing",
            "the pre-existing pending record must remain byte-identical, not overwritten"
        );
        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn processed_filename_collision_does_not_overwrite() {
        let (_dir, project_path, project_id) = create_test_project("processed-collision");
        ensure_lifecycle_directories(&project_path).unwrap();
        let existing_content = r#"{"marker":"existing-processed-record"}"#;
        fs::write(
            processed_dir(&project_path).join("reused-name.json"),
            existing_content,
        )
        .unwrap();

        let newcomer = sample_signal("s-newcomer", &project_id);
        let mut forced_names =
            vec!["reused-name".to_string(), "reused-name-2".to_string()].into_iter();
        write_signal_with_names(&project_path, &newcomer, move || {
            forced_names.next().unwrap()
        })
        .expect("write should retry past the processed collision");

        let untouched =
            fs::read_to_string(processed_dir(&project_path).join("reused-name.json")).unwrap();
        assert_eq!(
            untouched, existing_content,
            "the processed/ record must remain untouched"
        );
    }

    #[test]
    fn quarantine_filename_collision_does_not_overwrite() {
        let (_dir, project_path, project_id) = create_test_project("quarantine-collision");
        ensure_lifecycle_directories(&project_path).unwrap();
        let existing_content = r#"{"marker":"existing-quarantine-record"}"#;
        fs::write(
            quarantine_dir(&project_path).join("reused-name.json"),
            existing_content,
        )
        .unwrap();

        let newcomer = sample_signal("s-newcomer", &project_id);
        let mut forced_names =
            vec!["reused-name".to_string(), "reused-name-2".to_string()].into_iter();
        write_signal_with_names(&project_path, &newcomer, move || {
            forced_names.next().unwrap()
        })
        .expect("write should retry past the quarantine collision");

        let untouched =
            fs::read_to_string(quarantine_dir(&project_path).join("reused-name.json")).unwrap();
        assert_eq!(
            untouched, existing_content,
            "the quarantine/ record must remain untouched"
        );
    }

    #[test]
    fn duplicate_filename_collision_does_not_overwrite() {
        let (_dir, project_path, project_id) = create_test_project("duplicate-collision");
        ensure_lifecycle_directories(&project_path).unwrap();
        let existing_content = r#"{"marker":"existing-duplicate-record"}"#;
        fs::write(
            duplicate_dir(&project_path).join("reused-name.json"),
            existing_content,
        )
        .unwrap();

        let newcomer = sample_signal("s-newcomer", &project_id);
        let mut forced_names =
            vec!["reused-name".to_string(), "reused-name-2".to_string()].into_iter();
        write_signal_with_names(&project_path, &newcomer, move || {
            forced_names.next().unwrap()
        })
        .expect("write should retry past the duplicate collision");

        let untouched =
            fs::read_to_string(duplicate_dir(&project_path).join("reused-name.json")).unwrap();
        assert_eq!(
            untouched, existing_content,
            "the duplicate/ record must remain untouched"
        );
    }

    #[test]
    fn stray_tmp_file_is_ignored_by_the_lister() {
        let (_dir, project_path, _project_id) = create_test_project("stray-tmp");
        ensure_lifecycle_directories(&project_path).unwrap();
        fs::write(pending_dir(&project_path).join("crashed.tmp"), "partial").unwrap();

        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert!(
            entries.is_empty(),
            "a stray .tmp file must never be listed as a record"
        );
    }

    #[test]
    fn symlink_entry_is_ignored_by_the_lister() {
        let (_dir, project_path, _project_id) = create_test_project("symlink");
        ensure_lifecycle_directories(&project_path).unwrap();
        let target = pending_dir(&project_path).join("target.json");
        fs::write(&target, r#"{"marker":"target"}"#).unwrap();
        let link = pending_dir(&project_path).join("link.json");

        #[cfg(windows)]
        let created = std::os::windows::fs::symlink_file(&target, &link).is_ok();
        #[cfg(unix)]
        let created = std::os::unix::fs::symlink(&target, &link).is_ok();

        if created {
            let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
            assert_eq!(
                entries.len(),
                1,
                "the symlink must be ignored; only the real target file counts"
            );
        }
        // If this environment can't create symlinks (no Developer Mode/admin
        // rights on Windows), the test simply doesn't exercise this path —
        // consistent with ingestion.rs's own existing symlink test posture.
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.2, Checkpoint C — validation, quarantine, workspace.
    // ------------------------------------------------------------------

    fn fixture(name: &str) -> String {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let path = format!("{manifest_dir}/tests/fixtures/signals/{name}");
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read fixture {path}: {e}"))
    }

    fn plant_raw(project_path: &str, name: &str, content: &str) -> PathBuf {
        ensure_lifecycle_directories(project_path).unwrap();
        let path = pending_dir(project_path).join(name);
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn malformed_json_beside_valid_json_does_not_block_the_valid_record() {
        let (_dir, project_path, project_id) = create_test_project("malformed-beside-valid");
        write_signal_with_names(
            &project_path,
            &sample_signal("s-valid", &project_id),
            || "aaa-valid".to_string(),
        )
        .unwrap();
        plant_raw(
            &project_path,
            "zzz-malformed.json",
            &fixture("malformed.json"),
        );

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(
            summary.valid.len(),
            1,
            "the valid record must still be accepted"
        );
        assert_eq!(summary.quarantined.len(), 1);
        assert!(matches!(
            summary.quarantined[0].1,
            Classification::Malformed(_)
        ));
        assert!(processed_dir(&project_path).join("aaa-valid.json").exists());
        assert!(quarantine_dir(&project_path)
            .join("zzz-malformed.json")
            .exists());
    }

    #[test]
    fn oversized_raw_record_is_quarantined_as_invalid_semantics() {
        let (_dir, project_path, _project_id) = create_test_project("oversized-raw");
        // A raw file placed directly, bypassing write_signal's own size gate
        // entirely — the classifier must still catch it independently.
        let oversized = format!(
            r#"{{"signalId":"s-1","workspaceId":"ws-1","padding":"{}"}}"#,
            "x".repeat(MAX_RECORD_BYTES + 1024)
        );
        assert!(oversized.len() > MAX_RECORD_BYTES);
        plant_raw(&project_path, "oversized.json", &oversized);

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(matches!(
            summary.quarantined[0].1,
            Classification::InvalidSemantics(_)
        ));
    }

    #[test]
    fn future_protocol_version_is_unsupported_version() {
        let (_dir, project_path, _project_id) = create_test_project("future-version");
        plant_raw(
            &project_path,
            "future.json",
            &fixture("future-version.json"),
        );

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(matches!(
            summary.quarantined[0].1,
            Classification::UnsupportedVersion(ref v) if v == "99.0"
        ));
    }

    #[test]
    fn current_version_unrecognized_kind_is_malformed_not_unsupported_version() {
        let (_dir, project_path, _project_id) = create_test_project("unrecognized-kind");
        plant_raw(
            &project_path,
            "bad-kind.json",
            &fixture("unrecognized-kind.json"),
        );

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(
            matches!(summary.quarantined[0].1, Classification::Malformed(_)),
            "an unrecognized enum value under the CURRENT protocol version must be MALFORMED, \
             never misreported as a future-version compatibility case"
        );
    }

    #[test]
    fn raw_wrong_workspace_record_is_caught_by_the_classifier_independently() {
        let (_dir, project_path, _project_id) = create_test_project("raw-wrong-workspace");
        // Planted directly, bypassing write_signal entirely — proves the
        // classifier trusts nothing about how a record arrived (§3.2).
        plant_raw(
            &project_path,
            "wrong-workspace.json",
            &fixture("wrong-workspace.json"),
        );

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(matches!(
            summary.quarantined[0].1,
            Classification::WrongWorkspace { .. }
        ));
    }

    #[test]
    fn raw_invalid_timestamp_record_is_caught_by_the_shared_validator() {
        let (_dir, project_path, project_id) = create_test_project("raw-invalid-timestamp");
        // The fixture's workspace_id is "ws-1"; rewrite it to this project's
        // real id so ONLY the timestamp rule is under test here.
        let raw =
            fixture("invalid-timestamp.json").replace("\"ws-1\"", &format!("\"{project_id}\""));
        plant_raw(&project_path, "bad-timestamp.json", &raw);

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(
            matches!(summary.quarantined[0].1, Classification::InvalidSemantics(_)),
            "classifier must reject the same non-UTC timestamp write_signal rejects, via the shared validator"
        );
    }

    /// Closure-audit addition: the raw filesystem/classifier path must reject
    /// `-00:00` too, via the same shared validator `write_signal` uses — not
    /// only invalid syntax and a genuine non-UTC offset (already covered
    /// above), but the numerically-zero-but-unapproved `-00:00` form as well.
    #[test]
    fn raw_negative_zero_offset_record_is_caught_by_the_shared_validator() {
        let (_dir, project_path, project_id) = create_test_project("raw-negative-zero");
        let raw = fixture("negative-zero-timestamp.json")
            .replace("\"ws-1\"", &format!("\"{project_id}\""));
        plant_raw(&project_path, "bad-negative-zero.json", &raw);

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.quarantined.len(), 1);
        assert!(
            matches!(
                summary.quarantined[0].1,
                Classification::InvalidSemantics(_)
            ),
            "classifier must reject -00:00 via the same shared validator write_signal uses"
        );
    }

    #[test]
    fn missing_project_metadata_is_a_hard_processing_error_with_zero_files_touched() {
        let (_dir, project_path, project_id) = create_test_project("missing-metadata");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "will-not-move".to_string()
        })
        .unwrap();

        // Remove project.json entirely after the signal was written.
        fs::remove_file(
            Path::new(&project_path)
                .join(".openmesh")
                .join("project.json"),
        )
        .unwrap();

        let result = process_pending(&project_path);
        assert!(matches!(result, Err(SignalError::ProjectNotInitialized(_))));
        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "the pending record must be left completely untouched"
        );
        assert!(
            !processed_dir(&project_path)
                .join("will-not-move.json")
                .exists(),
            "nothing may be classified or moved when project metadata is unavailable"
        );
    }

    #[test]
    fn non_clobbering_lifecycle_move_refuses_to_overwrite_an_existing_destination() {
        let (_dir, project_path, project_id) = create_test_project("non-clobbering-move");
        ensure_lifecycle_directories(&project_path).unwrap();
        let existing_content = r#"{"marker":"pre-existing-processed-record"}"#;
        fs::write(
            processed_dir(&project_path).join("colliding-name.json"),
            existing_content,
        )
        .unwrap();

        // Plant the pending record directly (bypassing write_signal, whose
        // own write-time collision check — Checkpoint B — would otherwise
        // legitimately refuse this exact name). This isolates
        // process_pending's own move-time non-clobbering check, which
        // exists to defend against the genuine anomaly of an untrusted or
        // manually-placed file already occupying the destination.
        let valid_payload =
            serde_json::to_string_pretty(&sample_signal("s-newcomer", &project_id)).unwrap();
        plant_raw(&project_path, "colliding-name.json", &valid_payload);

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.valid.len(), 0);
        assert_eq!(summary.move_failed.len(), 1);
        assert!(matches!(
            summary.move_failed[0].classification,
            Classification::Valid
        ));

        let untouched =
            fs::read_to_string(processed_dir(&project_path).join("colliding-name.json")).unwrap();
        assert_eq!(
            untouched, existing_content,
            "the pre-existing destination must be untouched"
        );
        assert!(
            pending_dir(&project_path)
                .join("colliding-name.json")
                .exists(),
            "the source record must be left in pending/ for a human to inspect"
        );
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.2, Checkpoint D — duplicate identity, read-only replay.
    // ------------------------------------------------------------------

    fn snapshot_dir(dir: &Path) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = list_canonical_entries(dir)
            .unwrap()
            .into_iter()
            .map(|p| {
                let name = p.file_name().unwrap().to_string_lossy().into_owned();
                let content = fs::read_to_string(&p).unwrap();
                (name, content)
            })
            .collect();
        out.sort();
        out
    }

    fn full_signals_snapshot(project_path: &str) -> Vec<(String, String)> {
        let mut out = Vec::new();
        for dir in [
            pending_dir(project_path),
            processed_dir(project_path),
            quarantine_dir(project_path),
            duplicate_dir(project_path),
        ] {
            out.extend(snapshot_dir(&dir));
        }
        out.sort();
        out
    }

    #[test]
    fn same_signal_id_byte_identical_content_is_duplicate() {
        let (_dir, project_path, project_id) = create_test_project("dup-identical");
        let signal = sample_signal("s-shared", &project_id);

        write_signal_with_names(&project_path, &signal, || "aaa-first".to_string()).unwrap();
        process_pending(&project_path).unwrap();

        write_signal_with_names(&project_path, &signal, || "zzz-second".to_string()).unwrap();
        let summary = process_pending(&project_path).unwrap();

        assert_eq!(summary.valid.len(), 0);
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::Duplicate);
    }

    #[test]
    fn same_signal_id_any_byte_difference_is_duplicate_conflict() {
        let (_dir, project_path, project_id) = create_test_project("dup-conflict");
        let mut first = sample_signal("s-shared", &project_id);
        first.summary = "first version of the claim".to_string();
        let mut second = first.clone();
        second.summary = "a differently-worded version of the same claim".to_string();

        write_signal_with_names(&project_path, &first, || "aaa-first".to_string()).unwrap();
        process_pending(&project_path).unwrap();

        write_signal_with_names(&project_path, &second, || "zzz-second".to_string()).unwrap();
        let summary = process_pending(&project_path).unwrap();

        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::DuplicateConflict);
    }

    #[test]
    fn added_unknown_field_on_an_otherwise_identical_record_is_duplicate_conflict() {
        // Documented, accepted limitation (approved plan §9 Part A): byte
        // comparison is exact, so a manually-inserted extra field makes an
        // otherwise-"same" signal classify as DUPLICATE_CONFLICT, not DUPLICATE.
        let (_dir, project_path, project_id) = create_test_project("dup-extra-field");
        let signal = sample_signal("s-shared", &project_id);
        write_signal_with_names(&project_path, &signal, || "aaa-first".to_string()).unwrap();
        process_pending(&project_path).unwrap();

        let original_raw =
            fs::read_to_string(processed_dir(&project_path).join("aaa-first.json")).unwrap();
        let with_extra_field =
            original_raw.replacen('{', "{\n  \"extraField\": \"unexpected\",", 1);
        plant_raw(&project_path, "zzz-second.json", &with_extra_field);

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::DuplicateConflict);
    }

    #[test]
    fn same_batch_duplicate_is_resolved_deterministically() {
        let (_dir, project_path, project_id) = create_test_project("same-batch-dup");
        let signal = sample_signal("s-shared", &project_id);
        // Both records land in pending/ in the SAME batch, before any
        // process_pending call — "aaa" sorts before "zzz".
        write_signal_with_names(&project_path, &signal, || "aaa-first".to_string()).unwrap();
        write_signal_with_names(&project_path, &signal, || "zzz-second".to_string()).unwrap();

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(
            summary.valid.len(),
            1,
            "the first-in-order record becomes the anchor"
        );
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::Duplicate);
        assert!(processed_dir(&project_path).join("aaa-first.json").exists());
        assert!(duplicate_dir(&project_path)
            .join("zzz-second.json")
            .exists());
    }

    #[test]
    fn same_batch_conflict_is_resolved_deterministically() {
        let (_dir, project_path, project_id) = create_test_project("same-batch-conflict");
        let mut first = sample_signal("s-shared", &project_id);
        first.summary = "first version".to_string();
        let mut second = first.clone();
        second.summary = "conflicting version".to_string();

        write_signal_with_names(&project_path, &first, || "aaa-first".to_string()).unwrap();
        write_signal_with_names(&project_path, &second, || "zzz-second".to_string()).unwrap();

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.valid.len(), 1);
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::DuplicateConflict);
    }

    #[test]
    fn failed_move_does_not_claim_identity_for_a_later_colliding_record() {
        let (_dir, project_path, project_id) = create_test_project("failed-move-no-claim");
        ensure_lifecycle_directories(&project_path).unwrap();
        let signal = sample_signal("s-shared", &project_id);

        // Both land in pending/ in the same batch; "aaa-first" sorts first.
        write_signal_with_names(&project_path, &signal, || "aaa-first".to_string()).unwrap();
        write_signal_with_names(&project_path, &signal, || "zzz-second".to_string()).unwrap();

        // Force "aaa-first"'s move to processed/ to fail by pre-occupying its
        // destination with unrelated content.
        fs::write(
            processed_dir(&project_path).join("aaa-first.json"),
            r#"{"marker":"unrelated-pre-existing-record"}"#,
        )
        .unwrap();

        let summary = process_pending(&project_path).unwrap();

        assert_eq!(summary.move_failed.len(), 1);
        assert_eq!(summary.move_failed[0].classification, Classification::Valid);

        // The second, colliding record must NOT be misclassified as a
        // duplicate of the first — since the first's move failed, its
        // identity was never recorded, so the second is evaluated fresh
        // and becomes the accepted anchor itself.
        assert_eq!(
            summary.valid.len(),
            1,
            "the second record must become VALID, not wrongly classified as a duplicate"
        );
        assert!(processed_dir(&project_path)
            .join("zzz-second.json")
            .exists());
    }

    #[test]
    fn processed_anchor_precedence_outranks_an_earlier_pending_filename() {
        let (_dir, project_path, project_id) = create_test_project("processed-anchor-precedence");
        ensure_lifecycle_directories(&project_path).unwrap();
        let signal = sample_signal("s-shared", &project_id);
        let payload = serde_json::to_string_pretty(&signal).unwrap();

        // Plant the accepted original directly in processed/ with a filename
        // that sorts LATER than the pending retry, to isolate the precedence
        // rule from filename ordering entirely.
        fs::write(
            processed_dir(&project_path).join("200-original.json"),
            &payload,
        )
        .unwrap();
        // A lexicographically-earlier retry, byte-identical content, sitting
        // in pending/.
        fs::write(pending_dir(&project_path).join("100-retry.json"), &payload).unwrap();

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(
            summary.valid.len(),
            0,
            "the pre-existing processed/ record is already the anchor"
        );
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::Duplicate);
        assert!(duplicate_dir(&project_path).join("100-retry.json").exists());

        // Replay must report the identical precedence, never reversed by
        // the retry's lexicographically-earlier filename.
        let report = replay(&project_path).unwrap();
        let original = report
            .records
            .iter()
            .find(|r| r.path.ends_with("200-original.json"))
            .unwrap();
        let retry = report
            .records
            .iter()
            .find(|r| r.path.ends_with("100-retry.json"))
            .unwrap();
        assert_eq!(original.classification, Classification::Valid);
        assert_eq!(retry.classification, Classification::Duplicate);
    }

    #[test]
    fn processed_conflict_precedence_outranks_an_earlier_pending_filename() {
        let (_dir, project_path, project_id) = create_test_project("processed-conflict-precedence");
        ensure_lifecycle_directories(&project_path).unwrap();
        let mut original_signal = sample_signal("s-shared", &project_id);
        original_signal.summary = "the accepted original claim".to_string();
        let mut retry_signal = original_signal.clone();
        retry_signal.summary = "a conflicting later retry".to_string();

        fs::write(
            processed_dir(&project_path).join("200-original.json"),
            serde_json::to_string_pretty(&original_signal).unwrap(),
        )
        .unwrap();
        fs::write(
            pending_dir(&project_path).join("100-retry.json"),
            serde_json::to_string_pretty(&retry_signal).unwrap(),
        )
        .unwrap();

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.duplicates.len(), 1);
        assert_eq!(summary.duplicates[0].1, Classification::DuplicateConflict);

        let report = replay(&project_path).unwrap();
        let original = report
            .records
            .iter()
            .find(|r| r.path.ends_with("200-original.json"))
            .unwrap();
        let retry = report
            .records
            .iter()
            .find(|r| r.path.ends_with("100-retry.json"))
            .unwrap();
        assert_eq!(original.classification, Classification::Valid);
        assert_eq!(retry.classification, Classification::DuplicateConflict);
    }

    #[test]
    fn invalid_processed_record_does_not_claim_identity() {
        let (_dir, project_path, project_id) = create_test_project("invalid-processed-no-claim");
        ensure_lifecycle_directories(&project_path).unwrap();

        // A malformed record sitting directly in processed/ (an anomaly —
        // should never happen via normal operation) whose signal_id
        // conceptually collides with a genuinely valid pending record.
        fs::write(
            processed_dir(&project_path).join("bad-anomaly.json"),
            &fixture("malformed.json"),
        )
        .unwrap();

        let signal = sample_signal("s-shared", &project_id);
        write_signal_with_names(&project_path, &signal, || "genuinely-valid".to_string()).unwrap();

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(
            summary.valid.len(),
            1,
            "the invalid processed/ record must not have seeded identity, \
             so the genuinely valid pending record becomes the accepted anchor"
        );

        let report = replay(&project_path).unwrap();
        let anomaly = report
            .records
            .iter()
            .find(|r| r.path.ends_with("bad-anomaly.json"))
            .unwrap();
        assert!(matches!(
            anomaly.classification,
            Classification::Malformed(_)
        ));
        // Replay never moves anything — the anomaly must still be sitting in processed/.
        assert!(processed_dir(&project_path)
            .join("bad-anomaly.json")
            .exists());
    }

    #[test]
    fn repeated_replay_produces_an_identical_ordered_report() {
        let (_dir, project_path, project_id) = create_test_project("repeated-replay");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "sig-1".to_string()
        })
        .unwrap();
        write_signal_with_names(&project_path, &sample_signal("s-2", &project_id), || {
            "sig-2".to_string()
        })
        .unwrap();
        plant_raw(&project_path, "bad.json", &fixture("malformed.json"));
        process_pending(&project_path).unwrap();

        let before = full_signals_snapshot(&project_path);
        let report_a = replay(&project_path).unwrap();
        let report_b = replay(&project_path).unwrap();
        let after = full_signals_snapshot(&project_path);

        assert_eq!(
            report_a.records.len(),
            report_b.records.len(),
            "repeated replay must report the same number of records"
        );
        for (a, b) in report_a.records.iter().zip(report_b.records.iter()) {
            assert_eq!(a.path, b.path);
            assert_eq!(a.classification, b.classification);
        }
        assert_eq!(
            before, after,
            "replay must not move or mutate any file, ever"
        );
    }

    #[test]
    fn replay_does_not_mutate_any_file() {
        let (_dir, project_path, project_id) = create_test_project("replay-no-mutate");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "sig-1".to_string()
        })
        .unwrap();
        plant_raw(
            &project_path,
            "future.json",
            &fixture("future-version.json"),
        );

        let before = full_signals_snapshot(&project_path);
        let before_pending = list_canonical_entries(&pending_dir(&project_path))
            .unwrap()
            .len();
        replay(&project_path).unwrap();
        let after = full_signals_snapshot(&project_path);
        let after_pending = list_canonical_entries(&pending_dir(&project_path))
            .unwrap()
            .len();

        assert_eq!(before, after, "replay must change zero content");
        assert_eq!(
            before_pending, after_pending,
            "replay must move zero files out of pending/"
        );
    }

    #[test]
    fn replay_matches_process_pending_accepted_history_semantics() {
        let (_dir, project_path, project_id) = create_test_project("replay-matches-process");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "sig-1".to_string()
        })
        .unwrap();
        write_signal_with_names(
            &project_path,
            &sample_signal("s-1", &project_id), // same id, same content -> duplicate
            || "sig-1-retry".to_string(),
        )
        .unwrap();
        plant_raw(&project_path, "bad.json", &fixture("malformed.json"));

        let summary = process_pending(&project_path).unwrap();
        let report = replay(&project_path).unwrap();

        for path in &summary.valid {
            let found = report
                .records
                .iter()
                .find(|r| &r.path == path)
                .unwrap_or_else(|| panic!("replay must report every processed path: {path:?}"));
            assert_eq!(found.classification, Classification::Valid);
        }
        for (path, classification) in &summary.duplicates {
            let found = report.records.iter().find(|r| &r.path == path).unwrap();
            assert_eq!(&found.classification, classification);
        }
        for (path, classification) in &summary.quarantined {
            let found = report.records.iter().find(|r| &r.path == path).unwrap();
            assert_eq!(&found.classification, classification);
        }
    }

    #[test]
    fn two_separate_process_pending_calls_simulate_desktop_close_reopen() {
        let (_dir, project_path, project_id) = create_test_project("close-reopen");
        let signal = sample_signal("s-1", &project_id);

        // "Session 1": Desktop open, one signal arrives and is processed.
        write_signal_with_names(&project_path, &signal, || "session-1".to_string()).unwrap();
        let first_pass = process_pending(&project_path).unwrap();
        assert_eq!(first_pass.valid.len(), 1);

        // "Desktop closed" — nothing runs. A retry with the same signal_id
        // and identical content arrives later.
        write_signal_with_names(&project_path, &signal, || "session-2-retry".to_string()).unwrap();

        // "Session 2": Desktop reopens, processes again.
        let second_pass = process_pending(&project_path).unwrap();
        assert_eq!(
            second_pass.duplicates.len(),
            1,
            "cross-run duplicate detection must see the first session's accepted signal"
        );
        assert_eq!(second_pass.duplicates[0].1, Classification::Duplicate);
    }

    // ------------------------------------------------------------------
    // Dev Track 0.1.3.2, Checkpoint E — recovery / project-isolation
    // hardening. No new classification behavior is introduced here — every
    // test below exercises behavior already implemented in Checkpoints B-D
    // under adversarial/edge-case conditions.
    // ------------------------------------------------------------------

    #[test]
    fn two_real_project_inboxes_remain_fully_isolated() {
        let (_dir_a, project_a, project_a_id) = create_test_project("isolation-a");
        let (_dir_b, project_b, project_b_id) = create_test_project("isolation-b");

        write_signal(&project_a, &sample_signal("s-1", &project_a_id)).unwrap();
        process_pending(&project_a).unwrap();

        assert!(
            !signals_root(&project_b).exists(),
            "project B's entire signals/ tree must never be created or touched \
             by anything that happened in project A"
        );

        // Writing to B afterward must work normally and stay confined to B.
        write_signal(&project_b, &sample_signal("s-in-b", &project_b_id)).unwrap();
        let summary_b = process_pending(&project_b).unwrap();
        assert_eq!(summary_b.valid.len(), 1);
        assert_eq!(
            list_canonical_entries(&processed_dir(&project_a))
                .unwrap()
                .len(),
            1,
            "project A's own processed/ must be unaffected by project B's activity"
        );
    }

    #[test]
    fn corrupted_project_metadata_is_a_hard_processing_error_with_zero_files_touched() {
        let (_dir, project_path, project_id) = create_test_project("corrupted-metadata");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "will-not-move".to_string()
        })
        .unwrap();

        // Corrupt (not delete) project.json — genuinely invalid JSON, not merely absent.
        fs::write(
            Path::new(&project_path)
                .join(".openmesh")
                .join("project.json"),
            "{ this is not valid json at all",
        )
        .unwrap();

        let result = process_pending(&project_path);
        assert!(matches!(result, Err(SignalError::ProjectNotInitialized(_))));
        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "the pending record must be left completely untouched"
        );
        assert!(!processed_dir(&project_path)
            .join("will-not-move.json")
            .exists());
        assert!(!quarantine_dir(&project_path)
            .join("will-not-move.json")
            .exists());
        assert!(!duplicate_dir(&project_path)
            .join("will-not-move.json")
            .exists());

        // replay() must have the identical hard-fail posture.
        let replay_result = replay(&project_path);
        assert!(matches!(
            replay_result,
            Err(SignalError::ProjectNotInitialized(_))
        ));
    }

    #[test]
    fn genuinely_truncated_temp_file_is_ignored_and_causes_no_error() {
        let (_dir, project_path, project_id) = create_test_project("truncated-temp");
        write_signal_with_names(&project_path, &sample_signal("s-1", &project_id), || {
            "sig-1".to_string()
        })
        .unwrap();
        // Simulate a crash mid-write: a .tmp file with genuinely partial,
        // syntactically-incomplete JSON content (not just any .tmp file).
        fs::write(
            pending_dir(&project_path).join("crashed-mid-write.tmp"),
            r#"{"signalId": "s-crashed", "workspaceId": "ws-1", "produc"#,
        )
        .unwrap();

        let entries = list_canonical_entries(&pending_dir(&project_path)).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "the truncated .tmp file must never be listed"
        );

        let summary = process_pending(&project_path).unwrap();
        assert_eq!(summary.valid.len(), 1);
        assert_eq!(summary.quarantined.len(), 0);
        assert_eq!(summary.duplicates.len(), 0);
        assert!(
            pending_dir(&project_path)
                .join("crashed-mid-write.tmp")
                .exists(),
            "the stray temp file is harmless and simply left in place, never processed"
        );
    }

    #[test]
    fn control_character_in_signal_id_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("control-char-id");
        let mut signal = sample_signal("s-1", &project_id);
        signal.signal_id = "s-1\u{0007}bell".to_string();

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
        assert!(!pending_dir(&project_path).exists());
    }

    #[test]
    fn oversized_signal_id_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("oversized-signal-id");
        let mut signal = sample_signal("s-1", &project_id);
        signal.signal_id = "s".repeat(MAX_SIGNAL_ID_BYTES + 1);

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    #[test]
    fn signal_id_at_exactly_the_bound_is_accepted() {
        let (_dir, project_path, project_id) = create_test_project("signal-id-boundary");
        let mut signal = sample_signal("s-1", &project_id);
        signal.signal_id = "s".repeat(MAX_SIGNAL_ID_BYTES);
        assert_eq!(signal.signal_id.len(), MAX_SIGNAL_ID_BYTES);

        write_signal(&project_path, &signal).expect("exactly-at-bound signal_id must be accepted");
    }

    #[test]
    fn empty_signal_id_after_trim_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("empty-signal-id");
        let mut signal = sample_signal("s-1", &project_id);
        signal.signal_id = "   ".to_string();

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    #[test]
    fn summary_at_exactly_the_bound_is_accepted() {
        let (_dir, project_path, project_id) = create_test_project("summary-boundary");
        let mut signal = sample_signal("s-1", &project_id);
        signal.summary = "x".repeat(MAX_SUMMARY_BYTES);
        assert_eq!(signal.summary.len(), MAX_SUMMARY_BYTES);

        write_signal(&project_path, &signal).expect("exactly-at-bound summary must be accepted");
    }

    #[test]
    fn summary_one_byte_over_the_bound_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("summary-over-boundary");
        let mut signal = sample_signal("s-1", &project_id);
        signal.summary = "x".repeat(MAX_SUMMARY_BYTES + 1);

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    #[test]
    fn empty_summary_after_trim_is_rejected() {
        let (_dir, project_path, project_id) = create_test_project("empty-summary");
        let mut signal = sample_signal("s-1", &project_id);
        signal.summary = "   ".to_string();

        let result = write_signal(&project_path, &signal);
        assert!(matches!(result, Err(SignalError::InvalidSemantics(_))));
    }

    // ------------------------------------------------------------------
    // Closure audit addition — required proof coverage (approved plan §11):
    // a Sensitivity::Secret signal enters the inbox and is processed
    // normally, with no special-cased routing. `signals.rs` has zero
    // dependency edge to `index.rs` (the context search index) — verified
    // structurally: `grep -n "use crate::index" crates/openmesh-core/src/signals.rs`
    // returns nothing, which is a stronger guarantee than a runtime test
    // could provide, since it is impossible for this module to call into
    // the search-indexing pipeline at all, accidentally or otherwise.
    // ------------------------------------------------------------------

    #[test]
    fn secret_sensitivity_signal_enters_the_inbox_and_processes_normally() {
        let (_dir, project_path, project_id) = create_test_project("secret-sensitivity");
        let mut signal = sample_signal("s-secret", &project_id);
        signal.sensitivity = Sensitivity::Secret;

        write_signal(&project_path, &signal).expect("a Secret-sensitivity signal must be writable");
        let summary = process_pending(&project_path).unwrap();
        assert_eq!(
            summary.valid.len(),
            1,
            "a Secret-sensitivity signal must classify and move exactly like any other valid signal"
        );

        let restored: WorkSignal =
            serde_json::from_str(&fs::read_to_string(&summary.valid[0]).unwrap()).unwrap();
        assert_eq!(
            restored.sensitivity,
            Sensitivity::Secret,
            "sensitivity must be preserved exactly, not stripped or downgraded by processing"
        );
    }
}
