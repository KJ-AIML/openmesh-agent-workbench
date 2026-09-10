# A13.14 Release Workflow Validation Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Workflow File:** `.github/workflows/release.yml`
- **Guard Script:** `scripts/check-release-workflow.mjs`
- **Result:** **READY**

## Static Workflow Inspection Matrix

| Inspection Item | Specification / Requirement | Observed Configuration | Verdict |
| :--- | :--- | :--- | :---: |
| **1. Tag Pattern** | Trigger on `v*` tags (e.g. `v0.2.0-rc.1`) | `tags: ['v*']` matches candidate tag | **PASS** |
| **2. Artifact Naming** | Includes candidate version | Configured via `tauri-action` reading `tauri.conf.json` (`0.2.0-rc.1`) | **PASS** |
| **3. Empty Secret Guard** | No unpopulated APPLE_*/WINDOWS_* secret mappings | `npm run check:release-workflow` PASS (zero forbidden env mappings) | **PASS** |
| **4. Platform Matrix** | Covers macOS (Apple Silicon + Intel), Linux, Windows | `macos-latest` (aarch64 & x86_64), `ubuntu-22.04`, `windows-latest` | **PASS** |
| **5. Rust Toolchain** | Stable with multi-target support for macOS runners | `targets: aarch64-apple-darwin,x86_64-apple-darwin` | **PASS** |
| **6. Prerelease Flag** | Intended release classification | `prerelease: false` is hardcoded in `release.yml`; documented as requiring UI toggle or tag release flag if published as prerelease on GitHub | **NOTED** |

## Invariant Adherence
- **Zero Remote Push:** No git push attempted.
- **Zero Git Tag:** No local or remote tag created.
- **Release Gating:** Workflow will only run after explicit user approval and manual tag creation.
