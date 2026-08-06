> **Historical release notes (v0.1.0).** Current product truth: [`../README.md`](../README.md) · [`README.md`](./README.md) · [`LIMITATIONS.md`](./LIMITATIONS.md). Claims such as “no embedded terminal” and “Windows-first only” are outdated.

# OpenMesh v0.1.0 — Agent Workbench Preview

First Windows preview release of OpenMesh, a local-first desktop agent workbench.

## Highlights

- Custom dark desktop app shell with frameless window and titlebar controls
- Project workspace management with file-based storage
- Command palette (Ctrl+K / Cmd+K) with fuzzy search
- Open terminal in project folder
- Launch Codex / Claude Code / OpenCode from PATH (zero-config)
- Launch agents with copied workspace context prompt
- Work snapshots with markdown export
- Recent work memory across sessions
- Setup checklist and system status dashboard
- Agent session scanning

## Install

Download the `.exe` installer below.

> **Note:** This preview build is unsigned, so Windows may show a SmartScreen warning. Click "More info" → "Run anyway" to proceed.

## Windows

This is a Windows-first build. macOS and Linux are not tested in this release.

## Requirements

Optional agent CLIs (installed separately):

- `codex` — [OpenAI Codex CLI](https://github.com/openai/codex)
- `claude` — [Claude Code](https://docs.anthropic.com/en/docs/claude-code)
- `opencode` — [OpenCode](https://github.com/opencode-ai/opencode)

OpenMesh launches these from PATH by default. Custom command overrides are available in Settings but are not required.

## Storage

All data is stored locally on your machine:

- **Global settings:** `~/.openmesh/settings.json`
- **Per-project data:** `<project>/.openmesh/` (tasks, sessions, notes, snapshots)

No cloud sync. No telemetry. No accounts.

## Known Limitations

- Windows-first build only
- No embedded terminal (opens external terminal window)
- No auto-paste into terminal
- No cloud sync
- No signed installer
- No macOS/Linux testing
- Early preview — expect rough edges

## Tech Stack

- **Frontend:** Vue 3 + TypeScript + Tailwind CSS
- **Backend:** Rust (Tauri v2)
- **Storage:** File-based (JSON on disk)
- **Build:** Vite + Tauri CLI

## What's Next

- Embedded terminal
- Auto-paste agent context into terminal
- macOS / Linux support
- Code-signed installer
- Improved session management
