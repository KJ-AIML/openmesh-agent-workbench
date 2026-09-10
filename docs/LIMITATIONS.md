# Limitations — Want vs Reality

> Honest alpha boundaries. Prefer this over marketing language or stale README claims.  
> Index: [README.md](./README.md)

## Contents

1. [Product posture](#product-posture)
2. [Want vs Reality](#want-vs-reality)
3. [Security posture](#security-posture)
4. [Platform / install](#platform--install)
5. [Chat & agent](#chat--agent)
6. [Continuity / mesh](#continuity--mesh)
7. [Sessions & terminal](#sessions--terminal)
8. [Docs drift hall of shame](#docs-drift-hall-of-shame)

---

## Product posture

OpenMesh Desktop is **early preview (`0.x`)**: local dogfood, evolving APIs, unsigned installers. Useful as a workbench today — not a finished SaaS mesh.

---

## Want vs Reality

| Want / easy to assume | Reality today |
|----------------------|---------------|
| Streaming tokens in Chat | **No** — Agent Engine is non-streaming; tool steps and assistant text return on turn completion |
| Native Anthropic / Gemini direct in Chat | **No** — Chat requires OpenAI-compatible endpoint; Claude/Gemini must route via built-in proxy or compatible gateway |
| Cloud sync of projects | **No** — local `~/.openmesh/` + `<project>/.openmesh/` only |
| WAN / internet mesh | **No** — LAN only; no NAT traversal |
| E2E encrypted mesh product | **No** — LAN uses local pairing bearers, not E2E crypto or an IdP |
| WhatsApp-like DMs | **No** — Continuity Chat is LAN HTTP text only |
| Multi-tenant team cloud admin | **No** — local team registry; cloud sync is dry-run scaffold |
| Silent agent file writes | **No** — patches human-gated; Ask mode read-only tools |
| AXGA completely removed | **No** — AXGA is retained specifically for tool-free Work Proxy draft / evidence boundary (ADR-0002) |
| OAuth manages Agent Chat keys | **No** — OAuth connections configure built-in proxy upstreams; Agent Chat uses direct API key in user config |
| 100% Typed IPC | **No** — 80 commands typed; 54 legacy commands contained behind adapters; 10 unused |
| Cursor Canvas SDK (`.canvas.tsx`) | **No** — OpenMesh Auto UI is `openmesh.canvas/1` JSON |
| “Work Proxy answered” theater | Live ask uses **Agent Engine**; missing key fails closed |
| IdP / SSO trust | **No** — local trust-admin policy only |
| Signed / notarized releases | **No** — preview builds unsigned (SmartScreen / Gatekeeper friction) |
| Browser-complete product | **No** — PTY, secrets, most IPC need Tauri desktop |

---

## Security posture

Implemented v0.2 trust matrix: [architecture/TRUST_MODEL.md](./architecture/TRUST_MODEL.md).

- **LAN:** default bind `127.0.0.1`. Wildcard exposure needs explicit `--expose-lan` / `exposeLan`. Protected Agent Engine routes require a paired Bearer token + capability before the engine runs. `/v1/health` is unauthenticated by design for local reachability checks. Live-ask is budgeted (8 / 60s per peer). This is **not** E2E encryption or cloud identity.
- **Webview:** production CSP is set; `withGlobalTauri` is false; `plugin-fs` is not shipped. `style-src 'unsafe-inline'` remains required for Vue and Excalidraw styling. Dev CSP separately allows Vite HMR (`unsafe-eval` only there).
- **OAuth:** system browser, not the webview. Provider origins are not in CSP. Tokens encrypted in OS keychain.
- **Relay:** approve required; received packages quarantine; secret class denied on wire policy for alpha.
- **API keys:** user config file (mode `0600` on Unix) or env — not in project JSON. Pairing tokens hashed at rest outside project JSON.
- **Path confinement:** mutating IPC and engine tools resolve a registered project root, then `safe_child_path` / `path_safety`. **80 commands** are typed; remaining reads and some canvas/continuity commands still take a caller path with legacy containment.
- **Process:** PTY/terminal launchers use program + argv + registered cwd. A spawned PTY is a real OS shell process. Recipes run argv under the project cwd with no program allowlist.
- **Patch apply:** host-gated IPC (`agent_patch_apply`), never a model tool, never LAN.
- **Unsigned installers:** verify you trust the release channel; OS will warn.
- **No SECURITY.md** in-repo as of this writing — report issues via GitHub.

---

## Platform / install

| Claim | Status |
|-------|--------|
| Windows / macOS / Linux installers | Release CI builds multi-OS (`release.yml`) — quality varies; dogfood where you develop |
| “Windows-first only” (old README) | **Stale** — multi-OS pipeline exists; still early |
| Auto-update | Soft check against GitHub releases; not a polished signed updater |
| Signed / notarized macOS DMG | **No** — see dogfood workaround below |

### macOS Gatekeeper (“damaged” / won’t open)

Preview DMGs are **not** Apple Developer ID signed or notarized. After you drag `OpenMesh.app` from the DMG into Applications, macOS may refuse to launch and say the app is **damaged** or incomplete. That is almost always the `com.apple.quarantine` flag + lack of notarization — not a truncated download. Local `npm run tauri:dev` bypasses this path, so it can work while the Release app “won’t open.”

**Pick the matching asset**

| Mac CPU | Release asset |
|---------|---------------|
| Apple Silicon (`uname -m` → `arm64`) | `OpenMesh_*_aarch64.dmg` |
| Intel (`uname -m` → `x86_64`) | `OpenMesh_*_x64.dmg` |

**Reliable dogfood workaround** (after install to `/Applications`):

```bash
xattr -cr /Applications/OpenMesh.app
open /Applications/OpenMesh.app
```

If the app still lives under Downloads or elsewhere:

```bash
xattr -cr /path/to/OpenMesh.app
```

GUI alternative: Finder → right-click `OpenMesh.app` → **Open** → **Open**.  
Or: System Settings → Privacy & Security → scroll to the blocked-app message → **Open Anyway**.

Repo helper: [`scripts/macos-unquarantine.sh`](../scripts/macos-unquarantine.sh).

**Real fix (maintainers):** paid Apple Developer account → Developer ID Application certificate → sign + notarize in CI (`APPLE_*` secrets). Checklist in [DEVELOPMENT.md](./DEVELOPMENT.md#release).

---

## Chat & agent

- Agent Engine response model is non-streaming: turns return full assistant text and executed tool steps upon completion rather than per-token token streams.
- Direct Chat requires OpenAI-compatible endpoints: native Anthropic Claude or Google Gemini messages API formats require routing through the built-in HTTP proxy (`localhost:8317`) or an OpenAI-compatible gateway.
- Needs configured OpenAI-compatible provider + key stored in OS user config.
- DashScope **Coding Plan** keys ≠ Agent Engine chat/tools (fails closed).
- Max tool-loop iterations bounded; long turns can still be heavy (mitigated with spawn_blocking + debounced persist).
- Delegate / verify / patch depth is MVP — expect rough edges.
- Chat Continuity integration is boundary-based (patches, verify, handoff, import), not a transcript ledger.
- Chat routing is explicit: `/command` is local; ordinary language always goes to Agent Engine. Keyword/substring shortcuts are gone.
- Voice is optional and environment-dependent (mic permissions, TTS).

---

## Continuity / mesh

- UDP discovery flaky on VPN/loopback/cross-subnet
- Continuity UI `lanServeStart` without `exposeLan` is loopback-only (intentional)
- LAN Chat has no CLI surface
- Pack/approve relay is CLI-first
- Online Proxy mode labels may still say LocalScaffold while answers are live LLM
- Team cloud sync does **not** upload
- Agent Chat writes **WorkSignals** for semantic work boundaries (patch proposal/apply/reject, verify recipes, explicit handoff, import/continue). Assistant prose is not canonical. Promotion still does not run automatically; WorkEvents require the existing promotion/ledger path. See [CHAT_CONTINUITY_INTEGRATION.md](./architecture/CHAT_CONTINUITY_INTEGRATION.md).
- Remote live ask (`LanPeer` / `ContinuityQuery`) cannot persist project Continuity through Agent Engine.

---

## Sessions & terminal

- Continue-in-Chat quality varies by provider parser
- Resume-in-terminal: Codex / Claude / OpenCode only
- Embedded PTY ≠ Session resume target
- Embedded PTY cwd must be a registered project (no HOME fallback)
- Pure `npm run dev` (browser) cannot use PTY

---

## Docs drift hall of shame

These were true once; **ignore them as current product truth**:

| Stale claim | Where it lingered | Current truth |
|-------------|-------------------|---------------|
| Clone into `web-demo/` | Old README | Repo root is the app |
| No embedded terminal | Old README / early release notes | Chat PTY sidebar exists |
| Windows-first only / macOS untested | Old README | Multi-OS CI; still alpha |
| Human chat UI non-goal | `docs/development/openmesh-0.1.22-…` | Continuity → Chat exists |
| Live ask = Work Proxy only | Early LAN docs | Agent Engine live ask |
| LAN trust = reachability | Older LIMITATIONS / Continuity docs | Pairing + bind default `127.0.0.1` |
| `csp: null` / home-recursive plugin-fs | `v0.1.40` Tauri config | Production CSP set; plugin-fs removed |

Historical files under `docs/development/` and old `release-notes-v0.*` remain for archaeology — see [docs/README.md](./README.md).
