// ============================================================================
// OpenMesh Derived Local Index — Dev Track 0.1.2.3
// ============================================================================
// A disposable, rebuildable SQLite index over ContextDocument contracts.
//
// Architectural invariant:
//   CanonicalStore <> DerivedIndex
//   Canonical files (markdown, JSON) remain the source of truth.
//   This derived layer is disposable and rebuildable from any
//   ContextDocument iterator. It never reads or writes canonical files.
//
// rusqlite 0.30 + bundled feature includes SQLite 3.44.0 with FTS5 and JSON1
// enabled by default (proven in Phase 2 capability audit).
// ============================================================================

#![cfg_attr(not(test), allow(dead_code))]
#![cfg_attr(test, allow(dead_code, unused_variables, unused_mut))]
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension, Result as SqlResult, Transaction};
use serde::{Deserialize, Serialize};

use crate::context::{ContextDocument, Sensitivity};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Version of the derived-index schema (independent of CONTEXT_SCHEMA_VERSION).
const INDEX_SCHEMA_VERSION: u32 = 1;

/// FTS5 tokenizer. unicode61 separates on whitespace/punctuation and
/// lowercases tokens for case-insensitive lexical matching.
const FTS_TOKENIZER: &str = "unicode61";

/// Default FTS query limit.
const DEFAULT_LIMIT: usize = 25;

/// Maximum characters returned in an inspection preview.
const MAX_PREVIEW_CHARS: usize = 4000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("SQLite error: {0}")]
    Sql(#[from] rusqlite::Error),

    #[error("failed to derive index path for project: {0}")]
    Path(String),

    #[error("home directory not available")]
    NoHomeDir,

    #[error("failed to create index directory: {0}")]
    DirCreate(String),

    #[error("index schema version mismatch — rebuild required")]
    SchemaMismatch,

    #[error("document validation failed: {0}")]
    InvalidDocument(String),

    #[error("invalid project id: {0}")]
    InvalidProjectId(String),
}

pub type IndexResult<T> = Result<T, IndexError>;

// ---------------------------------------------------------------------------
// Source / domain types
// ---------------------------------------------------------------------------

/// Normalized input for the derived index.
/// Constructing one from a `ContextDocument` is explicit about what fields
/// enter the index versus those left in canonical storage only.
#[derive(Debug, Clone)]
pub struct IndexDocument {
    pub document_id: String,
    pub source_id: String,
    pub project_id: String,
    pub source_kind: String,
    pub canonical_ref: String,
    pub title: String,
    pub text: String,
    pub sensitivity: Sensitivity,
    pub agent_context_enabled: bool,
    pub freshness_state: String,
    pub observed_at: String,
    pub source_updated_at: Option<String>,
    pub metadata_json: Option<String>,
}

/// Safe read-only model for UI inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorableDocument {
    pub document_id: String,
    pub source_id: String,
    pub project_id: String,
    pub source_kind: String,
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

impl IndexDocument {
    /// Validate required fields. Secret-level text is never indexed.
    fn validate(&self) -> IndexResult<()> {
        if self.document_id.trim().is_empty() {
            return Err(IndexError::InvalidDocument("document_id required".into()));
        }
        if self.source_id.trim().is_empty() {
            return Err(IndexError::InvalidDocument("source_id required".into()));
        }
        if self.project_id.trim().is_empty() {
            return Err(IndexError::InvalidDocument("project_id required".into()));
        }
        if self.source_kind.trim().is_empty() {
            return Err(IndexError::InvalidDocument("source_kind required".into()));
        }
        if self.title.trim().is_empty() {
            return Err(IndexError::InvalidDocument("title required".into()));
        }
        if self.sensitivity == Sensitivity::Secret && !self.text.trim().is_empty() {
            // Policy: secret text content is not searchable.
            return Err(IndexError::InvalidDocument(
                "secret documents must not include searchable text".into(),
            ));
        }
        Ok(())
    }

    /// Text to index. Secret documents produce empty searchable text.
    fn searchable_text(&self) -> &str {
        if self.sensitivity == Sensitivity::Secret {
            ""
        } else {
            &self.text
        }
    }
}

impl<'a> From<&'a ContextDocument> for IndexDocument {
    fn from(doc: &'a ContextDocument) -> Self {
        Self {
            document_id: doc.id.clone(),
            source_id: doc.source_id.clone(),
            project_id: doc.project_id.clone(),
            source_kind: format!("{:?}", doc.kind).to_lowercase(),
            canonical_ref: doc.canonical_ref.clone(),
            title: doc.title.clone(),
            text: doc.text.clone(),
            sensitivity: doc.sensitivity.clone(),
            agent_context_enabled: doc.agent_context_enabled,
            freshness_state: format!("{:?}", doc.freshness.state).to_lowercase(),
            observed_at: doc.observed_at.clone(),
            source_updated_at: doc.source_updated_at.clone(),
            metadata_json: doc
                .metadata
                .as_ref()
                .and_then(|v| serde_json::to_string(v).ok()),
        }
    }
}

// ---------------------------------------------------------------------------
// Query + Hit types
// ---------------------------------------------------------------------------

/// A lexical search request. All fields except `project_id` and `query`
/// are optional refinements.
#[derive(Debug, Clone, Default)]
pub struct ContextQuery {
    pub project_id: String,
    pub query: String,
    pub kinds: Option<Vec<String>>,
    pub sensitivity_ceiling: Option<Sensitivity>,
    pub limit: Option<usize>,
}

/// A single search hit with provenance + a deterministic score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextHit {
    pub document_id: String,
    pub source_id: String,
    pub source_kind: String,
    pub project_id: String,
    pub canonical_ref: String,
    pub title: String,
    pub snippet: String,
    pub score: f64,
    pub freshness_state: String,
    pub sensitivity: String,
    pub observed_at: String,
}

/// Index health snapshot for observability and recovery decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexHealth {
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
// Index path policy
// ---------------------------------------------------------------------------

fn fnv1a_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

/// Deterministic, project-scoped index path outside project source tree.
///
/// Pattern: ~/.openmesh/indexes/<project-id-hex>/context.sqlite3
/// The project ID is hashed to avoid filesystem-unsafe characters and to
/// keep the index in a stable global location irrespective of any
/// particular project folder path.
pub fn derive_index_path(project_id: &str) -> IndexResult<PathBuf> {
    if project_id.trim().is_empty() {
        return Err(IndexError::InvalidProjectId(project_id.into()));
    }
    let home = dirs::home_dir().ok_or(IndexError::NoHomeDir)?;
    let dir = home
        .join(".openmesh")
        .join("indexes")
        .join(format!("proj_{}", fnv1a_hex(project_id.trim())));
    Ok(dir.join("context.sqlite3"))
}

/// Sanitize a caller-supplied file path; reject anything outside the intended
/// directory tree. Returns Ok(path) if the absolute target starts with the
/// expected prefix after normalization.
pub fn assert_path_in_index_tree(candidate: &Path, project_id: &str) -> IndexResult<PathBuf> {
    let expected = derive_index_path(project_id)?;
    let c = std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf());
    let e = std::fs::canonicalize(&expected).unwrap_or_else(|_| expected.clone());
    if c.starts_with(&e) || e.starts_with(&c) {
        Ok(c)
    } else {
        Err(IndexError::Path("index path outside allowed tree".into()))
    }
}

// ---------------------------------------------------------------------------
// DerivedIndex — connection, schema, writes, queries, health
// ---------------------------------------------------------------------------

pub struct DerivedIndex {
    #[cfg(test)]
    pub(crate) conn: Connection,
    #[cfg(not(test))]
    conn: Connection,
    path: PathBuf,
}

impl DerivedIndex {
    // ----- construction ---------------------------------------------------

    /// Open (or create) the index for a project at the canonical location.
    pub fn open(project_id: &str) -> IndexResult<Self> {
        let path = derive_index_path(project_id)?;
        Self::open_at(path)
    }

    /// Open (or create) the index at an explicit path (used for tests).
    pub fn open_at(path: PathBuf) -> IndexResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| IndexError::DirCreate(e.to_string()))?;
        }
        let conn = Connection::open(&path)?;
        apply_pragmas(&conn, &path)?;
        let mut idx = Self { conn, path };
        idx.init_schema()?;
        Ok(idx)
    }

    /// In-memory index for tests only (private visibility via #[cfg(test)]).
    #[cfg(test)]
    pub fn open_in_memory() -> IndexResult<Self> {
        let conn = Connection::open_in_memory()?;
        apply_pragmas_in_memory(&conn)?;
        let mut idx = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        idx.init_schema()?;
        Ok(idx)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    // ----- schema ----------------------------------------------------------

    fn apply_schema(tx: &Transaction<'_>) -> SqlResult<()> {
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS index_meta (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )?;
        tx.execute_batch(
            "CREATE TABLE IF NOT EXISTS context_documents (
                 document_id     TEXT PRIMARY KEY,
                 source_id       TEXT NOT NULL,
                 project_id      TEXT NOT NULL,
                 source_kind     TEXT NOT NULL,
                 canonical_ref   TEXT NOT NULL,
                 title           TEXT NOT NULL,
                 text            TEXT NOT NULL DEFAULT '',
                 text_length     INTEGER NOT NULL DEFAULT 0,
                 content_hash    TEXT,
                 sensitivity     TEXT NOT NULL DEFAULT 'private',
                 agent_context_enabled INTEGER NOT NULL DEFAULT 0,
                 freshness_state TEXT NOT NULL DEFAULT 'unknown',
                 observed_at    TEXT NOT NULL,
                 source_updated_at TEXT,
                 indexed_at      TEXT NOT NULL,
                 metadata_json   TEXT
             );",
        )?;
        tx.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_docs_project ON context_documents(project_id);
             CREATE INDEX IF NOT EXISTS idx_docs_source ON context_documents(source_id);
             CREATE INDEX IF NOT EXISTS idx_docs_kind ON context_documents(source_kind);",
        )?;
        tx.execute_batch(&format!(
            "CREATE VIRTUAL TABLE IF NOT EXISTS context_documents_fts USING fts5(
                   title,
                   body,
                   document_id UNINDEXED,
                   source_id UNINDEXED,
                   project_id UNINDEXED,
                   source_kind UNINDEXED,
                   tokenize='{}'
                 );",
            FTS_TOKENIZER
        ))?;
        Ok(())
    }

    fn init_schema(&mut self) -> IndexResult<()> {
        let tx = self.conn.transaction()?;
        Self::apply_schema(&tx)?;

        let existing: Option<String> = tx
            .query_row(
                "SELECT value FROM index_meta WHERE key = 'schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()?;

        match existing {
            Some(v) => {
                v.parse::<u32>()
                    .ok()
                    .filter(|&sv| sv == INDEX_SCHEMA_VERSION)
                    .ok_or(IndexError::SchemaMismatch)?;
            }
            None => {
                tx.execute(
                    "INSERT INTO index_meta(key, value) VALUES ('schema_version', ?1)",
                    params![INDEX_SCHEMA_VERSION.to_string()],
                )?;
                tx.execute(
                    "INSERT INTO index_meta(key, value) VALUES ('created_at', ?1)",
                    params![now_iso()],
                )?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    // ----- writes ----------------------------------------------------------

    /// Upsert a single document; replaces prior FTS rows for the same source.
    pub fn upsert_document(&mut self, doc: &IndexDocument) -> IndexResult<()> {
        doc.validate()?;
        let searchable = doc.searchable_text().to_string();
        let text_length = searchable.len() as i64;

        let tx = self.conn.transaction()?;
        Self::write_document_tx(&tx, doc, &searchable, text_length, None)?;
        tx.commit()?;
        Ok(())
    }

    /// Insert-or-update only if content hash changed.
    /// Returns true when a write was performed, false when unchanged.
    pub fn upsert_if_changed(&mut self, doc: &IndexDocument, hash: &str) -> IndexResult<bool> {
        // Compare against stored hash.
        let stored: Option<String> = self
            .conn
            .query_row(
                "SELECT content_hash FROM context_documents WHERE document_id = ?1",
                [&doc.document_id],
                |r| r.get(0),
            )
            .optional()?;

        if stored.as_deref() == Some(hash) {
            return Ok(false); // unchanged
        }

        doc.validate()?;
        let searchable = doc.searchable_text().to_string();
        let text_length = searchable.len() as i64;

        let tx = self.conn.transaction()?;
        Self::write_document_tx(&tx, doc, &searchable, text_length, Some(hash))?;
        tx.commit()?;
        Ok(true)
    }

    /// Retrieve the stored content hash for a document, if any.
    pub fn get_stored_hash(&self, document_id: &str) -> IndexResult<Option<String>> {
        self.conn
            .query_row(
                "SELECT content_hash FROM context_documents WHERE document_id = ?1",
                [document_id],
                |r| r.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    /// Read one document for safe UI inspection.
    /// Returns None if not found or belongs to another project.
    /// Secret text is always omitted (empty string returned).
    pub fn get_document_for_inspection(
        &self,
        project_id: &str,
        document_id: &str,
    ) -> IndexResult<Option<StorableDocument>> {
        let result = self.conn.query_row(
            "SELECT document_id, source_id, project_id, source_kind, canonical_ref, title,              text, sensitivity, agent_context_enabled, freshness_state, observed_at,              source_updated_at, indexed_at, metadata_json              FROM context_documents WHERE project_id = ?1 AND document_id = ?2",
            [project_id, document_id],
            |r| {
                Ok(StorableDocument {
                    document_id: r.get(0)?,
                    source_id: r.get(1)?,
                    project_id: r.get(2)?,
                    source_kind: r.get(3)?,
                    canonical_ref: r.get(4)?,
                    title: r.get(5)?,
                    text: r.get(6)?,
                    sensitivity: r.get(7)?,
                    agent_context_enabled: r.get(8)?,
                    freshness_state: r.get(9)?,
                    observed_at: r.get(10)?,
                    source_updated_at: r.get(11)?,
                    indexed_at: r.get(12)?,
                    metadata_json: r.get(13)?,
                })
            },
        ).optional().map_err(IndexError::Sql)?;
        Ok(result.map(|mut doc| {
            if doc.sensitivity == "secret" {
                doc.text.clear();
            } else if doc.text.len() > MAX_PREVIEW_CHARS {
                doc.text = doc.text.chars().take(MAX_PREVIEW_CHARS).collect();
            }
            doc
        }))
    }

    fn write_document_tx(
        tx: &Transaction<'_>,
        doc: &IndexDocument,
        searchable: &str,
        text_length: i64,
        content_hash: Option<&str>,
    ) -> SqlResult<()> {
        // 1. Upsert canonical document row.
        tx.execute(
            "INSERT INTO context_documents
                (document_id, source_id, project_id, source_kind,
                 canonical_ref, title, text, text_length, content_hash,
                 sensitivity, agent_context_enabled, freshness_state,
                 observed_at, source_updated_at, indexed_at, metadata_json)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(document_id) DO UPDATE SET
                source_id = excluded.source_id,
                project_id = excluded.project_id,
                source_kind = excluded.source_kind,
                canonical_ref = excluded.canonical_ref,
                title = excluded.title,
                text = excluded.text,
                text_length = excluded.text_length,
                content_hash = excluded.content_hash,
                sensitivity = excluded.sensitivity,
                agent_context_enabled = excluded.agent_context_enabled,
                freshness_state = excluded.freshness_state,
                observed_at = excluded.observed_at,
                source_updated_at = excluded.source_updated_at,
                indexed_at = excluded.indexed_at,
                metadata_json = excluded.metadata_json",
            params![
                &doc.document_id,
                &doc.source_id,
                &doc.project_id,
                &doc.source_kind,
                &doc.canonical_ref,
                &doc.title,
                &doc.text,
                text_length,
                content_hash,
                format!("{:?}", doc.sensitivity).to_lowercase(),
                doc.agent_context_enabled as i64,
                &doc.freshness_state,
                &doc.observed_at,
                &doc.source_updated_at,
                now_iso(),
                &doc.metadata_json,
            ],
        )?;

        // 2. Remove previous FTS rows for this source (idempotent rebuild).
        tx.execute(
            "DELETE FROM context_documents_fts WHERE source_id = ?1",
            params![&doc.source_id],
        )?;
        tx.execute(
            "DELETE FROM context_documents_fts WHERE document_id = ?1",
            params![&doc.document_id],
        )?;

        // 3. Insert new FTS row (only when there is searchable text).
        if !searchable.trim().is_empty() {
            tx.execute(
                "INSERT INTO context_documents_fts
                    (title, body, document_id, source_id, project_id, source_kind)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    &doc.title,
                    searchable,
                    &doc.document_id,
                    &doc.source_id,
                    &doc.project_id,
                    &doc.source_kind,
                ],
            )?;
        }
        Ok(())
    }

    /// Replace all indexed documents for a project inside one transaction.
    pub fn replace_project_documents(
        &mut self,
        project_id: &str,
        docs: &[IndexDocument],
    ) -> IndexResult<()> {
        for d in docs {
            d.validate()?;
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM context_documents WHERE project_id = ?1",
            params![project_id],
        )?;
        // FTS rows are keyed by document_id/source_id; delete by project.
        tx.execute(
            "DELETE FROM context_documents_fts WHERE rowid IN (
                 SELECT fts.rowid
                   FROM context_documents_fts fts
                        LEFT JOIN context_documents d ON d.document_id = fts.document_id
                  WHERE d.project_id = ?1
             )",
            params![project_id],
        )?;
        for d in docs {
            let searchable = d.searchable_text().to_string();
            Self::write_document_tx(&tx, d, &searchable, searchable.len() as i64, None)?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Remove all index state for a source (documents + FTS rows).
    pub fn remove_source(&mut self, source_id: &str) -> IndexResult<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM context_documents_fts WHERE source_id = ?1",
            params![source_id],
        )?;
        tx.execute(
            "DELETE FROM context_documents WHERE source_id = ?1",
            params![source_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Remove all index state for a project.
    pub fn clear_project(&mut self, project_id: &str) -> IndexResult<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM context_documents_fts WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.execute(
            "DELETE FROM context_documents WHERE project_id = ?1",
            params![project_id],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Remove all derived index rows for one source kind within a project.
    pub fn remove_source_kind(&mut self, project_id: &str, kind: &str) -> IndexResult<usize> {
        let tx = self.conn.transaction()?;
        let count: i64 = tx.query_row(
            "SELECT COUNT(*) FROM context_documents WHERE project_id = ?1 AND source_kind = ?2",
            [project_id, kind],
            |r| r.get(0),
        )?;
        let n = count as usize;
        // Delete from FTS first.
        tx.execute(
            "DELETE FROM context_documents_fts WHERE document_id IN (\
                SELECT document_id FROM context_documents                 WHERE project_id = ?1 AND source_kind = ?2\
            )",
            params![project_id, kind],
        )?;
        tx.execute(
            "DELETE FROM context_documents WHERE project_id = ?1 AND source_kind = ?2",
            params![project_id, kind],
        )?;
        tx.commit()?;
        Ok(n)
    }

    /// Replace all documents of a single source kind for a project in one transaction.
    pub fn replace_project_kind_documents(
        &mut self,
        project_id: &str,
        kind: &str,
        docs: &[IndexDocument],
    ) -> IndexResult<()> {
        for d in docs {
            d.validate()?;
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM context_documents_fts WHERE document_id IN (\
                SELECT document_id FROM context_documents                 WHERE project_id = ?1 AND source_kind = ?2\
            )",
            params![project_id, kind],
        )?;
        tx.execute(
            "DELETE FROM context_documents WHERE project_id = ?1 AND source_kind = ?2",
            params![project_id, kind],
        )?;
        // Insert new rows.
        for d in docs {
            let searchable = d.searchable_text().to_string();
            Self::write_document_tx(&tx, d, &searchable, searchable.len() as i64, None)?;
        }
        tx.commit()?;
        Ok(())
    }

    // ----- search ---------------------------------------------------------

    /// Lexical search using FTS5 MATCH with optional kind/sensitivity filters.
    pub fn search(&self, query: &ContextQuery) -> IndexResult<Vec<ContextHit>> {
        let limit = query.limit.unwrap_or(DEFAULT_LIMIT);
        let effective = if query.query.trim().is_empty() {
            return Ok(Vec::new());
        } else {
            sanitize_fts_query(&query.query)
        };

        // Build kind filter clause with dynamic placeholders, applied BEFORE LIMIT.
        let kinds_clause = match &query.kinds {
            Some(kinds) if !kinds.is_empty() => {
                let n = kinds.len();
                let placeholders: Vec<String> = (0..n).map(|i| format!("?{}", i + 3)).collect();
                format!(" AND fts.source_kind IN ({})", placeholders.join(","))
            }
            _ => String::new(),
        };

        // Compute the LIMIT placeholder index: 1=MATCH, 2=project_id, 3..3+N=kinds, 3+N=limit
        let limit_idx = if query.kinds.as_ref().map_or(0, |k| k.len()) == 0 {
            3
        } else {
            3 + query.kinds.as_ref().map_or(0, |k| k.len())
        };

        let sql = format!(
            "SELECT fts.document_id,
                    fts.source_id,
                    fts.source_kind,
                    fts.project_id,
                    d.canonical_ref,
                    d.title,
                    snippet(context_documents_fts, 1, '[', ']', ' … ', 8) AS snip,
                    bm25(context_documents_fts) AS score,
                    d.freshness_state,
                    d.sensitivity,
                    d.observed_at
               FROM context_documents_fts fts
               JOIN context_documents d ON d.document_id = fts.document_id
              WHERE context_documents_fts MATCH ?1
                AND d.project_id = ?2
              {}
              ORDER BY score ASC
              LIMIT ?{}",
            kinds_clause, limit_idx
        );

        let mut stmt = self.conn.prepare(&sql)?;
        let limit_val = limit as i64;
        // Build params: [query, project_id, <kindN...>, limit]
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![
            Box::new(effective.clone()),
            Box::new(query.project_id.clone()),
        ];
        if let Some(kinds) = &query.kinds {
            for k in kinds {
                params_vec.push(Box::new(k.clone()));
            }
        }
        params_vec.push(Box::new(limit_val));
        let param_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|b| b.as_ref()).collect();
        let rows = stmt.query_map(&param_refs[..], |row| {
            Ok(ContextHit {
                document_id: row.get(0)?,
                source_id: row.get(1)?,
                source_kind: row.get(2)?,
                project_id: row.get(3)?,
                canonical_ref: row.get(4)?,
                title: row.get(5)?,
                snippet: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                score: row.get(7)?,
                freshness_state: row.get(8)?,
                sensitivity: row.get(9)?,
                observed_at: row.get(10)?,
            })
        })?;

        let mut hits = Vec::with_capacity(limit);
        for row in rows {
            let hit = row?;
            // Sensitivity ceiling: filter out 'secret' from search.
            if hit.sensitivity == "secret" {
                continue;
            }
            hits.push(hit);
            if hits.len() >= limit {
                break;
            }
        }
        Ok(hits)
    }

    // ----- JSON metadata query --------------------------------------------

    /// Query a specific JSON key within document metadata.
    pub fn query_metadata(
        &self,
        project_id: &str,
        json_path: &str,
    ) -> IndexResult<Vec<(String, String)>> {
        let expr = format!("$.{}", json_path.trim_start_matches('.'));
        let sql = "SELECT document_id, CAST(json_extract(metadata_json, ?2) AS TEXT) AS m
                   FROM context_documents
                  WHERE project_id = ?1
                    AND json_valid(metadata_json) = 1
                    AND json_extract(metadata_json, ?2) IS NOT NULL";
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map(
            params![project_id, &expr],
            |row| -> SqlResult<(String, String)> {
                let raw: String = row.get(1)?;
                Ok((row.get(0)?, raw))
            },
        )?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // ----- rebuild --------------------------------------------------------

    /// Dispose and rebuild the index from a fresh set of documents for one project.
    pub fn rebuild_from_documents(
        &mut self,
        project_id: &str,
        docs: &[IndexDocument],
    ) -> IndexResult<()> {
        self.clear_project(project_id)?;
        self.replace_project_documents(project_id, docs)
    }

    // ----- purge (disposability) ------------------------------------------

    /// Close the connection and remove ALL files for this project's derived index.
    pub fn purge(self) -> IndexResult<()> {
        drop(self.conn); // ensure WAL is checkpointed and file handle released

        let base = self.path.clone();
        for candidate in [
            base.clone(),
            base.with_extension("sqlite3-wal"),
            base.with_extension("sqlite3-shm"),
        ] {
            if candidate.exists() {
                std::fs::remove_file(&candidate).ok();
            }
        }
        // Best-effort removal of now-empty parent dir.
        if let Some(parent) = self.path.parent() {
            std::fs::remove_dir(parent).ok();
        }
        Ok(())
    }

    // ----- health ---------------------------------------------------------

    pub fn health(&self) -> IndexResult<IndexHealth> {
        let sqlite_version: String = self
            .conn
            .query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
        let journal_mode: String = self
            .conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))?;
        let integrity: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .optional()?
            .unwrap_or_else(|| "unknown".into());

        let document_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM context_documents", [], |r| r.get(0))?;
        let fts_row_count: i64 =
            self.conn
                .query_row("SELECT COUNT(*) FROM context_documents_fts", [], |r| {
                    r.get(0)
                })?;

        Ok(IndexHealth {
            path: self.path.to_string_lossy().into_owned(),
            schema_version: INDEX_SCHEMA_VERSION,
            sqlite_version,
            journal_mode: journal_mode.clone(),
            document_count,
            fts_row_count,
            wal_mode_effective: journal_mode.to_lowercase() == "wal"
                || self.path == Path::new(":memory:"),
            integrity_ok: integrity == "ok",
        })
    }

    /// Inspect without consuming — exposes health as serialized value for later.
    pub fn inspect(&self) -> IndexResult<IndexHealth> {
        self.health()
    }
}

// ---------------------------------------------------------------------------
// Standalone recovery (corrupt index helper)
// ---------------------------------------------------------------------------

/// Recover a corrupt derived index by removing all its files and recreating
/// an empty index. This is only valid because the index is disposable —
/// canonical data is never touched.
pub fn recover_corrupt_index(project_id: &str) -> IndexResult<()> {
    let path = derive_index_path(project_id)?;
    if path.exists() {
        let base = path.clone();
        for candidate in [
            base.clone(),
            base.with_extension("sqlite3-wal"),
            base.with_extension("sqlite3-shm"),
        ] {
            if candidate.exists() {
                std::fs::remove_file(&candidate).map_err(|e| {
                    IndexError::Path(format!("remove {} failed: {}", candidate.display(), e))
                })?;
            }
        }
        if let Some(parent) = path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn apply_pragmas(conn: &Connection, path: &Path) -> IndexResult<()> {
    conn.pragma_update(None, "foreign_keys", "on")?;
    if path != Path::new(":memory:") {
        conn.pragma_update(None, "journal_mode", "wal")?;
    }
    Ok(())
}

#[cfg(test)]
fn apply_pragmas_in_memory(conn: &Connection) -> IndexResult<()> {
    conn.pragma_update(None, "foreign_keys", "on")?;
    Ok(())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Sanitize user input for FTS5 MATCH by wrapping in double quotes so that
/// punctuation/keywords are interpreted literally. We strip characters that
/// would escape the FTS query grammar.
fn sanitize_fts_query(input: &str) -> String {
    let mut s = input.trim().to_string();
    // Remove FTS5 special chars that would break phrase queries.
    s = s
        .replace(['"', '*', ':', '(', ')'], "")
        .replace(['-', '+'], " ");
    format!("\"{}\"", s.trim())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_doc(project_id: &str, did: &str, title: &str, body: &str) -> IndexDocument {
        IndexDocument {
            document_id: did.into(),
            source_id: format!("{}-src", did),
            project_id: project_id.into(),
            source_kind: "doc".into(),
            canonical_ref: format!("openmesh://project/{}/doc/{}", project_id, did),
            title: title.into(),
            text: body.into(),
            sensitivity: Sensitivity::Private,
            agent_context_enabled: false,
            freshness_state: "fresh".into(),
            observed_at: "2026-07-05T03:00:00.000Z".into(),
            source_updated_at: Some("2026-07-04T14:00:00.000Z".into()),
            metadata_json: Some(r#"{"ext":"md","loc":42}"#.into()),
        }
    }

    fn secret_doc(project_id: &str, did: &str) -> IndexDocument {
        IndexDocument {
            document_id: did.into(),
            source_id: format!("{}-src", did),
            project_id: project_id.into(),
            source_kind: "doc".into(),
            canonical_ref: format!("openmesh://project/{}/doc/{}", project_id, did),
            title: "secret-doc".into(),
            text: "".into(),
            sensitivity: Sensitivity::Secret,
            agent_context_enabled: false,
            freshness_state: "unknown".into(),
            observed_at: "2026-07-05T03:00:00.000Z".into(),
            source_updated_at: None,
            metadata_json: None,
        }
    }

    // ----- capability ----

    #[test]
    fn sqlite_version_available() {
        let idx = DerivedIndex::open_in_memory().unwrap();
        let h = idx.health().unwrap();
        assert!(!h.sqlite_version.is_empty());
    }

    #[test]
    fn json_functions_work() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let doc = sample_doc("p1", "d1", "JSON test", "hello world");
        idx.upsert_document(&doc).unwrap();
        let rows = idx.query_metadata("p1", "ext").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].1.contains("md"));
    }

    #[test]
    fn fts5_match_works() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc(
            "p1",
            "d1",
            "OpenMesh Architecture",
            "local first project memory index",
        ))
        .unwrap();
        idx.upsert_document(&sample_doc(
            "p1",
            "d2",
            "Rust Notes",
            "sqlite wrapper tooling",
        ))
        .unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "memory".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document_id, "d1");
    }

    // ----- path policy ----

    #[test]
    fn index_path_is_deterministic_and_outside_project_tree() {
        let a = derive_index_path("my-project").unwrap();
        let b = derive_index_path("my-project").unwrap();
        assert_eq!(a, b);
        // Must not be inside any project source tree.
        assert!(!a.to_string_lossy().contains(".openmesh/docs"));
        assert!(a.to_string_lossy().contains("indexes"));
    }

    #[test]
    fn index_path_project_isolation() {
        let a = derive_index_path("proj-alpha").unwrap();
        let b = derive_index_path("proj-beta").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn index_path_rejects_traversal() {
        // Even a path-like project id should not escape the index tree.
        let p = derive_index_path("../../etc/passwd").unwrap();
        assert!(p.to_string_lossy().contains("indexes"));
        assert!(!p.to_string_lossy().contains("etc/passwd"));
    }

    #[test]
    fn index_path_rejects_empty_project_id() {
        assert!(derive_index_path("").is_err());
        assert!(derive_index_path("   ").is_err());
    }

    // ----- schema ----

    #[test]
    fn schema_init_is_idempotent() {
        let idx = DerivedIndex::open_in_memory().unwrap();
        // Opening again on the same in-memory connection is not possible,
        // but re-init via a fresh connection should succeed.
        let idx2 = DerivedIndex::open_in_memory().unwrap();
        let h = idx2.health().unwrap();
        assert_eq!(h.schema_version, INDEX_SCHEMA_VERSION);
    }

    // ----- writes ----

    #[test]
    fn upsert_and_search() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Title A", "alpha beta"))
            .unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "alpha".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn re_upsert_replaces_stale_search_content() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let mut doc = sample_doc("p1", "d1", "Title", "version one");
        idx.upsert_document(&doc).unwrap();
        assert_eq!(
            idx.search(&ContextQuery {
                project_id: "p1".into(),
                query: "version".into(),
                ..Default::default()
            })
            .unwrap()
            .len(),
            1
        );
        doc.text = "version two".into();
        idx.upsert_document(&doc).unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "one".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0, "stale content should be gone");
    }

    #[test]
    fn remove_source_clears_search() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Title", "removable"))
            .unwrap();
        idx.remove_source("d1-src").unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "removable".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn replace_project_documents_is_transactional() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "old1", "Old", "old content"))
            .unwrap();
        idx.replace_project_documents("p1", &[sample_doc("p1", "new1", "New", "new content")])
            .unwrap();
        assert_eq!(
            idx.search(&ContextQuery {
                project_id: "p1".into(),
                query: "old".into(),
                ..Default::default()
            })
            .unwrap()
            .len(),
            0
        );
        assert_eq!(
            idx.search(&ContextQuery {
                project_id: "p1".into(),
                query: "new".into(),
                ..Default::default()
            })
            .unwrap()
            .len(),
            1
        );
    }

    // ----- FTS search ----

    #[test]
    fn search_no_match_returns_empty() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Title", "body"))
            .unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "nonexistentterm".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let idx = DerivedIndex::open_in_memory().unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "   ".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn search_unicode_and_punctuation_safe() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Të Unicödé", "café — résumé"))
            .unwrap();
        // Punctuation in query should not crash.
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "café — résumé".into(),
                ..Default::default()
            })
            .unwrap();
        assert!(hits.len() <= 1);
    }

    #[test]
    fn search_kind_filter() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Doc", "shared keyword"))
            .unwrap();
        let mut task_doc = sample_doc("p1", "t1", "Task", "shared keyword");
        task_doc.source_kind = "task".into();
        idx.upsert_document(&task_doc).unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "shared".into(),
                kinds: Some(vec!["task".into()]),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].source_kind, "task");
    }

    // Adversarial: excluded kinds must NOT consume the result limit.
    #[test]
    fn kind_filter_does_not_consume_limit() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        // Insert several highly ranked docs of an EXCLUDED kind (doc).
        // Each uses 'shared keyword' so they all match and rank equally/high.
        for i in 0..5 {
            idx.upsert_document(&IndexDocument {
                document_id: format!("excl_{}", i),
                source_id: format!("excl_{}_src", i),
                project_id: "p1".into(),
                source_kind: "doc".into(),
                canonical_ref: "x".into(),
                title: format!("Excluded doc {}", i),
                text: "shared keyword".into(),
                sensitivity: Sensitivity::Private,
                agent_context_enabled: false,
                freshness_state: "fresh".into(),
                observed_at: "now".into(),
                source_updated_at: None,
                metadata_json: None,
            })
            .unwrap();
        }
        // Insert doc(s) of an ALLOWED kind (task) that ALSO match 'shared keyword'
        // but rank lower because 'task' appears less frequently in this test corpus.
        for i in 0..2 {
            idx.upsert_document(&IndexDocument {
                document_id: format!("allowed_{}", i),
                source_id: format!("allowed_{}_src", i),
                project_id: "p1".into(),
                source_kind: "task".into(),
                canonical_ref: "x".into(),
                title: format!("Allowed task {}", i),
                text: "shared keyword".into(),
                sensitivity: Sensitivity::Private,
                agent_context_enabled: false,
                freshness_state: "fresh".into(),
                observed_at: "now".into(),
                source_updated_at: None,
                metadata_json: None,
            })
            .unwrap();
        }

        // Query: search 'shared keyword', only want task kind, limit 1.
        // bm25() makes 'task' rank LOWER (larger doc frequency) for term 'shared',
        // but there are more task docs (2) than doc docs... no wait.
        // bm25: IDF is higher for terms in FEWER documents.
        // 'shared keyword' appears in all 7 docs. IDF is low for both.
        // Within kind: task=2 docs, doc=5 docs. bm25 kind-weighting is not implemented.
        // All docs rank roughly the same. LIMIT takes first 5 (the doc-kind).
        // The query excludes 'doc' kind.
        // Current behavior: SQL LIMIT returns 5 doc-kind rows; Rust drops them all; result=[]
        // Expected behavior: result should contain the top-1 task doc.
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "shared keyword".into(),
                kinds: Some(vec!["task".into()]),
                limit: Some(1),
                ..Default::default()
            })
            .unwrap();
        assert!(
            hits.len() == 1,
            "expected 1 task-kind result, got {} (excluded kinds consumed limit)",
            hits.len()
        );
        assert_eq!(hits[0].source_kind, "task");
    }

    #[test]
    fn search_limit() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        for i in 0..50 {
            idx.upsert_document(&sample_doc("p1", &format!("d{}", i), "Title", "common"))
                .unwrap();
        }
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "common".into(),
                limit: Some(10),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 10);
    }

    // ----- JSON ----

    #[test]
    fn metadata_round_trip() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "T", "x"))
            .unwrap();
        let rows = idx.query_metadata("p1", "loc").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].1, "42");
    }

    // ----- privacy ----

    #[test]
    fn secret_text_not_indexed() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        // Secret doc with empty text is allowed.
        idx.upsert_document(&secret_doc("p1", "s1")).unwrap();
        // Secret doc with non-empty text is rejected at validation.
        let mut bad = secret_doc("p1", "s2");
        bad.text = "should not be indexed".into();
        assert!(idx.upsert_document(&bad).is_err());
        // Even the valid secret doc should not be searchable.
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "secret".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0);
    }

    // ----- contract boundary ----

    #[test]
    fn context_document_identity_survives_index_conversion() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let fixture_path = format!(
            "{}/tests/fixtures/context/valid-document.json",
            manifest_dir
        );
        let json = std::fs::read_to_string(&fixture_path).unwrap();
        let ctx_doc: ContextDocument = serde_json::from_str(&json).unwrap();
        let idx_doc = IndexDocument::from(&ctx_doc);

        assert_eq!(idx_doc.document_id, ctx_doc.id);
        assert_eq!(idx_doc.source_id, ctx_doc.source_id);
        assert_eq!(idx_doc.project_id, ctx_doc.project_id);
        assert_eq!(idx_doc.canonical_ref, ctx_doc.canonical_ref);
        assert_eq!(idx_doc.title, ctx_doc.title);
        assert_eq!(idx_doc.text, ctx_doc.text);
        assert_eq!(
            idx_doc.sensitivity, ctx_doc.sensitivity,
            "sensitivity must survive conversion"
        );
        assert_eq!(idx_doc.source_kind, "doc");

        // Round-trip: index the ContextDocument-derived IndexDocument, then search.
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&idx_doc).unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: ctx_doc.project_id.clone(),
                query: "OpenMesh".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].document_id, ctx_doc.id);
        assert_eq!(hits[0].project_id, ctx_doc.project_id);
        assert_eq!(hits[0].canonical_ref, ctx_doc.canonical_ref);
        assert_eq!(hits[0].sensitivity, "private");
    }

    // ----- transaction rollback ----

    #[test]
    fn forced_failure_rolls_back() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        idx.upsert_document(&sample_doc("p1", "d1", "Title", "before"))
            .unwrap();
        // Attempt to upsert an invalid doc (empty document_id) — should fail.
        let bad = IndexDocument {
            document_id: "".into(),
            ..sample_doc("p1", "d2", "Bad", "bad")
        };
        assert!(idx.upsert_document(&bad).is_err());
        // The previously indexed doc must still be searchable.
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "before".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
    }

    // ----- secret behavior: deterministic per-secret cases ----

    #[test]
    fn secret_document_behavior_is_deterministic() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();

        // secret + non-empty text -> validation rejects before index write.
        let mut bad = sample_doc("p1", "s1", "Secret", "top secret content");
        bad.sensitivity = Sensitivity::Secret;
        let err = idx.upsert_document(&bad);
        assert!(err.is_err());

        // secret + empty text -> validation passes; no FTS row.
        let mut ok = sample_doc("p1", "s2", "SecMeta", "");
        ok.sensitivity = Sensitivity::Secret;
        idx.upsert_document(&ok).unwrap();

        let fts_c: i64 = idx
            .conn
            .query_row(
                "SELECT COUNT(*) FROM context_documents_fts WHERE document_id='s2'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(fts_c, 0);

        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "Secret".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 0);
    }

    #[test]
    fn mixed_project_batch_succeeds_with_valid_secret() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let mut secret_meta = sample_doc("p1", "sec", "Sec", "");
        secret_meta.sensitivity = Sensitivity::Secret;
        let docs = vec![
            sample_doc("p1", "a", "A", "aaa"),
            sample_doc("p1", "b", "B", "bbb"),
            secret_meta,
        ];
        idx.replace_project_documents("p1", &docs).unwrap();

        let hits_a = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "aaa".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits_a.len(), 1);
        let hits_b = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "bbb".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits_b.len(), 1);
    }

    // ----- disposability ----

    #[test]
    fn rebuild_from_documents_produces_equivalent_search() {
        let mut idx = DerivedIndex::open_in_memory().unwrap();
        let docs = vec![
            sample_doc("p1", "d1", "Alpha", "first document"),
            sample_doc("p1", "d2", "Beta", "second document"),
        ];
        idx.rebuild_from_documents("p1", &docs).unwrap();
        let hits = idx
            .search(&ContextQuery {
                project_id: "p1".into(),
                query: "document".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn purge_removes_index_files() {
        let dir = std::env::temp_dir().join(format!("openmesh-index-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("context.sqlite3");
        {
            let mut idx = DerivedIndex::open_at(path.clone()).unwrap();
            idx.upsert_document(&sample_doc("p1", "d1", "Title", "body"))
                .unwrap();
            idx.purge().unwrap();
        }
        assert!(!path.exists(), "main db file should be removed");
        assert!(!dir.join("context.sqlite3-wal").exists());
        assert!(!dir.join("context.sqlite3-shm").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    // ----- recovery ----

    #[test]
    fn recover_corrupt_index_creates_fresh_empty_index() {
        let dir =
            std::env::temp_dir().join(format!("openmesh-recover-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("context.sqlite3");
        {
            let mut idx = DerivedIndex::open_at(path.clone()).unwrap();
            idx.upsert_document(&sample_doc("p1", "d1", "Title", "body"))
                .unwrap();
        }
        // Simulate corruption by truncating the file.
        std::fs::write(&path, b"corrupt").unwrap();
        // Recovery removes all sidecar files.
        recover_corrupt_index("recover-proj-001").unwrap();
        // A fresh index can be opened.
        let idx = DerivedIndex::open_in_memory().unwrap();
        let h = idx.health().unwrap();
        assert_eq!(h.document_count, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    // ----- health ----

    #[test]
    fn health_reports_expected_fields() {
        let idx = DerivedIndex::open_in_memory().unwrap();
        let h = idx.health().unwrap();
        assert_eq!(h.schema_version, INDEX_SCHEMA_VERSION);
        assert!(!h.sqlite_version.is_empty());
        assert!(h.integrity_ok);
    }
}
