# Dogfood checklist — OpenMesh Desktop v0.1.40

> **Historical / Superseded.** This checklist is a historical dogfood record for `v0.1.40`.
> For the current v0.2 release candidate test matrix, see [`docs/release/V0.2_RC_MATRIX.md`](./release/V0.2_RC_MATRIX.md).

**Build / tag:** `v0.1.40` (Historical)

## Release and launch

- [ ] Download the installer for this machine from the [v0.1.40 GitHub Release](https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.1.40), or run `npm run tauri:dev` locally.
- [ ] Launch the app and confirm the shell loads without a blank screen.
- [ ] Confirm the About/update surface reports `0.1.40`.
- [ ] Confirm the app can open Settings, Agent Chat, Continuity, and the proxy pages.

## Built-in proxy

- [ ] Open the Proxy Runtime page and confirm the built-in runtime reports healthy.
- [ ] Add or inspect an OpenAI-compatible provider without starting an external CLIProxyAPI/EasyCLI process.
- [ ] Confirm the API endpoint and model catalog are shown with credentials redacted.
- [ ] Exercise a local authenticated request with a test upstream, including one streamed response if available.
- [ ] Confirm an invalid client key is rejected without exposing upstream credentials.

## OAuth and account management

- [ ] Open OAuth and confirm provider/account status renders without token values.
- [ ] Verify provider aliases and account activation/priority changes persist after refresh.
- [ ] If test credentials are available, complete one supported browser or device flow and confirm logout removes the stored account.

## Regression checks

- [ ] Run `npm run verify`.
- [ ] Run `cargo test --workspace`.
- [ ] Run `npm run tauri:build`.
- [ ] Record any provider-specific or unsigned-installer limitation in the release notes.
