# OpenMesh Desktop — Docs Index

**Product:** OpenMesh Agent Workbench (desktop)
**Version baseline:** `0.2.0-rc.1`
**Status:** Release Candidate preparation — local-first Agent Workbench for developers

Start here to understand the architecture, capabilities, and release contracts of OpenMesh v0.2.

---

## Start here

| If you want… | Read |
|--------------|------|
| Canonical product center & ADR | [architecture/PRODUCT_CENTER.md](./architecture/PRODUCT_CENTER.md) · [ADR-0001](./architecture/ADR-0001-agent-workbench-center.md) |
| Complete v0.2 capability guide | [PRODUCT_GUIDE.md](./PRODUCT_GUIDE.md) |
| Honest boundaries & known limitations | [LIMITATIONS.md](./LIMITATIONS.md) |
| Product truth audit (v0.2) | [release/V0.2_PRODUCT_TRUTH_AUDIT.md](./release/V0.2_PRODUCT_TRUTH_AUDIT.md) |
| Release candidate test matrix | [release/V0.2_RC_MATRIX.md](./release/V0.2_RC_MATRIX.md) |
| Core domain decomposition (A11) | [architecture/V0.2_CORE_DECOMPOSITION.md](./architecture/V0.2_CORE_DECOMPOSITION.md) |
| Information architecture & nav (A9) | [architecture/V0.2_INFORMATION_ARCHITECTURE.md](./architecture/V0.2_INFORMATION_ARCHITECTURE.md) |
| Desktop IPC contract (A8) | [architecture/IPC_CONTRACT.md](./architecture/IPC_CONTRACT.md) |
| Chat command routing (A7) | [architecture/CHAT_ROUTING.md](./architecture/CHAT_ROUTING.md) |
| Chat ↔ Continuity provenance (A6) | [architecture/CHAT_CONTINUITY_INTEGRATION.md](./architecture/CHAT_CONTINUITY_INTEGRATION.md) |
| Unified provider runtime & proxy (A5) | [architecture/LLM_RUNTIME_INVENTORY.md](./architecture/LLM_RUNTIME_INVENTORY.md) |
| Trust & security boundary matrix (A3/A4) | [architecture/TRUST_MODEL.md](./architecture/TRUST_MODEL.md) |
| AXGA runtime role (ADR-0002) | [architecture/ADR-0002-axga-runtime-role.md](./architecture/ADR-0002-axga-runtime-role.md) |
| Developer build & test guide | [DEVELOPMENT.md](./DEVELOPMENT.md) |
| High-level architecture overview | [ARCHITECTURE.md](./ARCHITECTURE.md) |

---

## Capability guides

| Surface | Doc |
|---------|-----|
| Agent Chat (modes, `/` tools, `@` mentions, patches, PTY) | [CHAT.md](./CHAT.md) |
| External Agent Sessions (Codex, Claude, OpenCode, Cursor, Gemini, Grok) | [SESSIONS.md](./SESSIONS.md) |
| Provider Runtime & Built-in HTTP Proxy | [builtin-proxy-parity.md](./builtin-proxy-parity.md) |
| Continuity / Pending & LAN (provenance, pairing, live ask) | [CONTINUITY_MESH.md](./CONTINUITY_MESH.md) |
| Embedded PTY terminal in Chat | [TERMINAL.md](./TERMINAL.md) |
| Canvas (Auto UI `openmesh.canvas/1`, Network, Board) | [CANVAS.md](./CANVAS.md) |
| Settings hub (App preferences, launcher paths, dangerous presets) | [SETTINGS.md](./SETTINGS.md) |

---

## App entry points (routes)

OpenMesh v0.2 organizes routes via a single navigation registry (`src/lib/navigation.ts`):

| Group | Route | Surface | Role |
|-------|-------|---------|------|
| **Agents** | `/agent-chat` | **Agent Chat** | **Primary work surface** (default project landing) |
| **Agents** | `/agent-sessions` | **Sessions** | External agent scan & import |
| **Workspace** | `/` | **Home** | Project dashboard & quick launch |
| **Workspace** | `/context` | **Context** | Context indexing & search |
| **Workspace** | `/docs` | **Docs** | Project markdown documentation |
| **Workspace** | `/notes` | **Notes** | Scratchpad notes |
| **Workspace** | `/canvas` | **Canvas** | Auto UI scenes & diagrams |
| **Workspace** | `/sprint` | **Sprint** | Sprint tasks & backlog |
| **Runtime** | `/proxy-providers` | **Providers** | Direct OpenAI-compatible provider credentials |
| **Runtime** | `/oauth` | **Connections** | OAuth provider connection management |
| **Runtime** | `/proxy-runtime` | **HTTP proxy** | Built-in local HTTP proxy server control |
| **Runtime** | `/usage` | **Usage** | Provider token & request analytics |
| **Runtime** | `/continuity` | **Pending & LAN** | Continuity records, pairing, & LAN relay |
| **Settings** | `/settings` | **Settings** | App preferences (`?section=`) |

---

## Historical / archival (do not treat as current product truth)

| Path | Notes |
|------|--------|
| [`development/`](./development/) | Pre-v0.2 versioned execution plans & handoffs — historical record |
| [`DOGFOOD_v0.1.40.md`](./DOGFOOD_v0.1.40.md) | Historical smoke checklist for desktop v0.1.40 |
| [`DOGFOOD_v0.1.30.md`](./DOGFOOD_v0.1.30.md) | Historical smoke checklist for desktop v0.1.30 |
| [`DOGFOOD_v0.1.28.md`](./DOGFOOD_v0.1.28.md) | Historical smoke checklist for desktop v0.1.28 |
| [`dogfood-checklist.md`](./dogfood-checklist.md) | Historical v0.3 storage QA checklist |
| `release-notes-v0.*`, `storage-*`, `tauri-*`, `post-v0.1.0-*` | Historical release notes and migration specs |

When a plan contradicts these capability docs, **prefer the capability docs + CHANGELOG + code**.

---

## Related repo files

- App overview: [`../README.md`](../README.md)
- Changelog: [`../CHANGELOG.md`](../CHANGELOG.md)
- Canvas skill: [`../catalog/skills/openmesh-canvas/SKILL.md`](../catalog/skills/openmesh-canvas/SKILL.md)
- Voice skill: [`../catalog/skills/openmesh-voice/SKILL.md`](../catalog/skills/openmesh-voice/SKILL.md)

Parent Heli workspace docs (multi-repo harness) live under `../../.heli-harness/` — not product docs for this app.
