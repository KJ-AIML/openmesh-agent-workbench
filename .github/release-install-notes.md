## Install (unsigned preview)

**Pick the right macOS asset**

- Apple Silicon (M1/M2/M3/M4…): `OpenMesh_*_aarch64.dmg`
- Intel Mac: `OpenMesh_*_x64.dmg`
- Windows: `OpenMesh_*_x64-setup.exe` (or `.msi`)
- Linux: `.deb` / `.AppImage` / `.rpm`

**macOS Gatekeeper (“damaged” / won’t open)**

Preview builds are not Apple Developer ID signed or notarized. After dragging `OpenMesh.app` to Applications:

```bash
xattr -cr /Applications/OpenMesh.app
open /Applications/OpenMesh.app
```

Or: right-click → **Open** → **Open**. Repo helper: `scripts/macos-unquarantine.sh`.

**Windows:** SmartScreen may warn → More info → Run anyway.
