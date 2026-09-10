# A13.1 Audit Details Resolution

## 1. Version-Consistency Source Classification

The version consistency suite (`scripts/check-version-consistency.mjs`) evaluates version alignment across the project. The 7 versioned files in the repository divide into two strict categories:

1. **Authoritative Declaration Manifests (5):**
   - `package.json` (`version: "0.2.0-rc.1"`)
   - `src-tauri/tauri.conf.json` (`version: "0.2.0-rc.1"`)
   - `src-tauri/Cargo.toml` (`version = "0.2.0-rc.1"`)
   - `crates/openmesh-core/Cargo.toml` (`version = "0.2.0-rc.1"`)
   - `crates/openmesh-cli/Cargo.toml` (`version = "0.2.0-rc.1"`)

2. **Synchronized Lockfiles (2):**
   - `package-lock.json` (`packages[""].version: "0.2.0-rc.1"`)
   - `Cargo.lock` (`openmesh`, `openmesh-core`, `openmesh-cli` packages at `"0.2.0-rc.1"`)

**Conclusion:** The version checker counts the 5 primary declaration manifests as authoritative sources, and the 2 lockfiles are verified in sync. All 7 files reflect exact `0.2.0-rc.1` alignment.

---

## 2. Desktop IPC Count Movement (80 typed / 54 legacy → 80 typed / 56 legacy)

### Observation
- **A8 Freeze Report:** `registered=181 typed=80 legacy=54 unused=10 unknown=0`
- **A12 Freeze Report:** `registered=181 typed=80 legacy=56 unused=10 unknown=0`

### Root Cause Analysis
- In the initial A8 slice, `src/lib/updates/installUpdate.ts` imported the legacy invoke wrapper with an alias:
  ```ts
  import { legacyInvoke as invoke } from "../ipc";
  ```
  It subsequently executed:
  - `invoke<string>("get_host_arch")`
  - `invoke<InstallUpdateResult>("download_and_open_update")`
- The static analyzer `scripts/check-ipc-contract.mjs` extracts legacy calls using the regex:
  ```js
  \blegacyInvoke(?:<[^>]*>)?\(\s*(['"`])([A-Za-z_][A-Za-z0-9_]*)\1
  ```
  Because those two invocations used the local alias `invoke` rather than the identifier `legacyInvoke`, they were categorized under generic `invokeCalls` rather than `legacyCalls`, resulting in a reported legacy count of 54.
- During A12 typed-IPC import cleanup in `installUpdate.ts`, `get_host_os` was upgraded to `invokeTyped`, while `get_host_arch` and `download_and_open_update` were converted to explicit `legacyInvoke(...)`:
  ```ts
  invokeTyped<string>("get_host_os");
  legacyInvoke<string>("get_host_arch");
  legacyInvoke<InstallUpdateResult>("download_and_open_update");
  ```
- This syntactic cleanup allowed `check-ipc-contract.mjs` to properly identify both legacy invocations, accurately raising the static count from 54 to 56.

### Invariant Validation
- **Typed Catalog:** 80 commands (unchanged, 100% registered in Rust).
- **Unknown Commands:** 0 (no unregistered commands invoked anywhere in `src/`).
- **Raw Invoke Leak:** 0 (no `@tauri-apps/api/core` imports outside `src/lib/ipc/client.ts`).
- **Duplicates:** 0 duplicate handler or catalog registrations.
- **Legacy Containment:** All 56 legacy commands reside strictly behind adapter boundaries (`src/lib/store.ts`, `src/lib/continuityClient.ts`, `src/lib/updates/installUpdate.ts`).
