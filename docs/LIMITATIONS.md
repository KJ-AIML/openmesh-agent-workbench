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
| Cloud sync of projects | **No** — local `~/.openmesh/` + `<project>/.openmesh/` only |
| WAN / internet mesh | **No** — trusted-LAN alpha only; no NAT traversal |
| E2E encrypted mesh product | **No** — LAN trust = network reachability; no finished E2E crypto claim |
| WhatsApp-like DMs | **No** — Continuity Chat is LAN HTTP text only |
| Multi-tenant team cloud admin | **No** — local team registry; cloud sync is dry-run scaffold |
| Silent agent file writes | **No** — patches human-gated; Ask mode read-only tools |
| Cursor Canvas SDK (`.canvas.tsx`) | **No** — OpenMesh Auto UI is `openmesh.canvas/1` JSON |
| “Work Proxy answered” theater | Live ask uses **Agent Engine**; missing key fails closed |
| IdP / SSO trust | **No** — local trust-admin policy only |
| Signed / notarized releases | **No** — preview builds unsigned (SmartScreen / Gatekeeper friction) |
| Browser-complete product | **No** — PTY, secrets, most IPC need Tauri desktop |

---

## Security posture

- **Trusted-LAN alpha:** anyone who can reach your LAN HTTP port can hit health/ask/relay/chat endpoints — treat like an open local service
- **Relay:** approve required; received packages quarantine; secret class denied on wire policy for alpha
- **API keys:** user config file (mode `0600` on Unix) or env — not in project JSON; FS capability denies `.ssh` and agent key paths for tools
- **Path confinement:** workspace tools use `safe_child_path` + sensitive-path deny
- **Unsigned installers:** verify you trust the release channel; OS will warn
- **No SECURITY.md** in-repo as of this writing — report issues via GitHub

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

- Needs configured OpenAI-compatible provider + key
- DashScope **Coding Plan** keys ≠ Agent Engine chat/tools
- Max tool-loop iterations bounded; long turns can still be heavy (mitigated with spawn_blocking + debounced persist)
- Delegate / verify / patch depth is MVP — expect rough edges
- Voice is optional and environment-dependent (mic permissions, TTS)

---

## Continuity / mesh

- UDP discovery flaky on VPN/loopback/cross-subnet
- LAN Chat has no CLI surface
- Pack/approve relay is CLI-first
- Online Proxy mode labels may still say LocalScaffold while answers are live LLM
- Team cloud sync does **not** upload

---

## Sessions & terminal

- Continue-in-Chat quality varies by provider parser
- Resume-in-terminal: Codex / Claude / OpenCode only
- Embedded PTY ≠ Session resume target
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

Historical files under `docs/development/` and old `release-notes-v0.*` remain for archaeology — see [docs/README.md](./README.md).
