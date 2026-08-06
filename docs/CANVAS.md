# Canvas

> Route: `/canvas` (`?tab=auto-ui|network|board`) · Code: `src/pages/CanvasPage.vue`, `src/lib/canvas/`, `crates/openmesh-core/src/canvas/`  
> Skill: [`catalog/skills/openmesh-canvas/SKILL.md`](../catalog/skills/openmesh-canvas/SKILL.md) · Related: [CHAT.md](./CHAT.md)

## Contents

1. [Tabs](#tabs)
2. [Auto UI](#auto-ui)
3. [Network](#network)
4. [Board](#board)
5. [Chat integration](#chat-integration)
6. [App actions](#app-actions)
7. [Dogfood](#dogfood)
8. [Not this](#not-this)

---

## Tabs

Default tab: **Auto UI**.

| Tab | Query | Purpose |
|-----|-------|---------|
| Auto UI | `?tab=auto-ui` | Safe agent-authored JSON UI documents |
| Network | `?tab=network` | Node/edge graph visualization |
| Board | `?tab=board` | Excalidraw freeform boards |

Chrome is Continuity-aligned (same visual family as mesh/network work).

---

## Auto UI

| Fact | Detail |
|------|--------|
| Schema | `openmesh.canvas/1` |
| Tool | `canvas_upsert_auto_ui` (Plan/Act — **not** Ask) |
| Storage | `<project>/.openmesh/canvases/auto-ui/` |
| Blocks | h1/h2/text/callout/stat(s)/table/pills/todo/code/divider (see core + skill) |
| Renderer | `OmCanvasRenderer.vue` + FE `autoUi.ts` |

Agents create documents; humans browse/delete on the Canvas page. Inline chat fences also render (see below).

---

## Network

- Node/edge graph from canvas store (`src/lib/canvas/store.ts`)
- Fit / layout helpers; Continuity-flavored presentation
- Useful for mesh/relationship sketches — not a full graph database product

On-disk layout for network graphs is thinner/less documented than Auto UI & Board paths — treat as evolving.

---

## Board

| Fact | Detail |
|------|--------|
| Schema | `openmesh.board/1` |
| Editor | Excalidraw island (`BoardEditor.vue`, `excalidrawIsland.ts`) |
| Storage | `<project>/.openmesh/canvases/boards/` |
| Ops | create / save scene / delete; dirty indicator |

React Excalidraw is embedded inside the Vue app for the board tab only.

---

## Chat integration

From Agent Chat message fences (`src/lib/agentChat/markdown.ts`):

- ` ```canvas ` → live Auto UI render in `ArtifactPanel`
- Optional **Save to Canvas** into project Auto UI store
- Mermaid / generic artifacts also surface in the artifact panel

Ask mode cannot upsert Auto UI; switch to Plan/Act (skill documents this).

---

## App actions

Canvas page registers app-action handlers (`src/lib/appActions/`) so agents/UI can navigate or propose board/scene edits with human-visible context. Board remounts after external scene edits (`boardEpoch`).

---

## Dogfood

1. Open `/canvas` → Auto UI (empty is OK)  
2. Agent Chat → Plan mode → ask for a small Auto UI summary of the project  
3. Confirm artifact in chat + document appears under Canvas → Auto UI  
4. Board tab → create board → draw → save → reopen  
5. Network tab → open and fit graph if nodes exist  

---

## Not this

- **Not** Cursor `.canvas.tsx` / Cursor Canvas SDK (only runs inside Cursor)
- Not a replacement for Figma / full design tool
- Not WAN-shared multiplayer canvas
