# Agent Chat

> Route: `/agent-chat` · Code: `src/pages/AgentChatPage.vue`, `src/lib/agentChat/`  
> Related: [SESSIONS.md](./SESSIONS.md) · [TERMINAL.md](./TERMINAL.md) · [CANVAS.md](./CANVAS.md) · [LIMITATIONS.md](./LIMITATIONS.md)

## Contents

1. [Purpose](#purpose)
2. [Readiness](#readiness)
3. [Modes](#modes)
4. [Composer](#composer--and-)
5. [Slash tools](#slash-tools)
6. [Freeform Agent Engine](#freeform-agent-engine)
7. [Sessions & persistence](#sessions--persistence)
8. [Import / resume from other agents](#import--resume-from-other-agents)
9. [Patches, verify, delegate, continue](#patches-verify-delegate-continue)
10. [Rendering & artifacts](#rendering--artifacts)
11. [Stop, voice, terminal](#stop-voice-terminal)
12. [Dogfood](#dogfood)

---

## Purpose

Primary workspace surface: talk to the **OpenMesh Agent Engine** with confined tools, or run **slash fast-paths** over existing IPC (docs, continuity, git, …). Chat is first-class chrome — not buried under Agents.

---

## Readiness

Chat expects (Settings → Provider):

1. Provider / base URL configured  
2. API key saved (user secret file or env)  
3. Default / coding model set  

Incomplete setup shows a readiness gate → Settings.  
**Test connection** runs a minimal tool-free probe before relying on Chat.

Coding Plan caveat: DashScope Coding Plan endpoints are rejected for Agent Engine chat/tools — use openai / deepseek / xai / DashScope compatible-mode (see CHANGELOG 0.1.24).

---

## Modes

Cycled in the composer. Allowlists live in `crates/openmesh-core/src/agent_engine/registry.rs`.

| Mode | Intent | Model tools (summary) |
|------|--------|------------------------|
| **Ask** | Inspect only | Read/list/search/git/continuity summary/UI propose — **no** canvas upsert, patch propose, task writes |
| **Plan** | Gather + propose | Ask tools + mesh/pilot/rc/recipes + `propose_patch` + handoff draft + `canvas_upsert_auto_ui` |
| **Act** | Plan + continuity writes | Plan tools + `update_task` + `link_session` + trust-gated `mesh_query` |
| **Delegate** | Inspect + recipes | Same lean set as Ask for model tools; launch via `/delegate` / IPC |

**Hard rule:** source-file **apply** is human-gated (Apply/Reject UI). The model cannot silently write files.

Empty allowlist semantics = Ask read-only.

---

## Composer (`/` and `@`)

Code: `src/lib/agentChat/composerMenus.ts`, `ChatComposer.vue`.

### `/` slash

- Starters: `/pilot` `/read` `/diff` `/verify` `/continue`
- Full inventory: `/tools` or `/help`
- Filter as you type after `/`

### `@` mentions

Kinds: project · file · doc · note · terminal · shell · canvas  

Some items insert path text; others are actions (open Canvas, focus shell tab, open terminal panel).

---

## Slash tools

Frontend inventory: `src/lib/agentChat/tools.ts` (`AGENT_TOOLS`).

| Slash | Role |
|-------|------|
| `/project` | Active workspace metadata |
| `/docs` `/notes` `/sprint` | List project work data |
| `/search` | Context index search |
| `/git` | Branch / dirty counts |
| `/ls` `/read` `/grep` `/diff` | Confined workspace inspect |
| `/patch` | Patch propose/apply UX helpers |
| `/verify` | Approved verify recipes (streamed logs) |
| `/delegate` | Launch external agent CLI with brief |
| `/continue` | Continue tools (handoff / task / session / mesh) |
| `/continuity` `/pending` `/digest` | Continuity summaries |
| `/team` `/trust` `/connectors` `/org` | Team/trust/org views |
| `/pilot` `/rc` | Gate status |
| `/peers` | Mesh peers |
| `/ask` | Continuity / online proxy live ask path |

Slash paths are IPC fast-paths; freeform LLM tool-calling is separate (Agent Engine).

---

## Freeform Agent Engine

- Core: `openmesh_core::agent_engine` — `run_agent_turn` loop (bounded iterations)
- Desktop IPC: `agent_engine_turn` (runs via `spawn_blocking` so UI doesn’t beachball)
- Tools are path-confined (`safe_child_path` + sensitive-path deny: e.g. `.ssh`, `agent-api-key`)
- Prompt may inject enabled **extensions** (skills/hooks/plugins)

Live LAN / Proxy ask reuse the same engine helper (`live_ask`) — not LocalScaffold paste theater.

---

## Sessions & persistence

| Layer | Where |
|-------|--------|
| Durable | `<project>/.openmesh/agent/chats/` |
| Cache | `localStorage` key `openmesh.chat.v1:<projectPath>` (write-through; size-capped) |

UI: New chat · switch · rename · delete. Debounced persist (`persistQueue`) so long turns don’t freeze the UI.

Deep link: `/agent-chat?chat=<id>`.

---

## Import / resume from other agents

From **Agent Sessions** → Continue in Chat:

| Mode | Behavior |
|------|----------|
| Summarize & continue | Compact local summary seed |
| Import full & continue | Copy readable turns into a new OpenMesh chat |

Provenance notes that originals were **not modified**. Continuation uses **your** Agent Engine config.  
Code: `src/lib/agentChat/resumeIntoChat.ts`.

---

## Patches, verify, delegate, continue

| Feature | Notes |
|---------|--------|
| Propose patch | LLM `propose_patch` (Plan/Act); `PatchApprovalCard` for apply/reject/rollback |
| `/verify` | Approved recipes; streamed `agent-run-log`; cancel supported |
| `/delegate` | Brief + external CLI launch (Codex/Claude/OpenCode family) |
| `/continue` | Handoff draft/approve, `update_task`, `link_session`, trust-gated `mesh_query` |

---

## Rendering & artifacts

- Markdown via `marked` + `dompurify`
- Live Mermaid
- Fence extraction: ` ```mermaid `, ` ```canvas `, ` ```artifact `
- ` ```canvas ` → Auto UI (`openmesh.canvas/1`) via `ArtifactPanel` / Canvas page
- Optional save into project canvases

---

## Stop, voice, terminal

| Feature | Behavior |
|---------|----------|
| **Stop** | Cancel in-flight Agent Engine turn and/or verify recipe |
| **Voice** | Optional STT/TTS via Voice HUD + `voiceBridge` (Settings → Voice) |
| **Terminal** | Right sidebar embedded PTY — [TERMINAL.md](./TERMINAL.md) |

---

## Dogfood

1. Configure Provider + key + Test connection  
2. Ask: “What’s in this project?” or `/project`  
3. `/read README.md` (or a real relative path)  
4. Switch Plan → ask for a small patch proposal → Reject (don’t need to apply)  
5. Import a scanned session via Sessions → Continue in Chat  
6. Confirm messages survive app restart (disk under `.openmesh/agent/chats/`)
