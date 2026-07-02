# Openmesh v0.3 Release Notes

**Release Date:** 2026-01-15  
**Version:** 0.3.0  
**Status:** Storage Hardened - Ready for Dogfood

---

## Major Changes

### File-Based Storage Migration

**BREAKING CHANGE:** Openmesh has migrated from browser localStorage to file-based storage.

**What changed:**
- All data is now stored as JSON and Markdown files on your local filesystem
- Global data: `~/.openmesh/` (settings, project registry, app state)
- Project data: `<project>/.openmesh/` (docs, notes, tasks, sessions, presets, recent)
- No more localStorage - data persists across browsers and devices

**Why:**
- Better data portability
- Easier backup and version control
- Integration with Git workflows
- No browser storage limits

**Migration:**
- No automatic migration from v0.2 localStorage data
- Export your data from v0.2 before upgrading (if needed)
- v0.3 starts with a clean slate

---

## New Features

### Atomic Writes

All JSON file writes now use atomic operations to prevent corruption:
- Data is written to a temporary file first
- File is renamed atomically to final destination
- Protects against crashes during write operations

### Corrupt File Recovery

Automatic recovery from corrupted JSON files:
- Corrupt files are backed up with timestamp: `<filename>.corrupt-<timestamp>.bak`
- Default data is restored automatically
- Warning logged to console for debugging

### Git Safety

Automatic Git integration for project repositories:
- `.openmesh/` is added to `.git/info/exclude` when project is initialized
- Prevents accidental commits of Openmesh metadata
- Keeps your `.gitignore` clean
- Only affects local repository (collaborators need to add it themselves)

### Schema Versioning

Export files now include schema version:
- Enables future migration support
- Current version: `1.0.0`
- Format: `{ "schemaVersion": "1.0.0", ... }`

### Complete Reset

New "Reset All Data" feature:
- Deletes `~/.openmesh/` (global configuration)
- Deletes all `<project>/.openmesh/` directories
- Clears in-memory state
- Recreates default settings
- **Warning:** Irreversible - all Openmesh data will be lost

---

## Improvements

### Storage Architecture

- Global settings stored in `~/.openmesh/settings.json`
- Project registry in `~/.openmesh/projects.json`
- App state in `~/.openmesh/app-state.json`
- Project metadata in `<project>/.openmesh/project.json`
- Docs and notes as real Markdown files (not JSON blobs)

### Filesystem Permissions

- Removed unnecessary `fs:scope-appdata` permissions
- Retained `fs:scope-home-recursive` for project access
- Added documentation comment explaining permission requirements

### Data Safety

- All JSON writes are atomic (temp file + rename)
- Corrupt files are automatically recovered
- User source files are never modified or deleted
- Only `.openmesh/` directories are affected by Openmesh operations

---

## Bug Fixes

- Fixed incomplete `resetAll()` implementation (now actually deletes files from disk)
- Fixed missing `resetAll` export in useStore
- Fixed overly broad filesystem permissions (removed appdata scope)
- Fixed missing schema versioning in exports

---

## Known Limitations

1. **No Import Feature** - Cannot import exported JSON files yet
2. **No Migration Path** - v0.2 localStorage data cannot be imported automatically
3. **No Cloud Sync** - Data is local-only
4. **No Encryption** - Data is stored as plain text
5. **Broad Permissions** - Requires filesystem access to entire home directory

---

## Files Changed

### New Files
- `docs/storage-architecture-v0.3.md` - Comprehensive storage documentation
- `docs/release-notes-v0.3.md` - This file

### Modified Files
- `src-tauri/src/storage.rs` - Added atomic writes, corrupt recovery, Git safety, reset function
- `src-tauri/src/lib.rs` - Added `reset_all_data_cmd`, registered new command
- `src-tauri/capabilities/default.json` - Removed appdata permissions, added documentation
- `src/lib/store.ts` - Added `resetAllData()` method
- `src/lib/useStore.ts` - Implemented real `resetAll()` function

---

## Verification

### Build Status
- ✅ `npm run build` - PASS (15.23s)
- ✅ `cargo check` - PASS (12.41s)
- ✅ TypeScript - No errors
- ✅ Rust - No errors (only unused code warnings)

### Manual Testing Checklist

See `docs/dogfood-checklist.md` for complete testing guide.

**Critical tests:**
1. First launch with no `~/.openmesh/` - should create directory
2. Add project - should create `<project>/.openmesh/`
3. Create note - should persist after restart
4. Corrupt `tasks.json` manually - should recover safely
5. Delete project - should remove `.openmesh/` only
6. Reset all data - should delete all Openmesh data
7. Git repository - should auto-add `.openmesh/` to exclude

---

## Upgrade Guide

### From v0.2 to v0.3

**Before upgrading:**
1. Export your data from v0.2 (if you have important data)
2. Note your project paths
3. Backup `~/.openmesh/` if it exists

**After upgrading:**
1. Launch Openmesh v0.3
2. Re-add your projects
3. Re-import docs/notes manually
4. Re-configure settings

**Note:** There is no automatic migration. v0.3 starts fresh.

---

## Security Considerations

### Filesystem Access

Openmesh requires broad filesystem permissions:
- **Why:** Projects can be stored anywhere on disk
- **What:** Access to home directory and subdirectories
- **Safety:** Only writes to `.openmesh/` directories
- **Risk:** Low - user source files are never modified

### Data Privacy

- All data is stored locally (no cloud sync)
- Data is stored as plain text (no encryption)
- API keys are not stored (only configuration status)
- Session previews are redacted for common secret patterns

### Git Integration

- `.openmesh/` is auto-added to `.git/info/exclude`
- This is a local-only change (not shared with collaborators)
- Collaborators should add `.openmesh/` to their own exclude list

---

## Roadmap

### v0.4 (Planned)
- Import feature for exported JSON
- Selective backup/export options
- Automatic schema migration
- Narrower filesystem permissions (if possible)

### Future
- Encrypted storage option
- Cloud sync integration
- Plugin system for custom storage backends

---

## Credits

Built with:
- Tauri v2 (desktop framework)
- Vue 3 (frontend)
- Rust (backend)
- serde + serde_json (serialization)
- chrono (timestamps)
- dirs (home directory)

---

**End of Release Notes**
