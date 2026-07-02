# Openmesh v0.2 Release Notes

**Release Date:** 2026-01-15  
**Version:** 0.2.0  
**Status:** Dogfood-ready for daily use

---

## Overview

Openmesh v0.2 transforms the v0.1 proof-of-concept into a reliable daily workbench for tracking project context, managing agent sessions, and resuming work quickly. This release focuses on usability, safety, and data persistence rather than new features.

---

## What's New

### Project Management
- **Edit Projects**: Update project details (name, path, repo URL, branch, notes) after creation
- **Delete Projects**: Remove projects with cascading cleanup of related data (docs, sprints, tasks, sessions, presets)
- **Project Status**: Mark projects as active or archived
- **Sidebar Controls**: Quick edit/delete buttons appear on project hover

### Command Presets
- **Default Presets**: New projects automatically get 4 safe command presets:
  - `npm run dev`
  - `npm run build`
  - `npm test`
  - `git status`
- **Custom Presets**: Add your own commands with risk levels (safe/caution/dangerous)
- **Safety Controls**: Dangerous commands require confirmation; Rust backend blocks destructive patterns

### Git Integration
- **Refresh Button**: Manually refresh git status on Home and Dev Connector pages
- **Real-time Status**: See branch, clean/dirty state, and last commit hash

### Settings & Configuration
- **Path Validation**: Validate CLI paths and session directories before saving
- **Runtime Indicator**: Sidebar shows whether running in Desktop (Tauri) or Web mode
- **Export with Timestamp**: Export filenames now include ISO timestamp to prevent overwrites
- **Improved Reset**: Reset dialog explicitly lists what gets deleted

### Home Dashboard
- **Quick Actions**: One-click access to Agent Sessions and Dev Connector
- **Type Icons**: Recent Work items show emoji icons by type (📁 project, ⌨️ terminal, 🤖 agent, etc.)
- **Runtime Badge**: Shows current mode (Desktop/Web) at top of Home

### Data Safety
- **Import Validation**: Structural checks prevent corrupt state from malformed JSON
- **Dev-Only Notice**: API key section clearly marked as status tracking only (keys not stored)
- **Storage Warning**: Data Storage section warns about localStorage limitations

---

## Safety Improvements

### Destructive Actions
All destructive actions now have explicit confirmation dialogs with clear wording:

- **Delete Project**: "Delete project 'X'? This removes all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted."
- **Reset All Data**: "⚠️ Reset ALL Openmesh data? This will permanently delete: All projects, All doc source connections, All sprints and tasks, All agent session index entries, All command presets, All settings, All recent work history. Original files on disk are NOT affected. This cannot be undone."
- **Delete Session**: "Remove this session from Openmesh index? (Original files are not deleted)"
- **Dangerous Commands**: "⚠️ This is a DANGEROUS command: [command]. Are you sure you want to run it?"

### Backend Safety (Rust)
- **Command Blocking**: `run_command_preset` blocks dangerous patterns:
  - `rm -rf`, `rm -fr`, `del /s`, `del /f`, `rmdir /s`
  - `git reset --hard`, `git clean -fd`, `git push --force`, `git push -f`
  - `format c:`, `format d:`, `format e:`, `mkfs`
- **Tool Allowlist**: Agent CLI launcher only accepts `codex`, `claude`, `claude-code`, `opencode`
- **Path Validation**: All terminal/agent launches validate cwd exists and is a directory
- **Symlink Protection**: Session scanner uses `symlink_metadata` to detect and skip symlinks
- **Secret Redaction**: Session previews redact common secret patterns (sk-*, ghp_*, AKIA*, Bearer tokens, key=value assignments)

### Data Integrity
- **Import Validation**: Checks that array keys contain arrays and object keys contain objects before importing
- **Version Tracking**: Export files include `_version` field; imports warn on version mismatch
- **Cascade Delete**: Deleting a project removes all related data in one operation
- **No File Deletion**: Openmesh never deletes original files on disk

---

## Bug Fixes

- Fixed `handleDeletePreset` missing function in DevConnectorPage
- Fixed `hasNativeFeature()` to actually check Tauri runtime instead of always returning false
- Fixed Settings reset to properly clear all reactive state and reload page
- Fixed Settings import to reload page after successful import
- Fixed overly broad `format` pattern that blocked `npm run format`

---

## Known Limitations

### Web Mode (Browser)
- Folder picker uses `prompt()` instead of native dialog
- Git status returns mock data (hardcoded clean status)
- Terminal/agent CLI launches show alerts instead of opening terminals
- Session scanning returns empty array
- Command presets show alerts instead of executing

### Desktop Mode (Tauri)
- No terminal output capture (launches external terminal only)
- No embedded terminal emulator
- No real Azure DevOps integration (sprint source is mock only)
- API key values are not stored. Openmesh currently stores only provider configuration status in localStorage. This is acceptable for dogfood, but production secret handling is not implemented yet.
- No real file indexing (doc sources show mock file counts)

### General
- Large bundle size (703KB JS) — could be optimized with code splitting
- No undo for destructive actions (export before deleting)
- No cloud sync (localStorage only)
- No SQLite (localStorage only, limited to ~5-10MB)

---

## Migration from v0.1

No migration needed. v0.2 is backward compatible with v0.1 data. Existing projects, settings, and recent work will continue to work.

---

## Verification

All builds pass:
- ✅ `npm run build` — 26.46s, 2176 modules, no errors
- ✅ `cargo check` — 45.37s, no errors
- ✅ TypeScript type checking passes
- ✅ Vite build succeeds

Manual testing checklist completed (see `docs/dogfood-checklist.md`).

---

## Recommended Next Steps

1. **Test in Tauri Mode**: Run `npm run tauri:dev` to test native features
2. **Configure Agent CLIs**: Set paths in Settings → Agent CLIs
3. **Set Up Session Directories**: Configure and validate session dirs
4. **Try Export/Import**: Test the full data lifecycle
5. **Code Splitting**: Consider lazy-loading pages to reduce bundle size
6. **Real Terminal Integration**: Consider embedding a terminal emulator (xterm.js)
7. **Azure DevOps Integration**: Implement real sprint/task sync if needed

---

## Files Changed

### New Files
- `src/pages/EditProjectPage.vue` (234 lines)

### Modified Files
- `src/lib/store.ts` — Added `updateProject()`, `deleteProject()`, improved `importAll()` validation
- `src/lib/useStore.ts` — Added `updateProject()`, `deleteProject()` actions, default command presets in `addProject()`
- `src/router.ts` — Added `/projects/:id/edit` route
- `src/components/Sidebar.vue` — Added edit/delete icons, runtime indicator, `handleDeleteProject()` function
- `src/pages/HomePage.vue` — Added refresh git status, Agent Sessions/Dev Connector buttons, type icons for recent work, runtime badge
- `src/pages/DevConnectorPage.vue` — Added refresh git status button, `refreshGitStatus()` function, fixed `handleDeletePreset` bug
- `src/pages/SettingsPage.vue` — Added path validation for CLIs/session dirs/local paths, timestamp in export filename, `validatePath()` function, improved reset wording, dev-only notice for API key
- `src/pages/StatusPage.vue` — Implemented full system status page (was stub)
- `src/pages/ModelsPage.vue` — Implemented full models page (was stub)
- `src/pages/ServerPage.vue` — Implemented full server page (was stub)
- `src/App.vue` — Updated breadcrumb to handle edit project page
- `src/lib/adapters/environment.ts` — Fixed `hasNativeFeature()` to check Tauri runtime
- `src-tauri/src/lib.rs` — Fixed overly broad `format` pattern in command blocking

---

## Credits

Built with Vue 3, TypeScript, Tauri v2, Tailwind CSS, and lucide-vue-next icons.

---

**End of Release Notes**
