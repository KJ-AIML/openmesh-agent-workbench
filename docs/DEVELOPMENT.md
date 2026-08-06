# Development

> Run, test, and release pointers for OpenMesh Desktop.  
> Index: [README.md](./README.md) · Architecture: [ARCHITECTURE.md](./ARCHITECTURE.md)

## Contents

1. [Prerequisites](#prerequisites)
2. [Quick start](#quick-start)
3. [Useful scripts](#useful-scripts)
4. [Rust / Cargo notes](#rust--cargo-notes)
5. [Tests](#tests)
6. [CLI dogfood](#cli-dogfood)
7. [Release](#release) (macOS DMG dogfood + Apple signing checklist)
8. [Layout reminder](#layout-reminder)

---

## Prerequisites

- **Node** ≥ 20 (see `.nvmrc` / `package.json` engines)
- **npm** (packageManager `npm@11.6.2`; lockfile is `package-lock.json`)
- **Rust** toolchain compatible with `rust-version = 1.92.0` in Cargo manifests
- Tauri v2 system deps for your OS ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

---

## Quick start

From **repo root** (not a `web-demo/` subfolder — that path is obsolete):

```bash
git clone https://github.com/KJ-AIML/openmesh-agent-workbench.git
cd openmesh-agent-workbench
npm install
npm run tauri:dev
```

| Command | Result |
|---------|--------|
| `npm run tauri:dev` | Full desktop app (Vite + Tauri) — **preferred** |
| `npm run dev` | Frontend only at `http://localhost:3000` — no PTY/secrets/most IPC |

---

## Useful scripts

From `package.json`:

| Script | Purpose |
|--------|---------|
| `npm run verify` | `typecheck` + `lint` + `test` |
| `npm run typecheck` | `vue-tsc --noEmit` |
| `npm run lint` | ESLint |
| `npm run test` | Vitest once |
| `npm run test:watch` | Vitest watch |
| `npm run test:e2e` | Playwright smoke |
| `npm run test:e2e:install` | Install Chromium for Playwright |
| `npm run build` | Frontend production build |
| `npm run tauri:build` | Native binary + installers |
| `./scripts/macos-unquarantine.sh` | Clear Gatekeeper quarantine on installed `OpenMesh.app` (unsigned DMG dogfood) |

---

## Rust / Cargo notes

Workspace members: `src-tauri`, `crates/openmesh-core`, `crates/openmesh-cli`.

```bash
cargo test --workspace
cargo build -p openmesh-cli
```

### `CARGO_TARGET_DIR`

Not required for normal `tauri:dev`. Historical dogfood notes (execution ledger) mention sandbox / alternate `CARGO_TARGET_DIR` producing a CLI binary that looked “stale” relative to `target/debug/openmesh-cli` — if CLI behavior disagrees with source, **rebuild the binary you actually invoke** and check which path is on `PATH`.

Default Tauri/cargo output: `src-tauri/target/` and workspace `target/` depending on how you invoke cargo. Prefer explicit:

```bash
cargo build -p openmesh-cli
./target/debug/openmesh-cli --help   # path may vary by workspace layout
```

---

## Tests

| Layer | How |
|-------|-----|
| Frontend unit | `npm run test` (`tests/`, happy-dom) |
| Page contracts | Vitest under `tests/pages/` |
| Rust | `cargo test --workspace` |
| E2E smoke | `npm run test:e2e` (`e2e/smoke.spec.ts`) |

Focused FE check while iterating docs/UI: `npm run typecheck` or a single vitest file.

---

## CLI dogfood

Build CLI, then against a temp project:

```bash
cargo build -p openmesh-cli
# examples — see --help for current surface
./target/debug/openmesh-cli pilot --help
./target/debug/openmesh-cli lan --help
./target/debug/openmesh-cli relay --help
./target/debug/openmesh-cli agent --help
```

RC-oriented historical checklist: `docs/development/handoff-dogfood-rc-1.0.md`.  
Product capability dogfood: [PRODUCT_GUIDE.md](./PRODUCT_GUIDE.md).

---

## Release

- Version: `package.json` + `src-tauri/tauri.conf.json` + crate `Cargo.toml` files (keep in sync when bumping)
- Changelog: `CHANGELOG.md` (Keep a Changelog)
- CI: `.github/workflows/release.yml`
  - Triggers: tag `v*` or `workflow_dispatch`
  - Matrix: macOS aarch64 / x86_64, Ubuntu, Windows
  - `tauri-apps/tauri-action` → installers (**unsigned** preview today)
  - Tag-push releases prepend Gatekeeper install notes via `releaseBody`

### macOS assets (which DMG?)

| Runner target | Asset name pattern | Macs |
|---------------|-------------------|------|
| `aarch64-apple-darwin` | `OpenMesh_*_aarch64.dmg` (+ `.app.tar.gz`) | Apple Silicon |
| `x86_64-apple-darwin` | `OpenMesh_*_x64.dmg` (+ `.app.tar.gz`) | Intel |

Dogfood after DMG install (unsigned):

```bash
xattr -cr /Applications/OpenMesh.app
# or: ./scripts/macos-unquarantine.sh
open /Applications/OpenMesh.app
```

See [LIMITATIONS.md](./LIMITATIONS.md#macos-gatekeeper-damaged--wont-open).

### macOS packaging notes (current)

- Bundle id: `com.openmesh.app` (`tauri.conf.json`)
- Preview CI uses ad-hoc signing (`bundle.macOS.signingIdentity: "-"`) and `hardenedRuntime: false` so the `.app` is a consistent unsigned dogfood bundle. Gatekeeper still quarantines GitHub downloads — users need `xattr` / Open Anyway until notarization exists.
- `Info.plist` / `Entitlements.plist` supply Voice mic usage strings / audio entitlement for future signed builds.

Local bundle:

```bash
npm run tauri:build
```

Artifacts under `src-tauri/target/release/bundle/` (and platform-specific subdirs).

### Release smoke + workflow guard

After tagging, use the short human checklist in [RELEASE_SMOKE.md](./RELEASE_SMOKE.md).  
Dogfood the installed build with [DOGFOOD_v0.1.28.md](./DOGFOOD_v0.1.28.md).

**Do not** map unset `APPLE_*` / `WINDOWS_*` secrets into the tauri-action `env:` block. Empty secrets become `""` and break macOS codesign (`SecKeychainItemImport`). Local + CI guard:

```bash
npm run check:release-workflow
```

CI workflow `.github/workflows/ci.yml` runs that script when `release.yml` (or the guard) changes.

### Follow-up checklist — real Apple sign + notarize

Do **not** invent fake signing. When a paid Apple Developer account and certs exist:

1. Export a **Developer ID Application** certificate as `.p12` and base64-encode it for CI.
2. Create an [app-specific password](https://support.apple.com/en-us/102654) for notarytool.
3. Add GitHub Actions **secrets** (names below). **Only then** wire the matching keys into `release.yml` tauri-action `env:` (they are intentionally **not** pre-wired — see workflow comments + `npm run check:release-workflow`).

   | Secret | Purpose |
   |--------|---------|
   | `APPLE_CERTIFICATE` | base64 `.p12` |
   | `APPLE_CERTIFICATE_PASSWORD` | `.p12` password |
   | `APPLE_SIGNING_IDENTITY` | `Developer ID Application: Name (TEAMID)` |
   | `APPLE_ID` | Apple ID email |
   | `APPLE_PASSWORD` | app-specific password |
   | `APPLE_TEAM_ID` | 10-character Team ID |

4. In `src-tauri/tauri.conf.json` → `bundle.macOS`: set `hardenedRuntime` to `true`; remove or override `signingIdentity: "-"` (env `APPLE_SIGNING_IDENTITY` takes precedence when set).
5. Before notarizing, extend `Entitlements.plist` with WebView JIT keys Tauri needs under hardened runtime (`com.apple.security.cs.allow-jit`, `allow-unsigned-executable-memory`, and usually `allow-dyld-environment-variables`) in addition to mic audio-input.
6. Update `scripts/check-release-workflow.mjs` allowlist intentionally when enabling real cert env mappings.
7. Re-run Release on a `v*` tag; confirm `spctl -a -vv /Applications/OpenMesh.app` accepts the notarized app **without** `xattr`.
8. Optional Windows: `WINDOWS_CERTIFICATE` + `WINDOWS_CERTIFICATE_PASSWORD` for Authenticode.

---

## Layout reminder

```
src/                 Vue
src-tauri/           Tauri shell
crates/openmesh-*    Core + CLI
docs/                Product + capability docs (you are here)
docs/development/    Historical plans / handoffs
catalog/skills/      Builtin skills
plugins/             Sample plugins
```

Parent monorepo / Heli harness (if cloning via `openmesh-ws`) is separate — see workspace `.heli-harness/`. Product docs for the app live in **this** repo’s `docs/`.
