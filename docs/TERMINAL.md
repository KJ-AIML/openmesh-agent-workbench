# Embedded Terminal (PTY)

> UI: Agent Chat right sidebar · Code: `ChatTerminalPanel.vue`, `EmbeddedTerminal.vue`, `src-tauri/src/pty_desktop.rs`  
> Related: [CHAT.md](./CHAT.md) · [SESSIONS.md](./SESSIONS.md)

## Contents

1. [Two terminal paths](#two-terminal-paths)
2. [Embedded PTY](#embedded-pty)
3. [IPC](#ipc)
4. [Layout & prefs](#layout--prefs)
5. [External terminal / agent CLI](#external-terminal--agent-cli)
6. [Dogfood](#dogfood)
7. [Limits](#limits)

---

## Two terminal paths

| Path | What | When |
|------|------|------|
| **Embedded PTY** | In-app xterm + `portable-pty` | Chat Terminal panel (desktop) |
| **External** | OS terminal / agent CLI process | Home, Sessions “Resume in terminal”, `/delegate`, palette, “open external” |

Do not confuse them: Session resume and agent launchers use **external**; the Chat sidebar uses **PTY**.

---

## Embedded PTY

- Desktop-only (`ptyAdapter` throws in browser/web runtime)
- Tabbed sessions; multiple concurrent PTYs allowed
- Default cwd = active project folder (falls back to home)
- xterm + fit addon (`@xterm/xterm`, `@xterm/addon-fit`)
- Shell resolution is platform-native via `portable-pty`

---

## IPC

Rust: `src-tauri/src/pty_desktop.rs`  
FE: `src/lib/adapters/ptyAdapter.ts`

| Command / event | Role |
|-----------------|------|
| `pty_create` | Open session id + cwd + size |
| `pty_write` | stdin |
| `pty_resize` | cols/rows |
| `pty_kill` / `pty_kill_all` | teardown |
| `pty-data` / exit events | stdout stream + exit |

---

## Layout & prefs

- Default: **right sidebar** on Agent Chat (also supports bottom dock)
- Resizable; chat transcript stays independently scrollable
- Prefs in `localStorage` keys under `openmesh.chat.terminal.*`
- Composer `@` can mention terminal/shell tabs or open the panel

Shell tab model: `src/lib/agentChat/shellTabs.ts`.

---

## External terminal / agent CLI

- `src/lib/adapters/terminalAdapter.ts` → Tauri `open_terminal` / agent launch helpers in `src-tauri/src/lib.rs`
- Platform-specific terminal discovery (Windows / macOS / Linux common terms)
- Agent CLI launch can surface an OpenMesh brief path before starting Codex/Claude/OpenCode
- Used by Home “open terminal”, Sessions resume, Delegate

---

## Dogfood

1. `npm run tauri:dev` (not browser-only)  
2. Open a project → Agent Chat  
3. Open Terminal panel → confirm prompt in project cwd  
4. `pwd` / `ls` · open a second tab · resize sidebar  
5. Leave Chat page → confirm PTYs cleaned up (or explicitly kill)  
6. From Home, open **external** terminal and confirm it’s a separate OS window  

---

## Limits

- Not available in pure Vite browser preview  
- Not a full IDE terminal product (no remote SSH PTY product claim)  
- Agent “Resume in terminal” does **not** inject into the embedded PTY automatically  
- README historically said “no embedded terminal” — **outdated**; this doc is current
