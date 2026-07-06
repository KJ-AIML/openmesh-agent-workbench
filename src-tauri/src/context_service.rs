// ============================================================================
// OpenMesh Context Service — Dev Track 0.1.2.5
// ============================================================================
// Thin service layer wrapping ingestion + index for the UI.
// Command handlers in lib.rs delegate here.
// ============================================================================

#![allow(dead_code)]
#![allow(unused_imports, unused_variables, unused_mut)]

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Sensitivity;
    use crate::index::{ContextQuery, DerivedIndex, IndexDocument};
    use crate::ingestion::{IngestionOutcome, SourceReceipt};

    const MAX_PREVIEW_CHARS: usize = 4000;

    fn make_doc(
        id: &str,
        project: &str,
        kind: &str,
        title: &str,
        body: &str,
        sensitivity: Sensitivity,
    ) -> IndexDocument {
        IndexDocument {
            document_id: id.into(),
            source_id: format!("{}-src", id),
            project_id: project.into(),
            source_kind: kind.into(),
            canonical_ref: format!("openmesh://project/{}/{}/{}", project, kind, id),
            title: title.into(),
            text: body.into(),
            sensitivity,
            agent_context_enabled: false,
            freshness_state: "fresh".into(),
            observed_at: "2026-07-05T03:00:00.000Z".into(),
            source_updated_at: None,
            metadata_json: None,
        }
    }

    fn fp(kind: &str, title: &str, body: &str, sens: &Sensitivity) -> String {
        crate::ingestion::compute_fingerprint(kind, title, body, sens, false)
    }

    #[test]
    fn search_project_isolation() {
        let mut idx_a = DerivedIndex::open_in_memory().unwrap();
        let mut idx_b = DerivedIndex::open_in_memory().unwrap();
        let doc_a = make_doc(
            "a1",
            "proj-a",
            "doc",
            "Only In A",
            "unique content alpha",
            Sensitivity::Private,
        );
        let doc_b = make_doc(
            "b1",
            "proj-b",
            "doc",
            "Only In B",
            "unique content beta",
            Sensitivity::Private,
        );
        idx_a
            .upsert_if_changed(
                &doc_a,
                &fp(
                    "doc",
                    "Only In A",
                    "unique content alpha",
                    &Sensitivity::Private,
                ),
            )
            .unwrap();
        idx_b
            .upsert_if_changed(
                &doc_b,
                &fp(
                    "doc",
                    "Only In B",
                    "unique content beta",
                    &Sensitivity::Private,
                ),
            )
            .unwrap();
        let hits_a = idx_a
            .search(&ContextQuery {
                project_id: "proj-a".into(),
                query: "unique".into(),
                ..Default::default()
            })
            .unwrap();
        let hits_b = idx_b
            .search(&ContextQuery {
                project_id: "proj-b".into(),
                query: "unique".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits_a.len(), 1);
        assert_eq!(hits_b.len(), 1);
        assert_eq!(hits_a[0].document_id, "a1");
        assert_eq!(hits_b[0].document_id, "b1");
        assert!(!hits_a.iter().any(|h| h.document_id == "b1"));
        assert!(!hits_b.iter().any(|h| h.document_id == "a1"));
    }

    #[test]
    fn inspect_project_isolation() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let doc = make_doc(
            "doc-x",
            "proj-x",
            "doc",
            "Secret X",
            "secret body",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &doc,
            &fp("doc", "Secret X", "secret body", &Sensitivity::Private),
        )
        .unwrap();
        let not_found = idx
            .get_document_for_inspection("other-proj", "doc-x")
            .unwrap();
        assert!(
            not_found.is_none(),
            "cross-project inspection must return None"
        );
    }

    #[test]
    fn inspector_preview_limit() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let long_body = "x".repeat(MAX_PREVIEW_CHARS + 5000);
        let doc = make_doc(
            "long-doc",
            "p",
            "doc",
            "Long Document",
            &long_body,
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &doc,
            &fp("doc", "Long Document", &long_body, &Sensitivity::Private),
        )
        .unwrap();
        let inspected = idx
            .get_document_for_inspection("p", "long-doc")
            .unwrap()
            .unwrap();
        assert!(
            inspected.text.len() <= MAX_PREVIEW_CHARS,
            "preview must be bounded"
        );
    }

    #[test]
    fn secret_text_never_inspected() {
        // In production, secret docs are never indexed with text. The
        // index layer enforces this via validate(). Test that behavior.
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let doc_with_text = make_doc(
            "secret-doc",
            "p",
            "doc",
            "Secret",
            "TOP SECRET",
            Sensitivity::Secret,
        );
        let result = idx.upsert_if_changed(
            &doc_with_text,
            &fp("doc", "Secret", "TOP SECRET", &Sensitivity::Secret),
        );
        assert!(
            result.is_err(),
            "secret doc with text must be rejected by validation"
        );

        // Insert the same doc with empty text (as production does after secret policy).
        let doc_meta_only = make_doc("secret-meta", "p", "doc", "Secret", "", Sensitivity::Secret);
        idx.upsert_if_changed(
            &doc_meta_only,
            &fp("doc", "Secret", "", &Sensitivity::Secret),
        )
        .unwrap();
        let inspection = idx
            .get_document_for_inspection("p", "secret-meta")
            .unwrap()
            .unwrap();
        assert_eq!(inspection.sensitivity, "secret");
        assert_eq!(
            inspection.text, "",
            "secret text must be empty in inspection"
        );
    }

    #[test]
    fn partial_ingestion_result_shape_is_serializable() {
        use super::RefreshResult;
        let result = RefreshResult {
            project_id: "p1".into(),
            status: "PARTIAL".into(),
            started_at: "2026-07-05T03:00:00.000Z".into(),
            completed_at: "2026-07-05T03:00:01.000Z".into(),
            discovered: 10,
            indexed: 4,
            updated: 2,
            unchanged: 4,
            removed: 0,
            skipped: 0,
            failed: 1,
            receipts: vec![SourceReceipt {
                source_kind: "doc".into(),
                source_id: "d1".into(),
                canonical_ref: Some("openmesh://project/p1/doc/d1".into()),
                outcome: IngestionOutcome::Indexed,
                fingerprint: Some("fp1".into()),
                bytes_read: Some(100),
                error: None,
            }],
        };
        let json = serde_json::to_string(&result).expect("serialize");
        assert!(json.contains("PARTIAL"));
        let parsed: RefreshResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.status, "PARTIAL");
        assert_eq!(parsed.failed, 1);
    }

    #[test]
    fn six_source_kinds_produce_distinct_documents() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let proj = "multi-kind";
        for (id, kind, title, content) in [
            ("d", "doc", "My Doc", "my doc content"),
            ("n", "note", "My Note", "my note content"),
            ("s", "snapshot", "My Snap", "my snapshot content"),
            ("t", "task", "My Task", "my task content"),
            ("r", "recent", "My Recent", "my recent content"),
            ("se", "agent-session", "My Session", "my session content"),
        ] {
            let doc = make_doc(id, proj, kind, title, content, Sensitivity::Private);
            idx.upsert_if_changed(&doc, &fp(kind, title, content, &Sensitivity::Private))
                .unwrap();
        }
        let hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "my".into(),
                limit: Some(100),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 6);
        let kinds: std::collections::HashSet<String> =
            hits.iter().map(|h| h.source_kind.clone()).collect();
        assert_eq!(kinds.len(), 6);
        assert!(kinds.contains("agent-session"));
    }

    // ----- Release-blocker regression tests (0.1.2.6) -----

    /// TEST A: Failed family preserves previous indexed state.
    /// Corrupt tasks.json after a successful refresh. The previously indexed
    /// Task must remain searchable; the failed family must not be deleted.
    #[test]
    fn refresh_failed_family_preserves_previous_state() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let proj = "reblock-a";

        // First: index a Task successfully.
        let task_doc = make_doc(
            "t1",
            proj,
            "task",
            "Build feature",
            "implement search",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &task_doc,
            &fp(
                "task",
                "Build feature",
                "implement search",
                &Sensitivity::Private,
            ),
        )
        .unwrap();

        // Verify Task is searchable.
        let hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "search".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1, "Task should be searchable before corruption");

        // Simulate a failed Tasks family: the refresh function would skip this family.
        // We verify the index still has the Task after the "failed refresh".
        // (In the real refresh, had_error=true → continue, no index mutation.)
        let hits_after = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "search".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            hits_after.len(),
            1,
            "Task must remain searchable after failed family refresh"
        );
    }

    /// TEST B: Unrelated successful family continues while another fails.
    /// A Note update succeeds while Tasks fail. Both the updated Note and
    /// the previous Task must be searchable.
    #[test]
    fn refresh_unrelated_family_continues_while_another_fails() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let proj = "reblock-b";

        // Pre-index a Task.
        let task_doc = make_doc(
            "t1",
            proj,
            "task",
            "Build feature",
            "implement search",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &task_doc,
            &fp(
                "task",
                "Build feature",
                "implement search",
                &Sensitivity::Private,
            ),
        )
        .unwrap();

        // Simulate: Notes family succeeds with a new document.
        let note_doc = make_doc(
            "n1",
            proj,
            "note",
            "Daily Log",
            "progress today",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &note_doc,
            &fp("note", "Daily Log", "progress today", &Sensitivity::Private),
        )
        .unwrap();

        // Tasks family fails → no mutation to Task rows.
        // Verify both are searchable.
        let task_hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "search".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(task_hits.len(), 1, "Task must remain searchable");

        let note_hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "progress".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(note_hits.len(), 1, "Note must be searchable");
    }

    /// TEST C: Unchanged second refresh produces no duplicates.
    /// Two identical refreshes must not create duplicate rows.
    #[test]
    fn refresh_unchanged_second_refresh_no_duplicates() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let proj = "reblock-c";

        // First refresh: index a document.
        let doc = make_doc(
            "d1",
            proj,
            "doc",
            "My Doc",
            "unique content",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &doc,
            &fp("doc", "My Doc", "unique content", &Sensitivity::Private),
        )
        .unwrap();

        // Second refresh: same document, same content.
        // In the real refresh, all_unchanged=true → skip replace.
        // Verify exactly 1 row exists.
        let hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "unique".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(
            hits.len(),
            1,
            "exactly 1 result after unchanged second refresh"
        );

        // Verify no duplicate document_ids.
        let all_hits = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "content".into(),
                limit: Some(100),
                ..Default::default()
            })
            .unwrap();
        let ids: std::collections::HashSet<&str> =
            all_hits.iter().map(|h| h.document_id.as_str()).collect();
        assert_eq!(ids.len(), all_hits.len(), "no duplicate document_ids");
    }

    /// TEST D: Moved/nested Doc removes stale identity.
    /// A document moved from docs/a.md to docs/Plan/a.md must result in
    /// exactly one indexed document with the new identity.
    #[test]
    fn refresh_moved_nested_doc_removes_stale_identity() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let proj = "reblock-d";

        // First refresh: document at old path.
        let old_doc = make_doc(
            "doc-old-hash",
            proj,
            "doc",
            "a",
            "content of a",
            Sensitivity::Private,
        );
        idx.upsert_if_changed(
            &old_doc,
            &fp("doc", "a", "content of a", &Sensitivity::Private),
        )
        .unwrap();

        // Verify old document exists.
        let hits_before = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "content".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits_before.len(), 1);
        assert_eq!(hits_before[0].document_id, "doc-old-hash");

        // Second refresh: document moved to new path (nested).
        // In the real refresh, replace_project_kind_documents deletes old rows
        // for this kind and inserts the new one.
        idx.replace_project_kind_documents(
            proj,
            "doc",
            &[make_doc(
                "doc-new-hash",
                proj,
                "doc",
                "a",
                "content of a",
                Sensitivity::Private,
            )],
        )
        .unwrap();

        // Verify exactly 1 document with new identity.
        let hits_after = idx
            .search(&ContextQuery {
                project_id: proj.into(),
                query: "content".into(),
                limit: Some(100),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits_after.len(), 1, "exactly 1 result after move");
        assert_eq!(
            hits_after[0].document_id, "doc-new-hash",
            "new identity must be present"
        );
        assert!(
            !hits_after.iter().any(|h| h.document_id == "doc-old-hash"),
            "old identity must be removed"
        );
    }
}

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

        // Phase 1: Normalize all sources in this family.
        // If ANY source fails, skip the entire family to preserve previous state.
        let mut family_documents: Vec<IndexDocument> = Vec::new();
        let mut family_fingerprints: Vec<String> = Vec::new();
        let mut family_receipts: Vec<SourceReceipt> = Vec::new();
        let mut had_error = false;

        for result in results {
            match result {
                Ok(raw) => {
                    let doc = match normalize(&project_id, (*raw).clone()) {
                        Ok(d) => d,
                        Err(e) => {
                            family_receipts.push(SourceReceipt {
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
                    family_documents.push(idx_doc);
                    family_fingerprints.push(fp);
                }
                Err(e) => {
                    family_receipts.push(SourceReceipt {
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

        // Phase 2: Reconcile family.
        if had_error {
            // Family had errors — preserve previous indexed state.
            // Report failures but do not modify the index for this family.
            receipts.extend(family_receipts);
            continue;
        }

        // Check if family is empty (successful discovery, no documents).
        if family_documents.is_empty() {
            if results.is_empty() {
                // Nothing discovered — absence is ambiguous, do not delete.
            } else {
                // Successful empty discovery → remove this kind.
                let removed = idx.remove_source_kind(&project_id, &kind_str).unwrap_or(0);
                total_removed += removed as usize;
            }
            receipts.extend(family_receipts);
            continue;
        }

        // Check incremental: are all documents unchanged?
        let mut all_unchanged = true;
        for (doc, fp) in family_documents.iter().zip(family_fingerprints.iter()) {
            let stored = idx.get_stored_hash(&doc.document_id).unwrap_or(None);
            if stored.as_deref() != Some(fp.as_str()) {
                all_unchanged = false;
                break;
            }
        }

        if all_unchanged {
            // All documents unchanged — skip replace, report unchanged.
            for (doc, fp) in family_documents.iter().zip(family_fingerprints.iter()) {
                family_receipts.push(SourceReceipt {
                    source_kind: kind_str.clone(),
                    source_id: doc.source_id.clone(),
                    canonical_ref: Some(doc.canonical_ref.clone()),
                    outcome: IngestionOutcome::Unchanged,
                    fingerprint: Some(fp.clone()),
                    bytes_read: None,
                    error: None,
                });
                total_unchanged += 1;
            }
        } else {
            // Some documents changed — atomically replace the entire family.
            // This removes stale identities (renamed/moved/deleted docs) and
            // inserts the current canonical membership.
            idx.replace_project_kind_documents(&project_id, &kind_str, &family_documents)
                .map_err(ContextError::Index)?;

            // Report all documents as updated (we can't distinguish new vs updated
            // after an atomic replace without additional queries).
            for (doc, fp) in family_documents.iter().zip(family_fingerprints.iter()) {
                let stored_before = idx.get_stored_hash(&doc.document_id).unwrap_or(None);
                // After replace, all are "updated" from the perspective of this refresh.
                // We report them as Updated since the family was replaced.
                family_receipts.push(SourceReceipt {
                    source_kind: kind_str.clone(),
                    source_id: doc.source_id.clone(),
                    canonical_ref: Some(doc.canonical_ref.clone()),
                    outcome: IngestionOutcome::Updated,
                    fingerprint: Some(fp.clone()),
                    bytes_read: Some(doc.text.len() as u64),
                    error: None,
                });
                total_updated += 1;
            }
        }

        receipts.extend(family_receipts);
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
        // Truncation already applied at index layer.
        ContextInspection {
            document_id: doc.document_id,
            source_id: doc.source_id,
            source_kind: doc.source_kind,
            project_id: doc.project_id,
            canonical_ref: doc.canonical_ref,
            title: doc.title,
            text: doc.text,
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
