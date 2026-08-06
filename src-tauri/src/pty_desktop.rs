//! In-app PTY sessions for the Chat Terminal panel.
//!
//! Spawns a real interactive shell via `portable-pty`, streams output to the
//! frontend as `pty-data` events, and accepts stdin / resize / kill commands.
//! Multiple sessions may run concurrently (one per terminal tab).

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter, State};

const EVENT_DATA: &str = "pty-data";
const EVENT_EXIT: &str = "pty-exit";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyCreateResult {
    pub id: String,
    pub shell: String,
    pub cwd: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyDataPayload {
    id: String,
    data: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PtyExitPayload {
    id: String,
}

struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    child: Mutex<Box<dyn Child + Send + Sync>>,
}

#[derive(Default)]
pub struct PtyManager {
    sessions: Mutex<HashMap<String, PtySession>>,
}

/// Resolve an interactive shell binary and a short label for the tab UI.
pub fn resolve_shell() -> (PathBuf, String, Vec<String>) {
    #[cfg(windows)]
    {
        if let Ok(ps) = which_windows("powershell.exe") {
            return (
                ps,
                "powershell".to_string(),
                vec!["-NoLogo".to_string()],
            );
        }
        return (
            PathBuf::from("cmd.exe"),
            "cmd".to_string(),
            vec!["/K".to_string()],
        );
    }

    #[cfg(not(windows))]
    {
        let shell = std::env::var("SHELL")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_file())
            .or_else(|| {
                for candidate in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
                    let p = PathBuf::from(candidate);
                    if p.is_file() {
                        return Some(p);
                    }
                }
                None
            })
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));

        let label = shell
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("sh")
            .to_string();

        // Login shell so PATH / profile match Terminal.app expectations.
        let args = vec!["-l".to_string()];
        (shell, label, args)
    }
}

#[cfg(windows)]
fn which_windows(name: &str) -> Result<PathBuf, ()> {
    let path = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    Err(())
}

/// Validate cwd: must exist and be a directory. Empty → HOME/USERPROFILE.
pub fn resolve_cwd(cwd: &str) -> Result<PathBuf, String> {
    let trimmed = cwd.trim();
    let path = if trimmed.is_empty() {
        dirs::home_dir().ok_or_else(|| "No working directory (set HOME or open a project).".to_string())?
    } else {
        PathBuf::from(trimmed)
    };

    let meta = std::fs::metadata(&path)
        .map_err(|e| format!("Path does not exist: {e}"))?;
    if !meta.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    // Canonicalize when possible to normalize `.` / `..` for the child.
    Ok(path.canonicalize().unwrap_or(path))
}

fn spawn_session(
    app: AppHandle,
    manager: Arc<PtyManager>,
    id: String,
    cwd: PathBuf,
    cols: u16,
    rows: u16,
) -> Result<(String, String), String> {
    let (shell_path, shell_label, shell_args) = resolve_shell();
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows: rows.max(2),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to open PTY: {e}"))?;

    let mut cmd = CommandBuilder::new(&shell_path);
    for arg in &shell_args {
        cmd.arg(arg);
    }
    cmd.cwd(&cwd);
    // Ensure TERM is set for color-aware programs.
    #[cfg(unix)]
    {
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
    }

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {e}"))?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("Failed to clone PTY reader: {e}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("Failed to take PTY writer: {e}"))?;

    let session = PtySession {
        master: pair.master,
        writer: Mutex::new(writer),
        child: Mutex::new(child),
    };

    {
        let mut sessions = manager
            .sessions
            .lock()
            .map_err(|_| "PTY manager lock poisoned".to_string())?;
        if sessions.contains_key(&id) {
            // Kill the just-spawned child before failing.
            let _ = session.child.lock().ok().and_then(|mut c| c.kill().ok());
            return Err(format!("PTY session already exists: {id}"));
        }
        sessions.insert(id.clone(), session);
    }

    let read_id = id.clone();
    let read_manager = Arc::clone(&manager);
    let read_app = app.clone();
    thread::spawn(move || {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = read_app.emit(
                        EVENT_DATA,
                        PtyDataPayload {
                            id: read_id.clone(),
                            data,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        // Drop session entry if still present, then notify frontend.
        if let Ok(mut sessions) = read_manager.sessions.lock() {
            sessions.remove(&read_id);
        }
        let _ = read_app.emit(EVENT_EXIT, PtyExitPayload { id: read_id });
    });

    Ok((shell_label, cwd.display().to_string()))
}

#[tauri::command]
pub fn pty_create(
    app: AppHandle,
    state: State<'_, Arc<PtyManager>>,
    id: String,
    cwd: String,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<PtyCreateResult, String> {
    let id = id.trim().to_string();
    if id.is_empty() || id.len() > 128 {
        return Err("Invalid PTY session id".to_string());
    }
    let cwd_path = resolve_cwd(&cwd)?;
    let cols = cols.unwrap_or(80);
    let rows = rows.unwrap_or(24);
    let manager = Arc::clone(&state);
    let (shell, resolved_cwd) = spawn_session(app, manager, id.clone(), cwd_path, cols, rows)?;
    Ok(PtyCreateResult {
        id,
        shell,
        cwd: resolved_cwd,
    })
}

#[tauri::command]
pub fn pty_write(state: State<'_, Arc<PtyManager>>, id: String, data: String) -> Result<(), String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY manager lock poisoned".to_string())?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| format!("Unknown PTY session: {id}"))?;
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "PTY writer lock poisoned".to_string())?;
    writer
        .write_all(data.as_bytes())
        .map_err(|e| format!("Failed to write to PTY: {e}"))?;
    writer
        .flush()
        .map_err(|e| format!("Failed to flush PTY: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn pty_resize(
    state: State<'_, Arc<PtyManager>>,
    id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY manager lock poisoned".to_string())?;
    let session = sessions
        .get(&id)
        .ok_or_else(|| format!("Unknown PTY session: {id}"))?;
    session
        .master
        .resize(PtySize {
            rows: rows.max(2),
            cols: cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("Failed to resize PTY: {e}"))?;
    Ok(())
}

#[tauri::command]
pub fn pty_kill(state: State<'_, Arc<PtyManager>>, id: String) -> Result<(), String> {
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY manager lock poisoned".to_string())?;
    if let Some(session) = sessions.remove(&id) {
        if let Ok(mut child) = session.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    Ok(())
}

/// Kill every open PTY (page leave / app shutdown helpers).
#[tauri::command]
pub fn pty_kill_all(state: State<'_, Arc<PtyManager>>) -> Result<(), String> {
    let mut sessions = state
        .sessions
        .lock()
        .map_err(|_| "PTY manager lock poisoned".to_string())?;
    for (_id, session) in sessions.drain() {
        if let Ok(mut child) = session.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolve_shell_returns_label() {
        let (path, label, args) = resolve_shell();
        assert!(!label.is_empty());
        assert!(!args.is_empty() || cfg!(windows));
        // Path may not exist in exotic CI images; just ensure we got something.
        assert!(!path.as_os_str().is_empty());
    }

    #[test]
    fn resolve_cwd_rejects_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("not-a-dir");
        fs::write(&file, b"x").unwrap();
        let err = resolve_cwd(file.to_str().unwrap()).unwrap_err();
        assert!(err.contains("not a directory"));
    }

    #[test]
    fn resolve_cwd_accepts_directory() {
        let dir = tempfile::tempdir().unwrap();
        let resolved = resolve_cwd(dir.path().to_str().unwrap()).unwrap();
        assert!(resolved.is_dir());
    }

    #[test]
    fn resolve_cwd_empty_uses_home_or_errors() {
        match resolve_cwd("") {
            Ok(p) => assert!(p.is_dir()),
            Err(e) => assert!(e.contains("HOME") || e.contains("working directory")),
        }
    }

}
