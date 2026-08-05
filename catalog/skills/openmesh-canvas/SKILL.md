---
name: OpenMesh Canvas
description: Create Auto UI canvases (schema openmesh.canvas/1) or open freeform Boards. Auto UI ≠ Board.
---

# OpenMesh Canvas

OpenMesh Canvas is **not** Cursor `.canvas.tsx`. Do not import `cursor/canvas`.

There are three surfaces under **Canvas** in the desktop app:

| Tab | What it is | Who drives it |
|-----|------------|---------------|
| **Auto UI** | Safe JSON UI docs (`openmesh.canvas/1`) — dashboards, tables, checklists | Agents via `canvas_upsert_auto_ui` |
| **Board** | Freeform whiteboard (Excalidraw engine) — draw, text, pan/zoom | Humans + gated AppActions (`boardAddSticky`, `boardConnect`) |
| **Network** | Agent-controllable node/edge graph | Agents via canvas graph AppActions |

**Auto UI is not Board.** Do not treat structured Auto UI blocks as freehand strokes, and do not dump full Excalidraw JSON from the model — use small AppActions only.

## Auto UI — when to use
- Status boards, sprint pulses, comparison tables, checklists
- Prefer a canvas over dumping huge markdown tables into chat

## Auto UI — how
Call tool `canvas_upsert_auto_ui` with a `document` object:

- `schema`: must be `openmesh.canvas/1`
- `id`, `title`, optional `summary`
- `blocks`: allowlisted component tree

Allowed block types: `h1`, `h2`, `text`, `callout`, `stat`, `stats`, `table`, `pills`, `todo`, `code`, `divider`.

## Auto UI — after creating
Tell the user to open **Canvas → Auto UI**. You may also emit a ` ```canvas ` fence with the same JSON so chat renders it inline.

## Board — when to use
- User wants a freeform sketch, whiteboard, or diagram
- Open via AppAction `openBoard` (optional `boardId`)
- Add a sticky: `boardAddSticky` with `{ "text": "…" }` (soft confirm; never emit full scene JSON)
- Link stickies: `boardConnect` with `{ "from": "API", "to": "DB" }` (labels must already exist)
- Persist path: `.openmesh/canvases/boards/<id>.json` (`openmesh.board/1`)
- Do not invent stroke streams or CRDT multiplayer in v1
- Ask mode cannot upsert Auto UI or mutate Board; use Plan/Act
