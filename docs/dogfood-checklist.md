# Openmesh v0.2 Dogfood Checklist

**Purpose:** Verify Openmesh is ready for daily use before dogfooding.  
**Version:** 0.2.0  
**Last Updated:** 2026-01-15

---

## Pre-Flight Checks

### Build Verification
- [ ] `npm run build` completes without errors
- [ ] `cargo check` completes without errors
- [ ] `npm run dev` starts dev server successfully
- [ ] No TypeScript type errors

### Environment Detection
- [ ] App shows "Web" badge in sidebar when running in browser
- [ ] App shows "Desktop" badge in sidebar when running in Tauri
- [ ] Runtime badge color is correct (blue for Web, green for Desktop)

---

## Core Workflow Testing

### 1. Project Management
- [ ] Can add a new project with required fields (name, folder path)
- [ ] Can add a project with all optional fields (repo URL, branch, docs folder, terminal dir, default agent CLI, notes)
- [ ] Project appears in sidebar immediately after creation
- [ ] Can switch between multiple projects
- [ ] Current project persists after page refresh
- [ ] Can edit project details (name, path, repo URL, branch, etc.)
- [ ] Can change project status (active/archived)
- [ ] Can delete project from sidebar (hover to see delete button)
- [ ] Can delete project from Edit Project page
- [ ] Delete confirmation shows explicit warning about data deletion
- [ ] Delete confirmation mentions "Original files on disk are NOT deleted"
- [ ] After deletion, related data is cleaned up (docs, sprints, tasks, sessions, presets)
- [ ] After deletion, app redirects to Home with no project selected

### 2. Home Dashboard
- [ ] Home shows "No project selected" state when no project exists
- [ ] Home shows "Add Project" button in empty state
- [ ] Home shows project name and folder path when project is selected
- [ ] Git status section shows branch name
- [ ] Git status section shows clean/dirty badge
- [ ] Git status section shows last commit hash (if available)
- [ ] Git status section shows "Mock" badge in web mode
- [ ] Git status refresh button works (click to refresh)
- [ ] Quick actions row shows all buttons (Open Folder, Open Terminal, agent CLIs, View Sprint, View Docs, Agent Sessions, Dev Connector)
- [ ] Agent CLI buttons are disabled when CLI path is not configured
- [ ] Agent CLI buttons are enabled when CLI path is configured
- [ ] Setup Checklist section shows all 6 items
- [ ] Setup Checklist shows correct done/not-done status
- [ ] Setup Checklist items are clickable and navigate to correct pages
- [ ] Recent Work section shows recent items with type icons
- [ ] Recent Work section shows "No recent work yet" when empty
- [ ] Recent Work items show correct emoji icons (📁 project, ⌨️ terminal, 🤖 agent, etc.)
- [ ] Active Sprint section shows sprint name and task list
- [ ] Active Sprint section shows "No sprint configured" when no sprint exists
- [ ] Agent Sessions section shows recent sessions
- [ ] Agent Sessions section shows "No agent sessions" when empty
- [ ] System Status section shows provider and agent CLI status

### 3. Docs Page
- [ ] Docs page shows 7 doc source cards (specs, engineering, api, sprint, research, architecture, agent-instructions)
- [ ] Each card shows title, description, and connected/not-connected badge
- [ ] Can connect a doc source (click Connect button)
- [ ] Connecting a doc source opens folder picker (native in Tauri, prompt in web)
- [ ] After connecting, card shows connected path and file count
- [ ] Can disconnect a doc source (click Disconnect button)
- [ ] Can toggle agent context on/off for connected doc sources
- [ ] Agent context badge shows "Agent context: ON" when enabled

### 4. Sprint Page
- [ ] Sprint page shows "No sprint source configured" when no sprint exists
- [ ] Can create mock sprint (click "Use Mock Sprint" button)
- [ ] After creating mock sprint, page shows sprint name and status
- [ ] Sprint page shows task list with status and priority badges
- [ ] Can filter tasks by status (all, pending, in-progress, blocked, completed)
- [ ] Can click task to see detail panel
- [ ] Task detail panel shows status and priority dropdowns
- [ ] Can change task status via dropdown
- [ ] Can change task priority via dropdown
- [ ] Can mark task as active (click "Mark Active" button)
- [ ] Progress bar shows correct completion percentage
- [ ] Sprint data persists after page refresh

### 5. Agent Sessions Page
- [ ] Agent Sessions page shows mock sessions for current project
- [ ] Each session shows tool icon, title, status badge, and timestamp
- [ ] Mock sessions show "Mock" badge
- [ ] Can filter sessions by tool (all, codex, claude-code, opencode)
- [ ] Can click session to see detail panel
- [ ] Session detail panel shows tool, status, last active time, summary
- [ ] Session detail panel shows changed files list (if available)
- [ ] Can mark session as important (click "Mark Important" button)
- [ ] Important sessions show ⭐ icon
- [ ] Can delete session from index (click "Delete from Index" button)
- [ ] Delete confirmation mentions "Original files are not deleted"
- [ ] Can attach session to task (select task from dropdown)
- [ ] Scan Sessions button works (in Tauri mode with session dirs configured)
- [ ] Scanned sessions show "Real" badge
- [ ] Scanned sessions show file size and path
- [ ] Scanned sessions show preview (with secrets redacted)

### 6. Dev Connector Page
- [ ] Dev Connector page shows project context (name, path, terminal dir, agent CLI)
- [ ] Terminal Launcher section shows working directory
- [ ] Can open terminal (click "Open Terminal" button)
- [ ] Git Status section shows branch, clean/dirty status, last commit
- [ ] Git Status section shows "Mock" or "Real" badge
- [ ] Git Status refresh button works (click to refresh)
- [ ] Agent CLI Paths section shows configured paths for codex, claude, opencode
- [ ] Unconfigured CLIs show "Not configured" in yellow
- [ ] Configured CLIs show path in foreground color
- [ ] Can launch agent CLI (click "Open Codex/Claude Code/OpenCode" button)
- [ ] Unconfigured agent buttons are disabled
- [ ] Command Presets section shows list of presets
- [ ] Each preset shows name, command, args, and risk level badge
- [ ] Can run a preset (click "Run" button)
- [ ] Running a preset opens terminal and executes command (in Tauri mode)
- [ ] Running a dangerous preset shows confirmation dialog
- [ ] Running a caution preset shows confirmation dialog
- [ ] Can copy preset command (click "Copy" button)
- [ ] Can delete preset (click "✕" button)
- [ ] Delete preset shows confirmation dialog
- [ ] Can add new preset (fill form and click "Add Preset")
- [ ] New preset appears in list immediately

### 7. Settings Page
- [ ] Settings page shows Configuration Status section with 6 items
- [ ] Configuration Status shows correct done/not-done badges
- [ ] Provider section shows provider name, API key status, default model
- [ ] API key section shows "Dev-only" badge
- [ ] API key section shows "Status tracking only. The key value is not stored" notice
- [ ] Can mark API key as configured (enter key and click "Mark Configured")
- [ ] Can change API key (click "Change" button)
- [ ] Models section shows coding, research, summarization model inputs
- [ ] Can save models (click "Save Models" button)
- [ ] Server section shows API base URL, health status
- [ ] Can check server health (click "Check" button)
- [ ] Can save server settings (click "Save Server" button)
- [ ] Agent CLIs section shows codex, claude, opencode path inputs
- [ ] Each CLI path has "Validate" button
- [ ] Can validate CLI path (click "Validate" button)
- [ ] Validation shows "✓ Valid" or "✗ Invalid" feedback
- [ ] Can save agent CLIs (click "Save Agent CLIs" button)
- [ ] Session Directories section shows codex, claude, opencode dir inputs
- [ ] Each session dir has "Choose Folder" and "Validate" buttons
- [ ] Can choose session directory (click "Choose Folder" button)
- [ ] Can validate session directory (click "Validate" button)
- [ ] Can save session directories (click "Save Session Directories" button)
- [ ] Local Paths section shows default projects directory input
- [ ] Can choose projects directory (click "Choose Folder" button)
- [ ] Can validate projects directory (click "Validate" button)
- [ ] Can save local paths (click "Save Paths" button)
- [ ] Appearance section shows theme and font size dropdowns
- [ ] Can save appearance settings (click "Save Appearance" button)
- [ ] Data Storage section shows storage size
- [ ] Data Storage section shows "⚠️ Dev-only" warning about localStorage
- [ ] Can export data (click "Export Data" button)
- [ ] Export file has timestamp in filename (e.g., `openmesh-export-2026-01-15T14-30-45.json`)
- [ ] Can import data (click "Import Data" button, select file)
- [ ] Import shows success toast and reloads page
- [ ] Import shows error toast if file is invalid
- [ ] Can reset all data (click "Reset All Data" button)
- [ ] Reset confirmation shows explicit list of what gets deleted
- [ ] Reset confirmation mentions "Original files on disk are NOT affected"
- [ ] Reset confirmation mentions "This cannot be undone"
- [ ] Reset clears all data and reloads page

---

## Data Persistence Testing

### localStorage Persistence
- [ ] Projects persist after page refresh
- [ ] Current project persists after page refresh
- [ ] Doc sources persist after page refresh
- [ ] Sprints persist after page refresh
- [ ] Tasks persist after page refresh
- [ ] Agent sessions persist after page refresh
- [ ] Command presets persist after page refresh
- [ ] Settings persist after page refresh
- [ ] Recent work persists after page refresh

### Export/Import/Reset
- [ ] Export creates valid JSON file
- [ ] Export file contains all openmesh data
- [ ] Export file includes `_version` field
- [ ] Import restores all data correctly
- [ ] Import shows warnings for version mismatch
- [ ] Import rejects non-object JSON
- [ ] Import rejects arrays
- [ ] Import validates array keys (projects, doc-sources, etc.)
- [ ] Import validates object keys (settings, app-state)
- [ ] Import skips invalid keys with warnings
- [ ] Reset clears all openmesh data
- [ ] Reset does not affect non-openmesh localStorage keys

---

## Safety Testing

### Destructive Actions
- [ ] Delete project shows confirmation dialog
- [ ] Delete project confirmation mentions data deletion
- [ ] Delete project confirmation mentions "Original files on disk are NOT deleted"
- [ ] Delete project actually removes project from list
- [ ] Delete project actually removes related data (docs, sprints, tasks, sessions, presets)
- [ ] Delete project does NOT delete files on disk
- [ ] Reset shows confirmation dialog
- [ ] Reset confirmation lists all data that will be deleted
- [ ] Reset confirmation mentions "Original files on disk are NOT affected"
- [ ] Reset confirmation mentions "This cannot be undone"
- [ ] Reset actually clears all data
- [ ] Reset does NOT delete files on disk

### Command Preset Safety
- [ ] Safe presets run without confirmation
- [ ] Caution presets show confirmation dialog
- [ ] Dangerous presets show confirmation dialog with warning
- [ ] Rust backend blocks `rm -rf` pattern
- [ ] Rust backend blocks `rm -fr` pattern
- [ ] Rust backend blocks `del /s` pattern
- [ ] Rust backend blocks `del /f` pattern
- [ ] Rust backend blocks `rmdir /s` pattern
- [ ] Rust backend blocks `git reset --hard` pattern
- [ ] Rust backend blocks `git clean -fd` pattern
- [ ] Rust backend blocks `git push --force` pattern
- [ ] Rust backend blocks `git push -f` pattern
- [ ] Rust backend blocks `format c:` pattern
- [ ] Rust backend blocks `format d:` pattern
- [ ] Rust backend blocks `format e:` pattern
- [ ] Rust backend blocks `mkfs` pattern
- [ ] Rust backend does NOT block `npm run format` (false positive fixed)
- [ ] Blocked commands return error message with pattern name

### Agent CLI Safety
- [ ] Agent CLI launcher only accepts `codex`, `claude`, `claude-code`, `opencode`
- [ ] Agent CLI launcher rejects unknown tools with error message
- [ ] Agent CLI launcher validates cwd exists and is a directory
- [ ] Agent CLI launcher returns error if cwd is invalid

### Session Scanner Safety
- [ ] Session scanner validates directory exists
- [ ] Session scanner returns error if directory is invalid
- [ ] Session scanner only accepts `codex`, `claude`, `claude-code`, `opencode`
- [ ] Session scanner rejects unknown tools with error message
- [ ] Session scanner skips symlinks (does not follow them)
- [ ] Session scanner skips directories
- [ ] Session scanner only reads files with allowed extensions (json, jsonl, md, txt, log)
- [ ] Session scanner limits results to configured limit (default 100)
- [ ] Session scanner redacts secrets in preview (sk-*, ghp_*, AKIA*, Bearer tokens, key=value)

---

## Edge Cases

### Empty States
- [ ] Home shows empty state when no project exists
- [ ] Docs shows empty state when no doc sources exist (should not happen, but test anyway)
- [ ] Sprint shows empty state when no sprint exists
- [ ] Agent Sessions shows empty state when no sessions exist
- [ ] Recent Work shows empty state when no recent items exist
- [ ] Command Presets shows empty state when no presets exist

### Invalid Data
- [ ] App handles invalid project path gracefully (shows error, does not crash)
- [ ] App handles invalid session directory gracefully (shows error, does not crash)
- [ ] App handles invalid CLI path gracefully (shows error, does not crash)
- [ ] App handles invalid import JSON gracefully (shows error, does not crash)
- [ ] App handles missing localStorage gracefully (uses defaults, does not crash)

### Concurrent Operations
- [ ] Can switch projects while on any page
- [ ] Switching project updates all scoped data (docs, sprints, tasks, sessions, presets)
- [ ] Can add project while on Home page
- [ ] Can edit project while on Edit Project page
- [ ] Can delete project while on any page

---

## Web Mode vs Desktop Mode

### Web Mode (Browser)
- [ ] Folder picker uses `prompt()` dialog
- [ ] Git status returns mock data
- [ ] Terminal launch shows alert
- [ ] Agent CLI launch shows alert
- [ ] Session scanning returns empty array
- [ ] Command preset execution shows alert
- [ ] All features work without errors

### Desktop Mode (Tauri)
- [ ] Folder picker uses native dialog
- [ ] Git status returns real data
- [ ] Terminal launch opens terminal
- [ ] Agent CLI launch opens agent CLI
- [ ] Session scanning returns real sessions
- [ ] Command preset execution opens terminal and runs command
- [ ] All features work without errors

---

## Performance

- [ ] Home page loads in < 1 second
- [ ] Project switching is instant (< 100ms)
- [ ] Git status refresh is fast (< 500ms)
- [ ] Session scanning is fast (< 2 seconds for 100 sessions)
- [ ] Export is fast (< 1 second)
- [ ] Import is fast (< 1 second)
- [ ] Reset is instant (< 100ms)

---

## Final Checks

- [ ] All builds pass (`npm run build`, `cargo check`)
- [ ] No console errors in browser dev tools
- [ ] No TypeScript errors
- [ ] App feels smooth and responsive
- [ ] All destructive actions have clear warnings
- [ ] All safety controls work as expected
- [ ] Data persists correctly
- [ ] Export/import/reset work correctly
- [ ] App is ready for daily dogfood use

---

## Sign-Off

**Tester:** _________________  
**Date:** _________________  
**Result:** [ ] PASS  [ ] FAIL  
**Notes:** _______________________________________________

---

**End of Checklist**
