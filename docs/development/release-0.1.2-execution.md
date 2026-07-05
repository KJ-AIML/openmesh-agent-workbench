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
- Commit(s): pending
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

## 0.1.2.3 - Derived Local Index

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

## 0.1.2.4 - Context Ingestion Pipeline

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

