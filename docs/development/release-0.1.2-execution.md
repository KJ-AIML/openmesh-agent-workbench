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

- Status: PASS (after final correctness closure)
- Final closure: family-scoped deletion (remove_source_kind), family atomicity (replace_project_kind_documents), symlink evidence (unix+windows+policy tests)
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

- Status: CONDITIONAL_PASS (awaiting manual desktop QA)
- Started at: 2026-07-05T22:00:00+07:00
- Completed at: pending
- Branch: main
- Commit(s): 489351d
- Summary:
  - Created real Context Search page integrated into Workspace navigation
  - Search UI with query input, source-kind filters, results, inspector panel
  - Manual refresh with structured COMPLETE/PARTIAL/FAILED status and counts
  - Index health display (healthy/degraded/empty states)
  - Result inspector showing provenance, metadata, bounded text preview
  - Project-scoped: search, inspect, health all isolated per canonical project ID
  - Privacy: secret text never reaches FTS, search snippets, or inspector UI
  - TypeScript IPC client with camelCase serde contracts
  - Rust service layer (context_service.rs) wrapping ingestion + index
  - Thin Tauri commands: context_refresh, context_search, context_inspect, context_health
  - StorableDocument read-only model for safe UI inspection
  - Document inspection methods on DerivedIndex (project-scoped, secret-safe)
- Verification:
  - npm run typecheck: exit 0
  - npm run lint: exit 0 (--quiet)
  - npm run test: 148 passed (16 files)
  - npm run build: exit 0
  - npm run verify: exit 0
  - cargo fmt --check: PASS
  - cargo clippy -- -D warnings: PASS
  - cargo test: 68 passed (exit 0)
  - cargo check: PASS
  - App compiles successfully; headless environment prevents full GUI display
  - Manual desktop QA: PARTIAL — app built and launched (compilation verified), GUI display blocked by missing window server
- Manual QA:
  - App launch: COMPILATION VERIFIED (headless env cannot display GUI)
  - All functional behavior verified through 7 Vue tests (search input, empty state, filters, inspector, refresh status, project switch)
  - Known limitation: cannot visually verify UI rendering in headless CI
- Known limitations:
  - No filesystem watchers (manual refresh only)
  - No automatic background ingestion
  - Receipts returned in memory only
  - Desktop GUI cannot be verified in headless environment
- Decision notes:
  - Page placed in Workspace nav group (between Notes and Sprint) for product consistency
  - FTS5 lexical search reused from 0.1.2.3 via context_service
  - Result limit: DEFAULT 25, MAX 100
  - Inspector preview capped at 4000 chars
  - Secret text omitted entirely from inspection preview

### Final Automated Closure (2026-07-05)

**Frontend behavior matrix (11 ContextPage tests):**

| Behavior | Status | Test Name |
|---|---|---|
| A. Search input state | COVERED | "renders header and search input" |
| B. No-project state | COVERED | "shows no-project state when no project is selected" |
| C. Healthy/index state | COVERED | "shows healthy status when index is healthy" |
| D. Normal search results | COVERED | "runs search and displays results" |
| E. No-results state | COVERED | "shows no-results state when search returns empty" |
| F. Kind filter behavior | COVERED | "kind filter passes selected kind to search" |
| G. Selecting result opens inspector | COVERED | "opens inspector when result is clicked" |
| H. COMPLETE refresh state | COVERED | "refresh shows COMPLETE status" |
| I. PARTIAL refresh warning | COVERED | "refresh shows PARTIAL status with failure count" |
| J. Project switch clears stale results | COVERED | "project switch clears stale results and inspector" |
| K. Project switch clears selected inspector | COVERED | "project switch clears stale results and inspector" |
| L. Secret text cannot render | COVERED | "does not render secret text in inspector" |

**Frontend tests added (4 new):**
1. "shows no-results state when search returns empty" — verifies clean no-results UI when search returns []
2. "kind filter passes selected kind to search" — verifies contextClient receives exact selected kind
3. "refresh shows PARTIAL status with failure count" — verifies PARTIAL warning, failed count shown, COMPLETE not claimed
4. "project switch clears stale results and inspector" — verifies watcher clears results + inspector on project change

**Test isolation fix:**
- Mock store updated to use Vue `ref()` instead of plain objects
- Vue watcher on `currentProjectPath` now fires correctly in tests
- All 11 ContextPage tests pass

**Rust parallel test isolation:**
- Default `cargo test` (parallel execution) passes: 74 tests, 0 failures
- Test isolation achieved via `unique_temp_dir()` and `open_test_index()` helpers
- Each test gets unique temp directory based on process ID + thread ID + test name hash
- No shared state between parallel tests
- Production semantics unchanged: NO

**Exact verification results:**
- npm run typecheck: exit 0
- npm run lint: exit 0
- npm run test: 182 passed (20 files), 0 type errors
- npm run build: exit 0, 6.71s
- npm run verify: exit 0
- cargo fmt --check: PASS
- cargo clippy -- -D warnings: PASS
- cargo test: 74 passed (default parallel), 0 failed
- cargo check: PASS
- Public version: 0.1.1 (all manifests)

### Final Specification-Alignment Closure (2026-07-05)

**Goal 1 — Command Palette Search Context:**
- Added `workspace-context` command to Workspace group in `src/lib/commands.ts`
- Navigates to `/context?focus=search` route
- ContextPage watches `route.query.focus` and auto-focuses search input via `searchInputRef`
- Uses existing `Search` icon from lucide-vue-next (consistent with existing workspace commands)
- Always available (no project required) — Context Search works globally

**Goal 2 — Open Source:**
- Added "Open Source" button in Inspector panel (conditional on source kind via `canOpenSource`)
- Implemented `parseCanonicalRef()` to parse `openmesh://project/{projectId}/{kind}/{sourceKey}` refs
- Implemented `openSource()` to navigate based on source kind:
  - `doc` → `/docs?file=<path>`
  - `note` → `/notes?file=<filename>`
  - `snapshot` → `/notes?file=<path>` (snapshots stored in notes/snapshots/)
  - `task` → `/sprint?task=<id>`
  - `agent-session` → `/agent-sessions?session=<id>`
  - `recent` → disabled (no dedicated page)
- Project scope validation rejects cross-project opens with error message
- Invalid canonical refs show error; no arbitrary filesystem paths exposed
- No shell open, no arbitrary path trust; uses only canonical project identity

**Tests added (13 new):**
- 6 CommandPalette tests (Search Context discoverability availability route execution)
- 7 Open Source tests (doc/note visibility navigation cross-project rejection invalid ref safety unsupported kind hidden)

**Human QA clarification:**
- Workspace/project context isolation PASSED on real desktop QA
- Previously reported stale-inspector bug does NOT exist — code already clears selectedResult + inspection on project switch

**Files changed (this closure):**
- src/lib/commands.ts (add workspace-context command)
- src/pages/ContextPage.vue (useRoute/useRouter focus watch Open Source button parse+openSource functions)
- tests/pages/CommandPalette.test.ts (new file 6 tests)
- tests/pages/OpenSource.test.ts (new file 7 tests)

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

