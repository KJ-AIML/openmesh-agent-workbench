# OpenMesh v0.2.0-rc.2 Release Closure Report

- **Date:** 2026-09-10
- **Candidate Commit SHA:** `21fe4ca0ebf891f8525bcb2426765db2c1b0487e`
- **Release Tag:** `v0.2.0-rc.2`
- **Workflow Run:** https://github.com/KJ-AIML/openmesh-agent-workbench/actions/runs/34468095375
- **GitHub Release:** https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.2.0-rc.2
- **Classification:** **RC PUBLISHED — HEALTHY WITH WINDOWS RUNTIME SMOKE PENDING**

---

## 1. Summary of Resolution

In `v0.2.0-rc.1`, the release matrix produced a partial platform failure: macOS arm64, macOS x86_64, and Linux packages succeeded and published, but Windows packaging failed during WiX MSI bundling because Windows Installer rejects alphanumeric prerelease identifiers (such as `0.2.0-rc.1`).

In `v0.2.0-rc.2`:
1. The Windows release runner matrix was updated to build NSIS bundles (`--bundles nsis`).
2. An automated regression guard was added to `scripts/check-release-workflow.mjs` ensuring all prerelease versions restrict Windows packaging to NSIS.
3. Canonical verification (`verify:rc`) passed all 7 gates (version consistency, frontend verify, cargo fmt, cargo check, cargo test across 2147 tests, 93 Playwright e2e tests, git diff check).
4. Tag `v0.2.0-rc.2` triggered GitHub Actions release run `34468095375`. All 4 platforms completed with conclusion `success`.

---

## 2. GitHub Actions 4-Platform Build Matrix

| Platform | Runner | Target / Flags | Job ID | Status | Conclusion |
| :--- | :--- | :--- | :--- | :---: | :---: |
| **macOS (Intel)** | `macos-latest` | `--target x86_64-apple-darwin` | `102841318081` | completed | **success** |
| **macOS (Apple Silicon)** | `macos-latest` | `--target aarch64-apple-darwin` | `102841318201` | completed | **success** |
| **Linux (x64)** | `ubuntu-22.04` | Default (deb, AppImage, rpm) | `102841317856` | completed | **success** |
| **Windows (x64)** | `windows-latest` | `--bundles nsis` | `102841318528` | completed | **success** |

---

## 3. Published Release Artifacts & SHA-256 Checksums

All 8 distribution assets were downloaded from the published release and verified:

| Asset Name | Target Platform | Size | SHA-256 Checksum |
| :--- | :--- | :---: | :--- |
| `OpenMesh_0.2.0-rc.2_x64-setup.exe` | Windows x64 (NSIS) | 9.1 MB | `5e7b0c000aedab8c83f13f1ab4e5c4f3e6a46795062aef088d2ed5437372812b` |
| `OpenMesh_0.2.0-rc.2_aarch64.dmg` | macOS Apple Silicon | 12 MB | `d5fe55ade824446582bd0b5906f56a4ba91e9e70ef53ffa98eba288736629919` |
| `OpenMesh_0.2.0-rc.2_aarch64.app.tar.gz` | macOS Apple Silicon | 12 MB | `f1a7d276284d158998af95ca57ee8fd0bbadd28ac581b0d55088f8c76a10f55a` |
| `OpenMesh_0.2.0-rc.2_x64.dmg` | macOS Intel | 13 MB | `00e44b161a3bbd5f7411c2ca9374a5dbb642ed875644cb1b9b4af38eda6b0f2c` |
| `OpenMesh_0.2.0-rc.2_x64.app.tar.gz` | macOS Intel | 13 MB | `62c6fa0a3c9e53d2d44657eaeed3c1e98b336b1608d0b563fae93b4c7055291f` |
| `OpenMesh_0.2.0-rc.2_amd64.deb` | Linux (Debian / Ubuntu) | 15 MB | `ae02ca936b26f743c1bccbab86925a30ed5efdd9b982d6152b6fe32d9e392fc9` |
| `OpenMesh_0.2.0-rc.2_amd64.AppImage` | Linux (AppImage) | 88 MB | `c4df84b22bd78b0d279aba00ccbd0ffef8bee6c861fdbdc83b5af7988c49cee9` |
| `OpenMesh-0.2.0-rc.2-1.x86_64.rpm` | Linux (Fedora / RHEL) | 15 MB | `4cacc2254e8de627a31d58a16e13da3b844db3c45e55f6f3596c6a5d81bd632f` |

---

## 4. Local Binary Inspection

- **Windows Executable:** `file /tmp/openmesh-rc2-artifacts/OpenMesh_0.2.0-rc.2_x64-setup.exe`
  - Result: `PE32 executable (GUI) Intel 80386, for MS Windows, Nullsoft Installer self-extracting archive`
- **macOS DMG Bundle:** Mounted via `hdiutil attach`
  - Info.plist `CFBundleShortVersionString`: `0.2.0-rc.2`
  - Info.plist `CFBundleVersion`: `0.2.0-rc.2`

---

## 5. Packaged Runtime Smoke Validation

For full runtime verification details of the published artifact, see [RUNTIME_SMOKE.md](./RUNTIME_SMOKE.md).

- **Tested macOS DMG:** `OpenMesh_0.2.0-rc.2_aarch64.dmg` (SHA-256 `d5fe55ade824446582bd0b5906f56a4ba91e9e70ef53ffa98eba288736629919`)
- **Settings Toast Timer Race Fix (`src/pages/SettingsPage.vue`):** **VERIFIED**. Rapid sequential actions (`Export Project` → `Import Data`) properly reset the timer; toast persisted past the previous action's expiration window and expired cleanly after its 3000ms duration.
- **Process Lifecycle & Persistence:** **VERIFIED**. Clean startup, project state restoration from `~/.openmesh/app-state.json`, clean SIGTERM shutdown (exit code 0), and clean relaunch.
- **Windows Runtime Smoke:** **PENDING** (No Windows runtime host available in local Darwin environment).

