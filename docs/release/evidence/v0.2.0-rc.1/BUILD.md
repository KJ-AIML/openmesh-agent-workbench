# A13.3 Desktop Application Production Build Evidence

- **Date:** 2026-09-10
- **Candidate SHA:** `964920e4cae824a49353f65a81972ce2070fe968` (`0.2.0-rc.1`)
- **Platform:** macOS arm64 (Darwin 25.3.0)
- **Build Command:** `npm run tauri:build`
- **Compiler / Toolchain:** rustc 1.97.1, cargo 1.97.1, tauri-cli 2.11.4, node v26.5.1
- **Build Duration:** 2m 15s

## Generated Artifacts

### 1. Application Bundle
- **Path:** `target/release/bundle/macos/OpenMesh.app`
- **Executable:** `target/release/bundle/macos/OpenMesh.app/Contents/MacOS/openmesh`
- **File Type:** Mach-O 64-bit executable arm64
- **Size:** 32,226,112 bytes (~30.7 MB)
- **SHA-256 Checksum:** `ca8e554f43002b4e15b609f2c5aadecfcc29d32f1c778175ca62e11d05313356`
- **Bundle Identifier:** `com.openmesh.app`
- **CFBundleShortVersionString:** `0.2.0-rc.1`
- **CFBundleVersion:** `0.2.0-rc.1`
- **Codesign Status:** Ad-hoc signed (`Signature=adhoc`, `signingIdentity: "-"`)

### 2. Disk Image (DMG)
- **Path:** `target/release/bundle/dmg/OpenMesh_0.2.0-rc.1_aarch64.dmg`
- **Size:** 13,052,985 bytes (~12.4 MB)
- **SHA-256 Checksum:** `93043280ca4f8f4982154224220c5369c2f4dd6eb46dad4d57e1cdcfc7e0a180`
- **Installer Status:** Unsigned / ad-hoc (matches documented open-source release policy; requires standard macOS Gatekeeper quarantine override or `xattr -d com.apple.quarantine`)

## Build Warnings
- Expected notarization skip warning:
  `Warn skipping app notarization, no APPLE_ID & APPLE_PASSWORD & APPLE_TEAM_ID or APPLE_API_KEY & APPLE_API_ISSUER & APPLE_API_KEY_PATH environment variables found`
- Zero fatal errors, zero missing resource warnings.
