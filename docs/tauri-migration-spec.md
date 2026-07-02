# Openmesh Tauri Migration Spec

> Version: 1.0.0 · 2026-07-01  
> Status: Migration Readiness Audit  
> Scope: Web POC → Tauri v2 Desktop App

---

## 1. Current Web POC Summary

Openmesh is a Vue 3 web-first proof of concept running at `http://localhost:3001`. It serves as a personal workbench for resuming work across projects, sprints, docs, terminal sessions, and AI agent sessions.

**What exists now:**

- Home dashboard with 6 sections (Resume Workspace, Setup Checklist, Recent Work, Active Sprint, Agent Sessions, System Status)
- Add Project form with validation
- Project list in sidebar with active project highlighting
- Docs source cards (7 categories per project)
- Sprint view with mock tasks and task detail panel
- Recent work tracking (auto-generated from user actions)
- Dev Connector mock UI (terminal launcher, git status, command presets)
- Agent Sessions list with filters and session detail
- Settings for provider/model/server/agent CLIs
- localStorage persistence with import/export/reset
- Typed store and Vue composable

**Current architecture:**

- `src/lib/store.ts` — localStorage persistence layer (all read/write operations)
- `src/lib/useStore.ts` — Vue 3 reactive composable wrapping the store
- `src/types.ts` — TypeScript types for all data models
- Vue pages and components using the composable
- vue-router for navigation
- Vite for dev/build
- npm as package manager

**Mock behaviors identified:**

- DocsPage: `prompt()` for path input, random file counts
- HomePage: `alert()` for "Would open..." messages
- DevConnectorPage: toast messages, "Mock" badges on Terminal Launcher and Git Status
- SettingsPage: mock health check (random healthy/unreachable)
- store.ts: `initMockSessions()`, `createMockSprint()`, `createMockTasks()` generate mock data

**localStorage usage:**

- All persistence is in `src/lib/store.ts`
- Keys: `openmesh:projects`, `openmesh:doc-sources`, `openmesh:sprints`, `openmesh:tasks`, `openmesh:recent-items`, `openmesh:agent-sessions`, `openmesh:terminal-presets`, `openmesh:settings`, `openmesh:app-state`

---

## 2. Current Architecture

### Vue App Structure

```
src/
├── main.ts                 # App entry point
├── App.vue                 # Root component with router-view
├── router.ts               # Vue Router setup
├── types.ts                # TypeScript type definitions
├── style.css               # Global styles
├── lib/
│   ├── store.ts            # localStorage persistence layer
│   ├── useStore.ts         # Vue 3 reactive composable
│   └── format.ts           # Utility functions
├── components/
│   ├── Sidebar.vue         # Navigation sidebar
│   ├── DailyUsageChart.vue # Usage visualization
│   ├── ModelPerformanceTable.vue
│   ├── TotalMetrics.vue
│   └── UsageSummary.vue
└── pages/
    ├── HomePage.vue
    ├── AddProjectPage.vue
    ├── DocsPage.vue
    ├── SprintPage.vue
    ├── AgentSessionsPage.vue
    ├── DevConnectorPage.vue
    ├── SettingsPage.vue
    ├── StatusPage.vue
    ├── UsagePage.vue
    ├── ModelsPage.vue
    ├── ServerPage.vue
    └── ProjectsPage.vue
```

### Store Layer (`src/lib/store.ts`)

The store is a plain TypeScript object that wraps localStorage with typed get/set operations.

**Key characteristics:**

- All persistence operations go through `read<T>()` and `write<T>()` helpers
- Uses `KEY_PREFIX = "openmesh:"` for all localStorage keys
- Provides CRUD operations for all entities
- Generates mock data via `initMockSessions()`, `createMockSprint()`, `createMockTasks()`
- Export/import/reset functions for data management
- No Vue reactivity — pure data layer

**Functions exposed:**

- Projects: `getProjects()`, `saveProjects()`, `addProject()`, `getProject()`
- App State: `getAppState()`, `setCurrentProject()`, `getCurrentProjectId()`, `getCurrentProject()`
- Doc Sources: `getDocSources()`, `getDocSourcesForProject()`, `saveDocSources()`, `initDocSourcesForProject()`, `updateDocSource()`
- Sprints: `getSprints()`, `getSprintForProject()`, `saveSprints()`, `createMockSprint()`
- Tasks: `getTasks()`, `getTasksForProject()`, `getTasksForSprint()`, `saveTasks()`, `updateTask()`, `createMockTasks()`
- Recent Items: `getRecentItems()`, `getRecentItemsForProject()`, `addRecentItem()`
- Agent Sessions: `getAgentSessions()`, `getAgentSessionsForProject()`, `saveAgentSessions()`, `deleteAgentSession()`, `updateAgentSession()`, `initMockSessions()`
- Terminal Presets: `getTerminalPresets()`, `getTerminalPresetsForProject()`, `addTerminalPreset()`
- Settings: `getSettings()`, `saveSettings()`, `updateSettings()`
- Utilities: `exportAll()`, `importAll()`, `resetAll()`, `getStorageSize()`

### Composable Layer (`src/lib/useStore.ts`)

Wraps the store with Vue 3 reactivity using `ref()` and `computed()`.

**Key characteristics:**

- Global reactive state shared across all components
- Sync functions update both localStorage and reactive refs
- Actions call store methods, then sync reactive state
- Tracks recent work automatically on certain actions (project selection, doc connection, task status change)
- Exposes both reactive state and actions

**Reactive state:**

- `projects`, `currentProjectId`, `currentProject`
- `docSources`, `projectDocSources`
- `sprints`, `projectSprint`
- `tasks`, `projectTasks`
- `recentItems`
- `agentSessions`, `projectSessions`
- `terminalPresets`, `projectPresets`
- `settings`

**Actions:**

- `selectProject()`, `addProject()`, `updateDocSource()`, `createMockSprint()`, `updateTask()`, `addRecentItem()`, `deleteAgentSession()`, `updateAgentSession()`, `initMockSessions()`, `addTerminalPreset()`, `saveSettings()`, `resetAll()`

### Page Structure

All pages follow a consistent pattern:

1. Import `useStore()` composable
2. Destructure needed state and actions
3. Use computed properties for derived data
4. Render UI with conditional empty states
5. Call actions on user interactions

**Routing:**

- Uses `vue-router` with `createWebHistory()`
- Routes: `/`, `/status`, `/usage`, `/models`, `/server`, `/settings`, `/projects/new`, `/docs`, `/sprint`, `/agent-sessions`, `/dev-connector`
- No route guards or middleware

### Persistence

**Current approach:**

- All data stored in localStorage with `openmesh:` prefix
- JSON serialization for all entities
- Synchronous read/write operations
- No versioning or migration support
- Export/import as JSON file
- Reset clears all `openmesh:*` keys

**localStorage keys:**

- `openmesh:projects` — `Project[]`
- `openmesh:app-state` — `AppState` (currentProjectId)
- `openmesh:doc-sources` — `DocSource[]`
- `openmesh:sprints` — `Sprint[]`
- `openmesh:tasks` — `Task[]`
- `openmesh:recent-items` — `RecentItem[]`
- `openmesh:agent-sessions` — `AgentSession[]`
- `openmesh:terminal-presets` — `TerminalPreset[]`
- `openmesh:settings` — `Settings`

### Mock Data

**Where mocks exist:**

1. **DocsPage.vue** — `connectSource()` uses `prompt()` for path, generates random `fileCount`
2. **HomePage.vue** — `resumeAction()` shows `alert()` with "Would open..." messages
3. **DevConnectorPage.vue** — Terminal Launcher and Git Status sections have "Mock" badges, toast messages
4. **SettingsPage.vue** — `checkHealth()` randomly returns "healthy" or "unreachable"
5. **store.ts** — `initMockSessions()` generates 5 mock agent sessions, `createMockSprint()` creates sprint with 8 tasks

**Mock data is auto-generated:**

- When a project is added: `initDocSourcesForProject()` creates 7 doc sources, `initMockSessions()` creates 5 agent sessions
- When "Use Mock Sprint" is clicked: `createMockSprint()` creates sprint + 8 tasks

### Settings

Settings are stored in `openmesh:settings` with this structure:

```typescript
{
  workspace: { name?, defaultProjectId?, theme },
  provider: { name?, apiKeyConfigured, defaultModel?, fallbackModel?, usageTrackingEnabled },
  models: { codingModel?, researchModel?, summarizationModel?, localModelEnabled },
  server: { mode, apiBaseUrl, healthStatus, syncStatus },
  agentClis: { codexPath?, claudeCodePath?, opencodePath?, axgaPath? },
  localPaths: { defaultProjectsDir?, defaultTerminalDir?, dataStorageDir? },
  appearance: { theme, fontSize }
}
```

**API keys:** Only `apiKeyConfigured: boolean` is stored, never the raw key.

### Recent Work

Recent items are tracked automatically:

- When a project is selected
- When a doc source is connected
- When a task status changes
- When an agent session is selected

**Deduplication:** By `type + sourceId`  
**Pruning:** Max 50 items, sorted by `lastOpenedAt` descending  
**Filtering:** By current project on Home page

### Project Scoping

Most data is scoped to the current project:

- `projectDocSources` — filtered by `currentProjectId`
- `projectSprint` — filtered by `currentProjectId`
- `projectTasks` — filtered by `currentProjectId`
- `projectSessions` — filtered by `currentProjectId`
- `projectPresets` — filtered by `currentProjectId`

When no project is selected, some pages show empty states, others show all data.

---

## 3. Migration Goals

The Tauri v2 desktop app should make the following web-mocked features real:

1. **Native folder picker** — Replace `prompt()` with native dialog
2. **Real path validation** — Check if paths exist before saving
3. **Real local file/folder access** — Read actual doc folders, count files
4. **Real terminal launching** — Open terminal at project path
5. **Real agent CLI launching** — Launch Codex/Claude/OpenCode sessions
6. **Real Codex/Claude/OpenCode session directory reading** — Scan `~/.codex/sessions/`, `~/.claude/projects/`, etc.
7. **Real Git status** — Read `.git` directory, show actual branch/status
8. **SQLite or Tauri storage** — Replace localStorage with desktop persistence
9. **Safe local app-data persistence** — Store data in platform-appropriate locations

**Non-goals:**

- Do not change product behavior or UI
- Do not add new features
- Do not break the current web POC
- Do not implement team/multi-user features
- Do not add cloud sync

---

## 4. Native Capability Map

### Folder Picker

| Aspect | Detail |
|--------|--------|
| **Feature** | Select folder path for project, docs, terminal |
| **Current web behavior** | `prompt()` dialog in DocsPage, typed input in AddProjectPage |
| **Future Tauri behavior** | Native folder picker dialog via `@tauri-apps/plugin-dialog` |
| **Tauri/Rust responsibility** | Open native dialog, return selected path |
| **Frontend responsibility** | Call adapter, update project/doc source with path |
| **Safety notes** | Validate path exists before saving |

### Path Validation

| Aspect | Detail |
|--------|--------|
| **Feature** | Validate folder paths exist |
| **Current web behavior** | No validation, paths are trusted |
| **Future Tauri behavior** | Check path exists and is a directory |
| **Tauri/Rust responsibility** | `std::fs::metadata()`, return exists + is_dir |
| **Frontend responsibility** | Show error if path invalid |
| **Safety notes** | Must validate before any file operations |

### Open Folder

| Aspect | Detail |
|--------|--------|
| **Feature** | Open folder in system file browser |
| **Current web behavior** | `alert("Would open folder: ...")` |
| **Future Tauri behavior** | Open system file browser at path |
| **Tauri/Rust responsibility** | `open::that()` or platform-specific command |
| **Frontend responsibility** | Call adapter with path |
| **Safety notes** | Validate path exists first |

### Open Terminal

| Aspect | Detail |
|--------|--------|
| **Feature** | Open terminal at project path |
| **Current web behavior** | `alert("Would open terminal at: ...")` |
| **Future Tauri behavior** | Launch terminal app at working directory |
| **Tauri/Rust responsibility** | Platform-specific terminal launch (Windows Terminal, iTerm, GNOME Terminal) |
| **Frontend responsibility** | Call adapter with path |
| **Safety notes** | Validate path exists, handle platform differences |

### Run Agent CLI

| Aspect | Detail |
|--------|--------|
| **Feature** | Launch Codex/Claude/OpenCode CLI |
| **Current web behavior** | `alert("Would resume codex session")` |
| **Future Tauri behavior** | Spawn agent CLI process in terminal |
| **Tauri/Rust responsibility** | Validate CLI path exists, spawn process with correct args |
| **Frontend responsibility** | Call adapter with CLI path and working directory |
| **Safety notes** | **Critical:** Validate CLI path, never pass arbitrary user input as args, use allowlist of known CLIs |

### Read Agent Session Directory

| Aspect | Detail |
|--------|--------|
| **Feature** | Scan `~/.codex/sessions/`, `~/.claude/projects/`, etc. |
| **Current web behavior** | Mock sessions generated by `initMockSessions()` |
| **Future Tauri behavior** | Read session directories, parse metadata, return session list |
| **Tauri/Rust responsibility** | Scan directories, parse session files, extract metadata |
| **Frontend responsibility** | Display session list, allow selection |
| **Safety notes** | Only read known session directories, never modify session files |

### Read Docs Folders

| Aspect | Detail |
|--------|--------|
| **Feature** | Read connected doc folders, count files |
| **Current web behavior** | Random file count `Math.floor(Math.random() * 15) + 3` |
| **Future Tauri behavior** | Read directory, count files, return file list |
| **Tauri/Rust responsibility** | `std::fs::read_dir()`, count files, return metadata |
| **Frontend responsibility** | Display file count, show file list |
| **Safety notes** | Only read within connected paths, validate path is within project |

### Get Git Status

| Aspect | Detail |
|--------|--------|
| **Feature** | Show branch, status, last commit |
| **Current web behavior** | Mock: "Clean", "a1b2c3d — Initial commit" |
| **Future Tauri behavior** | Read `.git` directory, parse HEAD, status, log |
| **Tauri/Rust responsibility** | Use `git2` crate or shell out to `git` command |
| **Frontend responsibility** | Display git status |
| **Safety notes** | Only read git data, never modify |

### Local Persistence

| Aspect | Detail |
|--------|--------|
| **Feature** | Store app data locally |
| **Current web behavior** | localStorage with `openmesh:` prefix |
| **Future Tauri behavior** | SQLite database or JSON files in app data directory |
| **Tauri/Rust responsibility** | Manage database/files, provide CRUD API |
| **Frontend responsibility** | Call storage adapter |
| **Safety notes** | Encrypt sensitive data (API keys), handle migration from localStorage |

### Import/Export

| Aspect | Detail |
|--------|--------|
| **Feature** | Export/import all data as JSON |
| **Current web behavior** | Download/upload JSON file |
| **Future Tauri behavior** | Same, but via native file dialog |
| **Tauri/Rust responsibility** | Open save/open dialog, write/read file |
| **Frontend responsibility** | Call adapter, handle result |
| **Safety notes** | Validate JSON structure on import |

### Settings Storage

| Aspect | Detail |
|--------|--------|
| **Feature** | Store settings |
| **Current web behavior** | localStorage |
| **Future Tauri behavior** | SQLite or JSON in app data directory |
| **Tauri/Rust responsibility** | Read/write settings file |
| **Frontend responsibility** | Call storage adapter |
| **Safety notes** | Never store raw API keys, only "configured" boolean |

---

## 5. Adapter Boundary Plan

**Core principle:** No Vue page/component should call `invoke()` directly. All native calls go through adapter/service layers.

### Proposed Adapter Modules

```
src/lib/adapters/
├── storageAdapter.ts       # Persistence (localStorage now, SQLite later)
├── fileSystemAdapter.ts    # Folder picker, path validation, file reading
├── terminalAdapter.ts      # Terminal launching
├── agentSessionAdapter.ts  # Agent CLI launching, session directory reading
├── gitAdapter.ts           # Git status
└── dialogAdapter.ts        # Native dialogs (alerts, confirms)
```

### storageAdapter.ts

**Purpose:** Abstract persistence layer so Vue components don't know if data is in localStorage or SQLite.

**Current web/mock implementation:**

```typescript
export const storageAdapter = {
  getProjects(): Project[] { return store.getProjects(); },
  saveProjects(projects: Project[]): void { store.saveProjects(projects); },
  // ... all other store methods
};
```

**Future Tauri implementation:**

```typescript
export const storageAdapter = {
  async getProjects(): Promise<Project[]> {
    return await invoke('get_projects');
  },
  async saveProjects(projects: Project[]): Promise<void> {
    await invoke('save_projects', { projects });
  },
  // ... all other methods as async
};
```

**Functions exposed:**

- All CRUD operations for all entities
- `exportAll()`, `importAll()`, `resetAll()`

**Data types used:**

- All types from `src/types.ts`

**Error handling:**

- Return `Result<T, Error>` or throw typed errors
- Frontend catches and shows user-friendly messages

### fileSystemAdapter.ts

**Purpose:** Abstract file system operations (folder picker, path validation, file reading).

**Current web/mock implementation:**

```typescript
export const fileSystemAdapter = {
  async pickFolder(): Promise<string | null> {
    return prompt("Enter folder path:");
  },
  async validatePath(path: string): Promise<{ exists: boolean; isDir: boolean }> {
    return { exists: true, isDir: true }; // Mock: always valid
  },
  async readDir(path: string): Promise<{ name: string; isDir: boolean }[]> {
    return []; // Mock: empty
  },
  async countFiles(path: string): Promise<number> {
    return Math.floor(Math.random() * 15) + 3; // Mock: random
  },
};
```

**Future Tauri implementation:**

```typescript
export const fileSystemAdapter = {
  async pickFolder(): Promise<string | null> {
    return await invoke('pick_folder');
  },
  async validatePath(path: string): Promise<{ exists: boolean; isDir: boolean }> {
    return await invoke('validate_path', { path });
  },
  async readDir(path: string): Promise<{ name: string; isDir: boolean }[]> {
    return await invoke('read_dir', { path });
  },
  async countFiles(path: string): Promise<number> {
    const files = await invoke('read_dir', { path });
    return files.filter(f => !f.isDir).length;
  },
};
```

**Functions exposed:**

- `pickFolder(): Promise<string | null>`
- `validatePath(path: string): Promise<{ exists: boolean; isDir: boolean }>`
- `readDir(path: string): Promise<FileEntry[]>`
- `countFiles(path: string): Promise<number>`
- `openFolder(path: string): Promise<void>`

**Data types used:**

```typescript
type FileEntry = { name: string; path: string; isDir: boolean; size?: number };
```

**Error handling:**

- Return `null` for cancelled dialogs
- Throw `PathNotFoundError`, `PermissionDeniedError` for file errors

### terminalAdapter.ts

**Purpose:** Abstract terminal launching.

**Current web/mock implementation:**

```typescript
export const terminalAdapter = {
  async openTerminal(workingDir: string): Promise<void> {
    alert(`Would open terminal at: ${workingDir}`);
  },
};
```

**Future Tauri implementation:**

```typescript
export const terminalAdapter = {
  async openTerminal(workingDir: string): Promise<void> {
    await invoke('open_terminal', { workingDir });
  },
};
```

**Functions exposed:**

- `openTerminal(workingDir: string): Promise<void>`

**Data types used:**

- None (just path string)

**Error handling:**

- Throw `PathNotFoundError` if working dir doesn't exist
- Throw `TerminalLaunchError` if terminal can't be opened

### agentSessionAdapter.ts

**Purpose:** Abstract agent CLI launching and session directory reading.

**Current web/mock implementation:**

```typescript
export const agentSessionAdapter = {
  async launchAgentCli(cliPath: string, workingDir: string): Promise<void> {
    alert(`Would launch ${cliPath} at ${workingDir}`);
  },
  async listAgentSessions(tool: string): Promise<AgentSession[]> {
    return store.initMockSessions('mock-project-id'); // Mock
  },
};
```

**Future Tauri implementation:**

```typescript
export const agentSessionAdapter = {
  async launchAgentCli(cliPath: string, workingDir: string): Promise<void> {
    await invoke('launch_agent_cli', { cliPath, workingDir });
  },
  async listAgentSessions(tool: string): Promise<AgentSession[]> {
    return await invoke('list_agent_sessions', { tool });
  },
};
```

**Functions exposed:**

- `launchAgentCli(cliPath: string, workingDir: string): Promise<void>`
- `listAgentSessions(tool: string): Promise<AgentSession[]>`
- `readSessionSummary(sessionPath: string): Promise<string>`

**Data types used:**

- `AgentSession` from `src/types.ts`

**Error handling:**

- Throw `CliNotFoundError` if CLI path doesn't exist
- Throw `SessionNotFoundError` if session path invalid
- **Security:** Validate CLI path is in allowlist (codex, claude, opencode)

### gitAdapter.ts

**Purpose:** Abstract git status reading.

**Current web/mock implementation:**

```typescript
export const gitAdapter = {
  async getGitStatus(repoPath: string): Promise<GitStatus> {
    return {
      branch: 'main',
      isClean: true,
      modifiedFiles: 0,
      untrackedFiles: 0,
      lastCommitHash: 'a1b2c3d',
      lastCommitMessage: 'Initial commit',
    }; // Mock
  },
};
```

**Future Tauri implementation:**

```typescript
export const gitAdapter = {
  async getGitStatus(repoPath: string): Promise<GitStatus> {
    return await invoke('get_git_status', { repoPath });
  },
};
```

**Functions exposed:**

- `getGitStatus(repoPath: string): Promise<GitStatus>`

**Data types used:**

```typescript
type GitStatus = {
  branch: string;
  isClean: boolean;
  modifiedFiles: number;
  untrackedFiles: number;
  lastCommitHash: string;
  lastCommitMessage: string;
};
```

**Error handling:**

- Throw `NotAGitRepoError` if path is not a git repo
- Throw `GitError` for other git errors

### dialogAdapter.ts

**Purpose:** Abstract native dialogs (alerts, confirms, prompts).

**Current web/mock implementation:**

```typescript
export const dialogAdapter = {
  alert(message: string): void {
    window.alert(message);
  },
  confirm(message: string): boolean {
    return window.confirm(message);
  },
  prompt(message: string): string | null {
    return window.prompt(message);
  },
};
```

**Future Tauri implementation:**

```typescript
export const dialogAdapter = {
  async alert(message: string): Promise<void> {
    await invoke('show_alert', { message });
  },
  async confirm(message: string): Promise<boolean> {
    return await invoke('show_confirm', { message });
  },
};
```

**Functions exposed:**

- `alert(message: string): Promise<void>`
- `confirm(message: string): Promise<boolean>`

**Data types used:**

- None (just strings)

**Error handling:**

- None (dialogs don't fail)

### Adapter Usage in Pages

**Before (direct mock calls):**

```typescript
// DocsPage.vue
function connectSource(sourceId: string) {
  const path = prompt("Enter folder path for this doc source:");
  if (path) {
    updateDocSource(sourceId, { isConnected: true, connectedPath: path, fileCount: Math.floor(Math.random() * 15) + 3 });
  }
}
```

**After (adapter calls):**

```typescript
// DocsPage.vue
import { fileSystemAdapter } from '../lib/adapters/fileSystemAdapter';

async function connectSource(sourceId: string) {
  const path = await fileSystemAdapter.pickFolder();
  if (path) {
    const validation = await fileSystemAdapter.validatePath(path);
    if (!validation.exists || !validation.isDir) {
      await dialogAdapter.alert("Invalid path");
      return;
    }
    const fileCount = await fileSystemAdapter.countFiles(path);
    updateDocSource(sourceId, { isConnected: true, connectedPath: path, fileCount });
  }
}
```

---

## 6. Tauri Setup Plan

### Expected `src-tauri` Structure

```
src-tauri/
├── Cargo.toml
├── tauri.conf.json
├── build.rs
├── capabilities/
│   └── default.json
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── file_system.rs
│   │   ├── terminal.rs
│   │   ├── agent_sessions.rs
│   │   ├── git.rs
│   │   └── storage.rs
│   └── error.rs
└── icons/
    ├── 32x32.png
    ├── 128x128.png
    ├── 128x128@2x.png
    ├── icon.icns
    └── icon.ico
```

### Expected Config Files

**`tauri.conf.json`:**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Openmesh",
  "version": "0.3.0",
  "identifier": "com.openmesh.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:3000",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": false,
    "windows": [
      {
        "title": "Openmesh",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

**`Cargo.toml`:**

```toml
[package]
name = "openmesh"
version = "0.3.0"
description = "Personal workbench for resuming work"
authors = ["You"]
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
open = "5"
git2 = "0.19"
```

### Expected Package Scripts

**Add to `package.json`:**

```json
{
  "scripts": {
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build"
  }
}
```

### Expected Dev/Build Commands

**Development:**

```bash
npm run tauri:dev
```

**Production build:**

```bash
npm run tauri:build
```

### Required Dependencies

**npm:**

```bash
npm install --save-dev @tauri-apps/cli@^2
npm install @tauri-apps/api@^2
npm install @tauri-apps/plugin-dialog@^2
npm install @tauri-apps/plugin-fs@^2
npm install @tauri-apps/plugin-shell@^2
```

**Cargo:**

- `tauri` v2
- `tauri-plugin-dialog` v2
- `tauri-plugin-fs` v2
- `tauri-plugin-shell` v2
- `open` (cross-platform folder opening)
- `git2` (git operations)

### Static Frontend Output Requirements

**Vite config must output static files:**

```typescript
// vite.config.ts
export default defineConfig({
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
  },
});
```

**Current Vite config already does this.** No changes needed.

### What Must Be Verified Before Setup

1. ✅ TypeScript passes (`npm run build` succeeds)
2. ✅ Current web POC runs without errors
3. ✅ All localStorage operations work
4. ✅ No direct `invoke()` calls in Vue components
5. ✅ Adapter boundary plan is clear
6. ⚠️ Tauri CLI v2 is installed (`npm list -g @tauri-apps/cli`)
7. ⚠️ Rust toolchain is installed (`rustc --version`)
8. ⚠️ Platform-specific build tools are installed (Windows: Visual Studio Build Tools, macOS: Xcode, Linux: WebKitGTK)

---

## 7. Permission and Capability Plan

**Important:** Do not invent permission names. Run `pnpm tauri permission ls` before adding permissions.

### Filesystem Access

**Required permissions:**

- `fs:allow-read-dir` — Read directory contents
- `fs:allow-read-file` — Read file contents (for session summaries)
- `fs:allow-exists` — Check if path exists
- `fs:scope-app-data` — Allow access to app data directory

**Capability definition:**

```json
{
  "identifier": "fs:read",
  "description": "Read filesystem for docs, sessions, git",
  "windows": ["main"],
  "permissions": [
    "fs:allow-read-dir",
    "fs:allow-read-file",
    "fs:allow-exists"
  ]
}
```

**Safety notes:**

- Only read within project paths and known session directories
- Never write to project paths without explicit user action
- Scope filesystem access to specific directories

### Dialog/Folder Picker

**Required permissions:**

- `dialog:allow-open` — Open file/folder picker

**Capability definition:**

```json
{
  "identifier": "dialog:open",
  "description": "Open folder picker",
  "windows": ["main"],
  "permissions": ["dialog:allow-open"]
}
```

**Safety notes:**

- Folder picker is safe (user explicitly chooses)
- Never auto-open dialogs without user action

### Shell/Terminal Behavior

**Required permissions:**

- `shell:allow-open` — Open URL/folder in system handler
- `shell:allow-spawn` — Spawn process (for terminal, agent CLI)

**Capability definition:**

```json
{
  "identifier": "shell:terminal",
  "description": "Open terminal and launch agent CLI",
  "windows": ["main"],
  "permissions": [
    "shell:allow-open",
    "shell:allow-spawn"
  ]
}
```

**Safety notes:**

- **Critical:** Validate all paths before spawning
- Never pass arbitrary user input as command args
- Use allowlist of known agent CLIs (codex, claude, opencode)
- Restrict spawn to specific binaries

### Sidecar/Agent CLI Execution

**Required permissions:**

- `shell:allow-spawn` with sidecar config

**Capability definition:**

```json
{
  "identifier": "shell:agent-cli",
  "description": "Launch agent CLI tools",
  "windows": ["main"],
  "permissions": [
    {
      "identifier": "shell:allow-spawn",
      "allow": [
        {
          "name": "codex",
          "sidecar": false,
          "args": true
        },
        {
          "name": "claude",
          "sidecar": false,
          "args": true
        },
        {
          "name": "opencode",
          "sidecar": false,
          "args": true
        }
      ]
    }
  ]
}
```

**Safety notes:**

- Validate CLI path exists before spawning
- Never pass raw user input as args
- Use static or regex-validated args
- Sensitive commands must self-validate in Rust

### Storage/SQL

**Required permissions:**

- None (storage is internal to Rust backend)

**Implementation:**

- Use `tauri-plugin-store` for simple key-value storage
- Or use SQLite via `rusqlite` crate
- Or use JSON files in app data directory

**Safety notes:**

- Never store raw API keys
- Encrypt sensitive data if needed
- Handle migration from localStorage

### Opener (if needed)

**Required permissions:**

- `opener:allow-open-url` — Open URL in browser
- `opener:allow-open-path` — Open path in system handler

**Capability definition:**

```json
{
  "identifier": "opener:open",
  "description": "Open URLs and paths",
  "windows": ["main"],
  "permissions": [
    "opener:allow-open-url",
    "opener:allow-open-path"
  ]
}
```

**Safety notes:**

- Validate URLs before opening
- Only open paths within project directories

---

## 8. Rust Command Plan

### validate_path

**Input:**

```rust
#[derive(Deserialize)]
struct ValidatePathArgs {
    path: String,
}
```

**Output:**

```rust
#[derive(Serialize)]
struct PathValidation {
    exists: bool,
    is_dir: bool,
    is_readable: bool,
}
```

**Validation rules:**

- Path must be non-empty
- Normalize path (remove trailing slashes, resolve `.` and `..`)
- Check if path exists
- Check if path is a directory
- Check if path is readable

**Security concerns:**

- Prevent path traversal attacks
- Validate path is within allowed directories
- Never follow symlinks outside allowed directories

**Async:** No (fast operation)

**Frontend adapter:**

```typescript
async function validatePath(path: string): Promise<PathValidation> {
  return await invoke('validate_path', { path });
}
```

### open_folder

**Input:**

```rust
#[derive(Deserialize)]
struct OpenFolderArgs {
    path: String,
}
```

**Output:**

```rust
// None (returns void)
```

**Validation rules:**

- Path must exist
- Path must be a directory
- Path must be readable

**Security concerns:**

- Validate path before opening
- Use `open::that()` for cross-platform support

**Async:** No (fast operation)

**Frontend adapter:**

```typescript
async function openFolder(path: string): Promise<void> {
  await invoke('open_folder', { path });
}
```

### pick_folder

**Input:**

```rust
// None (opens dialog)
```

**Output:**

```rust
#[derive(Serialize)]
struct PickFolderResult {
    path: Option<String>, // None if cancelled
}
```

**Validation rules:**

- None (user chooses path)

**Security concerns:**

- None (user explicitly chooses)

**Async:** Yes (dialog is async)

**Frontend adapter:**

```typescript
async function pickFolder(): Promise<string | null> {
  const result = await invoke('pick_folder');
  return result.path;
}
```

### get_git_status

**Input:**

```rust
#[derive(Deserialize)]
struct GetGitStatusArgs {
    repo_path: String,
}
```

**Output:**

```rust
#[derive(Serialize)]
struct GitStatus {
    branch: String,
    is_clean: bool,
    modified_files: u32,
    untracked_files: u32,
    last_commit_hash: String,
    last_commit_message: String,
}
```

**Validation rules:**

- Path must exist
- Path must be a git repository (contains `.git` directory)

**Security concerns:**

- Only read git data, never modify
- Use `git2` crate for safe git operations

**Async:** Yes (git operations can be slow)

**Frontend adapter:**

```typescript
async function getGitStatus(repoPath: string): Promise<GitStatus> {
  return await invoke('get_git_status', { repoPath });
}
```

### list_agent_sessions

**Input:**

```rust
#[derive(Deserialize)]
struct ListAgentSessionsArgs {
    tool: String, // "codex", "claude-code", "opencode"
}
```

**Output:**

```rust
#[derive(Serialize)]
struct AgentSession {
    id: String,
    tool: String,
    title: String,
    project_id: Option<String>,
    source_path: Option<String>,
    status: String,
    summary: Option<String>,
    started_at: String,
    last_active_at: String,
    ended_at: Option<String>,
    changed_files: Option<Vec<String>>,
    linked_task_id: Option<String>,
    is_important: bool,
    created_at: String,
    updated_at: String,
}
```

**Validation rules:**

- Tool must be in allowlist: "codex", "claude-code", "opencode"
- Session directory must exist

**Security concerns:**

- Only read known session directories:
  - Codex: `~/.codex/sessions/`
  - Claude Code: `~/.claude/projects/`
  - OpenCode: `~/.opencode/sessions/`
- Never modify session files
- Parse session files safely (handle malformed data)

**Async:** Yes (file I/O)

**Frontend adapter:**

```typescript
async function listAgentSessions(tool: string): Promise<AgentSession[]> {
  return await invoke('list_agent_sessions', { tool });
}
```

### read_agent_session_summary

**Input:**

```rust
#[derive(Deserialize)]
struct ReadAgentSessionSummaryArgs {
    session_path: String,
}
```

**Output:**

```rust
#[derive(Serialize)]
struct SessionSummary {
    summary: String,
}
```

**Validation rules:**

- Path must exist
- Path must be within known session directories
- Path must be a file

**Security concerns:**

- Validate path is within allowed directories
- Read file safely (handle encoding errors)
- Limit file size (prevent DoS)

**Async:** Yes (file I/O)

**Frontend adapter:**

```typescript
async function readAgentSessionSummary(sessionPath: string): Promise<string> {
  const result = await invoke('read_agent_session_summary', { sessionPath });
  return result.summary;
}
```

### open_terminal

**Input:**

```rust
#[derive(Deserialize)]
struct OpenTerminalArgs {
    working_dir: String,
}
```

**Output:**

```rust
// None (returns void)
```

**Validation rules:**

- Working directory must exist
- Working directory must be a directory

**Security concerns:**

- Platform-specific terminal launch:
  - Windows: `wt.exe` or `cmd.exe`
  - macOS: `open -a Terminal`
  - Linux: `x-terminal-emulator` or `gnome-terminal`
- Validate working directory before launching

**Async:** Yes (process spawn)

**Frontend adapter:**

```typescript
async function openTerminal(workingDir: string): Promise<void> {
  await invoke('open_terminal', { workingDir });
}
```

### launch_agent_cli

**Input:**

```rust
#[derive(Deserialize)]
struct LaunchAgentCliArgs {
    cli_path: String,
    working_dir: String,
    args: Option<Vec<String>>,
}
```

**Output:**

```rust
// None (returns void)
```

**Validation rules:**

- CLI path must exist
- CLI path must be executable
- CLI path must be in allowlist (codex, claude, opencode)
- Working directory must exist
- Args must be validated (no shell injection)

**Security concerns:**

- **Critical:** Validate CLI path is in allowlist
- Never pass arbitrary user input as args
- Use `std::process::Command` with explicit args array
- Validate working directory
- Handle process spawn errors

**Async:** Yes (process spawn)

**Frontend adapter:**

```typescript
async function launchAgentCli(cliPath: string, workingDir: string, args?: string[]): Promise<void> {
  await invoke('launch_agent_cli', { cliPath, workingDir, args });
}
```

### get_app_data_status

**Input:**

```rust
// None
```

**Output:**

```rust
#[derive(Serialize)]
struct AppDataStatus {
    storage_type: String, // "localStorage", "sqlite", "json"
    storage_path: Option<String>,
    storage_size: u64,
    version: String,
}
```

**Validation rules:**

- None (read-only)

**Security concerns:**

- None (read-only)

**Async:** No (fast operation)

**Frontend adapter:**

```typescript
async function getAppDataStatus(): Promise<AppDataStatus> {
  return await invoke('get_app_data_status');
}
```

---

## 9. Storage Migration Plan

### Current localStorage Keys

- `openmesh:projects` — `Project[]`
- `openmesh:app-state` — `AppState`
- `openmesh:doc-sources` — `DocSource[]`
- `openmesh:sprints` — `Sprint[]`
- `openmesh:tasks` — `Task[]`
- `openmesh:recent-items` — `RecentItem[]`
- `openmesh:agent-sessions` — `AgentSession[]`
- `openmesh:terminal-presets` — `TerminalPreset[]`
- `openmesh:settings` — `Settings`

### Export/Import Compatibility

**Current export format:**

```json
{
  "openmesh:projects": [...],
  "openmesh:doc-sources": [...],
  ...
}
```

**Future import must support this format** for backward compatibility.

### App State Versioning

**Add version field to exported data:**

```json
{
  "version": "1.0.0",
  "openmesh:projects": [...],
  ...
}
```

**Migration strategy:**

- On import, check version
- If version < current, run migration functions
- If version > current, show error

### Migration Strategy

**Phase 1: Keep localStorage**

- Web POC continues using localStorage
- Tauri app can also use localStorage initially (via WebView)

**Phase 2: Add SQLite adapter**

- Implement `storageAdapter` with SQLite backend
- Migrate data on first Tauri launch:
  1. Check if SQLite database exists
  2. If not, read localStorage
  3. Write to SQLite
  4. Clear localStorage

**Phase 3: Remove localStorage**

- After migration is stable, remove localStorage code
- All operations go through SQLite

### Storage Options for v0

**Option 1: SQLite (recommended)**

- Pros: Robust, queryable, handles large data
- Cons: Requires `rusqlite` crate, more complex
- Use for: All entities

**Option 2: Tauri Store Plugin**

- Pros: Simple key-value API, built-in
- Cons: Not queryable, limited to JSON-serializable data
- Use for: Settings, app state

**Option 3: JSON Files**

- Pros: Simple, human-readable, easy to debug
- Cons: Not atomic, can corrupt on crash
- Use for: Export/import only

**Recommendation:** Use SQLite for all entities, Tauri Store for settings.

### What Should Not Be Stored Insecurely

**API keys:**

- Never store raw API keys in localStorage or SQLite
- Only store `apiKeyConfigured: boolean`
- If API key is needed, use system keychain (via `keyring` crate)

**Sensitive data:**

- Project paths (not sensitive, but private)
- Session data (may contain sensitive context)
- Recent work (may reveal work patterns)

**Recommendation:**

- Encrypt SQLite database if needed
- Use platform-appropriate storage locations
- Never sync sensitive data to cloud

---

## 10. Incremental Implementation Phases

### Phase 1: Add Tauri Shell

**Goal:** Run existing Vue app in Tauri window without changing behavior.

**Files likely changed:**

- `package.json` — Add Tauri dependencies
- `src-tauri/` — Create Tauri project structure
- `vite.config.ts` — Ensure static output

**Commands to run:**

```bash
npm install --save-dev @tauri-apps/cli@^2
npm install @tauri-apps/api@^2
npm run tauri init
npm run tauri:dev
```

**Pass criteria:**

- ✅ Tauri window opens
- ✅ Vue app loads
- ✅ All pages work
- ✅ localStorage persists
- ✅ No console errors

**Rollback notes:**

- Remove `src-tauri/` directory
- Remove Tauri dependencies from `package.json`

### Phase 2: Add Adapter Boundaries

**Goal:** Introduce adapter layer while still using mock/web implementations.

**Files likely changed:**

- `src/lib/adapters/` — Create adapter modules
- `src/pages/*.vue` — Replace direct mock calls with adapter calls

**Commands to run:**

```bash
npm run build
```

**Pass criteria:**

- ✅ TypeScript passes
- ✅ All pages work
- ✅ Adapters are called instead of direct mocks
- ✅ No `invoke()` calls in Vue components

**Rollback notes:**

- Revert adapter changes
- Restore direct mock calls

### Phase 3: Implement Native Folder Picker

**Goal:** Replace `prompt()` with native folder picker.

**Files likely changed:**

- `src-tauri/src/commands/file_system.rs` — Implement `pick_folder`
- `src/lib/adapters/fileSystemAdapter.ts` — Call `invoke('pick_folder')`
- `src/pages/DocsPage.vue` — Use adapter
- `src/pages/AddProjectPage.vue` — Add folder picker button

**Commands to run:**

```bash
npm run tauri:dev
```

**Pass criteria:**

- ✅ Native folder picker opens
- ✅ Selected path is returned
- ✅ Path is validated
- ✅ Web POC still works (adapter falls back to `prompt()`)

**Rollback notes:**

- Revert to `prompt()` in adapters

### Phase 4: Implement Real Open Folder and Terminal Launch

**Goal:** Open folder in system file browser, launch terminal.

**Files likely changed:**

- `src-tauri/src/commands/file_system.rs` — Implement `open_folder`
- `src-tauri/src/commands/terminal.rs` — Implement `open_terminal`
- `src/lib/adapters/fileSystemAdapter.ts` — Call `invoke('open_folder')`
- `src/lib/adapters/terminalAdapter.ts` — Call `invoke('open_terminal')`
- `src/pages/HomePage.vue` — Use adapters

**Commands to run:**

```bash
npm run tauri:dev
```

**Pass criteria:**

- ✅ Folder opens in system file browser
- ✅ Terminal launches at correct path
- ✅ Works on Windows, macOS, Linux
- ✅ Web POC still works (adapter shows alert)

**Rollback notes:**

- Revert to `alert()` in adapters

### Phase 5: Implement Real Agent Session Directory Scanning

**Goal:** Read real Codex/Claude/OpenCode session directories.

**Files likely changed:**

- `src-tauri/src/commands/agent_sessions.rs` — Implement `list_agent_sessions`
- `src/lib/adapters/agentSessionAdapter.ts` — Call `invoke('list_agent_sessions')`
- `src/pages/AgentSessionsPage.vue` — Use adapter

**Commands to run:**

```bash
npm run tauri:dev
```

**Pass criteria:**

- ✅ Real sessions are listed
- ✅ Session metadata is parsed correctly
- ✅ Works for Codex, Claude Code, OpenCode
- ✅ Web POC still works (adapter returns mock sessions)

**Rollback notes:**

- Revert to mock sessions in adapter

### Phase 6: Implement Real Git Status

**Goal:** Read real git status from `.git` directory.

**Files likely changed:**

- `src-tauri/src/commands/git.rs` — Implement `get_git_status`
- `src/lib/adapters/gitAdapter.ts` — Call `invoke('get_git_status')`
- `src/pages/DevConnectorPage.vue` — Use adapter

**Commands to run:**

```bash
npm run tauri:dev
```

**Pass criteria:**

- ✅ Real git status is shown
- ✅ Branch, status, last commit are correct
- ✅ Handles non-git directories gracefully
- ✅ Web POC still works (adapter returns mock status)

**Rollback notes:**

- Revert to mock git status in adapter

### Phase 7: Migrate Storage from localStorage to Desktop Storage

**Goal:** Move from localStorage to SQLite or Tauri Store.

**Files likely changed:**

- `src-tauri/src/commands/storage.rs` — Implement storage commands
- `src/lib/adapters/storageAdapter.ts` — Call `invoke()` for all operations
- `src/lib/store.ts` — Remove or deprecate

**Commands to run:**

```bash
npm run tauri:dev
```

**Pass criteria:**

- ✅ All data is stored in SQLite/Tauri Store
- ✅ Data persists across app restarts
- ✅ Import/export still works
- ✅ Migration from localStorage works
- ✅ Web POC still works (adapter falls back to localStorage)

**Rollback notes:**

- Revert to localStorage in adapter

### Phase 8: Add Packaging/Build Checks

**Goal:** Build production app for distribution.

**Files likely changed:**

- `src-tauri/tauri.conf.json` — Configure bundle
- `src-tauri/icons/` — Add app icons

**Commands to run:**

```bash
npm run tauri:build
```

**Pass criteria:**

- ✅ Production build succeeds
- ✅ Installer is created (`.msi`, `.dmg`, `.AppImage`)
- ✅ App installs and runs
- ✅ All features work in production build

**Rollback notes:**

- Remove `src-tauri/` directory
- Revert to web-only distribution

---

## 11. Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| **Breaking current web POC** | High | Medium | Use adapter pattern, keep web fallbacks, test both modes |
| **Tauri API mixed directly into components** | High | High | Enforce adapter boundary, code review, lint rules |
| **Broad filesystem permissions** | High | Medium | Scope permissions to specific directories, validate paths |
| **Arbitrary shell execution** | Critical | Medium | Allowlist of CLIs, validate args, never pass raw user input |
| **Unsafe agent CLI launch** | Critical | Medium | Validate CLI path, use allowlist, self-validate in Rust |
| **Secrets in localStorage** | High | Low | Only store `apiKeyConfigured` boolean, use system keychain for real keys |
| **Platform-specific terminal behavior** | Medium | High | Test on Windows, macOS, Linux; use `open` crate for cross-platform |
| **Windows path issues** | Medium | High | Normalize paths, handle `\` vs `/`, test on Windows |
| **Packaging differences between dev and release** | Medium | Medium | Test both `tauri dev` and `tauri build`, handle environment differences |
| **Mock behavior not clearly labeled** | Medium | Low | Add "Mock" badges, use toast messages, document mocks |
| **localStorage migration data loss** | High | Low | Test migration thoroughly, keep localStorage as fallback, export before migration |
| **Git operations slow on large repos** | Medium | Medium | Make git operations async, show loading state, cache results |
| **Session directory parsing errors** | Medium | Medium | Handle malformed session files, log errors, show user-friendly messages |
| **Tauri v2 API changes** | Medium | Low | Pin Tauri version, follow changelog, test after updates |

---

## 12. Acceptance Criteria Before Implementation

The repo is ready for Tauri implementation only if:

- ✅ TypeScript passes (`npm run build` succeeds)
- ✅ Current web POC still runs (`npm run dev` works)
- ✅ Migration spec is written (this document)
- ✅ Adapter boundary plan is clear (Section 5)
- ✅ Native capability map is clear (Section 4)
- ✅ Security risks are listed (Section 11)
- ✅ No Tauri v1 allowlist patterns are planned (using v2 capabilities)
- ✅ No permission names are guessed (will run `pnpm tauri permission ls` before adding)
- ✅ Implementation can be done phase by phase (Section 10)

**Pre-implementation checklist:**

- [ ] Tauri CLI v2 is installed
- [ ] Rust toolchain is installed
- [ ] Platform-specific build tools are installed
- [ ] All adapters are implemented (Phase 2)
- [ ] All adapters have web fallbacks
- [ ] Code review confirms no direct `invoke()` calls in Vue components

---

## Final Report

### Files Inspected

- `package.json` — npm package manager, Vue 3 + Vite setup
- `vite.config.ts` — Vite configuration with Vue and Tailwind plugins
- `src/main.ts` — App entry point
- `src/router.ts` — Vue Router setup with 11 routes
- `src/types.ts` — TypeScript types for all entities
- `src/lib/store.ts` — localStorage persistence layer (all CRUD operations)
- `src/lib/useStore.ts` — Vue 3 reactive composable
- `src/pages/*.vue` — All 11 pages inspected
- `src/components/Sidebar.vue` — Navigation sidebar

### Current Architecture Summary

**Stack:**

- Vue 3 + Vite + TypeScript
- vue-router for navigation
- localStorage for all persistence
- Tailwind CSS v4 for styling
- npm as package manager

**State flow:**

1. Vue components call `useStore()` composable
2. Composable wraps `store.ts` with Vue reactivity
3. Store reads/writes to localStorage
4. All persistence operations go through `store.ts`

**Mock behaviors:**

- DocsPage: `prompt()` for path input, random file counts
- HomePage: `alert()` for "Would open..." messages
- DevConnectorPage: toast messages, "Mock" badges
- SettingsPage: mock health check
- store.ts: `initMockSessions()`, `createMockSprint()`, `createMockTasks()`

**localStorage usage:**

- All in `src/lib/store.ts`
- 9 keys with `openmesh:` prefix
- JSON serialization for all entities

### Spec File Created

✅ `docs/tauri-migration-spec.md` — Comprehensive migration specification

### Biggest Migration Risks

1. **Tauri API mixed directly into components** — High likelihood, high impact. Mitigation: Enforce adapter boundary.
2. **Arbitrary shell execution** — Medium likelihood, critical impact. Mitigation: Allowlist CLIs, validate args.
3. **Platform-specific terminal behavior** — High likelihood, medium impact. Mitigation: Test on all platforms, use `open` crate.
4. **Windows path issues** — High likelihood, medium impact. Mitigation: Normalize paths, test on Windows.
5. **localStorage migration data loss** — Low likelihood, high impact. Mitigation: Test migration, keep fallback.

### Recommended First Implementation Phase

**Phase 1: Add Tauri Shell**

This is the safest starting point:

- No product behavior changes
- No adapter changes
- Just wrap existing Vue app in Tauri window
- Validates Tauri setup without risk

**Commands:**

```bash
npm install --save-dev @tauri-apps/cli@^2
npm install @tauri-apps/api@^2
npm run tauri init
npm run tauri:dev
```

**Pass criteria:**

- Tauri window opens
- Vue app loads
- All pages work
- localStorage persists

### Is It Safe to Proceed to Tauri Implementation?

**Yes, with caveats:**

✅ **Safe to proceed if:**

- Adapter boundary is enforced (Phase 2 before any native calls)
- All adapters have web fallbacks
- Code review confirms no direct `invoke()` in Vue components
- Testing on all platforms (Windows, macOS, Linux)
- Security review of all Rust commands

⚠️ **Not safe if:**

- Skipping adapter phase
- Mixing Tauri API into Vue components
- Not validating paths in Rust
- Not testing on all platforms
- Not reviewing security of shell execution

**Recommendation:**

1. Complete Phase 1 (Tauri shell) — 1 day
2. Complete Phase 2 (adapter boundaries) — 2 days
3. Review adapters before proceeding
4. Then proceed to Phase 3+ (native features)

**Estimated total migration time:** 2-3 weeks for all 8 phases.

---

*End of Tauri Migration Spec*
