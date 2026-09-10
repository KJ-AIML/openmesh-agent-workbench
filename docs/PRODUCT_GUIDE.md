# OpenMesh Desktop — Product Guide

> Capability bible for humans. Accurate to `v0.2.0-rc.2`.
> Limits: [LIMITATIONS.md](./LIMITATIONS.md) · Index: [README.md](./README.md)

## Contents

1. [What it is](#what-it-is)
2. [Mental model](#mental-model)
3. [Shell & navigation](#shell--navigation)
4. [Projects](#projects)
5. [Agent Chat](#agent-chat)
6. [Agent Sessions](#agent-sessions)
7. [Runtime & Infrastructure](#runtime--infrastructure)
8. [Continuity & Provenance](#continuity--provenance)
9. [Canvas](#canvas)
10. [Work surfaces](#work-surfaces-home-sprint-docs-notes-context)
11. [Terminal](#terminal)
12. [Settings](#settings)
13. [Storage & secrets](#storage--secrets)
14. [CLI vs Desktop](#cli-vs-desktop)
15. [Dogfood path (15 min)](#dogfood-path-15-min)

---

## What it is

**OpenMesh** is a **local-first Agent Workbench for developers** (Tauri v2 + Vue 3 + Rust `openmesh-core`):

- **Project-centric workflow**: Manage any local git or folder workspace without cloud lock-in
- **Agent Chat**: Primary work surface with Ask / Plan / Act / Delegate modes, confined workspace tools, and embedded PTY
- **Multi-session interoperability**: Scan and continue sessions from Codex, Claude Code, OpenCode, Cursor, Gemini, Grok
- **Human-gated patch review**: Structured patch proposals with explicit human Apply/Reject authority
- **Continuity work provenance**: Durable WorkSignal tracking for patch, verify, handoff, and import events
- **Unified provider runtime**: Direct OpenAI-compatible execution for Chat plus optional built-in HTTP proxy listener (`localhost:8317`)
- **Same-LAN collaboration**: Authenticated local peer pairing, live ask, and message relay

It is **not** a multi-tenant cloud SaaS, WhatsApp replacement, or WAN mesh with E2E encryption.

---

## Mental model

```text
┌─────────────────────────────────────────────────────────────┐
│  Desktop shell (Tauri v2)                                   │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────────┐ │
│  │ Sidebar  │  │ Pages (Vue 3)      │  │ Chat Terminal   │ │
│  │ Projects │  │ Chat / Sessions    │  │ (embedded PTY)  │ │
│  │ 4 Groups │  │ Workspace/Runtime  │  │                 │ │
│  └──────────┘  └─────────┬──────────┘  └────────┬────────┘ │
│                          │ Typed IPC (80 cmds)  │          │
│                 ┌────────▼──────────────────────▼────────┐ │
│                 │ openmesh-core (domain submodules)      │ │
│                 │ authorize_agent_turn ──► LlmRuntime    │ │
│                 └────────┬───────────────────────────────┘ │
└──────────────────────────┼─────────────────────────────────┘
                           │
              ~/.openmesh/ · <project>/.openmesh/ · OS Keychain
```

Same domain logic is also exposed via **`openmesh-cli`** for headless and CI workflows.

---

## Shell & navigation

The v0.2 workbench organizes navigation through a single registry (`src/lib/navigation.ts`):

- **Top**: Project switcher + **Primary Chat** (`/agent-chat` — primary work surface and default landing)
- **Workspace**: Home (`/`), Context (`/context`), Docs (`/docs`), Notes (`/notes`), Canvas (`/canvas`), Sprint (`/sprint`)
- **Agents**: Sessions (`/agent-sessions`)
- **Runtime**: Providers (`/proxy-providers`), Connections (`/oauth`), HTTP proxy (`/proxy-runtime`), Usage (`/usage`), Pending & LAN (`/continuity`)
- **Settings**: App preferences (`/settings`) in collapsible footer
- Frameless desktop window with macOS traffic-light clearance in sidebar
- **⌘K / Ctrl+K** command palette for instant navigation and actions

---

## Projects

- Add a folder via **Add Project** (`/projects/new`)
- Global registry: `~/.openmesh/projects.json` (+ settings / app-state)
- Per-project data: `<project>/.openmesh/`
- Selecting a project often lands you in **Agent Chat**
- Delete project from UI removes OpenMesh metadata association — **does not delete** original source files

---

## Agent Chat

Deep dive: [CHAT.md](./CHAT.md)

**In short:**

| Piece | Behavior |
|-------|----------|
| Modes | **Ask** (read-only tools) · **Plan** (propose) · **Act** (plan + continuity writes) · **Delegate** |
| Composer | `/` slash tools · `@` mentions (project/file/doc/note/terminal/shell/canvas) |
| Freeform | OpenAI-compatible Agent Engine tool loop (needs provider + API key + model) |
| Persist | `<project>/.openmesh/agent/chats/` (+ localStorage cache) |
| Stop | Cancels in-flight engine turn / verify recipe |
| Patches | Propose → human Apply/Reject (never silent LLM write) |
| Fences | Markdown, Mermaid, ` ```canvas ` Auto UI, artifacts |

Slash starters: `/pilot` `/read` `/diff` `/verify` `/continue` (+ many more via `/tools`).

---

## Agent Sessions

Deep dive: [SESSIONS.md](./SESSIONS.md)

- Scans provider session roots for the **open project cwd**
- Providers: **Codex, Claude Code, OpenCode, Cursor, Gemini, Grok** (auto-detect; optional path overrides in Settings)
- **Continue in Chat:** summarize or import into a new OpenMesh chat (original files untouched)
- **Resume in terminal:** Codex / Claude / OpenCode when project is open (external agent CLI)

---

## Continuity / mesh / LAN

Deep dive: [CONTINUITY_MESH.md](./CONTINUITY_MESH.md)

Tab groups on `/continuity`:

| Group | Tabs |
|-------|------|
| You | Pending, Digest |
| Team | Workspace, Trust, Connectors, Org |
| Mesh | Peers, LAN, Chat, Relay, Proxy |
| Gate | Pilot, RC |

**Trusted-LAN alpha:** UDP `41777` + HTTP `41778`; no WAN/NAT; no product E2E crypto claim. LAN Chat is local text over HTTP — **not WhatsApp**.

---

## Canvas

Deep dive: [CANVAS.md](./CANVAS.md)

| Tab | What |
|-----|------|
| Auto UI | Safe JSON UI docs `schema: openmesh.canvas/1` (agent + chat fences) |
| Network | Node/edge graph (Continuity-aligned) |
| Board | Excalidraw boards `openmesh.board/1` |

Not Cursor `.canvas.tsx` (that only runs inside Cursor).

---

## Work surfaces (Home, Sprint, Docs, Notes, Context)

| Page | Real behavior |
|------|----------------|
| **Home** | Project hub: git status, sprint snapshot, recent items, recent scanned sessions, open external terminal |
| **Sprint** | Local task board (backlog → done); no mock seed tasks |
| **Docs** | Markdown tree under `.openmesh/docs/` |
| **Notes** | Flat markdown notes; deep-link `?file=` |
| **Context** | Search / refresh local context index (docs, notes, tasks, snapshots, sessions, …) |

---

## Terminal

Deep dive: [TERMINAL.md](./TERMINAL.md)

- **Embedded PTY** — right sidebar on Agent Chat (tabbed xterm; desktop-only)
- **External terminal** — OS terminal / agent CLI launch from Home, Sessions, palette

---

## Settings

Deep dive: [SETTINGS.md](./SETTINGS.md)

OpenMesh v0.2 separates infrastructure configuration from user preferences:
- **Runtime Group** (`/proxy-providers`, `/oauth`, `/proxy-runtime`, `/usage`): Provider API credentials, OAuth connections, local HTTP proxy listener, and usage analytics.
- **Settings Page** (`/settings`): App preferences (Overview, Tools, Paths, Appearance, Data, About & Updates).

Legacy routes (`/models`, `/server`, `/status`, `/dev-connector`) redirect safely into corresponding Settings and Runtime surfaces.

---

## Storage & secrets

| Kind | Location |
|------|----------|
| Global app data | `~/.openmesh/` (`settings.json`, `projects.json`, `app-state.json`, …) |
| Per-project | `<project>/.openmesh/` (docs, notes, tasks, agent chats, relay, lan, canvases, …) |
| Agent API key | User config file `{config_dir}/openmesh/agent-api-key` (macOS ≈ `~/Library/Application Support/openmesh/agent-api-key`) — **never** in project JSON |
| Env fallback | `OPENMESH_AGENT_API_KEY` → `OPENAI_API_KEY` → `DEEPSEEK_API_KEY` |

No cloud sync of project data.

---

## CLI vs Desktop

| Prefer Desktop when… | Prefer CLI when… |
|----------------------|------------------|
| Chat, Canvas, Sessions UI, PTY, Settings | Pack/approve relay packages, scripted `lan serve`, pilot/rc gates, CI/dogfood |
| Live Continuity tabs + presence/chat UX | Headless or remote shell workflows |

Both share `openmesh-core`. See [ARCHITECTURE.md](./ARCHITECTURE.md).

---

## Dogfood path (15 min)

1. `npm install` → `npm run tauri:dev` ([DEVELOPMENT.md](./DEVELOPMENT.md))
2. Add a real project folder
3. Settings → Provider: endpoint + model; Save API key; **Test connection**
4. Agent Chat → Ask mode → `/pilot` or a read question → try `/read <file>`
5. Agent Sessions → Scan → **Continue in Chat** on one session
6. Canvas → Auto UI (or ask in Plan/Act to upsert)
7. Continuity → LAN (optional, two machines/projects on same LAN) — read [LIMITATIONS.md](./LIMITATIONS.md) first
8. Open Chat Terminal sidebar; run a quick shell command

Installer releases are typically **unsigned** — macOS “damaged” ≈ Gatekeeper (`xattr -cr /Applications/OpenMesh.app`); see Settings → About / Updates and [LIMITATIONS.md](./LIMITATIONS.md#macos-gatekeeper-damaged--wont-open).
