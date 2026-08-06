# OpenMesh Desktop — Product Guide

> Capability bible for humans. Accurate to `0.1.27`.  
> Limits: [LIMITATIONS.md](./LIMITATIONS.md) · Index: [README.md](./README.md)

## Contents

1. [What it is](#what-it-is)
2. [Mental model](#mental-model)
3. [Shell & navigation](#shell--navigation)
4. [Projects](#projects)
5. [Agent Chat](#agent-chat)
6. [Agent Sessions](#agent-sessions)
7. [Continuity / mesh / LAN](#continuity--mesh--lan)
8. [Canvas](#canvas)
9. [Work surfaces](#work-surfaces-home-sprint-docs-notes-context)
10. [Terminal](#terminal)
11. [Settings](#settings)
12. [Storage & secrets](#storage--secrets)
13. [CLI vs Desktop](#cli-vs-desktop)
14. [Dogfood path (15 min)](#dogfood-path-15-min)

---

## What it is

**OpenMesh** is a **local-first desktop agent workbench** (Tauri v2 + Vue 3 + Rust `openmesh-core`):

- Manage a project folder as a workspace
- Chat with an **Agent Engine** (OpenAI-compatible API) that can use confined workspace tools
- Scan / continue sessions from Cursor, Claude Code, Codex, OpenCode, Gemini, Grok
- Keep sprint/docs/notes/context on disk under `<project>/.openmesh/`
- Optional **trusted-LAN alpha** peer relay, live ask, and human LAN chat
- Embedded **PTY terminal** beside Chat; Canvas Auto UI / Network / Board

It is **not** a finished multi-tenant cloud product, WhatsApp replacement, or WAN mesh with E2E crypto.

---

## Mental model

```
┌─────────────────────────────────────────────────────────────┐
│  Desktop shell (Tauri)                                      │
│  ┌──────────┐  ┌────────────────────┐  ┌─────────────────┐ │
│  │ Sidebar  │  │ Pages (Vue)        │  │ Chat Terminal   │ │
│  │ Projects │  │ Chat / Continuity  │  │ (embedded PTY)  │ │
│  │ Nav      │  │ Sessions / Canvas  │  │                 │ │
│  └──────────┘  └─────────┬──────────┘  └────────┬────────┘ │
│                          │ IPC                  │          │
│                 ┌────────▼──────────────────────▼────────┐ │
│                 │ openmesh-core (+ desktop adapters)     │ │
│                 └────────┬───────────────────────────────┘ │
└──────────────────────────┼─────────────────────────────────┘
                           │
              ~/.openmesh/ · <project>/.openmesh/ · user config secrets
```

Same domain logic is also exposed via **`openmesh-cli`** for pack/approve/pilot/rc/LAN serve without the GUI.

---

## Shell & navigation

- Frameless desktop window; macOS traffic-light clearance in sidebar
- **⌘K / Ctrl+K** command palette
- Sidebar groups:
  - **Work:** Home, Sprint, Docs, Notes, Canvas, Context
  - **Team / Mesh:** Continuity
  - **Agents:** Sessions
- **Agent Chat** is a primary surface (sidebar + titlebar), not nested under Agents
- Collapsed sidebar supports hover-peek without changing pin preference

Routes: see [docs/README.md](./README.md#app-entry-points-routes).

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

Groups: **Setup** (Overview, Provider, Voice) · **Runtime** (Agents, Extensions, Sessions, Server) · **Project** (Tools, Paths) · **App** (Appearance, Data, About/Updates).

Legacy routes `/models` `/server` `/status` `/usage` `/dev-connector` redirect into Settings sections.

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
