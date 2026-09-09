//! Command authorization convention for sensitive Tauri IPC.
//!
//! A webview-supplied path is not authority. Sensitive commands must:
//! 1. normalize/canonicalize the caller path
//! 2. resolve it to a registered project root (`projects.json`)
//! 3. then execute
//!
//! Applied in A4.2 to a bounded sensitive subset (not all ~180 commands):
//! - filesystem mutation: `write_snapshot`
//! - patch apply / rollback: `agent_patch_apply`, `agent_patch_rollback`
//! - PTY / process: `pty_create`, `open_terminal`, `open_agent_cli`, `run_command_preset`
//! - model turn: `agent_engine_turn`
//! - LAN start: `lan_serve_start`
//!
//! Secret mutation (`agent_secret_set` / `agent_secret_clear`) has no path:
//! keys live in user config, not a caller-supplied filesystem root.
//! Reset (`reset_all_data_cmd`) already iterates the registered list only.
//! Remaining storage commands get the same path pipeline in A4.3.

use openmesh_core::storage::read_global;
use std::path::{Path, PathBuf};

pub const ERR_INVALID_PROJECT_PATH: &str = "invalid_project_path";
pub const ERR_UNREGISTERED_PROJECT_PATH: &str = "unregistered_project_path";

/// Registered project roots from global `projects.json`.
pub fn registered_projects() -> Vec<String> {
    read_global::<Vec<String>>("projects.json").unwrap_or_default()
}

/// Canonicalize `candidate` and require it to be a registered project root
/// or a directory inside one. Fail closed on missing/malformed/escaped paths.
pub fn require_registered_project(
    candidate: &str,
    registered: &[String],
) -> Result<PathBuf, String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return Err(ERR_INVALID_PROJECT_PATH.into());
    }
    let candidate_canon = canonicalize_existing_dir(Path::new(trimmed))?;
    for raw in registered {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let Ok(root) = canonicalize_existing_dir(Path::new(raw)) else {
            continue;
        };
        if path_is_same_or_within(&candidate_canon, &root) {
            return Ok(candidate_canon);
        }
    }
    Err(ERR_UNREGISTERED_PROJECT_PATH.into())
}

/// Production helper: resolve against the live `projects.json` list.
pub fn require_registered_project_path(candidate: &str) -> Result<PathBuf, String> {
    require_registered_project(candidate, &registered_projects())
}

/// Canonical path as a host string. Strips Windows `\\?\` so terminal launchers
/// and AppleScript receive an ordinary path.
pub fn host_path_string(path: &Path) -> String {
    let lossy = path.to_string_lossy();
    #[cfg(windows)]
    {
        const VERBATIM: &str = r"\\?\";
        if let Some(rest) = lossy.strip_prefix(VERBATIM) {
            if !rest.starts_with("UNC\\") {
                return rest.to_string();
            }
        }
    }
    lossy.into_owned()
}

fn canonicalize_existing_dir(path: &Path) -> Result<PathBuf, String> {
    let meta = std::fs::metadata(path).map_err(|_| ERR_INVALID_PROJECT_PATH.to_string())?;
    if !meta.is_dir() {
        return Err(ERR_INVALID_PROJECT_PATH.into());
    }
    path.canonicalize()
        .map_err(|_| ERR_INVALID_PROJECT_PATH.to_string())
}

fn path_is_same_or_within(candidate: &Path, root: &Path) -> bool {
    candidate == root || candidate.starts_with(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn touch_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn empty_and_whitespace_are_invalid() {
        let dir = touch_dir();
        let registered = vec![dir.path().to_string_lossy().into_owned()];
        assert_eq!(
            require_registered_project("", &registered).unwrap_err(),
            ERR_INVALID_PROJECT_PATH
        );
        assert_eq!(
            require_registered_project("   ", &registered).unwrap_err(),
            ERR_INVALID_PROJECT_PATH
        );
    }

    #[test]
    fn missing_path_is_invalid() {
        let dir = touch_dir();
        let missing = dir.path().join("does-not-exist");
        let registered = vec![dir.path().to_string_lossy().into_owned()];
        assert_eq!(
            require_registered_project(&missing.to_string_lossy(), &registered).unwrap_err(),
            ERR_INVALID_PROJECT_PATH
        );
    }

    #[test]
    fn file_is_invalid() {
        let dir = touch_dir();
        let file = dir.path().join("file.txt");
        fs::write(&file, b"x").unwrap();
        let registered = vec![dir.path().to_string_lossy().into_owned()];
        assert_eq!(
            require_registered_project(&file.to_string_lossy(), &registered).unwrap_err(),
            ERR_INVALID_PROJECT_PATH
        );
    }

    #[test]
    fn registered_root_is_accepted() {
        let dir = touch_dir();
        let registered = vec![dir.path().to_string_lossy().into_owned()];
        let got = require_registered_project(&registered[0], &registered).unwrap();
        assert_eq!(got, dir.path().canonicalize().unwrap());
    }

    #[test]
    fn child_of_registered_root_is_accepted() {
        let dir = touch_dir();
        let child = dir.path().join("src");
        fs::create_dir(&child).unwrap();
        let registered = vec![dir.path().to_string_lossy().into_owned()];
        let got = require_registered_project(&child.to_string_lossy(), &registered).unwrap();
        assert_eq!(got, child.canonicalize().unwrap());
    }

    #[test]
    fn sibling_prefix_is_not_authority() {
        let parent = touch_dir();
        let proj = parent.path().join("proj");
        let decoy = parent.path().join("proj-evil");
        fs::create_dir(&proj).unwrap();
        fs::create_dir(&decoy).unwrap();
        let registered = vec![proj.to_string_lossy().into_owned()];
        assert_eq!(
            require_registered_project(&decoy.to_string_lossy(), &registered).unwrap_err(),
            ERR_UNREGISTERED_PROJECT_PATH
        );
    }

    #[test]
    fn traversal_escape_is_rejected() {
        let parent = touch_dir();
        let proj = parent.path().join("proj");
        let outside = parent.path().join("outside");
        fs::create_dir(&proj).unwrap();
        fs::create_dir(&outside).unwrap();
        let registered = vec![proj.to_string_lossy().into_owned()];
        let escape = proj.join("src").join("..").join("..").join("outside");
        fs::create_dir_all(proj.join("src")).unwrap();
        assert_eq!(
            require_registered_project(&escape.to_string_lossy(), &registered).unwrap_err(),
            ERR_UNREGISTERED_PROJECT_PATH
        );
    }

    #[test]
    fn unregistered_directory_is_rejected() {
        let a = touch_dir();
        let b = touch_dir();
        let registered = vec![a.path().to_string_lossy().into_owned()];
        assert_eq!(
            require_registered_project(&b.path().to_string_lossy(), &registered).unwrap_err(),
            ERR_UNREGISTERED_PROJECT_PATH
        );
    }

    #[test]
    fn empty_registry_rejects_existing_path() {
        let dir = touch_dir();
        assert_eq!(
            require_registered_project(&dir.path().to_string_lossy(), &[]).unwrap_err(),
            ERR_UNREGISTERED_PROJECT_PATH
        );
    }

    #[test]
    fn host_path_string_roundtrips_unix_paths() {
        let dir = touch_dir();
        let canon = dir.path().canonicalize().unwrap();
        let rendered = host_path_string(&canon);
        assert!(!rendered.is_empty());
        #[cfg(windows)]
        assert!(!rendered.starts_with(r"\\?\"));
        assert!(std::path::Path::new(&rendered).is_dir());
    }
}
