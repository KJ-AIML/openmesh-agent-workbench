> **Mostly still accurate for file-based storage roots**, but incomplete vs later surfaces (agent chats, relay/LAN, canvases, secrets file). Current map: [`ARCHITECTURE.md`](./ARCHITECTURE.md) · [`PRODUCT_GUIDE.md`](./PRODUCT_GUIDE.md#storage--secrets). Dogfood: prefer [`PRODUCT_GUIDE.md`](./PRODUCT_GUIDE.md) over the v0.3 checklist alone.

# Openmesh v0.3 Storage Architecture

## Overview

Openmesh v0.3 uses a **file-based storage system** that stores all data as JSON and Markdown files on your local filesystem. This replaces the previous localStorage-based approach and provides better data portability, backup capabilities, and integration with version control systems.

## Storage Locations

### Global Data (`~/.openmesh/`)

The global configuration directory stores app-wide settings and the project registry:

```
~/.openmesh/
├── settings.json       # App settings (theme, agent CLI paths, etc.)
├── projects.json       # Registry of all project paths
└── app-state.json      # Current active project ID
```

**Location by platform:**
- **Windows:** `C:\Users\<username>\.openmesh\`
- **macOS:** `/Users/<username>/.openmesh/`
- **Linux:** `/home/<username>/.openmesh/`

### Project Data (`<project>/.openmesh/`)

Each project has its own `.openmesh/` directory containing all project-specific data:

```
<project>/.openmesh/
├── project.json        # Project metadata (name, folder path, etc.)
├── docs/               # Markdown documentation files
│   ├── spec.md
│   └── architecture.md
├── notes/              # Markdown notes
│   ├── ideas.md
│   └── meeting-2024-01-15.md
├── sessions.json       # Agent session history
├── tasks.json          # Task list
├── presets.json        # Command presets
└── recent.json         # Recent work history
```

## Data Safety Features

### Atomic Writes

All JSON file writes use **atomic write operations** to prevent data corruption:

1. Data is written to a temporary file (`.tmp` extension)
2. The temporary file is renamed to the final filename
3. This ensures files are never left in a half-written state

**Implementation:** `storage.rs::atomic_write()`

### Corrupt File Recovery

If a JSON file becomes corrupted (e.g., due to disk errors or incomplete writes):

1. The corrupt file is automatically backed up with a timestamp: `<filename>.corrupt-<timestamp>.bak`
2. Default data is restored
3. A warning is logged to the console

**Implementation:** `storage.rs::read_with_recovery()`

### Git Safety

When a project is initialized in a Git repository, Openmesh automatically adds `.openmesh/` to `.git/info/exclude`:

```
# Openmesh metadata (auto-added by Openmesh app)
.openmesh/
```

This prevents accidental commits of Openmesh metadata while keeping your `.gitignore` clean.

**Implementation:** `storage.rs::add_to_git_exclude()`

**Note:** This only affects your local repository. Other collaborators will need to add `.openmesh/` to their own exclude list or `.gitignore`.

### Schema Versioning

All exported data includes a `schemaVersion` field to support future migrations:

```json
{
  "schemaVersion": "1.0.0",
  "project": { ... },
  "tasks": [ ... ],
  ...
}
```

Current schema version: **1.0.0**

## Filesystem Permissions

Openmesh requires broad filesystem permissions because projects can be stored anywhere on disk:

**Required permissions:**
- `fs:scope-home-recursive` - Access to home directory and subdirectories
- `fs:allow-read-file` - Read JSON and Markdown files
- `fs:allow-write-file` - Write JSON and Markdown files
- `fs:allow-read-dir` - List directory contents
- `fs:allow-mkdir` - Create `.openmesh/` directories
- `fs:allow-remove` - Delete files and directories (for reset)
- `fs:allow-exists` - Check if files exist

**Security note:** Openmesh only writes to `.openmesh/` directories and never modifies user source files. The broad permissions are necessary to support projects stored in arbitrary locations.

## What is Safe to Delete

### Safe to Delete
- `~/.openmesh/` - Global configuration (will be recreated on next launch)
- `<project>/.openmesh/` - Project metadata (docs and notes will be lost)
- Individual files in `docs/` or `notes/` - Only affects Openmesh, not your project

### Never Deleted by Openmesh
- User source files (code, assets, etc.)
- Git history
- Files outside of `.openmesh/` directories
- Original files referenced by docs (only paths are stored)

## Backup and Export

### Manual Backup

You can manually backup your data by copying:
1. `~/.openmesh/` - Global configuration
2. `<project>/.openmesh/` - Project data (for each project)

### Export Feature

Use the **Export Project** feature in Settings to create a JSON backup of a single project:

```json
{
  "schemaVersion": "1.0.0",
  "project": { ... },
  "docs": [
    { "filename": "spec.md", "content": "..." }
  ],
  "notes": [
    { "filename": "ideas.md", "content": "..." }
  ],
  "tasks": [ ... ],
  "sessions": [ ... ],
  "presets": [ ... ],
  "recent": [ ... ]
}
```

**Note:** Export does not include global settings or other projects.

### Import Feature

Import is not yet implemented in v0.3. To restore data:
1. Manually copy `.openmesh/` directories from backup
2. Or recreate projects and re-import docs/notes

## Reset Behavior

The **Reset All Data** feature:
1. Deletes `~/.openmesh/` (global configuration)
2. Deletes all `<project>/.openmesh/` directories
3. Clears in-memory state
4. Recreates `~/.openmesh/` with default settings

**Warning:** This action is irreversible. All Openmesh data will be lost. User source files are never affected.

## Known Limitations

1. **No Import Feature** - Cannot import exported JSON files yet
2. **No Cloud Sync** - Data is local-only
3. **No Encryption** - Data is stored as plain text
4. **Broad Permissions** - Requires filesystem access to entire home directory
5. **No Migration Path** - Schema changes require manual intervention

## Troubleshooting

### Corrupt File Warning

If you see a warning about a corrupt file:
1. Check the console for the backup location
2. Inspect the backup file to see what went wrong
3. Fix any issues and rename the backup back to the original filename
4. Or delete the backup and let Openmesh recreate default data

### Missing Project Data

If a project's `.openmesh/` directory is missing:
1. The project will still appear in the project list
2. Openmesh will show an error when you try to access it
3. Delete the project from the list and re-add it

### Git Conflicts

If `.openmesh/` files appear in Git:
1. Run `git rm -r --cached .openmesh/` to remove from tracking
2. Add `.openmesh/` to `.gitignore` or `.git/info/exclude`
3. Commit the changes

## Future Improvements

Potential enhancements for future versions:
- Import feature for exported JSON
- Selective backup/export (docs only, tasks only, etc.)
- Encrypted storage option
- Narrower filesystem permissions
- Automatic schema migration
- Cloud sync integration
