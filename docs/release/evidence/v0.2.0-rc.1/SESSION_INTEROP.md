# A13.9 Session Interoperability Smoke Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Module:** `openmesh-core::session_readers`
- **Result:** **PASS** (Representative live fixtures verified; synthetic fixtures verified across all 6 scanners)

## Scanner Support Classification Matrix

| Scanner / Agent Tool | Platform Location Checked | Fixture Status | Discovery Verification | Non-Mutating Read | Provenance Preserved | Status |
| :--- | :--- | :---: | :---: | :---: | :---: | :---: |
| **Codex** | `~/.codex/sessions` | Real local sessions present | Rollout JSONL files discovered with timestamps and user queries | SHA-256 and mtime unchanged before & after | `source: codex`, links recorded | **VERIFIED** |
| **Grok** | `~/.grok/sessions` | Real local sessions present | Project sessions discovered with summary and prompt history | SHA-256 and mtime unchanged before & after | `source: grok`, links recorded | **VERIFIED** |
| **Claude Code** | `~/.claude/projects` | No local session files on host | Synthetic test fixture verified in `continuity_readers` test suite | Read-only mode verified | `source: claude`, links recorded | **NO LOCAL FIXTURE (SUITE VERIFIED)** |
| **Cursor** | `~/.cursor/projects` | No chat transcript files on host | Synthetic test fixture verified in `continuity_readers` test suite | Read-only mode verified | `source: cursor`, links recorded | **NO LOCAL FIXTURE (SUITE VERIFIED)** |
| **OpenCode** | `~/.config/opencode` | Only node_modules/pkg config present | Synthetic test fixture verified in `continuity_readers` test suite | Read-only mode verified | `source: opencode`, links recorded | **NO LOCAL FIXTURE (SUITE VERIFIED)** |
| **Gemini** | `~/.gemini/tmp` | Only CLI config present | Synthetic test fixture verified in `continuity_readers` test suite | Read-only mode verified | `source: gemini`, links recorded | **NO LOCAL FIXTURE (SUITE VERIFIED)** |

## Core Invariants Proven

1. **Non-Mutation:**
   - Real local Codex session (`rollout-2026-09-03T18-31-19-01a06709-a00c-7c81-9d75-b7670c25476c.jsonl`) and Grok session (`summary.json`) were inspected. SHA-256 hashes and modification timestamps remained identical before and after reading.
2. **Provenance Preservation:**
   - Importing or linking foreign sessions invokes `link_session` in `continue_ops.rs`, persisting `SessionLink` with foreign tool identity, session path, and recording a `WorkBoundary::SessionImported` event into the project continuity history.
3. **Automated Scanner Test Suite:**
   - 18 unit tests in `session_readers::discovery`, `session_readers::parse`, and `session_readers::transcript` passed with 0 failures.
