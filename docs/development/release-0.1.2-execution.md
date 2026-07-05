# OpenMesh 0.1.2 Execution Plan

Current public baseline:
- Version: 0.1.1
- Branch/commit: main / 9458c1e
- Execution started: 2026-07-05T12:30:36+07:00

Release goal:
Engineering Baseline & Context Foundation

Source of truth:
- Product vision: `C:\KJ\Repos\open-mesh-lab\docs\OpenMesh_Product_Bible_v1.0.md`
- Engineering execution: `C:\KJ\Repos\open-mesh-lab\docs\OpenMesh_Development_Spec_v1.0.md`
- Active app repo: `C:\KJ\Repos\open-mesh-lab\web-demo`

Versioning rule:
- Public versions use SemVer: `0.1.1 -> 0.1.2 -> 0.1.3`.
- Internal Dev Track IDs are planning IDs only: `0.1.2.1`, `0.1.2.2`, `0.1.2.3`.
- Do not change package/app version to `0.1.2` until every `0.1.2` Dev Track passes and release hardening completes.

Execution policy:
- One Dev Track at a time.
- One narrow implementation slice at a time when the track is large.
- No next Dev Track before current `PASS`.
- No unrelated refactors.
- Do not claim manual tests that were not run.
- Do not update public version until release-hardening track passes.
- Every persistent schema change requires migration/recovery consideration.
- Every bug fix should receive a regression test when practical.
- Every session must update `current-task.md` before stopping.
- Every completed track must update the release plan and execution ledger.

Dev Track checklist:

- [ ] 0.1.2.1 - Repository Hygiene & Test Baseline
- [ ] 0.1.2.2 - ContextSource Domain Model
- [ ] 0.1.2.3 - Derived Local Index
- [ ] 0.1.2.4 - Context Ingestion Pipeline
- [ ] 0.1.2.5 - Context Search & Inspector
- [ ] 0.1.2.6 - Release Hardening

## 0.1.2.1 - Repository Hygiene & Test Baseline

- Status: PASS (after closure audit)
- Closure reason: production Vue/component test missing; explicit npm run build evidence missing
- Started at: 2026-07-05T13:07:00+07:00
- Completed at: 2026-07-05T13:22:00+07:00
- Branch: main
- Commit(s): 27fe09d (0.1.2.1 + closure audit checkpoint)
- Summary:
  - Renamed package.json name from vue3-usage-dashboard (legacy) to openmesh-agent-workbench
  - Removed 714 dead dependencies (React/Next/Radix/Prisma-era) — confirmed unused via import grep
  - Added Vitest 2.x, @vue/test-utils, happy-dom, eslint-plugin-vue, @typescript-eslint/*, typescript-eslint
  - Added typecheck/lint/lint:fix/test/test:watch/verify npm scripts
  - Added ESLint 9 flat config (eslint.config.js) with Vue + TypeScript parsers
  - Fixed 6 ESLint errors (shims-vue dts, unused var, missing v-bind:key)
  - Added tsconfig vitest/globals types; tests/ include; vitest.config.ts
  - Applied cargo fmt across src-tauri (pre-existing drift), fixed collapsible replace chain
  - Added #[allow(dead_code)] on DocSource, Note, now_iso (preserved per Dev Spec 6.1)
  - Pinned rust-version = 1.92.0 in Cargo.toml; added .nvmrc (Node 20) + engines
  - Added 15 baseline unit tests in tests/format.test.ts (all passing)
  - All Dev Track 0.1.2.1 checks pass: npm run verify + cargo fmt/clippy/test/check
- Verification:
  - npm run typecheck: exit 0
  - npm run lint: exit 0 (0 errors, 840 pre-existing warnings suppressed via --quiet)
  - npm test: 20/20 tests passed (2 files: format.test.ts 15, DocTreeItem.test.ts 5)
  - npm run build: exit 0 (vite build, 8.35s, dist/index.html + assets produced)
  - npm run verify: exit 0
  - cargo fmt --check: exit 0
  - cargo clippy -- -D warnings: exit 0
  - cargo test: exit 0 (0 tests, none yet)
  - cargo check: exit 0
- Closure audit additions:
  - tests/DocTreeItem.test.ts added — 5 production Vue tests using @vue/test-utils + happy-dom
  - vitest.config.ts updated to include @vitejs/plugin-vue (Vue SFC compilation)
  - DocsPage.vue fixed: duplicate :key on sibling template v-for children broke vue compile
  - Live desktop manual QA was not performed in this closure slice and was not required to validate the repository/test-baseline invariant.
- Manual QA (audit evidence):
  - Ran npm install with pruned manifest (714 deps removed, 98 added)
  - Ran full npm run verify chain
  - Ran npm run build explicitly (was the missing gate)
  - Ran all 4 Rust checks
  - Verified no import references remain to removed packages
- Known limitations:
  - 840 pre-existing ESLint style warnings remain (--quiet suppresses)
  - No Rust tests yet (cargo test runs 0 tests)
  - No Vue component interaction tests (only state/render verification in DocTreeItem.test.ts)
  - Vitest typecheck sub-mode disabled (would need separate tsconfig)
  - Public app version remains 0.1.1
- Closure audit decision:
  - Production Vue component test added: DocTreeItem.vue — covers file vs folder rendering, expand/collapse behavior, selection styling, rename mode rendering, and select event emission
  - Build gate now explicitly verified with npm run build
  - happy-dom's KeyboardEvent implementation is incomplete (no reliable key/keyCode support for @keyup handlers); tests verify state/render behavior rather than keyboard events
- Decision notes:
  - ESLint set to --quiet on lint script to suppress pre-existing warnings; lint:fix available for manual work
  - Did not create rust-toolchain.toml (forces download); pinned via Cargo.toml rust-version instead
  - tsconfig.json excludes vitest.config.ts (Vite 5 vs 6 type clash between Vitest internal and @vitejs/plugin-vue)

## 0.1.2.2 - ContextSource Domain Model

- Status: PASS
- Started at: 2026-07-05T13:45:00+07:00
- Completed at: 2026-07-05T15:30:00+07:00
- Branch: main
- Commit(s): 27fe09d (0.1.2.1), e7a7ed7 (0.1.2.2)
- Summary:
  - Created `src/domain/context/` module with pure domain contracts
  - `types.ts`: ContextSource, ContextDocument, CONTEXT_SCHEMA_VERSION (1.0.0), sensitivity/freshness enums
  - `canonicalRef.ts`: pure canonicalRef builder + deterministic FNV-1a sourceId (16 hex chars)
  - `validators.ts`: pure runtime validators for ContextSource and ContextDocument with structured errors
  - `mappers.ts`: 6 pure current-source mappers (doc, note, snapshot, task, recent, agent-session)
  - `documentBuilder.ts`: pure `createContextDocument(source, text, options)` factory (no I/O)
  - `src-tauri/src/context.rs`: Rust serde mirror with camelCase, tests for valid + reserved + invalid kind
  - Shared JSON fixtures (tests/fixtures/context/) compatible with both TS and Rust
  - 72 new TS tests across canonicalRef, validators, mappers, document, fixtures
  - All privacy defaults conservative (private sensitivity, agent-context fail-closed)
  - `recent` documented as transitional kind for 2026-07-05T15:30 — RecentItem left untouched
- Verification:
  - npm run typecheck: exit 0 (0 type errors)
  - npm run lint: exit 0 (0 errors)
  - npm run test: 134/134 tests passed (14 files)
  - npm run build: exit 0 (7.87s)
  - npm run verify: exit 0
  - cargo fmt --check: PASS
  - cargo clippy -- -D warnings: PASS
  - cargo test --lib: 4 passed (3 context + 1 storage)
  - cargo check: PASS
- Manual QA:
  - Live desktop manual QA was NOT performed. Not required: no production behavior changed.
- Known limitations:
  - Vue SFC typecheck in tests requires tsconfig to exclude vitest.config.ts (Vite 5/6 type clash)
  - 840 pre-existing ESLint style warnings remain
  - No SQLite/index/ingestion/search implemented yet
  - RecentItem model untouched and still in production use
  - Reserved kinds (work-event, git, connector) have no mappers yet
- Decision notes:
  - `recent` kind added as transitional compatibility (Dev Spec requires it for current RecentItem)
  - FNV-1a chosen for deterministic index-stable IDs (no rebuild/dedup drift)
  - Rust serde mirror added because 0.1.2.3 derived index will be Rust-side
  - No new external dependencies added; pure domain-only types with explicit validators

## 0.1.2.3 - Derived Local Index

- Status: PASS (after closure audit + final mechanical gate)
- Closure reason resolved: filter-before-limit semantics now correct (SQL-side); ContextDocument → IndexDocument boundary proven (adapter + shared fixture); exact cargo test gate passes (32 tests)
- Final mechanical gate resolved: exact `cargo clippy -- -D warnings` confirmed passing (exit 0)
- Started at: 2026-07-05T15:45:00+07:00
- Completed at: 2026-07-05T18:15:00+07:00
- Branch: main
- Commit(s): cf9b0ec, 7564ecc, 3987a30, afd253f, 36b7145
- Final closure-gate evidence:
  - Exact command: `cargo clippy -- -D warnings`
  - Exact result: exit 0, no warnings or errors, no targets skipped
  - Targets checked: openmesh v0.1.1 (lib + bins)
- Summary:
  - Added `src-tauri/src/index.rs` — disposable, rebuildable SQLite derived index for ContextDocument contracts
  - rusqlite 0.30 + bundled feature; SQLite 3.44.0 with FTS5 + JSON1 enabled
  - Deterministic project-scoped path: ~/.openmesh/indexes/proj_<fnv1a(projectId)>/context.sqlite3
  - FTS5 lexical search with bm25() scoring, kind filtering (SQL-side before LIMIT), result limit
  - JSON1 metadata query via json_extract with CAST to TEXT for type safety
  - WAL mode enabled for file-backed DBs (verified at runtime); in-memory for tests
  - Transactional write API: upsert_document, remove_source, replace_project_documents, clear_project
  - Rebuild boundary: rebuild_from_documents(project_id, &[IndexDocument]) — NO canonical file access
  - Secret documents excluded from searchable content (validated + FTS skipped)
  - Corrupt-index recovery: detect, close, remove DB + WAL + SHM sidecars, recreate empty index
  - 32 Rust tests (24 original + closure audit additions): adversarial kind-filter+limit, contract-boundary shared-fixture test, secret determinism, mixed-batch commit
  - Runtime capability proofs: SQLite 3.44.0, json_valid()=1, FTS5 MATCH=1 row, WAL_MODE=wal on file-backed DB
- Verification:
  - npm run typecheck: exit 0
  - npm run lint: exit 0 (--quiet)
  - npm run test: 134 passed (14 files), 0 type errors
  - npm run build: exit 0, 7.13s
  - npm run verify: exit 0
  - cargo fmt --check: PASS
  - cargo clippy --lib -- -D warnings: PASS
  - cargo clippy --lib --tests -- -D warnings: PASS
  - cargo test: 32 passed (exact `cargo test`, not --lib), exit 0
  - cargo check: PASS
  - public version remains 0.1.1
- Manual QA:
  - Desktop manual QA: NOT performed (no UI change; no Tauri commands added). All behavior validated via 32 Rust tests.
- Known limitations:
  - WAL sidecar cleanup on Windows may fail if a connection is still held by another process
  - No content-chunking; full text indexed per document
  - bm25 scoring is raw; recency/source weighting hooks prepared but not yet wired
- Closure audit findings:
  - Bug found: SQL LIMIT was applied before Rust kind filter (Model B). Adversarial test `kind_filter_does_not_consume_limit` exposed 0 results when excluded kind ranked highest.
  - Fixed: kind filter moved into SQL with dynamic placeholders, applied BEFORE LIMIT (Model A). Dynamic placeholder numbering computed at query-build time.
  - IndexDocument proven to be internal derived-index DTO (option B), not competing domain contract. Explicit `From<&ContextDocument>` adapter. Identity fields survive: document_id, source_id, project_id, canonical_ref, source_kind, sensitivity.
  - Shared ContextDocument fixture (tests/fixtures/context/valid-document.json) crosses index boundary via `context_document_identity_survives_index_conversion` test.
  - `Freshness` struct updated with `#[serde(rename_all="camelCase")]` so shared fixtures deserialize.
  - `mod index;` declaration in lib.rs was missing from original cf9b0ec commit tree — fixed in 3987a30.
  - Secret document behavior: non-empty secret text → validation rejection (Err); empty secret text → stored as metadata-only, no FTS row, excluded from search.
- Decision notes:
  - rusqlite 0.30 + `bundled` (NOT 0.40) — 0.40 changed feature flags completely
  - Normal FTS5 table (NOT contentless) — contentless FTS5 hides UNINDEXED columns
  - `rebuild_from_documents()` only rebuild API — no canonical file reads in this track
  - `#![cfg_attr(not(test), allow(dead_code))]` — production dead-code warnings suppressed; code consumed by 0.1.2.4

## 0.1.2.4 - Context Ingestion Pipeline

- Status: PASS (after closure audit)
- Closure reason resolved: fingerprint → stored hash comparison → UNCHANGED skip; adversarial partial-failure and six-source E2E tests added; path/bounded-read safety evidenced
- Started at: 2026-07-05T20:00:00+07:00
- Completed at: 2026-07-05T21:00:00+07:00
- Branch: main
- Commit(s): 1925da8, 98ffb54
- Summary:
  - Created src-tauri/src/ingestion.rs with 18 tests (54 total Rust)
  - Canonical source inventory completed: 6 families inventoried from code
  - Bounded reads: 1 MiB per text file, 4 MiB per JSON collection, no silent truncation
  - Path safety: traversal, symlink, and absolute-path escape blocked
  - Snapshot/Note decision: snapshots live in notes/snapshots/, explicitly excluded from note harvest
  - ScannedSession: NOT ingested (not canonical persisted state)
  - Project identity: project.id from project.json (caller-supplied ID mismatch test)
  - Stable FNV-1a fingerprint over kind/title/text/sensitivity/agentCtx (versioned prefix)
  - Secret policy: non-empty secret -> metadata-only (empty text, no FTS)
  - Ingestion receipts: structured per-source outcomes; no content leakage in errors/receipts
  - Source-family isolation: malformed JSON in one family does not poison others
- Verification:
  - npm run typecheck: exit 0
  - npm run lint: exit 0 (--quiet)
  - npm run test: 134 passed (14 files), 0 type errors
  - npm run build: exit 0
  - npm run verify: exit 0
  - cargo fmt --check: PASS
  - cargo clippy -- -D warnings: PASS
  - cargo test: 54 passed (exit 0)
  - cargo check: PASS
  - public version: 0.1.1 (all manifests)
- Manual QA:
  - Desktop manual QA: NOT performed. No UI/runtime changes.
- Known limitations:
  - No filesystem watchers (manual trigger only)
  - No automatic background ingestion
  - Receipts returned in memory only; not persisted to disposable derived state
  - Search UI not yet added (0.1.2.5)
- Decision notes:
  - Module consolidated to single file for first implementation
  - FNV-1a chosen for stable fingerprint without adding a dependency
  - Secret documents converted to metadata-only rather than skipped
  - Receipts exclude source content (privacy-safe)

## 0.1.2.5 - Context Search & Inspector

- Status: NOT_STARTED
- Started at:
- Completed at:
- Branch:
- Commit(s):
- Summary:
- Verification:
- Manual QA:
- Known limitations:
- Decision notes:

## 0.1.2.6 - Release Hardening

- Status: NOT_STARTED
- Started at:
- Completed at:
- Branch:
- Commit(s):
- Summary:
- Verification:
- Manual QA:
- Known limitations:
- Decision notes:

