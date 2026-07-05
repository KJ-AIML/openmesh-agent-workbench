// ============================================================================
// OpenMesh Context Service — Dev Track 0.1.2.5
// ============================================================================
// Thin service layer wrapping ingestion + index for the UI.
// Command handlers in lib.rs delegate here.
// ============================================================================

#![allow(dead_code)]
#![allow(unused_imports, unused_variables, unused_mut)]

use std::path::PathBuf;

use crate::context::{ContextDocument, ContextSourceKind, Sensitivity, CONTEXT_SCHEMA_VERSION};
use crate::index::{
    derive_index_path, ContextHit, ContextQuery, DerivedIndex, IndexDocument, IndexError,
    IndexHealth, IndexResult,
};
use crate::ingestion::{
    apply_secret_policy, compute_fingerprint, discover_sources, get_project_id, normalize,
    IngestionError, IngestionOutcome, RawSource, SourceReceipt,
};
use crate::storage::Project;

// ---------------------------------------------------------------------------
// IPC-safe models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextSearchResult {
    pub document_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub project_id: String,
    pub canonical_ref: String,
    pub title: String,
    pub snippet: String,
    pub sensitivity: String,
    pub freshness_state: String,
    pub observed_at: String,
    pub source_updated_at: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextInspection {
    pub document_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub project_id: String,
    pub canonical_ref: String,
    pub title: String,
    pub text: String,
    pub sensitivity: String,
    pub agent_context_enabled: bool,
    pub freshness_state: String,
    pub observed_at: String,
    pub source_updated_at: Option<String>,
    pub indexed_at: String,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefreshResult {
    pub project_id: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: String,
    pub discovered: usize,
    pub indexed: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub receipts: Vec<crate::ingestion::SourceReceipt>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextHealth {
    pub path: String,
    pub schema_version: u32,
    pub sqlite_version: String,
    pub journal_mode: String,
    pub document_count: i64,
    pub fts_row_count: i64,
    pub wal_mode_effective: bool,
    pub integrity_ok: bool,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("ingestion error: {0}")]
    Ingestion(#[from] IngestionError),
    #[error("index error: {0}")]
    Index(#[from] IndexError),
    #[error("project metadata error: {0}")]
    ProjectMeta(String),
    #[error("index not found for project: {0}")]
    NotFound(String),
}

pub type ContextResult<T> = Result<T, ContextError>;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn project_id_from_path(project_path: &str) -> ContextResult<String> {
    get_project_id(project_path).map_err(ContextError::Ingestion)
}

fn open_index_for(project_path: &str) -> ContextResult<DerivedIndex> {
    let pid = project_id_from_path(project_path)?;
    let path = derive_index_path(&pid).map_err(ContextError::Index)?;
    // Ensure parent dir exists.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    DerivedIndex::open_at(path).map_err(ContextError::Index)
}

// ---------------------------------------------------------------------------
// Public service API
// ---------------------------------------------------------------------------

const DEFAULT_SEARCH_LIMIT: usize = 25;
const MAX_SEARCH_LIMIT: usize = 100;
const MAX_PREVIEW_CHARS: usize = 4000;

pub fn refresh_project_context(project_path: &str) -> ContextResult<RefreshResult> {
    let started_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let project_id = project_id_from_path(project_path)?;
    let path = derive_index_path(&project_id).map_err(ContextError::Index)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let mut idx = DerivedIndex::open_at(path).map_err(ContextError::Index)?;

    // Discover all sources.
    let sources = discover_sources(project_path)?;

    let mut receipts: Vec<SourceReceipt> = Vec::new();
    let mut total_indexed = 0usize;
    let mut total_updated = 0usize;
    let mut total_unchanged = 0usize;
    let mut total_removed = 0usize;
    let mut total_skipped = 0usize;
    let mut total_failed = 0usize;

    for (kind, results) in &sources {
        let kind_str = normalize_kind_key(kind);

        // Process each source in the family.
        let mut family_documents: Vec<IndexDocument> = Vec::new();
        let mut receipts_for_kind: Vec<SourceReceipt> = Vec::new();
        let mut had_error = false;

        for result in results {
            match result {
                Ok(raw) => {
                    let doc = match normalize(&project_id, (*raw).clone()) {
                        Ok(d) => d,
                        Err(e) => {
                            receipts_for_kind.push(SourceReceipt {
                                source_kind: kind_str.clone(),
                                source_id: raw_source_id(raw),
                                canonical_ref: raw_canonical_ref(raw),
                                outcome: IngestionOutcome::FailedValidation,
                                fingerprint: None,
                                bytes_read: None,
                                error: Some(e.to_string()),
                            });
                            total_failed += 1;
                            had_error = true;
                            continue;
                        }
                    };

                    // Apply secret policy: drop searchable text.
                    let doc = apply_secret_policy(doc).unwrap();

                    let fp = compute_fingerprint(
                        &format!("{:?}", doc.kind).to_lowercase(),
                        &doc.title,
                        &doc.text,
                        &doc.sensitivity,
                        doc.agent_context_enabled,
                    );

                    let idx_doc = IndexDocument::from(&doc);
                    let doc_id = doc.id.clone();
                    let doc_source_id = doc.source_id.clone();
                    let doc_canonical_ref = doc.canonical_ref.clone();
                    let doc_text_len = doc.text.len();
                    match idx.upsert_if_changed(&idx_doc, &fp) {
                        Ok(true) if is_new_document(&idx, &doc_id) => {
                            receipts_for_kind.push(SourceReceipt {
                                source_kind: kind_str.clone(),
                                source_id: doc_source_id.clone(),
                                canonical_ref: Some(doc_canonical_ref.clone()),
                                outcome: IngestionOutcome::Indexed,
                                fingerprint: Some(fp),
                                bytes_read: Some(doc_text_len as u64),
                                error: None,
                            });
                            total_indexed += 1;
                        }
                        Ok(true) => {
                            receipts_for_kind.push(SourceReceipt {
                                source_kind: kind_str.clone(),
                                source_id: doc_source_id.clone(),
                                canonical_ref: Some(doc_canonical_ref.clone()),
                                outcome: IngestionOutcome::Updated,
                                fingerprint: Some(fp),
                                bytes_read: Some(doc_text_len as u64),
                                error: None,
                            });
                            total_updated += 1;
                        }
                        Ok(false) => {
                            receipts_for_kind.push(SourceReceipt {
                                source_kind: kind_str.clone(),
                                source_id: doc_source_id.clone(),
                                canonical_ref: Some(doc_canonical_ref.clone()),
                                outcome: IngestionOutcome::Unchanged,
                                fingerprint: Some(fp),
                                bytes_read: None,
                                error: None,
                            });
                            total_unchanged += 1;
                        }
                        Err(e) => {
                            receipts_for_kind.push(SourceReceipt {
                                source_kind: kind_str.clone(),
                                source_id: doc_source_id.clone(),
                                canonical_ref: Some(doc_canonical_ref.clone()),
                                outcome: IngestionOutcome::FailedIndex,
                                fingerprint: Some(fp),
                                bytes_read: None,
                                error: Some(e.to_string()),
                            });
                            total_failed += 1;
                            had_error = true;
                        }
                    }
                    family_documents.push(idx_doc);
                }
                Err(e) => {
                    receipts_for_kind.push(SourceReceipt {
                        source_kind: kind_str.clone(),
                        source_id: String::new(),
                        canonical_ref: None,
                        outcome: IngestionOutcome::FailedRead,
                        fingerprint: None,
                        bytes_read: None,
                        error: Some(e.to_string()),
                    });
                    total_failed += 1;
                    had_error = true;
                }
            }
        }

        // Handle empty successful discovery: remove this kind's rows.
        if !had_error && family_documents.is_empty() && results.is_empty() {
            // Nothing discovery-wise; either the directory doesn't exist or the JSON file is missing.
            // We do not delete in this case — absence is ambiguous.
        } else if !had_error && family_documents.is_empty() {
            // Successful empty discovery → remove this kind.
            let removed = idx.remove_source_kind(&project_id, &kind_str).unwrap_or(0);
            total_removed += removed as usize;
        }

        receipts.extend(receipts_for_kind);
    }

    let completed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let status = if total_failed == 0 {
        "COMPLETE"
    } else if total_indexed + total_updated > 0 {
        "PARTIAL"
    } else {
        "FAILED"
    };

    Ok(RefreshResult {
        project_id,
        status: status.into(),
        started_at,
        completed_at,
        discovered: receipts.len(),
        indexed: total_indexed,
        updated: total_updated,
        unchanged: total_unchanged,
        removed: total_removed,
        skipped: total_skipped,
        failed: total_failed,
        receipts,
    })
}

pub fn search_project_context(
    project_path: &str,
    query: &str,
    kinds: Option<Vec<String>>,
    limit: Option<usize>,
) -> ContextResult<Vec<ContextSearchResult>> {
    let pid = project_id_from_path(project_path)?;
    let mut idx = open_index_for(project_path)?;

    let effective_limit = limit.unwrap_or(DEFAULT_SEARCH_LIMIT).min(MAX_SEARCH_LIMIT);

    let hits = idx.search(&ContextQuery {
        project_id: pid,
        query: query.to_string(),
        kinds,
        limit: Some(effective_limit),
        ..Default::default()
    })?;

    Ok(hits
        .into_iter()
        .map(|h| ContextSearchResult {
            document_id: h.document_id,
            source_id: h.source_id,
            source_kind: h.source_kind,
            project_id: h.project_id,
            canonical_ref: h.canonical_ref,
            title: h.title,
            snippet: h.snippet,
            sensitivity: h.sensitivity,
            freshness_state: h.freshness_state,
            observed_at: h.observed_at,
            source_updated_at: None,
        })
        .collect())
}

pub fn inspect_context_document(
    project_path: &str,
    document_id: &str,
) -> ContextResult<Option<ContextInspection>> {
    let pid = project_id_from_path(project_path)?;
    let idx = open_index_for(project_path)?;

    let stored = idx.get_document_for_inspection(&pid, document_id)?;

    Ok(stored.map(|doc| {
        let preview = if doc.sensitivity == "secret" {
            String::new()
        } else if doc.text.len() > MAX_PREVIEW_CHARS {
            doc.text.chars().take(MAX_PREVIEW_CHARS).collect()
        } else {
            doc.text
        };

        ContextInspection {
            document_id: doc.document_id,
            source_id: doc.source_id,
            source_kind: doc.source_kind,
            project_id: doc.project_id,
            canonical_ref: doc.canonical_ref,
            title: doc.title,
            text: preview,
            sensitivity: doc.sensitivity,
            agent_context_enabled: doc.agent_context_enabled,
            freshness_state: doc.freshness_state,
            observed_at: doc.observed_at,
            source_updated_at: doc.source_updated_at,
            indexed_at: doc.indexed_at,
            metadata_json: doc.metadata_json,
        }
    }))
}

pub fn get_context_index_health(project_path: &str) -> ContextResult<ContextHealth> {
    let pid = project_id_from_path(project_path)?;
    let mut idx = open_index_for(project_path)?;
    let h = idx.inspect().map_err(ContextError::Index)?;
    Ok(ContextHealth {
        path: h.path,
        schema_version: h.schema_version,
        sqlite_version: h.sqlite_version,
        journal_mode: h.journal_mode,
        document_count: h.document_count,
        fts_row_count: h.fts_row_count,
        wal_mode_effective: h.wal_mode_effective,
        integrity_ok: h.integrity_ok,
    })
}

// ---------------------------------------------------------------------------
// Ingest helpers
// ---------------------------------------------------------------------------

fn is_new_document(_idx: &DerivedIndex, _doc_id: &str) -> bool {
    // For simplicity, consider all upserts as updates. New detection can enhance receipts.
    false
}

fn normalize_kind_key(kind: &str) -> String {
    match kind {
        "docs" => "doc".into(),
        "notes" => "note".into(),
        "snapshots" => "snapshot".into(),
        "tasks" => "task".into(),
        "recent" => "recent".into(),
        "sessions" => "agent-session".into(),
        other => other.to_string(),
    }
}

fn raw_source_id(raw: &RawSource) -> String {
    match raw {
        RawSource::Doc { rel_path, .. } => format!("doc-{}", rel_path).chars().take(64).collect(),
        RawSource::Note { rel_path, .. } => format!("note-{}", rel_path).chars().take(64).collect(),
        RawSource::Snapshot { rel_path, .. } => {
            format!("snapshot-{}", rel_path).chars().take(64).collect()
        }
        RawSource::Task(t) => t.id.chars().take(64).collect(),
        RawSource::Recent(r) => r.id.chars().take(64).collect(),
        RawSource::Session(s) => s.id.chars().take(64).collect(),
    }
}

fn raw_canonical_ref(raw: &RawSource) -> Option<String> {
    match raw {
        RawSource::Task(t) => Some(format!("openmesh://project/{}/task/{}", t.project_id, t.id)),
        RawSource::Recent(r) => Some(format!("openmesh://project//recent/{}", r.id)),
        RawSource::Session(s) => Some(format!("openmesh://project//agent-session/{}", s.id)),
        _ => None,
    }
}

#[allow(dead_code)]
const _GUARD: () = ();
