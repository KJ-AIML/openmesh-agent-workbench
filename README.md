# OpenMesh — Agent Workbench

<p align="center">
  <a href="https://github.com/KJ-AIML/openmesh-agent-workbench/releases/latest">
    <img src="https://img.shields.io/github/v/release/KJ-AIML/openmesh-agent-workbench?sort=semver&style=for-the-badge&color=7c3aed&labelColor=1a1a2e" alt="Latest Release" />
  </a>
  <a href="https://github.com/KJ-AIML/openmesh-agent-workbench/releases">
    <img src="https://img.shields.io/github/downloads/KJ-AIML/openmesh-agent-workbench/total?style=for-the-badge&color=3b82f6&labelColor=1a1a2e" alt="Downloads" />
  </a>
  <a href="https://github.com/KJ-AIML/openmesh-agent-workbench/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/KJ-AIML/openmesh-agent-workbench?style=for-the-badge&color=10b981&labelColor=1a1a2e" alt="License" />
  </a>
  <a href="https://tauri.app">
    <img src="https://img.shields.io/badge/Tauri-v2-ffc131?style=for-the-badge&labelColor=1a1a2e" alt="Tauri v2" />
  </a>
  <a href="https://vuejs.org">
    <img src="https://img.shields.io/badge/Vue-3-4fc08d?style=for-the-badge&labelColor=1a1a2e&logo=vue.js&logoColor=4fc08d" alt="Vue 3" />
  </a>
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/Rust-2021-dea584?style=for-the-badge&labelColor=1a1a2e&logo=rust&logoColor=dea584" alt="Rust" />
  </a>
</p>

<p align="center">
  <strong>Local-first desktop agent workbench</strong> — Agent Chat, sessions scan, Continuity/LAN alpha, Canvas, sprint/docs/notes, and an embedded PTY — backed by Rust <code>openmesh-core</code> + CLI.
</p>

<p align="center">
  <a href="./docs/README.md"><strong>📚 Docs index (start here)</strong></a>
  ·
  <a href="./docs/PRODUCT_GUIDE.md">Product guide</a>
  ·
  <a href="./docs/LIMITATIONS.md">Limitations</a>
  ·
  <a href="./docs/DEVELOPMENT.md">Development</a>
</p>

---

<p align="center">
  <img src="docs/assets/openmesh-hero.png" alt="OpenMesh Hero" width="100%" />
</p>

---

## Features (current)

| Area | What you get |
|------|----------------|
| **Agent Chat** | Ask / Plan / Act / Delegate · `/` tools · `@` mentions · durable chats · Stop · patches (human-gated) |
| **Sessions** | Scan Codex / Claude / OpenCode / Cursor / Gemini / Grok · Continue in Chat · Resume in terminal (CLI agents) |
| **Continuity** | Pending/digest · Team/Trust · Mesh peers · trusted-LAN relay/ask/chat · Pilot/RC |
| **Canvas** | Auto UI (`openmesh.canvas/1`) · Network graph · Excalidraw boards |
| **Work** | Home · Sprint · Docs · Notes · Context search |
| **Terminal** | Embedded PTY sidebar in Chat + external OS/agent CLI launch |
| **Settings** | Provider & models · Voice · Extensions · Sessions paths · Appearance · Updates |
| **Local-first** | `~/.openmesh/` + `<project>/.openmesh/` · API key in user config (not project JSON) |

Honest boundaries (no WAN mesh, no WhatsApp, no finished E2E crypto, unsigned previews): **[docs/LIMITATIONS.md](./docs/LIMITATIONS.md)**.

---

## Quick Start

```bash
git clone https://github.com/KJ-AIML/openmesh-agent-workbench.git
cd openmesh-agent-workbench   # repo root — there is no web-demo/ folder
npm install
npm run tauri:dev
```

Frontend-only (limited — no PTY / most IPC): `npm run dev` → `http://localhost:3000`.

Full commands: **[docs/DEVELOPMENT.md](./docs/DEVELOPMENT.md)**.

---

## Install

Download from [Releases](https://github.com/KJ-AIML/openmesh-agent-workbench/releases/latest). CI builds macOS / Windows / Linux installers on `v*` tags.

| Your machine | Download |
|--------------|----------|
| Apple Silicon Mac (M1+) | `OpenMesh_*_aarch64.dmg` |
| Intel Mac | `OpenMesh_*_x64.dmg` |
| Windows | `OpenMesh_*_x64-setup.exe` (or `.msi`) |
| Linux | `.deb` / `.AppImage` / `.rpm` |

> **Unsigned preview builds.** macOS often shows **“OpenMesh is damaged and can’t be opened”** after installing from the DMG — that is Gatekeeper quarantine, not a broken file. Clear it, then open:
>
> ```bash
> xattr -cr /Applications/OpenMesh.app
> open /Applications/OpenMesh.app
> ```
>
> Or right-click → **Open** → **Open**. Repo helper: [`scripts/macos-unquarantine.sh`](./scripts/macos-unquarantine.sh).  
> Windows may show SmartScreen → **More info → Run anyway**. Details: [docs/LIMITATIONS.md](./docs/LIMITATIONS.md) · signing follow-up: [docs/DEVELOPMENT.md](./docs/DEVELOPMENT.md#release).

---

## Agent support

| Surface | Agents |
|---------|--------|
| **Agent Engine (Chat)** | Any OpenAI-compatible API (configure in Settings → Provider) |
| **Session scan** | Codex, Claude Code, OpenCode, Cursor, Gemini, Grok |
| **External CLI launch / resume** | Codex, Claude Code, OpenCode (PATH or Settings override) |

---

## Project structure

```
.
├── src/                  # Vue 3 frontend
├── src-tauri/            # Tauri v2 desktop shell
├── crates/
│   ├── openmesh-core/    # Domain, storage, LAN, agent engine
│   └── openmesh-cli/     # CLI over core
├── docs/                 # Product + capability documentation
├── catalog/skills/       # Builtin skill packs
├── plugins/              # Sample plugins
├── tests/ · e2e/         # Vitest · Playwright
└── package.json
```

---

## Storage

| Kind | Location |
|------|----------|
| Global | `~/.openmesh/` |
| Per-project | `<project>/.openmesh/` |
| Agent API key | `{OS config dir}/openmesh/agent-api-key` (never in project JSON) |

No cloud sync.

---

## Current status

> [!NOTE]
> OpenMesh is early preview software (`0.x`). Version baseline **0.1.28**. Intended for local dogfooding. See [docs/PRODUCT_GUIDE.md](./docs/PRODUCT_GUIDE.md) and [CHANGELOG.md](./CHANGELOG.md).

### Known limitations (summary)

- Trusted-LAN alpha only — no WAN/NAT, no product E2E mesh crypto
- LAN Chat ≠ WhatsApp / cloud DMs
- Unsigned installers
- Desktop app required for PTY and most native features

Full list: **[docs/LIMITATIONS.md](./docs/LIMITATIONS.md)**.

---

## License

MIT

---

<div align="center">

**[Docs](./docs/README.md)** · **[Download](https://github.com/KJ-AIML/openmesh-agent-workbench/releases/latest)** · **[Issues](https://github.com/KJ-AIML/openmesh-agent-workbench/issues)** · **[Discussions](https://github.com/KJ-AIML/openmesh-agent-workbench/discussions)**

</div>
