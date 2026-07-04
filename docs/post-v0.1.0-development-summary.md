# OpenMesh Post-v0.1.0 Development Summary

Date: July 5, 2026

Audience: discussion notes for reviewing progress with an assistant or collaborator.

## Baseline: v0.1.0

OpenMesh v0.1.0 shipped as a Windows preview release for a local-first desktop agent workbench.

The release baseline included:

- Tauri v2 desktop shell with Vue 3 and TypeScript frontend.
- File-based local storage under `~/.openmesh/` and per-project `.openmesh/`.
- Project management.
- Terminal launch.
- Agent CLI launch for Codex, Claude Code, and OpenCode.
- Command palette.
- Work snapshots.
- Notes and Docs basics.
- Recent work memory.
- Git status.
- Settings.
- Export/import/reset.
- Dark desktop UI.

Primary known rough edges after release:

- Docs drag and drop was not usable.
- Docs rename was unreliable, especially with nested folders.
- Notes rename was implemented in the UI with a fragile write/delete flow.
- Notes markdown import needed clearer behavior.
- Some UI polish was still missing around agent icons and startup loading.

## Summary Of Work Completed After v0.1.0

### 1. Heli-Harness Setup For OpenMesh

Heli was prepared to treat `web-demo` as the active OpenMesh target.

Completed:

- Registered `web-demo` in `.heli-harness/workspace/index.json`.
- Switched `.heli-harness/workspace/target.json` to `web-demo`.
- Added `.heli-harness/profiles/web-demo.md`.
- Reset task tracking in `.heli-harness/state/current-task.md`.

Why it matters:

- Future work now has a clear target repo.
- Dirty-file boundaries are documented.
- Validation expectations are recorded.

### 2. Docs Tree And Folder Support

Docs moved from a flatter list-style experience toward an IDE-like tree.

Completed:

- Added `DocTreeItem.vue` as the recursive docs tree component.
- Added nested folder tree support in the frontend.
- Added `list_docs_tree` backend command.
- Added `DocTreeNode` types in frontend and Rust storage.
- Added folder create, rename, delete, and file move command wiring.

Important files:

- `src/components/DocTreeItem.vue`
- `src/pages/DocsPage.vue`
- `src/lib/store.ts`
- `src/lib/useStore.ts`
- `src-tauri/src/storage.rs`
- `src-tauri/src/lib.rs`

### 3. Docs Rename Fix

The original rename path failed for nested docs because backend validation only accepted simple filenames.

Completed:

- Added safe relative child path handling in Rust storage.
- Allowed nested relative paths like `folder/file.md`.
- Kept traversal protection against invalid path components.
- Added a regression test for nested doc rename.

Evidence:

- Test first failed with `Err("Invalid filename")`.
- Test passed after the storage fix.

Validation command:

```bash
cd web-demo/src-tauri
cargo test rename_doc_keeps_nested_relative_path_inside_docs
```

### 4. Docs Move Into Folder

Native HTML drag/drop still showed a blocked cursor in the Tauri UI, so the implementation was rebuilt.

Completed:

- Replaced internal Docs tree move with pointer-based dragging.
- Detects the folder under the cursor with `document.elementFromPoint`.
- Calls `moveDoc(sourcePath, targetFolder)` directly.
- Shows a floating label while dragging.
- Keeps external `.md` import separate from internal tree movement.

Manual result confirmed by user:

```text
Moved "s.md" to "Spec"
```

Why this matters:

- The feature now behaves closer to VS Code or an IDE tree.
- It avoids Tauri/WebView native drag/drop quirks.

### 5. Notes Rename Fix

Notes rename was previously simulated by writing a new file and deleting the old one from the UI.

Completed:

- Added backend `rename_note` command.
- Added `rename_note_fn` in Rust storage.
- Added frontend store/useStore wrapper.
- Updated `NotesPage.vue` to call backend rename directly.
- Flushes pending note content before rename.

Why this matters:

- Less risk of data loss.
- Simpler and more reliable behavior.

### 6. Notes Markdown Import Clarity

Notes drag/drop import was narrowed to external file drops.

Completed:

- Page-level drag-over now checks for external file items.
- `.md` import behavior remains separate from Docs internal movement.

Why this matters:

- Prevents internal UI interactions from being treated like file imports.

### 7. Agent CLI Button Icon Polish

The Home page had broken or generic emoji-like icons for Codex, Claude, and OpenCode.

Completed:

- Replaced visible broken/generic icon rendering with inline SVG marks.
- Avoided new icon packages or downloaded logo assets.
- Kept the existing button layout.

Important file:

- `src/pages/HomePage.vue`

Note:

- These are brand-style inline marks, not official bundled logo assets.
- Official brand assets can be added later if exact logo fidelity is required.

### 8. Startup Loading Screen

An in-app startup splash was added, inspired by the Codex loading reference.

Completed:

- Added full-window dark startup overlay.
- Uses existing `public/logo.svg`.
- Shows a centered pulsing OpenMesh mark.
- Waits for store loading plus a minimum display time of 700ms.

Important file:

- `src/App.vue`

Note:

- This is an in-app splash after the WebView loads.
- A true native splash before WebView paint would require Tauri window/show coordination.

## Validation Completed

Commands run during the post-v0.1.0 work:

```bash
cd web-demo/src-tauri
cargo test rename_doc_keeps_nested_relative_path_inside_docs
```

Result:

- Failed first with the expected nested rename bug.
- Passed after the storage fix.

```bash
cd web-demo
npm run build
```

Result:

- Passed after Docs/Notes changes.
- Passed after pointer-based Docs move rebuild.
- Passed after Home page agent icon changes.
- Passed after startup splash changes.
- Vite large chunk warning remains.

```bash
cd web-demo/src-tauri
cargo check
```

Result:

- Passed.
- Existing dead-code warnings remain for `DocSource`, `Note`, and `now_iso`.

## Current Known State

Working:

- Docs nested rename backend path is fixed.
- Docs file move into folder works with pointer-based movement.
- Notes rename uses backend rename.
- Notes external `.md` import behavior is clearer.
- Home page agent icons no longer show broken/generic emoji.
- App has an in-app startup splash.

Still needs manual verification:

- Docs rename in root and inside nested folder.
- Docs move between two different folders.
- Notes rename in the running Tauri app.
- Notes `.md` import in the running Tauri app.
- Startup splash visual timing in the running Tauri app.

Known limitations:

- Startup splash is not a native pre-WebView splash.
- Agent icons are inline approximations, not official logo assets.
- Vite reports a large chunk warning.
- Tauri desktop behavior still needs live manual checks after reload/restart.

## Files Touched In This Post-Release Pass

Main app files:

- `src/App.vue`
- `src/components/DocTreeItem.vue`
- `src/pages/DocsPage.vue`
- `src/pages/HomePage.vue`
- `src/pages/NotesPage.vue`
- `src/lib/store.ts`
- `src/lib/useStore.ts`
- `src-tauri/src/lib.rs`
- `src-tauri/src/storage.rs`

Heli files:

- `.heli-harness/workspace/index.json`
- `.heli-harness/workspace/target.json`
- `.heli-harness/profiles/web-demo.md`
- `.heli-harness/state/current-task.md`

## Suggested Discussion Topics

### Product Direction

- Should OpenMesh focus first on being a strong local project workbench, or a full AI-agent session manager?
- Should Docs and Notes become one knowledge system, or remain separate?
- Should sprint/tasks be lightweight local planning, or integrate with GitHub/Jira later?

### UX Priorities

- Improve Docs tree to include right-click actions: rename, delete, move, duplicate.
- Add keyboard shortcuts: F2 rename, Delete delete, Ctrl+N new doc.
- Add drag reordering later only if ordering becomes user-visible.
- Add a simple "Move to folder" menu as a fallback for touchpad or accessibility.

### Technical Priorities

- Decide whether to keep file-based storage or move toward SQLite.
- Add focused tests for storage commands.
- Add manual Tauri QA checklist before each release.
- Reduce large bundle size if startup gets slow.

### Release Priorities

- Verify all v0.1.0 regressions are fixed in the desktop app.
- Prepare v0.1.1 as a bugfix release if Docs/Notes stability is the main goal.
- Prepare v0.2.0 only if adding broader features like embedded terminal or session parsing.

## Recommended Next Step

Ship a small `v0.1.1` bugfix release after manual QA.

Suggested scope:

- Docs nested rename.
- Docs move file into folder.
- Notes rename.
- Notes markdown import.
- Startup splash.
- Home page icon polish.

Avoid adding large new features until the core local workbench flows are stable.
