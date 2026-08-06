# Agent Sessions

> Route: `/agent-sessions` · Code: `src/pages/AgentSessionsPage.vue`, `src/lib/scanConfiguredSessions.ts`, `crates/openmesh-core/src/session_readers/`  
> Related: [CHAT.md](./CHAT.md) · [SETTINGS.md](./SETTINGS.md) · [TERMINAL.md](./TERMINAL.md)

## Contents

1. [Purpose](#purpose)
2. [Providers](#providers)
3. [Scan behavior](#scan-behavior)
4. [Continue in Chat](#continue-in-chat)
5. [Resume in terminal](#resume-in-terminal)
6. [Home preview](#home-preview)
7. [Settings overrides](#settings-overrides)
8. [Dogfood](#dogfood)
9. [Thin areas](#thin-areas)

---

## Purpose

Discover agent CLI / IDE sessions on disk that touch the **currently open project folder**, then:

- Continue inside **OpenMesh Agent Chat** (owned copy), or  
- Resume in an **external** agent CLI terminal (where supported)

OpenMesh does **not** mutate foreign session files.

---

## Providers

Auto-detected when roots exist (plus optional Settings path overrides):

| Provider | Scan | Continue in Chat | Resume in terminal |
|----------|------|------------------|--------------------|
| Codex | yes | yes | yes |
| Claude Code | yes | yes | yes |
| OpenCode | yes | yes | yes |
| Cursor | yes | yes | no (not wired like CLI resume) |
| Gemini | yes | yes | no |
| Grok | yes | yes | no |

Discovery defaults: `crates/openmesh-core/src/session_readers/discovery.rs`.

---

## Scan behavior

- IPC: `scan_workspace_agent_sessions`
- Requires an open project cwd — otherwise empty list
- Missing provider roots are skipped (no per-provider enable flags required)
- FE helper: `scanConfiguredSessions` / `scanConfiguredSessionsResult`

Empty state shows a Scan CTA (no permanent Mock badge).

---

## Continue in Chat

Modal modes (`src/lib/agentChat/resumeIntoChat.ts`):

| Mode | Result |
|------|--------|
| **Summarize & continue** | Compact offline summary seed (head/tail turns + metadata) |
| **Import full & continue** | Copy readable transcript turns into a new OpenMesh chat |

Then navigates to `/agent-chat?chat=<id>` with provenance noting:

- Original provider session untouched  
- Continue with configured Agent Engine  

If transcript IPC is thin, may fall back to preview/metadata-only context.

---

## Resume in terminal

For Codex / Claude / OpenCode (project open):

- Launches external agent CLI with resume session id via `openAgentCli` / `terminalAdapter`
- Distinct from embedded Chat PTY — opens OS terminal / agent process

---

## Home preview

Home shows a most-recent-first preview (top ~4) via `pickRecentAgentSessions` after scan.

---

## Settings overrides

Settings → Runtime → **Sessions**: optional directory overrides for each provider.

- Empty = auto-detect on this OS/device  
- Overrides only — not a required enable matrix  

---

## Dogfood

1. Open a project that you’ve used with Claude/Codex/Cursor  
2. Sessions → Scan  
3. Pick one → **Summarize & continue** → confirm new chat + provenance system note  
4. (Optional) Codex/Claude/OpenCode → **Resume in terminal**  
5. Confirm foreign session files’ mtimes unchanged  

---

## Thin areas

- Transcript depth/quality varies by provider parser; Cursor/Gemini/Grok import may be thinner than Codex/Claude/OpenCode  
- “Resume in terminal” is not claimed for Cursor/Gemini/Grok  
- Session list is workspace-filtered — global sessions for other folders won’t appear
