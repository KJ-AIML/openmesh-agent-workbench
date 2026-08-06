# Architecture

> High-level modules as of `0.1.26`. Index: [README.md](./README.md)

## Contents

1. [Stack](#stack)
2. [Workspace layout](#workspace-layout)
3. [Data flow](#data-flow)
4. [Frontend modules](#frontend-modules)
5. [Desktop (Tauri) modules](#desktop-tauri-modules)
6. [openmesh-core](#openmesh-core)
7. [openmesh-cli](#openmesh-cli)
8. [Adapters pattern](#adapters-pattern)
9. [Storage layout](#storage-layout)

---

## Stack

| Layer | Tech |
|-------|------|
| UI | Vue 3 + TypeScript + Vite + Tailwind v4 + vue-router |
| Desktop shell | Tauri v2 (`src-tauri`, product name **OpenMesh**) |
| Domain / storage | Rust crate `openmesh-core` |
| CLI | Rust crate `openmesh-cli` (thin clap over core) |
| Tests | Vitest (FE), cargo tests (crates), Playwright smoke (`e2e/`) |

Versions: `package.json` / `src-tauri/Cargo.toml` / `crates/*/Cargo.toml` track **0.1.26**.

---

## Workspace layout

```
openmesh-agent-workbench/          ← repo root (not web-demo/)
├── src/                           Vue app
├── src-tauri/                     Tauri binary + IPC
├── crates/
│   ├── openmesh-core/             Domain + storage + LAN + agent engine
│   └── openmesh-cli/              CLI binary
├── catalog/skills/                Builtin skill packs
├── plugins/                       Sample plugins
├── docs/                          This documentation set
├── tests/                         Vitest
├── e2e/                           Playwright
└── package.json                   npm scripts (tauri:dev, verify, …)
```

Cargo workspace members: `src-tauri`, `crates/openmesh-core`, `crates/openmesh-cli` (`Cargo.toml`).

---

## Data flow

```
Vue page / component
    → lib/* client or adapter
        → Tauri invoke / event
            → src-tauri desktop module
                → openmesh_core::*
                    → disk under ~/.openmesh/ or <project>/.openmesh/
```

Browser-only `npm run dev` can load UI but **desktop IPC** (PTY, most continuity, secrets, agent engine) requires `npm run tauri:dev`.

---

## Frontend modules

| Path | Role |
|------|------|
| `src/pages/` | Route pages (Home, Chat, Continuity, Canvas, Settings, …) |
| `src/components/chat/` | Composer, terminal, markdown, patches, verify log |
| `src/components/canvas/` | Auto UI renderer, Excalidraw board editor |
| `src/components/settings/` | Appearance, Extensions, Updates, Tools, Voice |
| `src/lib/adapters/` | Environment, FS, git, storage, terminal, PTY, sessions, window |
| `src/lib/agentChat/` | Sessions store, tools, runner, menus, resume/import, persist |
| `src/lib/canvas/` | Auto UI + boards clients |
| `src/lib/voice/` | STT/TTS session bridge |
| `src/lib/appActions/` | App action dispatcher / audit |
| `src/domain/context/` | Context document types/validators |
| `src/router.ts` | Routes + legacy redirects into Settings |

---

## Desktop (Tauri) modules

Notable modules under `src-tauri/src/` (names may grow; see `lib.rs` command registration):

| Area | Examples |
|------|----------|
| Shell / projects / git | storage, git status, open folder/terminal |
| Agent Engine | `agent_engine_desktop` — turn, secrets, patches, recipes |
| Continuity / LAN | `continuity_desktop` — peers, relay, lan, chat, trust, … |
| PTY | `pty_desktop` — `pty_create|write|resize|kill|kill_all` |
| Canvas | `canvas_desktop` |
| Extensions | `extensions_desktop` |
| Voice | `voice_*` |
| Sessions scan | workspace agent session scan IPC |

Config: `src-tauri/tauri.conf.json` (version, window, bundle). Multi-OS installers via `.github/workflows/release.yml`.

---

## openmesh-core

Domain crate. Important areas:

| Module area | Responsibility |
|-------------|----------------|
| `agent_engine/` | Tool loop, registry/modes, secrets, chat store, patches, recipes, extensions, live ask |
| `session_readers/` | Scan Codex/Claude/OpenCode/Cursor/Gemini/Grok |
| `lan/` | UDP beacon, HTTP server/client, chat, presence health |
| `mesh/`, `relay/` | Peers, envelopes, pack/approve/quarantine |
| `online_proxy/` | Continuity proxy ask (live Agent Engine path) |
| `team/`, `trust_admin/`, `connectors/`, `org_graph/` | Local team + trust policy |
| `pilot/`, `rc/` | Readiness / RC evaluation packs |
| `canvas/` | Auto UI + boards persistence |
| `context*` / `ingestion` | Context index |
| `proxy_*` / AXGA | Older Work Proxy **draft** path (tool-free; parallel to Agent Engine) |

---

## openmesh-cli

Thin CLI over core. Representative command groups (see `crates/openmesh-cli/src/main.rs`):

`signal`, `event`, `profile`, `context`, `proxy`, `handoff`, `state`, `pending`, `digest`, `mesh`, `relay`, `lan`, `online-proxy`, `team`, `trust-admin`, `connector`, `org`, `pilot`, `rc`, `agent`, …

**Split examples:**

- Relay **pack/approve** — CLI-first; Desktop lists approved + sends over LAN
- LAN **chat / presence UI** — Desktop; CLI has `lan serve|discover|send|ask|status`
- Agent Chat UI — Desktop; CLI has `agent ask|secret-status` etc.

---

## Adapters pattern

Frontend isolates platform calls in `src/lib/adapters/`:

- Prefer extending an adapter over calling Tauri APIs from pages
- Web runtime gracefully degrades (e.g. PTY throws “desktop app required”)

---

## Storage layout

**Global** (`~/.openmesh/`):

- `settings.json`, `projects.json`, `app-state.json`, …

**Per project** (`<project>/.openmesh/`):

- `project.json`, `docs/`, `notes/`, `tasks.json`, presets/recent, …
- `agent/chats/` — durable Agent Chat sessions
- `relay/` — staging / approved / sent / received / audit
- `lan/` — chat messages, etc.
- `canvases/` — auto-ui + boards

**Secrets:** user config `…/openmesh/agent-api-key` — see [SETTINGS.md](./SETTINGS.md) and [LIMITATIONS.md](./LIMITATIONS.md).
