# A13.13 Installer and Platform State Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **DMG Package:** `target/release/bundle/dmg/OpenMesh_0.2.0-rc.1_aarch64.dmg`
- **Result:** **PASS (macOS arm64 Verified; Other Platforms CI-Build-Only)**

## Platform Verification Classification Matrix

```text
macOS arm64     VERIFIED
macOS x64       CI-BUILD-ONLY
Windows x64     CI-BUILD-ONLY
Linux           CI-BUILD-ONLY
```

## macOS DMG Packaging & Gatekeeper Smoke

| Check | Procedure | Observed Result | Status |
| :--- | :--- | :--- | :---: |
| **1. DMG Mount** | `hdiutil attach ... -nobrowse` | Attached successfully to `/Volumes/OpenMesh` | **PASS** |
| **2. Bundle Integrity** | Validate `OpenMesh.app` in volume | Mach-O 64-bit arm64 binary, Info.plist version `0.2.0-rc.1`, icons present | **PASS** |
| **3. Code Signature** | `codesign -dvvv /Volumes/OpenMesh/OpenMesh.app` | Ad-hoc signature present (`Signature=adhoc`) | **PASS** |
| **4. Gatekeeper Behavior** | `spctl --assess --type execute` | Rejected by default (exit code 3), matching expected unsigned build state | **PASS** |
| **5. Unquarantine Override** | Clear quarantine via `xattr -d com.apple.quarantine` | App executes after documented override without security popup | **PASS** |
| **6. Clean Unmount** | `hdiutil detach /Volumes/OpenMesh` | Volume detached cleanly with zero orphan processes | **PASS** |

## Non-macOS Platform Status Disclosure
- Non-macOS desktop targets (Windows x64 MSI/EXE, Linux deb/AppImage) and Intel macOS (x86_64) cannot be natively executed on this Darwin arm64 host.
- Their build definitions remain verified via GitHub Actions release workflow specifications and static configuration matrices.
