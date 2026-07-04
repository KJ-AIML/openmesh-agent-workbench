# OpenMesh v0.1.1 - Local Workbench Reliability

OpenMesh v0.1.1 is a Windows preview reliability update for the local workbench layer.

## Highlights

- Docs now support nested tree navigation with folder organization.
- Docs can be moved into folders with the rebuilt pointer-based move flow.
- Docs nested rename is fixed with safer backend path handling.
- Notes rename now uses a backend rename command instead of a fragile write/delete flow.
- Notes markdown import handling is clearer for external `.md` drops.
- Home page agent launcher icons for Codex, Claude, and OpenCode were polished.
- Startup now shows an in-app OpenMesh loading screen.

## Why This Release Matters

This release strengthens the local work memory foundation. Docs and Notes are not just UI polish; they are future evidence sources for Work Proxy answers.

## Validation

- `cargo test rename_doc_keeps_nested_relative_path_inside_docs`
- `cargo check`
- `npm run build`

## Known Limitations

- Windows-first preview build only.
- Installer remains unsigned.
- Startup splash is in-app after WebView load, not a native pre-WebView splash.
- Work Proxy, Team Mesh, and cloud sync are not included in this release.

## Next Direction

The next strategic product layer is My Work Proxy: a local, evidence-backed proxy profile that can answer from work memory with boundaries, freshness, and citations.
