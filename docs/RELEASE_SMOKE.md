# Release smoke checklist

Short human checklist after tagging a desktop release, plus what CI already covers.

**Related:** [DEVELOPMENT.md](./DEVELOPMENT.md) · [DOGFOOD_v0.1.28.md](./DOGFOOD_v0.1.28.md) · [LIMITATIONS.md](./LIMITATIONS.md) · `.github/workflows/release.yml`

---

## What CI already covers (`Release` workflow)

On `v*` tag push (or manual `workflow_dispatch` with a tag):

| Platform | Artifact |
|----------|----------|
| macOS aarch64 | `.dmg` / `.app.tar.gz` |
| macOS x64 | `.dmg` / `.app.tar.gz` |
| Ubuntu | `.deb` / AppImage (tauri-action defaults) |
| Windows | `.msi` / `.exe` (tauri-action defaults) |

Also:

- Frontend `npm ci` + Tauri bundle per matrix cell
- Tag-push releases prepend Gatekeeper / arch picker notes from `.github/release-install-notes.md`
- **Unsigned / ad-hoc** macOS signing today — do **not** map empty `APPLE_*` / `WINDOWS_*` secrets into the tauri-action `env:` block (empty `APPLE_CERTIFICATE` breaks `security import`)

Local guard (also run in CI on PRs that touch the workflow):

```bash
npm run check:release-workflow
```

---

## Human smoke (post-tag, ~10 minutes)

Do this on a clean machine or after downloading release assets — not only `tauri:dev`.

### 1. Assets exist

- [ ] GitHub Release for the tag has macOS aarch64 + x64 DMGs (and Win/Linux if you care this cycle)
- [ ] Release body includes Gatekeeper / arch picker notes (tag-push path)

### 2. macOS install

- [ ] Install correct arch DMG
- [ ] Expect quarantine; clear with `xattr -cr /Applications/OpenMesh.app` or `./scripts/macos-unquarantine.sh`
- [ ] App launches

### 3. Critical paths

- [ ] Add/select project
- [ ] Settings → Provider → Test connection
- [ ] Agent Chat: one Ask turn + `/tools`
- [ ] Terminal sidebar: open PTY tab
- [ ] Settings → Check for updates (reports current or newer semver)

### 4. Signing regression watch

- [ ] macOS job logs do **not** show `SecKeychainItemImport` / empty-cert import failures
- [ ] Confirm `release.yml` still has **no** `APPLE_CERTIFICATE: ${{ secrets… }}` lines unless real secrets are intentionally enabled

---

## When adding real Apple / Windows certs

1. Put non-empty values in GitHub Actions secrets.
2. Only then add the matching `env:` keys under tauri-action (see comments in `release.yml`).
3. Flip `tauri.conf.json` hardened runtime / signing identity as documented in [DEVELOPMENT.md](./DEVELOPMENT.md).
4. Re-run a tag or `workflow_dispatch` and verify notarization / SmartScreen as applicable.
