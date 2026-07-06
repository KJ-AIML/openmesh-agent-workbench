# Changelog

All notable changes to OpenMesh are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.2] - 2026-07-06

### Added

- **Context domain foundation**: ContextSource and ContextDocument contracts with schema versioning, canonical references, and deterministic source IDs
- **Local derived index**: Disposable SQLite index with FTS5 full-text search, project-scoped isolation, and WAL mode for concurrent access
- **Safe canonical ingestion**: Bounded reads (1 MiB text, 4 MiB JSON), path traversal protection, symlink rejection, and family-scoped transactional writes
- **Local Context Search**: FTS5 lexical search with bm25 ranking, kind filtering, result limits, and project-scoped queries
- **Context Inspector**: Read-only document inspection with provenance metadata, bounded text preview (4000 chars), and secret text redaction
- **Manual refresh and index health**: Structured refresh status (COMPLETE/PARTIAL/FAILED), ingestion receipts, and index health reporting (document count, schema version, integrity check)
- **Command Palette integration**: "Search Context" command for quick access to Context Search
- **Open Source navigation**: Deep-link from search results to source documents (Docs, Notes, Snapshots, Tasks, Sessions)

### Security / Privacy

- Secret text excluded from searchable context (metadata-only storage, no FTS indexing)
- Project-scoped context isolation (each project has separate derived index)
- Bounded reads prevent resource exhaustion (1 MiB text files, 4 MiB JSON collections)
- Symlink rejection prevents path traversal attacks
- Canonical data remains source of truth (derived index is disposable and rebuildable)
- No arbitrary filesystem access (all paths validated against project boundaries)

### Known Issues

- Open Source may not deep-link correctly to Docs nested under folders. Root-level Docs and Notes work correctly. Search, inspection, provenance, and workspace isolation are unaffected.

### Technical Details

- **Schema versions**: CONTEXT_SCHEMA_VERSION 1.0.0, INDEX_SCHEMA_VERSION 1
- **Test coverage**: 212 frontend tests (22 files), 77 Rust tests
- **Dependencies**: Added rusqlite 0.30 with bundled SQLite 3.44.0 (FTS5 + JSON1 enabled)
- **Architecture**: Derived index stored at `~/.openmesh/indexes/proj_<hash>/context.sqlite3`
- **Compatibility**: 0.1.1 projects open seamlessly; derived index auto-creates on first Context refresh

## [0.1.1] - 2026-07-05

### Added

- Engineering baseline with TypeScript, Vue 3, Tauri v2
- Project management with local JSON storage
- Docs tree with nested folder support
- Notes with Markdown files
- Command palette for quick navigation
- Agent session scanning and indexing
- Work snapshots and recent work timeline
- Git status integration

[0.1.2]: https://github.com/owner/openmesh-agent-workbench/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/owner/openmesh-agent-workbench/releases/tag/v0.1.1
