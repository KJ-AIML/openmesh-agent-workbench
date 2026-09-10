# OpenMesh v0.2.0-rc.2 Packaged Runtime Smoke Evidence

- **Date:** 2026-09-10
- **Candidate Commit SHA:** `21fe4ca0ebf891f8525bcb2426765db2c1b0487e`
- **Release Tag:** `v0.2.0-rc.2`
- **Release Run ID:** [34468095375](https://github.com/KJ-AIML/openmesh-agent-workbench/actions/runs/34468095375)
- **GitHub Release URL:** https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.2.0-rc.2
- **Tested Artifact:** `OpenMesh_0.2.0-rc.2_aarch64.dmg` (Downloaded directly from GitHub Releases, SHA-256 `d5fe55ade824446582bd0b5906f56a4ba91e9e70ef53ffa98eba288736629919`)
- **Installed Binary:** `/tmp/openmesh-published-rc2/Applications/OpenMesh.app/Contents/MacOS/openmesh`
- **Classification:** **RC PUBLISHED — HEALTHY WITH WINDOWS RUNTIME SMOKE PENDING**

---

## 1. Objective

Validate that the published `v0.2.0-rc.2` packaged desktop distribution functions correctly at runtime in an end-to-end environment, specifically exercising the product code change introduced in `v0.2.0-rc.2` (`src/pages/SettingsPage.vue` toast timeout race fix) alongside process lifecycle and project state persistence.

---

## 2. Runtime Platform Matrix

| Platform | Distribution Asset | Runtime Validation Mode | Result | Notes |
| :--- | :--- | :--- | :---: | :--- |
| **macOS (arm64)** | `OpenMesh_0.2.0-rc.2_aarch64.dmg` | Live packaged execution via WKWebView on Darwin arm64 | **PASS** | Full UI navigation, toast race fix, persistence & clean shutdown verified |
| **macOS (x86_64)** | `OpenMesh_0.2.0-rc.2_x64.dmg` | Binary structure & CI build validation | **PASS** | Shared codebase and Tauri packaging pipeline with arm64 |
| **Linux (x64)** | `OpenMesh_0.2.0-rc.2_amd64.deb` / AppImage | CI runner packaging & compilation | **PASS** | Linux deb, AppImage, and rpm packages cleanly built |
| **Windows (x64)** | `OpenMesh_0.2.0-rc.2_x64-setup.exe` | Binary structure (NSIS PE32) verified; runtime execution | **PENDING** | Local environment is macOS Darwin; no Windows runtime host available |

---

## 3. Verified Behaviors (macOS Packaged Binary)

| Stage | Action / Check | Target / Element | Result | Observed Evidence |
| :--- | :--- | :--- | :---: | :--- |
| **1. Artifact Mount & Extraction** | Mount published DMG | `hdiutil attach OpenMesh_0.2.0-rc.2_aarch64.dmg` | **PASS** | DMG mounted cleanly; `OpenMesh.app` copied to `/tmp/openmesh-published-rc2/Applications/` |
| **2. Bundle Metadata** | Verify Info.plist | `CFBundleShortVersionString`, `CFBundleVersion` | **PASS** | Both report `0.2.0-rc.2`; bundle ID `com.openmesh.app` |
| **3. Packaged App Launch** | Launch native binary | `/tmp/openmesh-published-rc2/Applications/OpenMesh.app/Contents/MacOS/openmesh` | **PASS** | Spawned process (PID 24964), native macOS window `OpenMesh` mounted, WKWebView initialized |
| **4. State Restoration** | Load active project | `~/.openmesh/app-state.json` | **PASS** | Project `/tmp/openmesh-packaged-dogfood` ("Test") attached; top navbar reflects active workspace |
| **5. Navigation** | Switch to Settings | Sidebar `SETTINGS` → `Settings` | **PASS** | Header displays `Settings · 4/6 ready · Openrouter · server unreachable · v0.2.0-rc.2` |
| **6. Settings Toast: Initial Click** | Click `Export Project` | `button "Export Project"` in Data panel | **PASS** | Toast mounts with text `Project exported` |
| **7. Toast Timer Race Test (RC.2 Fix)** | Rapid click `Import Data` within 1.0s of Export | `button "Import Data"` in Data panel | **PASS** | Toast immediately updates to `Import not yet implemented for file-based storage`. Prior timer cancelled via `clearTimeout(toastTimeout)`. |
| **8. Timer Persistence Check** | Inspect toast at $t = t_0 + 3.2\text{s}$ (past Export 3.0s mark) | Accessibility hierarchy & screenshot | **PASS** | Toast remains visible with text `Import not yet implemented for file-based storage`. In RC.1, this was prematurely cleared at 3.0s. |
| **9. Timer Expiration Check** | Inspect toast at $t = t_{\text{import}} + 3.5\text{s}$ | Accessibility hierarchy & screenshot | **PASS** | Toast element unmounts cleanly after its dedicated 3000ms duration. Group count returns to 8. |
| **10. Graceful Shutdown** | Send SIGTERM | `kill -15 24964` | **PASS** | Process terminated cleanly with exit code 0; zero orphan processes or unhandled panics. |
| **11. Relaunch & State Check** | Launch binary again | Process PID 29613 | **PASS** | Window opened immediately, active project "Test" (`/tmp/openmesh-packaged-dogfood`) loaded from disk, setup checklist (4/6) rendered. |
| **12. Clean Exit** | Terminate relaunch instance | `kill -15 29613` | **PASS** | Process terminated cleanly with exit code 0. |

---

## 4. Toast Race Condition Verification Timing Trace

The timing script executed directly against the running packaged WKWebView UI via macOS Accessibility:

```text
Step 0: Wait 4s to ensure clean initial state...
Initial toast state: ''
Step 1: Clicking Export Project...
[0.46s] Toast after Export: 'Project exported'
Step 2: Waiting 1.0s then clicking Import Data...
[1.81s] (Import+0.49s) Toast after Import: 'Import not yet implemented for file-based storage'
Step 3: Sleeping 1.71s to check past t0+3.0s (Export timer expiry)...
[3.71s] (Import+2.39s) Toast state: 'Import not yet implemented for file-based storage'
>>> SUCCESS: Toast persisted past previous timeout!
Step 4: Sleeping 0.97s to verify expiration...
[5.07s] (Import+3.75s) Toast state after expiry: ''
>>> SUCCESS: Toast cleanly expired after 3.0s window!
Toast race condition fix VERIFIED in packaged v0.2.0-rc.2 binary!
```

---

## 5. Conclusion & Status Promotion

1. Candidate commit SHA `21fe4ca0ebf891f8525bcb2426765db2c1b0487e` and immutable release tag `v0.2.0-rc.2` are confirmed sound.
2. The product-code fix in `src/pages/SettingsPage.vue` is validated in the actual packaged release artifact.
3. Because all 4 CI platforms succeeded and macOS runtime verification passed with zero regressions, but a Windows runtime host is not locally available on this Darwin development machine, the formal classification for `v0.2.0-rc.2` is:

**RC PUBLISHED — HEALTHY WITH WINDOWS RUNTIME SMOKE PENDING**
