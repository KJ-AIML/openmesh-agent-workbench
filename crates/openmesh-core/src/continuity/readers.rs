//! Read-only loaders for local continuity inputs (Dev Track 0.1.3.7 Checkpoint B).
//!
//! These APIs never call `process_pending`, `replay`, `write_signal`, `append_event`,
//! or `apply_promotion_decision`. Malformed on-disk records are surfaced as
//! diagnostics instead of panicking.

use crate::domain::{
    validate_work_signal_semantics, ProducerRef, SourceCounts, WorkEvent, WorkSignal,
};
use crate::events::{self, classify_ledger_record, LedgerClassification};
use crate::promotion::{self, classify_decision_record, PromotionDecisionRecord};
use crate::signals::{
    duplicate_dir, list_canonical_entries, pending_dir, processed_dir, quarantine_dir,
};
use crate::storage::{get_project_dir, read_project, Project};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

fn format_ledger_classification(classification: LedgerClassification) -> String {
    match classification {
        LedgerClassification::Valid => "valid".into(),
        LedgerClassification::Malformed(msg) => format!("malformed: {msg}"),
        LedgerClassification::UnsupportedVersion(version) => {
            format!("unsupported protocol version: {version}")
        }
        LedgerClassification::InvalidSemantics(msg) => format!("invalid semantics: {msg}"),
        LedgerClassification::WrongWorkspace { expected, found } => {
            format!("workspace mismatch: expected {expected}, found {found}")
        }
    }
}

/// Which signal inbox bucket a record was loaded from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignalBucket {
    Pending,
    Processed,
    Quarantine,
    Duplicate,
}

/// One read-only diagnostic from a continuity loader.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityDiagnostic {
    pub kind: ContinuityDiagnosticKind,
    pub location: String,
    pub message: String,
}

/// Category for a continuity loader diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ContinuityDiagnosticKind {
    SignalBucket,
    WorkEventLedger,
    PromotionAudit,
}

/// Valid WorkSignals loaded from one inbox bucket plus any parse/validation diagnostics.
#[derive(Debug, Clone)]
pub struct LoadedSignalBucket {
    pub bucket: SignalBucket,
    pub signals: Vec<WorkSignal>,
    pub diagnostics: Vec<ContinuityDiagnostic>,
}

/// WorkEvents loaded read-only from the ledger plus diagnostics for invalid records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedWorkEvents {
    pub events: Vec<WorkEvent>,
    pub diagnostics: Vec<ContinuityDiagnostic>,
}

/// Promotion audit records loaded read-only plus diagnostics for invalid files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedPromotionAudit {
    pub records: Vec<PromotionDecisionRecord>,
    pub diagnostics: Vec<ContinuityDiagnostic>,
}

/// Unified read-only snapshot of all continuity inputs for builders (Checkpoint C/D).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuityInputSnapshot {
    pub workspace_id: String,
    pub loaded_at: String,
    pub pending_signals: Vec<WorkSignal>,
    pub processed_signals: Vec<WorkSignal>,
    pub quarantine_signals: Vec<WorkSignal>,
    pub duplicate_signals: Vec<WorkSignal>,
    pub work_events: Vec<WorkEvent>,
    pub promotion_audit_records: Vec<PromotionDecisionRecord>,
    pub diagnostics: Vec<ContinuityDiagnostic>,
    pub source_counts: SourceCounts,
}

#[derive(Debug, thiserror::Error)]
pub enum ContinuityReaderError {
    #[error("project not initialized at {0}")]
    ProjectNotInitialized(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("event ledger error: {0}")]
    EventLedger(#[from] events::EventError),
    #[error("promotion audit error: {0}")]
    PromotionAudit(#[from] promotion::PromotionAuditError),
}

fn load_project(project_path: &str) -> Result<Project, ContinuityReaderError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or_else(|| ContinuityReaderError::ProjectNotInitialized(project_path.to_string()))
}

fn bucket_dir(project_path: &str, bucket: SignalBucket) -> PathBuf {
    match bucket {
        SignalBucket::Pending => pending_dir(project_path),
        SignalBucket::Processed => processed_dir(project_path),
        SignalBucket::Quarantine => quarantine_dir(project_path),
        SignalBucket::Duplicate => duplicate_dir(project_path),
    }
}

fn list_canonical_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    list_canonical_entries(dir)
}

fn utc_loaded_at() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn signal_diagnostic(
    bucket: SignalBucket,
    location: &str,
    message: impl Into<String>,
) -> ContinuityDiagnostic {
    ContinuityDiagnostic {
        kind: ContinuityDiagnosticKind::SignalBucket,
        location: format!("{bucket:?}:{location}"),
        message: message.into(),
    }
}

/// Lists valid WorkSignals from one inbox bucket in deterministic filename order.
pub fn list_signal_bucket(
    project_path: &str,
    bucket: SignalBucket,
) -> Result<LoadedSignalBucket, ContinuityReaderError> {
    let project = load_project(project_path)?;
    let dir = bucket_dir(project_path, bucket);
    let mut signals = Vec::new();
    let mut diagnostics = Vec::new();

    for path in list_canonical_files(&dir)? {
        let filename = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        let raw = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(err) => {
                diagnostics.push(signal_diagnostic(
                    bucket,
                    &filename,
                    format!("cannot read file: {err}"),
                ));
                continue;
            }
        };
        let signal: WorkSignal = match serde_json::from_str(&raw) {
            Ok(signal) => signal,
            Err(err) => {
                diagnostics.push(signal_diagnostic(
                    bucket,
                    &filename,
                    format!("invalid JSON: {err}"),
                ));
                continue;
            }
        };
        if signal.workspace_id != project.id {
            diagnostics.push(signal_diagnostic(
                bucket,
                &filename,
                format!(
                    "workspace_id mismatch: expected {}, found {}",
                    project.id, signal.workspace_id
                ),
            ));
            continue;
        }
        if let Err(err) = validate_work_signal_semantics(&signal) {
            diagnostics.push(signal_diagnostic(
                bucket,
                &filename,
                format!("semantic validation failed: {err}"),
            ));
            continue;
        }
        signals.push(signal);
    }

    Ok(LoadedSignalBucket {
        bucket,
        signals,
        diagnostics,
    })
}

/// Lists valid pending WorkSignals (read-only).
pub fn list_pending_signals(
    project_path: &str,
) -> Result<LoadedSignalBucket, ContinuityReaderError> {
    list_signal_bucket(project_path, SignalBucket::Pending)
}

/// Lists valid processed WorkSignals (read-only).
pub fn list_processed_signals(
    project_path: &str,
) -> Result<LoadedSignalBucket, ContinuityReaderError> {
    list_signal_bucket(project_path, SignalBucket::Processed)
}

/// Lists valid quarantined WorkSignals (read-only).
pub fn list_quarantine_signals(
    project_path: &str,
) -> Result<LoadedSignalBucket, ContinuityReaderError> {
    list_signal_bucket(project_path, SignalBucket::Quarantine)
}

/// Lists valid duplicate WorkSignals (read-only).
pub fn list_duplicate_signals(
    project_path: &str,
) -> Result<LoadedSignalBucket, ContinuityReaderError> {
    list_signal_bucket(project_path, SignalBucket::Duplicate)
}

/// Loads WorkEvents read-only using `classify_ledger_record` — does not quarantine files.
pub fn load_work_events(project_path: &str) -> Result<LoadedWorkEvents, ContinuityReaderError> {
    let project = load_project(project_path)?;
    let mut events = Vec::new();
    let mut diagnostics = Vec::new();

    for path in events::list_ledger_entries(project_path)? {
        let location = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        match classify_ledger_record(&path, &project) {
            Ok((_, event)) => events.push(event),
            Err(classification) => diagnostics.push(ContinuityDiagnostic {
                kind: ContinuityDiagnosticKind::WorkEventLedger,
                location,
                message: format_ledger_classification(classification),
            }),
        }
    }

    Ok(LoadedWorkEvents {
        events,
        diagnostics,
    })
}

/// Returns correction events that directly reference `event_id` from a loaded event set.
pub fn corrections_for_event<'a>(events: &'a [WorkEvent], event_id: &str) -> Vec<&'a WorkEvent> {
    events
        .iter()
        .filter(|event| event.corrects_event_id.as_deref() == Some(event_id))
        .collect()
}

/// Loads promotion audit decision records read-only. Missing directory yields empty results.
pub fn load_promotion_audit_records(
    project_path: &str,
) -> Result<LoadedPromotionAudit, ContinuityReaderError> {
    let project = load_project(project_path)?;
    let dir = promotion::promotion_decisions_dir(project_path);
    let mut records = Vec::new();
    let mut diagnostics = Vec::new();

    if !dir.exists() {
        return Ok(LoadedPromotionAudit {
            records,
            diagnostics,
        });
    }

    for path in list_canonical_files(&dir)? {
        let location = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("<unknown>")
            .to_string();
        match classify_decision_record(&path, &project) {
            Ok(record) => records.push(record),
            Err(err) => diagnostics.push(ContinuityDiagnostic {
                kind: ContinuityDiagnosticKind::PromotionAudit,
                location,
                message: err.to_string(),
            }),
        }
    }

    Ok(LoadedPromotionAudit {
        records,
        diagnostics,
    })
}

/// Classifies a producer for `sourceCounts` producer breakdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerSignalBucket {
    Reporter,
    Git,
    Heli,
    Other,
}

/// Maps a `ProducerRef` to a producer-level source count bucket.
pub fn classify_producer_signal_bucket(producer: &ProducerRef) -> ProducerSignalBucket {
    match producer {
        ProducerRef::Reporter(_) => ProducerSignalBucket::Reporter,
        ProducerRef::Git => ProducerSignalBucket::Git,
        ProducerRef::Heli => ProducerSignalBucket::Heli,
        ProducerRef::Native => ProducerSignalBucket::Other,
    }
}

/// Computes `SourceCounts` from loaded inputs without interpretation.
pub fn compute_source_counts(
    pending: &[WorkSignal],
    processed: &[WorkSignal],
    quarantine: &[WorkSignal],
    duplicate: &[WorkSignal],
    work_events: &[WorkEvent],
    promotion_audit: &[PromotionDecisionRecord],
) -> SourceCounts {
    let mut reporter_signals = 0u32;
    let mut git_signals = 0u32;
    let mut heli_signals = 0u32;
    let mut other_producer_signals = 0u32;

    for signal in pending.iter().chain(processed.iter()) {
        match classify_producer_signal_bucket(&signal.producer) {
            ProducerSignalBucket::Reporter => reporter_signals += 1,
            ProducerSignalBucket::Git => git_signals += 1,
            ProducerSignalBucket::Heli => heli_signals += 1,
            ProducerSignalBucket::Other => other_producer_signals += 1,
        }
    }

    SourceCounts {
        work_events: work_events.len() as u32,
        processed_signals: processed.len() as u32,
        pending_signals: pending.len() as u32,
        promotion_audit_records: promotion_audit.len() as u32,
        quarantine_signals: quarantine.len() as u32,
        duplicate_signals: duplicate.len() as u32,
        reporter_signals,
        git_signals,
        heli_signals,
        unknown_producer_signals: 0,
        other_producer_signals,
    }
}

/// Loads a unified continuity input snapshot for Checkpoint C/D builders.
pub fn load_continuity_input_snapshot(
    project_path: &str,
) -> Result<ContinuityInputSnapshot, ContinuityReaderError> {
    let project = load_project(project_path)?;
    let pending = list_pending_signals(project_path)?;
    let processed = list_processed_signals(project_path)?;
    let quarantine = list_quarantine_signals(project_path)?;
    let duplicate = list_duplicate_signals(project_path)?;
    let work_events = load_work_events(project_path)?;
    let promotion_audit = load_promotion_audit_records(project_path)?;

    let source_counts = compute_source_counts(
        &pending.signals,
        &processed.signals,
        &quarantine.signals,
        &duplicate.signals,
        &work_events.events,
        &promotion_audit.records,
    );

    let mut diagnostics = Vec::new();
    diagnostics.extend(pending.diagnostics);
    diagnostics.extend(processed.diagnostics);
    diagnostics.extend(quarantine.diagnostics);
    diagnostics.extend(duplicate.diagnostics);
    diagnostics.extend(work_events.diagnostics);
    diagnostics.extend(promotion_audit.diagnostics);

    Ok(ContinuityInputSnapshot {
        workspace_id: project.id,
        loaded_at: utc_loaded_at(),
        pending_signals: pending.signals,
        processed_signals: processed.signals,
        quarantine_signals: quarantine.signals,
        duplicate_signals: duplicate.signals,
        work_events: work_events.events,
        promotion_audit_records: promotion_audit.records,
        diagnostics,
        source_counts,
    })
}

/// Returns the projections directory path for isolation checks (does not create it).
pub fn projections_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("projections")
}
