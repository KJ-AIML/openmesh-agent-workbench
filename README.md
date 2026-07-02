# OpenMesh — Agent Workbench

OpenMesh is a local-first desktop agent workbench for managing projects, terminals, AI agent CLIs, command presets, work snapshots, notes, sessions, and sprint context.

![OpenMesh Hero](docs/assets/openmesh-hero.png)

## Features

- Local-first desktop app (Tauri v2 + Vue 3)
- Project workspace management
- Open terminal in project folder
- Launch Codex, Claude Code, and OpenCode from PATH
- Launch agents with copied project context
- Command palette (Ctrl+K)
- Work snapshots with agent context prompts
- Recent work memory
- File-based storage (`~/.openmesh/` + per-project `.openmesh/`)
- Custom dark desktop shell with frameless window
- Windows-first preview

## Install

Download the latest Windows installer from [Releases](https://github.com/KJ-AIML/openmesh-agent-workbench/releases).

> **Note:** This preview build is unsigned. Windows may show a SmartScreen / Unknown Publisher warning. Click "More info" → "Run anyway" to proceed.

## Agent CLI Support

OpenMesh uses default commands from PATH:

| Agent       | Default command |
| ----------- | --------------- |
| Codex       | `codex`         |
| Claude Code | `claude`        |
| OpenCode    | `opencode`      |

Custom command overrides are optional in Settings. If no override is set, OpenMesh runs the default command from your system PATH.

## Development

```bash
# Install dependencies
npm install

# Run in development mode (Tauri + Vite)
npm run tauri:dev

# Frontend only (browser)
npm run dev
```

## Build

```bash
# Full build (frontend + native binary + installer)
npm run tauri:build
```

Build artifacts are output to `src-tauri/target/release/bundle/`.

## Project Structure

```
web-demo/
├── src/                  # Vue 3 frontend
│   ├── pages/            # Route pages (Home, Projects, Settings, etc.)
│   ├── lib/              # Store, adapters, utilities
│   └── components/       # Shared components
├── src-tauri/            # Rust backend (Tauri v2)
│   ├── src/              # Rust source (commands, storage, git)
│   └── Cargo.toml
├── docs/                 # Internal docs and release notes
└── db/                   # Legacy database files (deprecated)
```

## Storage

OpenMesh uses file-based storage:

- **Global config:** `~/.openmesh/settings.json`
- **Per-project data:** `<project>/.openmesh/` (tasks, sessions, notes, snapshots)

All data is stored locally. No cloud sync.

## Current Status

OpenMesh is early preview software (`0.x` = unstable/early). The current release is intended for local dogfooding and Windows-first testing.

### Known Limitations

- Windows-first build (macOS/Linux not tested)
- No embedded terminal (opens external terminal window)
- No auto-paste into terminal
- No cloud sync
- No code-signed installer

## License

MIT
