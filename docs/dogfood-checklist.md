# Openmesh v0.3 Dogfood Checklist

**Purpose:** Verify Openmesh v0.3 storage architecture is stable for daily use  
**Version:** 0.3.0  
**Last Updated:** 2026-01-15

> **Current RC / 1.0 path:** use [`docs/development/handoff-dogfood-rc-1.0.md`](development/handoff-dogfood-rc-1.0.md) (post v0.1.21). This v0.3 checklist remains historical for early storage QA.

---

## Pre-Flight Checks

### Build Verification
- [ ] `npm run build` completes without errors
- [ ] `cargo check` completes without errors
- [ ] No TypeScript type errors
- [ ] No Rust compilation errors (warnings OK)

### Environment Setup
- [ ] Delete `~/.openmesh/` if it exists (clean slate)
- [ ] Identify a test project directory (not your real projects!)
- [ ] Ensure test project is NOT a Git repository (for initial tests)

---

## Storage Architecture Tests

### 1. First Launch (Clean State)

**Setup:** Delete `~/.openmesh/` directory

**Tests:**
- [ ] Launch app - should not crash
- [ ] Check `~/.openmesh/` is created automatically
- [ ] Check `~/.openmesh/settings.json` exists with default values
- [ ] Check `~/.openmesh/projects.json` exists (empty array)
- [ ] Check `~/.openmesh/app-state.json` exists (null currentProjectId)
- [ ] UI shows "No project selected" state
- [ ] No errors in browser console
- [ ] No errors in Rust console

**Expected files:**
```
~/.openmesh/
├── settings.json       # Default settings
├── projects.json       # []
└── app-state.json      # { "currentProjectId": null }
```

### 2. Add Project

**Setup:** Have a test project directory ready (e.g., `C:\KJ\Repos\test-project`)

**Tests:**
- [ ] Click "Add Project" button
- [ ] Select test project folder via dialog
- [ ] Enter project name
- [ ] Click "Save Project"
- [ ] Check `~/.openmesh/projects.json` contains the project path
- [ ] Check `~/.openmesh/app-state.json` has currentProjectId set
- [ ] Check `<project>/.openmesh/` directory is created
- [ ] Check `<project>/.openmesh/project.json` exists with project metadata
- [ ] Check `<project>/.openmesh/docs/` directory is created
- [ ] Check `<project>/.openmesh/notes/` directory is created
- [ ] Check `<project>/.openmesh/sessions.json` exists (empty array)
- [ ] Check `<project>/.openmesh/tasks.json` exists (empty array)
- [ ] Check `<project>/.openmesh/presets.json` exists (default presets)
- [ ] Check `<project>/.openmesh/recent.json` exists (empty array)
- [ ] UI shows project in sidebar
- [ ] UI shows project as active

**Expected files:**
```
<project>/.openmesh/
├── project.json        # Project metadata
├── docs/               # Empty directory
├── notes/              # Empty directory
├── sessions.json       # []
├── tasks.json          # []
├── presets.json        # Default presets
└── recent.json         # []
```

### 3. Create and Edit Notes

**Tests:**
- [ ] Navigate to Notes page
- [ ] Click "New Note" button
- [ ] Enter note title
- [ ] Enter note content (markdown)
- [ ] Wait 1 second (auto-save)
- [ ] Check `<project>/.openmesh/notes/<filename>.md` exists
- [ ] Check file content matches what you typed
- [ ] Edit the note
- [ ] Wait 1 second
- [ ] Check file is updated
- [ ] Create another note
- [ ] Check both notes appear in list
- [ ] Close app
- [ ] Reopen app
- [ ] Check notes persist
- [ ] Check note content is intact

### 4. Create and Edit Docs

**Tests:**
- [ ] Navigate to Docs page
- [ ] Click "New Doc" button
- [ ] Enter doc title
- [ ] Enter doc content (markdown)
- [ ] Check `<project>/.openmesh/docs/<filename>.md` exists
- [ ] Check file content matches what you typed
- [ ] Edit the doc
- [ ] Check file is updated
- [ ] Close app
- [ ] Reopen app
- [ ] Check docs persist

### 5. Drag and Drop Import

**Setup:** Create a test markdown file on your desktop

**Tests:**
- [ ] Drag `.md` file onto Notes page
- [ ] Check file is imported to `<project>/.openmesh/notes/`
- [ ] Check file content is preserved
- [ ] Check note appears in list
- [ ] Drag `.md` file onto Docs page
- [ ] Check file is imported to `<project>/.openmesh/docs/`
- [ ] Check file content is preserved
- [ ] Check doc appears in list

### 6. Switch Projects

**Setup:** Add a second project

**Tests:**
- [ ] Add second project
- [ ] Check both projects appear in sidebar
- [ ] Click first project
- [ ] Check first project's notes/docs are shown
- [ ] Click second project
- [ ] Check second project's notes/docs are shown (should be empty)
- [ ] Check `~/.openmesh/app-state.json` updates to second project
- [ ] Close app
- [ ] Reopen app
- [ ] Check second project is still active

### 7. Delete Project

**Tests:**
- [ ] Select a project with notes/docs
- [ ] Click delete button (or use Edit Project page)
- [ ] Confirm deletion
- [ ] Check `<project>/.openmesh/` directory is deleted
- [ ] Check project is removed from sidebar
- [ ] Check `~/.openmesh/projects.json` no longer contains the path
- [ ] **CRITICAL:** Check user source files are NOT deleted
- [ ] Check other projects are not affected
- [ ] Check app does not crash

### 8. Corrupt File Recovery

**Setup:** Close the app

**Tests:**
- [ ] Open `<project>/.openmesh/tasks.json` in a text editor
- [ ] Replace content with invalid JSON: `{ this is not valid json }`
- [ ] Save the file
- [ ] Launch app
- [ ] Check app does not crash
- [ ] Check console shows warning about corrupt file
- [ ] Check `tasks.json.corrupt-<timestamp>.bak` exists
- [ ] Check `tasks.json` is recreated with default data (empty array)
- [ ] Check app continues to work normally
- [ ] Repeat test with `sessions.json`
- [ ] Repeat test with `presets.json`
- [ ] Repeat test with `recent.json`
- [ ] Repeat test with `~/.openmesh/settings.json`
- [ ] Repeat test with `~/.openmesh/projects.json`
- [ ] Repeat test with `~/.openmesh/app-state.json`

### 9. Atomic Write Test

**Setup:** This is hard to test manually, but we can verify the code

**Tests:**
- [ ] Check `storage.rs` has `atomic_write()` function
- [ ] Check all `write_global()` calls use atomic write
- [ ] Check all `write_project()` calls use atomic write
- [ ] Check temp files have `.tmp` extension
- [ ] Check rename operation is used (not copy + delete)

### 10. Git Safety Test

**Setup:** Initialize a Git repository in test project

**Tests:**
- [ ] Run `git init` in test project
- [ ] Add project to Openmesh (or delete and re-add)
- [ ] Check `.git/info/exclude` exists
- [ ] Check `.git/info/exclude` contains `.openmesh/`
- [ ] Check `.git/info/exclude` contains comment about Openmesh
- [ ] Create a note
- [ ] Run `git status`
- [ ] Check `.openmesh/` is NOT shown as untracked
- [ ] Check other files are still tracked normally
- [ ] Run `git add .`
- [ ] Run `git status`
- [ ] Check `.openmesh/` is still not staged

### 11. Export Test

**Tests:**
- [ ] Navigate to Settings page
- [ ] Click "Export Project" button
- [ ] Check JSON file is downloaded
- [ ] Open JSON file
- [ ] Check `schemaVersion` field exists and is "1.0.0"
- [ ] Check `project` field contains project metadata
- [ ] Check `docs` array contains all docs with content
- [ ] Check `notes` array contains all notes with content
- [ ] Check `tasks` array contains all tasks
- [ ] Check `sessions` array contains all sessions
- [ ] Check `presets` array contains all presets
- [ ] Check `recent` array contains recent items
- [ ] Validate JSON is well-formed

### 12. Reset All Data Test

**WARNING:** This will delete ALL Openmesh data!

**Setup:** Add multiple projects with notes/docs

**Tests:**
- [ ] Navigate to Settings page
- [ ] Click "Reset All Data" button
- [ ] Confirm the warning dialog
- [ ] Check `~/.openmesh/` is deleted
- [ ] Check all `<project>/.openmesh/` directories are deleted
- [ ] Check `~/.openmesh/` is recreated with default settings
- [ ] Check all projects are removed from sidebar
- [ ] Check app returns to "No project selected" state
- [ ] **CRITICAL:** Check user source files are NOT deleted
- [ ] Check app continues to work normally

### 13. Concurrent Access Test

**Setup:** Open app in two windows (if possible)

**Tests:**
- [ ] Open app in window 1
- [ ] Open app in window 2 (if Tauri allows)
- [ ] Edit a note in window 1
- [ ] Check note is saved
- [ ] Edit the same note in window 2
- [ ] Check note is saved (last write wins)
- [ ] Check no corruption occurs
- [ ] Check no temp files are left behind

### 14. Large Data Test

**Setup:** Create many notes/docs

**Tests:**
- [ ] Create 50 notes
- [ ] Check all notes appear in list
- [ ] Check list is scrollable
- [ ] Check performance is acceptable
- [ ] Create 50 docs
- [ ] Check all docs appear in list
- [ ] Check performance is acceptable
- [ ] Close app
- [ ] Reopen app
- [ ] Check all data persists
- [ ] Check startup time is acceptable

### 15. Path Edge Cases

**Tests:**
- [ ] Add project with spaces in path
- [ ] Add project with special characters in path
- [ ] Add project with unicode characters in path
- [ ] Add project with very long path
- [ ] Check all projects work correctly
- [ ] Check notes/docs can be created
- [ ] Check data persists correctly

### 16. Missing Project Directory

**Setup:** Delete a project directory while app is closed

**Tests:**
- [ ] Close app
- [ ] Delete project directory from filesystem
- [ ] Launch app
- [ ] Check app does not crash
- [ ] Check project still appears in sidebar (or is removed)
- [ ] Check error message is shown (if any)
- [ ] Check other projects are not affected
- [ ] Check app continues to work

### 17. Permission Denied Test

**Setup:** Make a project directory read-only

**Tests:**
- [ ] Make `<project>/.openmesh/` read-only (if possible)
- [ ] Try to create a note
- [ ] Check error message is shown
- [ ] Check app does not crash
- [ ] Restore write permissions
- [ ] Try to create a note again
- [ ] Check note is created successfully

### 18. Disk Full Test

**Setup:** This is hard to test, but we can verify error handling

**Tests:**
- [ ] Check `storage.rs` has error handling for write failures
- [ ] Check errors are propagated to frontend
- [ ] Check error messages are shown to user
- [ ] Check app does not crash on write failure

### 19. Schema Migration Test (Future)

**Note:** This test is for future versions when schema changes

**Tests:**
- [ ] Export data with v0.3 schema
- [ ] Modify `schemaVersion` to "2.0.0"
- [ ] Try to import (when import is implemented)
- [ ] Check migration warning is shown
- [ ] Check data is migrated correctly (or rejected)

### 20. Concurrent Project Access

**Setup:** Two projects with same note filename

**Tests:**
- [ ] Create `test.md` in project 1
- [ ] Create `test.md` in project 2
- [ ] Check both notes exist independently
- [ ] Edit note in project 1
- [ ] Check note in project 2 is not affected
- [ ] Check files are in separate directories

---

## Performance Tests

### Startup Time
- [ ] Measure time from launch to "ready" state
- [ ] Should be < 2 seconds on SSD
- [ ] Should be < 5 seconds on HDD

### Note List Rendering
- [ ] Create 100 notes
- [ ] Measure time to render list
- [ ] Should be < 500ms

### Search Performance
- [ ] Create 100 notes
- [ ] Search for a note
- [ ] Should be < 200ms

### Export Performance
- [ ] Create 50 notes with large content
- [ ] Export project
- [ ] Should complete in < 5 seconds

---

## Security Tests

### File Permissions
- [ ] Check `.openmesh/` directories are created with appropriate permissions
- [ ] Check JSON files are not world-readable (if possible)
- [ ] Check sensitive data is not logged to console

### Path Traversal
- [ ] Try to create note with `../` in filename
- [ ] Check file is created inside `notes/` directory
- [ ] Check no files are created outside `.openmesh/`

### SQL Injection (N/A)
- [ ] Not applicable (no SQL database)

### XSS (N/A)
- [ ] Not applicable (no web server)

---

## Data Integrity Tests

### JSON Validation
- [ ] Check all JSON files are valid
- [ ] Use `jq` or similar tool to validate
- [ ] Check no trailing commas
- [ ] Check proper escaping of special characters

### Markdown Rendering
- [ ] Create note with complex markdown
- [ ] Check rendering is correct
- [ ] Check code blocks are preserved
- [ ] Check links are preserved
- [ ] Check images are preserved (if supported)

### File Encoding
- [ ] Create note with unicode characters
- [ ] Check file is saved as UTF-8
- [ ] Check characters are preserved after reload

---

## User Experience Tests

### Error Messages
- [ ] Trigger various errors (corrupt file, permission denied, etc.)
- [ ] Check error messages are clear and actionable
- [ ] Check error messages are not technical jargon

### Loading States
- [ ] Check loading indicator is shown during async operations
- [ ] Check UI is not frozen during loading
- [ ] Check loading completes successfully

### Empty States
- [ ] Check "No project selected" state is clear
- [ ] Check "No notes yet" state is clear
- [ ] Check "No docs yet" state is clear
- [ ] Check empty states have helpful guidance

### Confirmation Dialogs
- [ ] Check delete project has confirmation
- [ ] Check reset all data has confirmation
- [ ] Check confirmation messages are clear about consequences

---

## Regression Tests

### v0.2 Features Still Work
- [ ] Add project
- [ ] Edit project
- [ ] Delete project
- [ ] Switch projects
- [ ] Open terminal
- [ ] Launch agent CLI
- [ ] Run command preset
- [ ] View git status
- [ ] Scan agent sessions

### Settings Persistence
- [ ] Change theme
- [ ] Change agent CLI paths
- [ ] Change session directories
- [ ] Close app
- [ ] Reopen app
- [ ] Check settings persist

---

## Final Checks

### Console Errors
- [ ] Check browser console for errors
- [ ] Check Rust console for errors
- [ ] All errors should be investigated and fixed

### Memory Leaks
- [ ] Use app for 30 minutes
- [ ] Check memory usage is stable
- [ ] Check no memory leaks in Task Manager

### File Handles
- [ ] Use app for 30 minutes
- [ ] Check no file handle leaks
- [ ] Check all files are closed properly

### Temp Files
- [ ] Use app for 30 minutes
- [ ] Check no `.tmp` files are left behind
- [ ] Check no `.corrupt-*.bak` files unless intentional

---

## Sign-Off

**Tester:** _________________  
**Date:** _________________  
**Result:** [ ] PASS  [ ] FAIL  
**Notes:** _______________________________________________

**Issues Found:**
1. _______________________________________________
2. _______________________________________________
3. _______________________________________________

**Recommendation:** [ ] Ready for daily use  [ ] Needs fixes  [ ] Not ready

---

**End of Checklist**
