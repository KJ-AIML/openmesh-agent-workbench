# A13.2 Clean Checkout Reproduction Evidence

- **Date:** 2026-09-10
- **Candidate Commit SHA:** `964920e4cae824a49353f65a81972ce2070fe968`
- **Isolation:** Dedicated detached worktree `worktrees/clean-v0.2.0-rc.1`
- **Initial Working Tree:** Clean (0 untracked files, 0 local configs)

## Execution Sequence & Evidence

1. **Dependency Installation:**
   - Command: `npm ci`
   - Exit Code: `0`
   - Duration: 8s (audited 587 packages)
   - Tracked Lockfile Mutation: None (`git status` reported `nothing to commit, working tree clean`)

2. **Automated RC Gate Verification:**
   - Command: `npm run verify:rc`
   - Exit Code: `0`
   - Total Steps: 7/7 passed

| Gate Step | Command | Result | Details |
| :--- | :--- | :---: | :--- |
| **1. Version Consistency** | `node scripts/check-version-consistency.mjs 0.2.0-rc.1` | **PASS** | All 5 manifests aligned at `0.2.0-rc.1` |
| **2. Frontend Verify** | `npm run verify` | **PASS** | 718 tests passed (108 files); IPC contract 181/80/56/10/0; CSP & security verified |
| **3. Rust Format** | `cargo fmt --all -- --check` | **PASS** | 0 formatting discrepancies |
| **4. Rust Compilation** | `cargo check --workspace` | **PASS** | Clean build for core, desktop shell, CLI |
| **5. Rust Test Suite** | `cargo test --workspace --no-fail-fast` | **PASS** | 2147 passed, 0 failed, 1 ignored |
| **6. Playwright E2E** | `npm run test:e2e` | **PASS** | 93 passed, 0 failed (85.4s) |
| **7. Git Diff Hygiene** | `git diff --check` | **PASS** | 0 whitespace or conflict marker errors |

## Conclusion
The candidate SHA `964920e` reproduces 100% cleanly from scratch with zero dependency mutations, zero developer-local state, and passes all canonical gates.
