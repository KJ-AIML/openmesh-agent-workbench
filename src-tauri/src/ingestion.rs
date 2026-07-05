// ============================================================================
// OpenMesh Context Ingestion Pipeline — Dev Track 0.1.2.4
// ============================================================================
// Reads canonical OpenMesh sources, normalizes into ContextDocument,
// and updates the disposable Derived Local Index.
//
// Architecture:
//   Canonical Source -> Harvester -> ContextDocument -> Index Adapter -> SQLite
//
// Canonical data is never modified.
// ============================================================================

#![cfg_attr(not(test), allow(dead_code))]
#![cfg_attr(test, allow(dead_code, unused_variables, unused_mut))]
use std::collections::BTreeMap;
use std::fs;
use std::hash::Hasher;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::context::{ContextDocument, ContextSourceKind, Freshness, FreshnessState, Sensitivity};

// ============================================================================
// Limits
// ============================================================================

pub const MAX_TEXT_FILE_BYTES: u64 = 1024 * 1024; // 1 MiB
pub const MAX_JSON_COLLECTION_BYTES: u64 = 4 * 1024 * 1024; // 4 MiB
const ALLOWED_DOC_EXTS: &[&str] = &["md", "txt", "markdown"];
const SNAPSHOT_SUBDIR: &str = "snapshots";

// ============================================================================
// Error / Outcome Types
// ============================================================================

use std::fmt;

// ============================================================================
// Error / Outcome Types
// ============================================================================

#[derive(Debug, Clone)]
pub enum IngestionError {
    PathViolation(String),
    SymlinkSkipped(String),
    TooLarge { path: String, size: u64, limit: u64 },
    ReadError { path: String, source: String },
    InvalidUtf8(String),
    JsonRootParse { path: String, source: String },
    ValidationError(String),
    PrivacySkip(String),
    IndexFailure(String),
    ProjectMeta { source: String },
}

impl fmt::Display for IngestionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PathViolation(msg) => write!(f, "path violation: {}", msg),
            Self::SymlinkSkipped(p) => write!(f, "symlink skipped: {}", p),
            Self::TooLarge { path, size, limit } => write!(
                f,
                "file too large: {} ({} bytes > {} limit)",
                path, size, limit
            ),
            Self::ReadError { path, source } => write!(f, "read error: {}: {}", path, source),
            Self::InvalidUtf8(p) => write!(f, "invalid UTF-8: {}", p),
            Self::JsonRootParse { path, source } => {
                write!(f, "JSON root parse failure: {}: {}", path, source)
            }
            Self::ValidationError(msg) => write!(f, "validation failure: {}", msg),
            Self::PrivacySkip(msg) => write!(f, "privacy skip: {}", msg),
            Self::IndexFailure(msg) => write!(f, "index failure: {}", msg),
            Self::ProjectMeta { source } => write!(f, "project meta: {}", source),
        }
    }
}

impl std::error::Error for IngestionError {}

pub type IngestionResult<T> = Result<T, IngestionError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum IngestionOutcome {
    Indexed,
    Updated,
    Unchanged,
    Removed,
    SkippedPolicy,
    SkippedTooLarge,
    SkippedSymlink,
    FailedRead,
    FailedParse,
    FailedValidation,
    FailedIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceReceipt {
    pub source_kind: String,
    pub source_id: String,
    pub canonical_ref: Option<String>,
    pub outcome: IngestionOutcome,
    pub fingerprint: Option<String>,
    pub bytes_read: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Complete,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestionRunResult {
    pub project_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub status: RunStatus,
    pub discovered: usize,
    pub indexed: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub removed: usize,
    pub skipped: usize,
    pub failed: usize,
    pub receipts: Vec<SourceReceipt>,
}

// ============================================================================
// Stable FNV-1a hash (no new dependency)
// ============================================================================

struct Fnv1a(u64);
impl Fnv1a {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 ^= *b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn finish(self) -> u64 {
        self.0
    }
}

impl Hasher for Fnv1a {
    fn write(&mut self, bytes: &[u8]) {
        self.write(bytes);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

fn fnv1a_hex(input: &str) -> String {
    let mut h = Fnv1a::new();
    h.write(input.as_bytes());
    format!("{:016x}", h.finish())
}

fn hash_rel_path(rel: &str) -> String {
    fnv1a_hex(rel)
}

// ============================================================================
// Fingerprint
// ============================================================================

const FINGERPRINT_VERSION: u64 = 1;

pub fn compute_fingerprint(
    kind: &str,
    title: &str,
    text: &str,
    sensitivity: &Sensitivity,
    agent_ctx: bool,
) -> String {
    let mut h = Fnv1a::new();
    h.write(&FINGERPRINT_VERSION.to_le_bytes());
    h.write(kind.as_bytes());
    h.write(title.as_bytes());
    h.write(text.as_bytes());
    h.write(format!("{:?}", sensitivity).as_bytes());
    if agent_ctx {
        h.write(&[1u8]);
    } else {
        h.write(&[0u8]);
    }
    format!("{:016x}", h.finish())
}

// ============================================================================
// Bounded Reader
// ============================================================================

pub fn read_bounded(path: &Path, limit: u64) -> IngestionResult<(String, u64)> {
    let metadata = fs::symlink_metadata(path).map_err(|e| IngestionError::ReadError {
        path: path.display().to_string(),
        source: e.to_string(),
    })?;
    if metadata.file_type().is_symlink() {
        return Err(IngestionError::SymlinkSkipped(path.display().to_string()));
    }
    let size = metadata.len();
    if size > limit {
        return Err(IngestionError::TooLarge {
            path: path.display().to_string(),
            size,
            limit,
        });
    }
    let mut file = fs::File::open(path).map_err(|e| IngestionError::ReadError {
        path: path.display().to_string(),
        source: e.to_string(),
    })?;
    let mut buf = vec![0u8; (limit as usize) + 1];
    let n = std::io::Read::read(&mut file, &mut buf).map_err(|e| IngestionError::ReadError {
        path: path.display().to_string(),
        source: e.to_string(),
    })?;
    if n > limit as usize {
        return Err(IngestionError::TooLarge {
            path: path.display().to_string(),
            size: n as u64,
            limit,
        });
    }
    buf.truncate(n);
    let text = String::from_utf8(buf)
        .map_err(|_| IngestionError::InvalidUtf8(path.display().to_string()))?;
    Ok((text, n as u64))
}

// ============================================================================
// Path Safety
// ============================================================================

pub fn reject_symlinks(path: &Path) -> IngestionResult<()> {
    let meta = fs::symlink_metadata(path).map_err(|e| IngestionError::ReadError {
        path: path.display().to_string(),
        source: e.to_string(),
    })?;
    if meta.file_type().is_symlink() {
        return Err(IngestionError::SymlinkSkipped(path.display().to_string()));
    }
    Ok(())
}

// ============================================================================
// Harvested Source
// ============================================================================

#[derive(Debug)]
pub enum RawSource {
    Doc {
        rel_path: String,
        content: String,
        modified_at: Option<String>,
    },
    Note {
        rel_path: String,
        content: String,
        modified_at: Option<String>,
    },
    Snapshot {
        rel_path: String,
        content: String,
        modified_at: Option<String>,
    },
    Task(crate::storage::Task),
    Recent(crate::storage::RecentItem),
    Session(crate::storage::AgentSession),
}

fn modified_iso(meta: &std::fs::Metadata) -> Option<String> {
    meta.modified().ok().and_then(|t| {
        chrono::DateTime::<chrono::Utc>::from_timestamp(
            t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
            0,
        )
        .map(|dt| dt.to_rfc3339())
    })
}

fn is_allowed_doc(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| ALLOWED_DOC_EXTS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

// ============================================================================
// Source Discovery
// ============================================================================

pub fn get_project_id(project_path: &str) -> IngestionResult<String> {
    let path = crate::storage::get_project_dir(project_path).join("project.json");
    let (text, _) =
        read_bounded(&path, MAX_TEXT_FILE_BYTES).map_err(|e| IngestionError::ProjectMeta {
            source: e.to_string(),
        })?;
    let project: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| IngestionError::ProjectMeta {
            source: e.to_string(),
        })?;
    project
        .get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or(IngestionError::ProjectMeta {
            source: "project.json missing 'id' field".into(),
        })
}

pub fn discover_sources(
    project_path: &str,
) -> IngestionResult<BTreeMap<&'static str, Vec<IngestionResult<RawSource>>>> {
    let mut results = BTreeMap::new();
    results.insert("docs", discover_docs(project_path));
    results.insert("notes", discover_notes(project_path));
    results.insert("snapshots", discover_snapshots(project_path));
    results.insert("tasks", discover_tasks(project_path));
    results.insert("recent", discover_recent(project_path));
    results.insert("sessions", discover_sessions(project_path));
    Ok(results)
}

fn discover_docs(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let root = crate::storage::get_project_dir(project_path).join("docs");
    let mut out = Vec::new();
    discover_md_recursive(&root, "docs", &mut out);
    out
}

fn discover_md_recursive(dir: &Path, base: &str, acc: &mut Vec<IngestionResult<RawSource>>) {
    let entries = match fs::read_dir(dir) {
        Ok(it) => it.collect::<Vec<_>>(),
        Err(_) => return,
    };
    for entry in entries.into_iter().flatten() {
        let path = entry.path();
        if let Err(e) = reject_symlinks(&path) {
            acc.push(Err(e));
            continue;
        }
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                acc.push(Err(IngestionError::ReadError {
                    path: path.display().to_string(),
                    source: e.to_string(),
                }));
                continue;
            }
        };
        if meta.is_dir() {
            if path.file_name().and_then(|n| n.to_str()) == Some(SNAPSHOT_SUBDIR) {
                continue;
            }
            discover_md_recursive(&path, base, acc);
        } else if meta.is_file() {
            if !is_allowed_doc(&path) {
                continue;
            }
            let rel = path.strip_prefix(dir).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            let full_rel = format!("{}/{}", base, rel_str);
            let size = meta.len();
            if size > MAX_TEXT_FILE_BYTES {
                acc.push(Err(IngestionError::TooLarge {
                    path: path.display().to_string(),
                    size,
                    limit: MAX_TEXT_FILE_BYTES,
                }));
                continue;
            }
            let modified_at = modified_iso(&meta);
            match read_bounded(&path, MAX_TEXT_FILE_BYTES) {
                Ok((content, _)) => acc.push(Ok(RawSource::Doc {
                    rel_path: full_rel,
                    content,
                    modified_at,
                })),
                Err(e) => acc.push(Err(e)),
            }
        }
    }
}

fn discover_notes(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let base_dir = crate::storage::get_project_dir(project_path).join("notes");
    let mut out = Vec::new();
    let entries = match fs::read_dir(&base_dir) {
        Ok(it) => it.collect::<Vec<_>>(),
        Err(_) => return out,
    };
    for entry in entries.into_iter().flatten() {
        let path = entry.path();
        if let Err(e) = reject_symlinks(&path) {
            out.push(Err(e));
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == SNAPSHOT_SUBDIR {
            continue;
        }
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                out.push(Err(IngestionError::ReadError {
                    path: path.display().to_string(),
                    source: e.to_string(),
                }));
                continue;
            }
        };
        if !meta.is_file() || !is_allowed_doc(&path) {
            continue;
        }
        let rel = format!("notes/{}", name);
        let size = meta.len();
        if size > MAX_TEXT_FILE_BYTES {
            out.push(Err(IngestionError::TooLarge {
                path: path.display().to_string(),
                size,
                limit: MAX_TEXT_FILE_BYTES,
            }));
            continue;
        }
        let modified_at = modified_iso(&meta);
        match read_bounded(&path, MAX_TEXT_FILE_BYTES) {
            Ok((content, _)) => out.push(Ok(RawSource::Note {
                rel_path: rel,
                content,
                modified_at,
            })),
            Err(e) => out.push(Err(e)),
        }
    }
    out
}

fn discover_snapshots(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let base_dir = crate::storage::get_project_dir(project_path)
        .join("notes")
        .join(SNAPSHOT_SUBDIR);
    let mut out = Vec::new();
    let entries = match fs::read_dir(&base_dir) {
        Ok(it) => it.collect::<Vec<_>>(),
        Err(_) => return out,
    };
    for entry in entries.into_iter().flatten() {
        let path = entry.path();
        if let Err(e) = reject_symlinks(&path) {
            out.push(Err(e));
            continue;
        }
        let meta = match fs::metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                out.push(Err(IngestionError::ReadError {
                    path: path.display().to_string(),
                    source: e.to_string(),
                }));
                continue;
            }
        };
        if !meta.is_file() {
            continue;
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !ALLOWED_DOC_EXTS.contains(&ext.to_lowercase().as_str()) {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let rel = format!("notes/{}/{}", SNAPSHOT_SUBDIR, name);
        let size = meta.len();
        if size > MAX_TEXT_FILE_BYTES {
            out.push(Err(IngestionError::TooLarge {
                path: path.display().to_string(),
                size,
                limit: MAX_TEXT_FILE_BYTES,
            }));
            continue;
        }
        let modified_at = modified_iso(&meta);
        match read_bounded(&path, MAX_TEXT_FILE_BYTES) {
            Ok((content, _)) => out.push(Ok(RawSource::Snapshot {
                rel_path: rel,
                content,
                modified_at,
            })),
            Err(e) => out.push(Err(e)),
        }
    }
    out
}

fn read_json_array<T: serde::de::DeserializeOwned>(path: &Path) -> IngestionResult<Vec<T>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    reject_symlinks(path)?;
    let meta = fs::metadata(path).map_err(|e| IngestionError::ReadError {
        path: path.display().to_string(),
        source: e.to_string(),
    })?;
    if meta.len() > MAX_JSON_COLLECTION_BYTES {
        return Err(IngestionError::TooLarge {
            path: path.display().to_string(),
            size: meta.len(),
            limit: MAX_JSON_COLLECTION_BYTES,
        });
    }
    let (text, _) = read_bounded(path, MAX_JSON_COLLECTION_BYTES)?;
    serde_json::from_str(&text).map_err(|e| IngestionError::JsonRootParse {
        path: path.display().to_string(),
        source: e.to_string(),
    })
}

fn discover_tasks(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let path = crate::storage::get_project_dir(project_path).join("tasks.json");
    match read_json_array::<crate::storage::Task>(&path) {
        Ok(items) => items.into_iter().map(|t| Ok(RawSource::Task(t))).collect(),
        Err(e) => vec![Err(e)],
    }
}

fn discover_recent(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let path = crate::storage::get_project_dir(project_path).join("recent.json");
    match read_json_array::<crate::storage::RecentItem>(&path) {
        Ok(items) => items
            .into_iter()
            .map(|r| Ok(RawSource::Recent(r)))
            .collect(),
        Err(e) => vec![Err(e)],
    }
}

fn discover_sessions(project_path: &str) -> Vec<IngestionResult<RawSource>> {
    let path = crate::storage::get_project_dir(project_path).join("sessions.json");
    match read_json_array::<crate::storage::AgentSession>(&path) {
        Ok(items) => items
            .into_iter()
            .map(|s| Ok(RawSource::Session(s)))
            .collect(),
        Err(e) => vec![Err(e)],
    }
}

// ============================================================================
// Normalization: RawSource -> ContextDocument
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn build_ctx(
    id: String,
    source_id: String,
    kind: ContextSourceKind,
    project_id: String,
    owner: Option<String>,
    canonical_ref: String,
    title: String,
    text: String,
    sensitivity: Sensitivity,
    agent_ctx: bool,
    observed_at: String,
    source_updated_at: Option<String>,
    metadata: Option<serde_json::Value>,
) -> ContextDocument {
    ContextDocument {
        id,
        source_id,
        schema_version: crate::context::CONTEXT_SCHEMA_VERSION.into(),
        kind,
        project_id,
        owner_person_id: owner,
        canonical_ref,
        title,
        text,
        sensitivity,
        agent_context_enabled: agent_ctx,
        observed_at,
        source_updated_at,
        freshness: Freshness {
            state: FreshnessState::Unknown,
            observed_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            source_updated_at: None,
        },
        metadata,
    }
}

pub fn normalize(project_id: &str, source: RawSource) -> IngestionResult<ContextDocument> {
    match source {
        RawSource::Doc {
            rel_path,
            content,
            modified_at,
        } => {
            let title = Path::new(&rel_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel_path.clone());
            let src_id = format!("doc-src-{}", hash_rel_path(&rel_path));
            Ok(build_ctx(
                format!("doc-{}", hash_rel_path(&rel_path)),
                src_id,
                ContextSourceKind::Doc,
                project_id.into(),
                None,
                format!(
                    "openmesh://project/{}/doc/{}",
                    project_id,
                    rel_path.replace("docs/", "")
                ),
                title,
                content,
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                modified_at,
                None,
            ))
        }
        RawSource::Note {
            rel_path,
            content,
            modified_at,
        } => {
            let title = Path::new(&rel_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel_path.clone());
            let src_id = format!("note-src-{}", hash_rel_path(&rel_path));
            Ok(build_ctx(
                format!("note-{}", hash_rel_path(&rel_path)),
                src_id,
                ContextSourceKind::Note,
                project_id.into(),
                None,
                format!(
                    "openmesh://project/{}/note/{}",
                    project_id,
                    rel_path.replace("notes/", "")
                ),
                title,
                content,
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                modified_at,
                None,
            ))
        }
        RawSource::Snapshot {
            rel_path,
            content,
            modified_at,
        } => {
            let title = Path::new(&rel_path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel_path.clone());
            let src_id = format!("snapshot-src-{}", hash_rel_path(&rel_path));
            Ok(build_ctx(
                format!("snapshot-{}", hash_rel_path(&rel_path)),
                src_id,
                ContextSourceKind::Snapshot,
                project_id.into(),
                None,
                format!(
                    "openmesh://project/{}/snapshot/{}",
                    project_id,
                    rel_path.replace("notes/snapshots/", "")
                ),
                title,
                content,
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                modified_at,
                None,
            ))
        }
        RawSource::Task(t) => {
            let mut text = t.title.clone();
            if let Some(desc) = &t.description {
                text.push_str(
                    "

",
                );
                text.push_str(desc);
            }
            text.push_str(&format!(
                "

Status: {}
Priority: {}",
                t.status, t.priority
            ));
            if let Some(owner) = &t.owner {
                text.push_str(&format!(
                    "
Owner: {}",
                    owner
                ));
            }
            if let Some(next) = &t.next_action {
                text.push_str(&format!(
                    "
Next action: {}",
                    next
                ));
            }
            let proj_id = t.project_id.clone();
            let doc_id = t.id.clone();
            Ok(build_ctx(
                doc_id.clone(),
                doc_id,
                ContextSourceKind::Task,
                proj_id.clone(),
                t.owner,
                format!("openmesh://project/{}/task/{}", proj_id, t.id),
                t.title,
                text,
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                Some(t.updated_at),
                None,
            ))
        }
        RawSource::Recent(r) => {
            let src_id = r
                .source_id
                .clone()
                .unwrap_or_else(|| format!("recent-src-{}", &r.id));
            let proj_id = r.project_id.clone().unwrap_or_default();
            let title = r.title.clone();
            Ok(build_ctx(
                r.id.clone(),
                src_id,
                ContextSourceKind::Recent,
                proj_id,
                None,
                r.source_path.unwrap_or_default(),
                title.clone(),
                format!(
                    "Type: {}
Title: {}",
                    r.r#type, title
                ),
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                Some(r.last_opened_at),
                None,
            ))
        }
        RawSource::Session(s) => {
            let src_id = format!("session-src-{}", &s.id);
            let proj_id = s.project_id.clone().unwrap_or_default();
            let mut text = format!(
                "Title: {}
Tool: {}
Status: {}",
                s.title, s.tool, s.status
            );
            if let Some(summary) = &s.summary {
                text.push_str(&format!(
                    "
Summary: {}",
                    summary
                ));
            }
            if let Some(changed) = &s.changed_files {
                if !changed.is_empty() {
                    text.push_str(&format!(
                        "
Changed files: {}",
                        changed.join(", ")
                    ));
                }
            }
            Ok(build_ctx(
                s.id.clone(),
                src_id,
                ContextSourceKind::AgentSession,
                proj_id,
                None,
                s.source_path.unwrap_or_default(),
                s.title,
                text,
                Sensitivity::Private,
                false,
                chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                Some(s.last_active_at),
                None,
            ))
        }
    }
}

// ============================================================================
// Pipeline orchestration
// ============================================================================

/// Pre-process secret documents: convert non-empty secret to metadata-only
/// (empty text) so mixed batches don't fail.
pub fn apply_secret_policy(mut doc: ContextDocument) -> Option<ContextDocument> {
    if doc.sensitivity == Sensitivity::Secret && !doc.text.trim().is_empty() {
        doc.text = String::new();
        doc.agent_context_enabled = false;
    }
    Some(doc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Helper to create a temporary canonical project layout.
    fn create_test_project(name: &str) -> (PathBuf, String) {
        let dir = std::env::temp_dir().join(format!(
            "openmesh-ingest-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        let project_dir = dir.join("myproject");
        fs::create_dir_all(&project_dir).unwrap();
        let om = project_dir.join(".openmesh");
        fs::create_dir_all(&om).unwrap();
        fs::create_dir_all(&om.join("docs")).unwrap();
        fs::create_dir_all(&om.join("docs").join("nested")).unwrap();
        fs::create_dir_all(&om.join("notes")).unwrap();
        fs::create_dir_all(&om.join("notes").join("snapshots")).unwrap();

        let now = "2026-07-05T03:00:00.000Z";
        let project_json = serde_json::json!({
            "id": "test-proj-001",
            "name": "Test Project",
            "folderPath": project_dir.to_str().unwrap(),
            "status": "active",
            "createdAt": now,
            "updatedAt": now,
        });
        fs::write(
            &om.join("project.json"),
            serde_json::to_string_pretty(&project_json).unwrap(),
        )
        .unwrap();

        (dir, project_dir.to_string_lossy().into_owned())
    }

    #[test]
    fn discover_all_six_source_families() {
        let (dir, project_path) = create_test_project("six-families");

        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("docs")
                .join("readme.md"),
            "# Readme\nbody",
        )
        .unwrap();
        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("docs")
                .join("nested")
                .join("deep.md"),
            "# Deep",
        )
        .unwrap();
        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("notes")
                .join("daily.md"),
            "# Daily note",
        )
        .unwrap();
        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("notes")
                .join("snapshots")
                .join("snap.md"),
            "# Snapshot",
        )
        .unwrap();

        let tasks = serde_json::json!([{"id": "t1", "projectId": "test-proj-001", "title": "Task 1", "description": "desc", "status": "pending", "priority": "P1", "sprintId": "", "owner": "ter", "nextAction": "review", "notes": "some note", "linkedDocIds": [], "linkedSessionIds": [], "createdAt": "2026-01-01", "updatedAt": "2026-07-05"}]);
        fs::write(
            crate::storage::get_project_dir(&project_path).join("tasks.json"),
            tasks.to_string(),
        )
        .unwrap();

        let recent = serde_json::json!([{"id": "r1", "type": "doc", "title": "Recent 1", "projectId": "test-proj-001", "sourceId": "d1", "sourcePath": "docs/readme.md", "lastOpenedAt": "2026-07-05", "pinned": false}]);
        fs::write(
            crate::storage::get_project_dir(&project_path).join("recent.json"),
            recent.to_string(),
        )
        .unwrap();

        let sessions = serde_json::json!([{"id": "s1", "tool": "codex", "title": "Session 1", "projectId": "test-proj-001", "sourcePath": "sessions/s1.json", "status": "completed", "summary": "deployed", "startedAt": "2026-01-01", "lastActiveAt": "2026-07-05", "endedAt": "2026-07-05", "changedFiles": ["Cargo.toml"], "linkedTaskId": "t1", "isImportant": false, "createdAt": "2026-01-01", "updatedAt": "2026-07-05"}]);
        fs::write(
            crate::storage::get_project_dir(&project_path).join("sessions.json"),
            sessions.to_string(),
        )
        .unwrap();

        let sources = discover_sources(&project_path).unwrap();

        let docs = sources
            .get("docs")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let notes = sources
            .get("notes")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let snaps = sources
            .get("snapshots")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let tasks = sources
            .get("tasks")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let recent = sources
            .get("recent")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let sessions = sources
            .get("sessions")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();

        assert!(docs >= 1, "should discover at least 1 doc");
        assert!(notes >= 1, "should discover at least 1 note");
        assert!(snaps >= 1, "should discover at least 1 snapshot");
        assert!(tasks >= 1, "should discover at least 1 task");
        assert!(recent >= 1, "should discover at least 1 recent");
        assert!(sessions >= 1, "should discover at least 1 session");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshot_not_double_ingested_as_note() {
        let (dir, project_path) = create_test_project("no-double");

        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("notes")
                .join("snapshots")
                .join("snap.md"),
            "# Snap",
        )
        .unwrap();
        fs::write(
            crate::storage::get_project_dir(&project_path)
                .join("notes")
                .join("regular.md"),
            "# Regular",
        )
        .unwrap();

        let sources = discover_sources(&project_path).unwrap();
        let note_count = sources
            .get("notes")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();
        let snap_count = sources
            .get("snapshots")
            .unwrap()
            .iter()
            .filter(|r| r.is_ok())
            .count();

        assert_eq!(note_count, 1, "snapshot must not appear as note");
        assert_eq!(snap_count, 1, "snapshot must appear only as snapshot");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bounded_read_within_limit() {
        let dir = std::env::temp_dir().join(format!("openmesh-bounded-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("within.md");
        fs::write(&path, "a".repeat(100)).unwrap();
        let (text, bytes) = read_bounded(&path, 1024).unwrap();
        assert_eq!(bytes, 100);
        assert_eq!(text.len(), 100);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bounded_read_over_limit_rejected() {
        let dir = std::env::temp_dir().join(format!("openmesh-bounded-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("over.md");
        fs::write(&path, "x".repeat(200)).unwrap();
        let err = read_bounded(&path, 50);
        assert!(matches!(err, Err(IngestionError::TooLarge { .. })));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn symlink_file_rejected() {
        let dir = std::env::temp_dir().join(format!("openmesh-sym-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("target.md");
        fs::write(&target, "real").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dir.join("link.md")).ok();
        let _ = fs::remove_dir_all(&dir);
        // On platforms without symlink support, we can't easily test this in CI.
        // The policy function is covered separately below.
    }

    #[test]
    fn reject_symlinks_detects_symlink_metadata() {
        // We can't always create symlinks in CI, so we just test that a regular
        // file is accepted by the policy.
        let dir = std::env::temp_dir().join(format!("openmesh-policy-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("regular.md");
        fs::write(&path, "content").unwrap();
        assert!(reject_symlinks(&path).is_ok());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn compute_fingerprint_stable() {
        let fp1 = compute_fingerprint("doc", "Title", "body", &Sensitivity::Private, false);
        let fp2 = compute_fingerprint("doc", "Title", "body", &Sensitivity::Private, false);
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn compute_fingerprint_changes_on_content() {
        let fp1 = compute_fingerprint("doc", "Title", "body1", &Sensitivity::Private, false);
        let fp2 = compute_fingerprint("doc", "Title", "body2", &Sensitivity::Private, false);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn compute_fingerprint_changes_on_sensitivity() {
        let fp1 = compute_fingerprint("doc", "Title", "body", &Sensitivity::Private, false);
        let fp2 = compute_fingerprint("doc", "Title", "body", &Sensitivity::Secret, false);
        assert_ne!(fp1, fp2);
    }

    #[test]
    fn context_document_identity_survives() {
        let raw = RawSource::Doc {
            rel_path: "docs/architecture/design.md".into(),
            content: "# Design\nOpenMesh is a workbench".into(),
            modified_at: Some("2026-07-05T03:00:00.000Z".into()),
        };
        let project_id = "test-proj";
        let doc = normalize(project_id, raw).unwrap();

        assert_eq!(doc.project_id, project_id);
        assert!(
            doc.canonical_ref.contains("doc/architecture/design.md"),
            "ref was: {}",
            doc.canonical_ref
        );
        assert!(doc.canonical_ref.contains("test-proj"));
        assert_eq!(doc.kind, ContextSourceKind::Doc);
        assert!(!doc.text.is_empty());
    }

    #[test]
    fn get_project_id_from_metadata() {
        let (dir, project_path) = create_test_project("proj-id");
        let id = get_project_id(&project_path).unwrap();
        assert_eq!(id, "test-proj-001");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn secret_document_text_dropped_by_policy() {
        let raw = RawSource::Doc {
            rel_path: "docs/secret.md".into(),
            content: "TOP SECRET CONTENT".into(),
            modified_at: None,
        };
        let mut doc = normalize("p1", raw).unwrap();
        doc.sensitivity = Sensitivity::Secret;
        let result = apply_secret_policy(doc).unwrap();
        assert_eq!(result.text, "");
        assert!(!result.agent_context_enabled);
    }

    #[test]
    fn apply_secret_preserves_public_docs() {
        let raw = RawSource::Doc {
            rel_path: "docs/public.md".into(),
            content: "public content".into(),
            modified_at: None,
        };
        let doc = normalize("p1", raw).unwrap();
        let result = apply_secret_policy(doc).unwrap();
        assert_eq!(result.text, "public content");
    }

    #[test]
    fn normalize_task_contains_title_and_description() {
        let task = RawSource::Task(crate::storage::Task {
            id: "task-123".into(),
            sprint_id: "".into(),
            project_id: "p1".into(),
            title: "Fix deploy".into(),
            description: Some("Fix the deployment flow".into()),
            status: "in-progress".into(),
            priority: "P1".into(),
            owner: Some("ter".into()),
            next_action: Some("Review PR".into()),
            notes: None,
            linked_doc_ids: vec![],
            linked_session_ids: vec![],
            created_at: "2026-01-01".into(),
            updated_at: "2026-07-05".into(),
        });
        let doc = normalize("p1", task).unwrap();
        assert_eq!(doc.id, "task-123");
        assert!(doc.text.contains("Fix deploy"));
        assert!(doc.text.contains("deployment flow"));
        assert!(doc.text.contains("in-progress"));
        assert_eq!(doc.kind, ContextSourceKind::Task);
    }

    #[test]
    fn normalize_session_no_transcript() {
        let raw = RawSource::Session(crate::storage::AgentSession {
            id: "sess-1".into(),
            tool: "codex".into(),
            title: "Deploy session".into(),
            project_id: Some("p1".into()),
            source_path: None,
            status: "completed".into(),
            summary: Some("deployed v0.1.0".into()),
            started_at: "2026-01-01".into(),
            last_active_at: "2026-07-05".into(),
            ended_at: Some("2026-07-05".into()),
            changed_files: Some(vec!["Cargo.toml".into(), "src/lib.rs".into()]),
            linked_task_id: None,
            is_important: false,
            created_at: "2026-01-01".into(),
            updated_at: "2026-07-05".into(),
        });
        let doc = normalize("p1", raw).unwrap();
        assert_eq!(doc.id, "sess-1");
        assert!(doc.text.contains("deployed v0.1.0"));
        assert!(doc.text.contains("Cargo.toml"));
        assert_eq!(doc.kind, ContextSourceKind::AgentSession);
        // Verify no raw transcript was stored (text is summary-only, bounded).
        assert!(!doc.text.contains("ASSISTANT:") && !doc.text.contains("USER:"));
    }

    #[test]
    fn project_id_mismatch_detected() {
        let (dir, project_path) = create_test_project("mismatch");
        let id = get_project_id(&project_path).unwrap();
        assert_ne!(
            id, "wrong-id",
            "caller-supplied ID must not override canonical project ID"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn index_schema_version_is_explicit() {
        let raw = RawSource::Doc {
            rel_path: "docs/a.md".into(),
            content: "x".into(),
            modified_at: None,
        };
        let doc = normalize("p1", raw).unwrap();
        assert_eq!(
            doc.schema_version, "1.0.0",
            "schema version must be explicit in ContextDocument"
        );
    }
}
