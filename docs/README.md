# OpenMesh Desktop — Docs Index

**Product:** OpenMesh Agent Workbench (desktop)  
**Version baseline:** `0.1.27`  
**Status:** early / alpha — local-first dogfood, not a finished cloud mesh product

Start here if the app feels big. These docs map **what exists in code today**, not a roadmap pitch.

---

## Start here

| If you want… | Read |
|--------------|------|
| What the app can do (user bible) | [PRODUCT_GUIDE.md](./PRODUCT_GUIDE.md) |
| Honest “not yet / never claim this” | [LIMITATIONS.md](./LIMITATIONS.md) |
| macOS DMG “damaged” / won’t open | [LIMITATIONS.md — Gatekeeper](./LIMITATIONS.md#macos-gatekeeper-damaged--wont-open) |
| Run / test / build / release signing checklist | [DEVELOPMENT.md](./DEVELOPMENT.md) |
| Module map (Tauri / Vue / core / CLI) | [ARCHITECTURE.md](./ARCHITECTURE.md) |

---

## Capability guides

| Surface | Doc |
|---------|-----|
| Agent Chat (modes, `/` `@`, tools, persist) | [CHAT.md](./CHAT.md) |
| Continuity / LAN / Team / Trust / Chat | [CONTINUITY_MESH.md](./CONTINUITY_MESH.md) |
| Agent Sessions scan + Continue in Chat | [SESSIONS.md](./SESSIONS.md) |
| Embedded PTY terminal (right sidebar) | [TERMINAL.md](./TERMINAL.md) |
| Canvas (Auto UI, Network, Board) | [CANVAS.md](./CANVAS.md) |
| Settings hub | [SETTINGS.md](./SETTINGS.md) |

---

## App entry points (routes)

| Route | Page |
|-------|------|
| `/` | Home |
| `/agent-chat` | Agent Chat (primary) |
| `/agent-sessions` | Sessions scan |
| `/continuity` | Continuity / mesh / LAN |
| `/canvas` | Canvas |
| `/sprint` `/docs` `/notes` `/context` | Work surfaces |
| `/settings` | Settings (`?section=`) |

Sidebar: Work (Home → Context) · Team/Mesh (Continuity) · Agents (Sessions) · Chat is primary chrome, not buried under Agents.

---

## Historical / archival (do not treat as current product truth)

| Path | Notes |
|------|--------|
| [`development/`](./development/) | Versioned execution plans, handoffs, unlock matrix — useful history; many claims are superseded |
| [`dogfood-checklist.md`](./dogfood-checklist.md) | v0.3 storage QA — historical; see RC handoff + PRODUCT_GUIDE |
| `release-notes-v0.*`, `storage-*`, `tauri-*`, `post-v0.1.0-*` | Point-in-time notes from early releases |

When a plan contradicts these capability docs, **prefer the capability docs + CHANGELOG + code**.

---

## Related repo files

- App overview: [`../README.md`](../README.md)
- Changelog: [`../CHANGELOG.md`](../CHANGELOG.md)
- Canvas skill: [`../catalog/skills/openmesh-canvas/SKILL.md`](../catalog/skills/openmesh-canvas/SKILL.md)
- Voice skill: [`../catalog/skills/openmesh-voice/SKILL.md`](../catalog/skills/openmesh-voice/SKILL.md)

Parent Heli workspace docs (multi-repo harness) live under `../../.heli-harness/` — not product docs for this app.
