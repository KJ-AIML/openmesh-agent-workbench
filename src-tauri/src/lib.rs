mod continuity_desktop;
mod agent_engine_desktop;
mod extensions_desktop;

use openmesh_core::context_service;
use openmesh_core::session_readers;
use openmesh_core::storage;
use openmesh_core::storage::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Normalize a launchable agent CLI identifier.
///
/// The app's data model (Vue types, stored sessions, project defaults) uses
/// `"claude-code"` while earlier backend code used the bare `"claude"`. Both
/// forms appear in real callers, so we accept either and canonicalize to the
/// short form used for binary lookup. This avoids any data migration.
///
/// Session *scanning* supports a wider set (cursor/gemini/grok) via
/// `session_readers::normalize_tool` — those are not launched from PATH here.
fn normalize_tool(tool: &str) -> Option<&'static str> {
    match tool {
        "codex" => Some("codex"),
        "claude" | "claude-code" => Some("claude"),
        "opencode" => Some("opencode"),
        _ => None,
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AppError {
    pub message: String,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Openmesh.", name)
}

/// Reliable host OS for chrome layout (frontend UA can be wrong in WKWebView).
#[tauri::command]
fn get_host_os() -> String {
    std::env::consts::OS.to_string()
}

#[derive(Serialize)]
struct PathValidation {
    exists: bool,
    is_directory: bool,
    is_file: bool,
    normalized_path: Option<String>,
    error: Option<String>,
}

#[tauri::command]
fn validate_path(path: String) -> PathValidation {
    let path_buf = PathBuf::from(&path);

    // Normalize the path (resolve . and ..)
    let normalized = match std::fs::canonicalize(&path_buf) {
        Ok(p) => Some(p.to_string_lossy().to_string()),
        Err(_) => None,
    };

    // Check if path exists
    let metadata = match std::fs::metadata(&path_buf) {
        Ok(m) => m,
        Err(e) => {
            return PathValidation {
                exists: false,
                is_directory: false,
                is_file: false,
                normalized_path: normalized,
                error: Some(format!("Path does not exist: {}", e)),
            };
        }
    };

    PathValidation {
        exists: true,
        is_directory: metadata.is_dir(),
        is_file: metadata.is_file(),
        normalized_path: normalized,
        error: None,
    }
}

#[derive(Serialize)]
struct OpenFolderResult {
    success: bool,
    error: Option<String>,
}

#[tauri::command]
fn open_folder(path: String) -> OpenFolderResult {
    let path_buf = PathBuf::from(&path);

    // Validate path exists and is a directory
    match std::fs::metadata(&path_buf) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return OpenFolderResult {
                    success: false,
                    error: Some("Path is not a directory".to_string()),
                };
            }
        }
        Err(e) => {
            return OpenFolderResult {
                success: false,
                error: Some(format!("Path does not exist: {}", e)),
            };
        }
    }

    // Open the folder in the system file manager
    match open::that(&path_buf) {
        Ok(_) => OpenFolderResult {
            success: true,
            error: None,
        },
        Err(e) => OpenFolderResult {
            success: false,
            error: Some(format!("Failed to open folder: {}", e)),
        },
    }
}

#[derive(Serialize)]
struct GitStatusResult {
    success: bool,
    is_repo: bool,
    branch: Option<String>,
    dirty_count: u32,
    staged_count: u32,
    untracked_count: u32,
    last_commit_hash: Option<String>,
    last_commit_message: Option<String>,
    error: Option<String>,
}

#[tauri::command]
fn get_git_status(path: String) -> GitStatusResult {
    let path_buf = PathBuf::from(&path);

    // Try to open the repository
    let repo = match git2::Repository::open(&path_buf) {
        Ok(r) => r,
        Err(e) => {
            return GitStatusResult {
                success: false,
                is_repo: false,
                branch: None,
                dirty_count: 0,
                staged_count: 0,
                untracked_count: 0,
                last_commit_hash: None,
                last_commit_message: None,
                error: Some(format!("Not a git repository: {}", e)),
            };
        }
    };

    // Get current branch
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));

    // Get last commit
    let (last_commit_hash, last_commit_message) = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .map(|commit| {
            let hash = commit.id().to_string();
            let message = commit
                .message()
                .unwrap_or("No message")
                .lines()
                .next()
                .unwrap_or("No message")
                .to_string();
            (Some(hash), Some(message))
        })
        .unwrap_or((None, None));

    // Get status counts
    let mut dirty_count = 0;
    let mut staged_count = 0;
    let mut untracked_count = 0;

    if let Ok(statuses) = repo.statuses(None) {
        for status in statuses.iter() {
            let status_flags = status.status();

            // Count untracked files
            if status_flags.contains(git2::Status::WT_NEW) {
                untracked_count += 1;
            }

            // Count modified files (working directory changes)
            if status_flags.contains(git2::Status::WT_MODIFIED)
                || status_flags.contains(git2::Status::WT_DELETED)
                || status_flags.contains(git2::Status::WT_RENAMED)
                || status_flags.contains(git2::Status::WT_TYPECHANGE)
            {
                dirty_count += 1;
            }

            // Count staged files (index changes)
            if status_flags.contains(git2::Status::INDEX_NEW)
                || status_flags.contains(git2::Status::INDEX_MODIFIED)
                || status_flags.contains(git2::Status::INDEX_DELETED)
                || status_flags.contains(git2::Status::INDEX_RENAMED)
                || status_flags.contains(git2::Status::INDEX_TYPECHANGE)
            {
                staged_count += 1;
            }
        }
    }

    GitStatusResult {
        success: true,
        is_repo: true,
        branch,
        dirty_count,
        staged_count,
        untracked_count,
        last_commit_hash,
        last_commit_message,
        error: None,
    }
}

#[derive(Serialize)]
struct TerminalLaunchResult {
    success: bool,
    error: Option<String>,
}

#[tauri::command]
fn open_terminal(cwd: String) -> TerminalLaunchResult {
    let cwd_path = PathBuf::from(&cwd);

    // Validate cwd exists and is a directory
    match std::fs::metadata(&cwd_path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return TerminalLaunchResult {
                    success: false,
                    error: Some("Path is not a directory".to_string()),
                };
            }
        }
        Err(e) => {
            return TerminalLaunchResult {
                success: false,
                error: Some(format!("Path does not exist: {}", e)),
            };
        }
    }

    // Platform-specific terminal launching
    #[cfg(target_os = "windows")]
    {
        // Try Windows Terminal first
        match Command::new("wt").arg("-d").arg(&cwd).spawn() {
            Ok(_) => {
                return TerminalLaunchResult {
                    success: true,
                    error: None,
                };
            }
            Err(_) => {
                // Windows Terminal not available, try fallback
            }
        }

        // Fallback to PowerShell
        match Command::new("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg(format!("Set-Location -Path '{}'", cwd))
            .spawn()
        {
            Ok(_) => {
                return TerminalLaunchResult {
                    success: true,
                    error: None,
                };
            }
            Err(_) => {
                // PowerShell not available, try cmd
            }
        }

        // Final fallback to cmd
        match Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .arg("/K")
            .arg(format!("cd /d \"{}\"", cwd))
            .spawn()
        {
            Ok(_) => TerminalLaunchResult {
                success: true,
                error: None,
            },
            Err(e) => TerminalLaunchResult {
                success: false,
                error: Some(format!("Failed to open terminal: {}", e)),
            },
        }
    }

    #[cfg(target_os = "macos")]
    {
        match Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&cwd)
            .spawn()
        {
            Ok(_) => TerminalLaunchResult {
                success: true,
                error: None,
            },
            Err(e) => TerminalLaunchResult {
                success: false,
                error: Some(format!("Failed to open terminal: {}", e)),
            },
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try common Linux terminals
        let terminals = ["gnome-terminal", "konsole", "xterm", "terminator"];

        for terminal in terminals.iter() {
            if let Ok(_) = Command::new(terminal)
                .arg("--working-directory")
                .arg(&cwd)
                .spawn()
            {
                return TerminalLaunchResult {
                    success: true,
                    error: None,
                };
            }
        }

        TerminalLaunchResult {
            success: false,
            error: Some("No supported terminal found. Install gnome-terminal, konsole, xterm, or terminator.".to_string()),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        TerminalLaunchResult {
            success: false,
            error: Some("Unsupported platform".to_string()),
        }
    }
}

#[derive(Serialize)]
struct AgentCliLaunchResult {
    success: bool,
    error: Option<String>,
}

#[tauri::command]
fn open_agent_cli(
    tool: String,
    cwd: String,
    cli_path: Option<String>,
    resume_session_id: Option<String>,
    extra_args: Option<Vec<String>>,
) -> AgentCliLaunchResult {
    let cwd_path = PathBuf::from(&cwd);

    // Validate cwd exists and is a directory
    match std::fs::metadata(&cwd_path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return AgentCliLaunchResult {
                    success: false,
                    error: Some("Path is not a directory".to_string()),
                };
            }
        }
        Err(e) => {
            return AgentCliLaunchResult {
                success: false,
                error: Some(format!("Path does not exist: {}", e)),
            };
        }
    }

    // Validate tool is in allowlist (accepts both "claude" and "claude-code")
    let canonical_tool = match normalize_tool(&tool) {
        Some(t) => t,
        None => {
            return AgentCliLaunchResult {
                success: false,
                error: Some(format!(
                    "Tool '{}' is not in the allowlist. Allowed: codex, claude, opencode",
                    tool
                )),
            };
        }
    };

    // Determine command to run: prefer configured path, else the canonical tool name
    let mut command = cli_path.unwrap_or_else(|| canonical_tool.to_string());
    // Append resume / extra args (no shell expansion — space-joined literals only).
    if let Some(sid) = resume_session_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if sid.chars().any(|c| c.is_whitespace() || c == ';' || c == '|' || c == '&' || c == '`' || c == '$') {
            return AgentCliLaunchResult {
                success: false,
                error: Some("invalid resume session id".into()),
            };
        }
        let resume_flag = match canonical_tool {
            "codex" => format!(" resume {sid}"),
            "claude" => format!(" --resume {sid}"),
            "opencode" => format!(" --session {sid}"),
            _ => format!(" --resume {sid}"),
        };
        command.push_str(&resume_flag);
    }
    if let Some(args) = extra_args {
        for a in args {
            let a = a.trim();
            if a.is_empty() {
                continue;
            }
            if a.chars().any(|c| c == ';' || c == '|' || c == '&' || c == '`' || c == '$' || c == '\n') {
                return AgentCliLaunchResult {
                    success: false,
                    error: Some("invalid extra arg".into()),
                };
            }
            command.push(' ');
            command.push_str(a);
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Escape double quotes inside the command for safe embedding in cmd /k "...".
        let escaped_command = command.replace('"', "\"\"");
        let cwd_escaped = cwd.replace('"', "\"\"");

        // Strategy: open a visible terminal that runs the command and stays open.
        // Priority: Windows Terminal (wt.exe) → PowerShell → cmd.exe.

        // 1. Try Windows Terminal: wt.exe -d "<cwd>" cmd /k "<command>"
        //    cmd /k keeps the shell open after the command exits.
        let wt_result = Command::new("wt")
            .arg("-d")
            .arg(&cwd)
            .arg("cmd")
            .arg("/k")
            .arg(&escaped_command)
            .spawn();

        if wt_result.is_ok() {
            if cfg!(debug_assertions) {
                eprintln!(
                    "[open_agent_cli] Windows Terminal launched: {} in {}",
                    command, cwd
                );
            }
            return AgentCliLaunchResult {
                success: true,
                error: None,
            };
        }

        // 2. Fallback to PowerShell: powershell -NoExit -Command "Set-Location '<cwd>'; & '<command>'"
        let ps_result = Command::new("powershell")
            .arg("-NoExit")
            .arg("-Command")
            .arg(format!(
                "Set-Location '{}'; & '{}'",
                cwd_escaped, escaped_command
            ))
            .spawn();

        if ps_result.is_ok() {
            if cfg!(debug_assertions) {
                eprintln!(
                    "[open_agent_cli] PowerShell launched: {} in {}",
                    command, cwd
                );
            }
            return AgentCliLaunchResult {
                success: true,
                error: None,
            };
        }

        // 3. Final fallback to cmd.exe: cmd /K "cd /d "<cwd>" && <command>"
        let cmd_result = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .arg("/K")
            .arg(format!("cd /d \"{}\" && {}", cwd_escaped, escaped_command))
            .spawn();

        match cmd_result {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    eprintln!("[open_agent_cli] cmd.exe launched: {} in {}", command, cwd);
                }
                AgentCliLaunchResult {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                let msg = format!(
                    "Could not launch {} using `{}`. It may not be installed or not available in PATH for Tauri. Try setting a command override in Settings. (Error: {})",
                    canonical_tool, command, e
                );
                if cfg!(debug_assertions) {
                    eprintln!("[open_agent_cli] All terminal launchers failed: {}", msg);
                }
                AgentCliLaunchResult {
                    success: false,
                    error: Some(msg),
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Use osascript to tell Terminal.app to open a new window, cd to cwd, and run the command.
        // The trailing "; exec bash" keeps the shell open after the command exits.
        let script = format!(
            "tell application \"Terminal\" to do script \"cd '{}' && {}; exec bash\"",
            cwd.replace('\'', "'\\''"),
            command.replace('\'', "'\\''")
        );

        match Command::new("osascript").arg("-e").arg(&script).spawn() {
            Ok(_) => {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "[open_agent_cli] Terminal.app launched: {} in {}",
                        command, cwd
                    );
                }
                AgentCliLaunchResult {
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                let msg = format!(
                    "Could not launch {} using `{}`. (Error: {})",
                    canonical_tool, command, e
                );
                AgentCliLaunchResult {
                    success: false,
                    error: Some(msg),
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        // Try common Linux terminals. Each runs the command in cwd and keeps the shell open.
        let terminals: &[(&str, &[&str])] = &[
            (
                "gnome-terminal",
                &["--working-directory", "--", "bash", "-c"],
            ),
            ("konsole", &["--workdir", "--", "bash", "-c"]),
            ("xterm", &["-e", "bash", "-c"]),
        ];

        let keep_open_cmd = format!(
            "cd '{}' && {}; exec bash",
            cwd.replace('\'', "'\\''"),
            command.replace('\'', "'\\''")
        );

        for (terminal, prefix_args) in terminals.iter() {
            let mut cmd = Command::new(terminal);
            for arg in *prefix_args {
                cmd.arg(arg);
            }
            // For gnome-terminal/konsole, the -c arg comes after --; for xterm, after -e.
            cmd.arg(&keep_open_cmd);

            if cmd.spawn().is_ok() {
                if cfg!(debug_assertions) {
                    eprintln!(
                        "[open_agent_cli] {} launched: {} in {}",
                        terminal, command, cwd
                    );
                }
                return AgentCliLaunchResult {
                    success: true,
                    error: None,
                };
            }
        }

        let msg = format!(
            "Could not launch {} using `{}`. No supported terminal found. Install gnome-terminal, konsole, or xterm.",
            canonical_tool, command
        );
        AgentCliLaunchResult {
            success: false,
            error: Some(msg),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        AgentCliLaunchResult {
            success: false,
            error: Some("Unsupported platform".to_string()),
        }
    }
}

// Phase 6: Agent Session Scanner (format-aware via openmesh-core::session_readers)
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanAgentSessionsResult {
    success: bool,
    sessions: Vec<session_readers::ScannedForeignSession>,
    error: Option<String>,
}

#[tauri::command]
fn scan_agent_sessions(
    tool: String,
    directory_path: String,
    limit: Option<u32>,
    workspace_cwd: Option<String>,
) -> ScanAgentSessionsResult {
    match session_readers::scan_agent_sessions(
        &tool,
        &directory_path,
        limit,
        workspace_cwd.as_deref(),
    ) {
        Ok(sessions) => ScanAgentSessionsResult {
            success: true,
            sessions,
            error: None,
        },
        Err(error) => ScanAgentSessionsResult {
            success: false,
            sessions: vec![],
            error: Some(error),
        },
    }
}

/// Auto-detect provider roots on this OS/device and scan sessions for the open project.
#[tauri::command]
fn scan_workspace_agent_sessions(
    workspace_cwd: String,
    limit: Option<u32>,
    overrides: Option<session_readers::SessionScanOverrides>,
) -> ScanAgentSessionsResult {
    match session_readers::scan_workspace_sessions(
        &workspace_cwd,
        limit,
        overrides.as_ref(),
    ) {
        Ok(sessions) => ScanAgentSessionsResult {
            success: true,
            sessions,
            error: None,
        },
        Err(error) => ScanAgentSessionsResult {
            success: false,
            sessions: vec![],
            error: Some(error),
        },
    }
}

#[tauri::command]
fn detect_agent_session_roots(
    overrides: Option<session_readers::SessionScanOverrides>,
) -> Vec<session_readers::DetectedProviderRoot> {
    session_readers::detect_provider_roots(overrides.as_ref())
}

// Phase 6: Command Preset Runner
#[derive(Serialize)]
struct RunCommandPresetResult {
    success: bool,
    error: Option<String>,
}

#[tauri::command]
fn run_command_preset(command: String, args: Vec<String>, cwd: String) -> RunCommandPresetResult {
    let cwd_path = PathBuf::from(&cwd);

    // Validate cwd exists and is a directory
    match std::fs::metadata(&cwd_path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return RunCommandPresetResult {
                    success: false,
                    error: Some("Working directory is not a directory".to_string()),
                };
            }
        }
        Err(e) => {
            return RunCommandPresetResult {
                success: false,
                error: Some(format!("Working directory does not exist: {}", e)),
            };
        }
    }

    // Block dangerous commands
    let dangerous_patterns = [
        "rm -rf",
        "rm -fr",
        "del /s",
        "del /f",
        "rmdir /s",
        "git reset --hard",
        "git clean -fd",
        "git push --force",
        "git push -f",
        "format c:",
        "format d:",
        "format e:",
        "mkfs",
    ];

    let full_command = format!("{} {}", command, args.join(" "));
    for pattern in dangerous_patterns.iter() {
        if full_command.contains(pattern) {
            return RunCommandPresetResult {
                success: false,
                error: Some(format!(
                    "Command blocked: contains dangerous pattern '{}'",
                    pattern
                )),
            };
        }
    }

    // Launch command in terminal
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("start").arg("cmd").arg("/K");
        cmd.arg(&command);
        for arg in &args {
            cmd.arg(arg);
        }
        cmd.current_dir(&cwd);

        match cmd.spawn() {
            Ok(_) => RunCommandPresetResult {
                success: true,
                error: None,
            },
            Err(e) => RunCommandPresetResult {
                success: false,
                error: Some(format!("Failed to run command: {}", e)),
            },
        }
    }

    #[cfg(target_os = "macos")]
    {
        // Escape single quotes for AppleScript (double them) and backslashes.
        let esc = |s: &str| s.replace('\\', "\\\\").replace('\'', "'\\''");
        let full_cmd = format!("{} {}", command, args.join(" "));
        let script = format!(
            "tell application \"Terminal\" to do script \"cd '{}' && {}\"",
            esc(&cwd),
            esc(&full_cmd)
        );
        match Command::new("osascript").arg("-e").arg(&script).spawn() {
            Ok(_) => RunCommandPresetResult {
                success: true,
                error: None,
            },
            Err(e) => RunCommandPresetResult {
                success: false,
                error: Some(format!("Failed to run command: {}", e)),
            },
        }
    }

    #[cfg(target_os = "linux")]
    {
        let terminals = ["gnome-terminal", "konsole", "xterm"];
        for terminal in terminals.iter() {
            let mut cmd = Command::new(terminal);
            cmd.arg("--working-directory").arg(&cwd);
            cmd.arg("-e").arg(format!("{} {}", command, args.join(" ")));

            if cmd.spawn().is_ok() {
                return RunCommandPresetResult {
                    success: true,
                    error: None,
                };
            }
        }

        RunCommandPresetResult {
            success: false,
            error: Some("No supported terminal found".to_string()),
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        RunCommandPresetResult {
            success: false,
            error: Some("Unsupported platform".to_string()),
        }
    }
}

// ============================================================================
// File-Based Storage Commands
// ============================================================================

// --- Global Settings ---

#[tauri::command]
fn get_settings() -> Settings {
    read_global::<Settings>("settings.json").unwrap_or_else(default_settings)
}

#[tauri::command]
fn save_settings(settings: Settings) -> Result<(), String> {
    write_global("settings.json", &settings)
}

// --- Projects List (global) ---

#[tauri::command]
fn get_projects_list() -> Vec<String> {
    read_global::<Vec<String>>("projects.json").unwrap_or_default()
}

#[tauri::command]
fn add_project_to_list(path: String) -> Result<(), String> {
    let mut projects = get_projects_list();
    if !projects.contains(&path) {
        projects.push(path);
        write_global("projects.json", &projects)?;
    }
    Ok(())
}

#[tauri::command]
fn remove_project_from_list(path: String) -> Result<(), String> {
    let projects = get_projects_list();
    let filtered: Vec<String> = projects.into_iter().filter(|p| p != &path).collect();
    write_global("projects.json", &filtered)
}

// --- App State (global) ---

#[tauri::command]
fn get_app_state() -> AppState {
    read_global::<AppState>("app-state.json").unwrap_or_else(default_app_state)
}

#[tauri::command]
fn save_app_state(state: AppState) -> Result<(), String> {
    write_global("app-state.json", &state)
}

// --- Project Init / Read / Delete ---

#[tauri::command]
fn init_project_cmd(project_path: String) -> Result<(), String> {
    init_project(&project_path)?;
    add_project_to_list(project_path).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_project(project_path: String) -> Option<Project> {
    read_project::<Project>(&project_path, "project.json")
}

#[tauri::command]
fn save_project(project_path: String, project: Project) -> Result<(), String> {
    write_project(&project_path, "project.json", &project)
}

#[tauri::command]
fn delete_project_cmd(project_path: String) -> Result<(), String> {
    delete_project_data(&project_path)?;
    remove_project_from_list(project_path).map_err(|e| e.to_string())?;
    Ok(())
}

// --- Project-Scoped Data ---

#[tauri::command]
fn get_sessions(project_path: String) -> Vec<AgentSession> {
    read_project::<Vec<AgentSession>>(&project_path, "sessions.json").unwrap_or_default()
}

#[tauri::command]
fn save_sessions(project_path: String, sessions: Vec<AgentSession>) -> Result<(), String> {
    write_project(&project_path, "sessions.json", &sessions)
}

#[tauri::command]
fn get_sprint(project_path: String) -> Option<Sprint> {
    read_project::<Sprint>(&project_path, "sprint.json")
}

#[tauri::command]
fn save_sprint(project_path: String, sprint: Sprint) -> Result<(), String> {
    write_project(&project_path, "sprint.json", &sprint)
}

#[tauri::command]
fn get_tasks(project_path: String) -> Vec<Task> {
    read_project::<Vec<Task>>(&project_path, "tasks.json").unwrap_or_default()
}

#[tauri::command]
fn save_tasks(project_path: String, tasks: Vec<Task>) -> Result<(), String> {
    write_project(&project_path, "tasks.json", &tasks)
}

#[tauri::command]
fn get_presets(project_path: String) -> Vec<CommandPreset> {
    read_project::<Vec<CommandPreset>>(&project_path, "presets.json").unwrap_or_default()
}

#[tauri::command]
fn save_presets(project_path: String, presets: Vec<CommandPreset>) -> Result<(), String> {
    write_project(&project_path, "presets.json", &presets)
}

#[tauri::command]
fn get_recent(project_path: String) -> Vec<RecentItem> {
    read_project::<Vec<RecentItem>>(&project_path, "recent.json").unwrap_or_default()
}

#[tauri::command]
fn save_recent(project_path: String, items: Vec<RecentItem>) -> Result<(), String> {
    write_project(&project_path, "recent.json", &items)
}

// --- Docs (markdown files in .openmesh/docs/) ---

#[tauri::command]
fn list_docs(project_path: String) -> Vec<FileEntry> {
    let docs_dir = get_project_dir(&project_path).join("docs");
    list_files(&docs_dir, &["md", "txt"])
}

#[tauri::command]
fn list_docs_tree(project_path: String) -> Vec<DocTreeNode> {
    let docs_dir = get_project_dir(&project_path).join("docs");
    list_docs_tree_fn(&docs_dir, "")
}

#[tauri::command]
fn read_doc(project_path: String, filename: String) -> Result<String, String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("docs"), &filename)?;
    read_file_content(&path.to_string_lossy())
}

#[tauri::command]
fn write_doc(project_path: String, filename: String, content: String) -> Result<(), String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("docs"), &filename)?;
    write_file_content(&path.to_string_lossy(), &content)
}

#[tauri::command]
fn delete_doc(project_path: String, filename: String) -> Result<(), String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("docs"), &filename)?;
    delete_file(&path.to_string_lossy())
}

#[tauri::command]
fn create_doc_folder(project_path: String, folder_name: String) -> Result<(), String> {
    create_docs_folder(&project_path, &folder_name)
}

#[tauri::command]
fn rename_doc_folder(
    project_path: String,
    old_name: String,
    new_name: String,
) -> Result<(), String> {
    rename_docs_folder(&project_path, &old_name, &new_name)
}

#[tauri::command]
fn delete_doc_folder(project_path: String, folder_name: String) -> Result<(), String> {
    delete_docs_folder(&project_path, &folder_name)
}

#[tauri::command]
fn move_doc(project_path: String, filename: String, target_folder: String) -> Result<(), String> {
    storage::move_doc_fn(&project_path, &filename, &target_folder)
}

#[tauri::command]
fn rename_doc(
    project_path: String,
    old_filename: String,
    new_filename: String,
) -> Result<(), String> {
    storage::rename_doc_fn(&project_path, &old_filename, &new_filename)
}

// --- Notes (markdown files in .openmesh/notes/) ---

#[tauri::command]
fn list_notes(project_path: String) -> Vec<FileEntry> {
    let notes_dir = get_project_dir(&project_path).join("notes");
    list_files(&notes_dir, &["md", "txt"])
}

#[tauri::command]
fn read_note(project_path: String, filename: String) -> Result<String, String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("notes"), &filename)?;
    read_file_content(&path.to_string_lossy())
}

#[tauri::command]
fn write_note(project_path: String, filename: String, content: String) -> Result<(), String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("notes"), &filename)?;
    write_file_content(&path.to_string_lossy(), &content)
}

#[tauri::command]
fn delete_note(project_path: String, filename: String) -> Result<(), String> {
    let path = safe_child_path(&get_project_dir(&project_path).join("notes"), &filename)?;
    delete_file(&path.to_string_lossy())
}

#[tauri::command]
fn rename_note(
    project_path: String,
    old_filename: String,
    new_filename: String,
) -> Result<(), String> {
    storage::rename_note_fn(&project_path, &old_filename, &new_filename)
}

#[tauri::command]
fn import_file(
    project_path: String,
    folder: String,
    filename: String,
    content: String,
) -> Result<(), String> {
    if folder != "docs" && folder != "notes" {
        return Err("Invalid import folder".to_string());
    }
    let path = safe_child_path(&get_project_dir(&project_path).join(&folder), &filename)?;
    write_file_content(&path.to_string_lossy(), &content)
}

// --- Export / Import (whole project) ---

#[tauri::command]
fn export_project(project_path: String) -> Result<String, String> {
    let project = get_project(project_path.clone());
    let sessions = get_sessions(project_path.clone());
    let tasks = get_tasks(project_path.clone());
    let presets = get_presets(project_path.clone());
    let recent = get_recent(project_path.clone());
    let sprint = get_sprint(project_path.clone());

    let docs = list_docs(project_path.clone());
    let notes = list_notes(project_path.clone());

    let export = serde_json::json!({
        "schemaVersion": SCHEMA_VERSION,
        "project": project,
        "sessions": sessions,
        "tasks": tasks,
        "presets": presets,
        "recent": recent,
        "sprint": sprint,
        "docs": docs.iter().map(|d| {
            let content = read_doc(project_path.clone(), d.name.clone()).unwrap_or_default();
            serde_json::json!({"filename": d.name, "content": content})
        }).collect::<Vec<_>>(),
        "notes": notes.iter().map(|n| {
            let content = read_note(project_path.clone(), n.name.clone()).unwrap_or_default();
            serde_json::json!({"filename": n.name, "content": content})
        }).collect::<Vec<_>>(),
    });

    serde_json::to_string_pretty(&export).map_err(|e| e.to_string())
}

// --- Reset All Data ---

#[tauri::command]
fn reset_all_data_cmd() -> Result<(), String> {
    let project_paths = get_projects_list();
    reset_all_data(&project_paths)
}

// --- Work Snapshot ---

#[derive(Serialize)]
struct WriteSnapshotResult {
    success: bool,
    filename: Option<String>,
    error: Option<String>,
}

#[tauri::command]
fn write_snapshot(project_path: String, filename: String, content: String) -> WriteSnapshotResult {
    // Sanitize filename to prevent path traversal
    let safe_filename = filename.replace("..", "").replace(['/', '\\'], "");

    if safe_filename.is_empty() {
        return WriteSnapshotResult {
            success: false,
            filename: None,
            error: Some("Invalid filename".to_string()),
        };
    }

    // Create snapshots directory if missing
    let snapshots_dir = get_project_dir(&project_path)
        .join("notes")
        .join("snapshots");

    if let Err(e) = std::fs::create_dir_all(&snapshots_dir) {
        return WriteSnapshotResult {
            success: false,
            filename: None,
            error: Some(format!("Failed to create snapshots directory: {}", e)),
        };
    }

    // Check for filename collision and append counter if needed
    let final_path = snapshots_dir.join(&safe_filename);
    let final_filename = if final_path.exists() {
        // Append counter
        let stem = safe_filename.trim_end_matches(".md");
        let mut counter = 1;
        loop {
            let candidate = format!("{}-{}.md", stem, counter);
            let candidate_path = snapshots_dir.join(&candidate);
            if !candidate_path.exists() {
                break candidate;
            }
            counter += 1;
        }
    } else {
        safe_filename.clone()
    };

    let write_path = snapshots_dir.join(&final_filename);

    match atomic_write(&write_path, &content) {
        Ok(_) => WriteSnapshotResult {
            success: true,
            filename: Some(final_filename),
            error: None,
        },
        Err(e) => WriteSnapshotResult {
            success: false,
            filename: None,
            error: Some(format!("Failed to write snapshot: {}", e)),
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_host_os,
            validate_path,
            open_folder,
            get_git_status,
            open_terminal,
            open_agent_cli,
            scan_agent_sessions,
            scan_workspace_agent_sessions,
            detect_agent_session_roots,
            run_command_preset,
            // File-based storage commands
            get_settings,
            save_settings,
            get_projects_list,
            add_project_to_list,
            remove_project_from_list,
            get_app_state,
            save_app_state,
            init_project_cmd,
            get_project,
            save_project,
            delete_project_cmd,
            get_sessions,
            save_sessions,
            get_sprint,
            save_sprint,
            get_tasks,
            save_tasks,
            get_presets,
            save_presets,
            get_recent,
            save_recent,
            list_docs,
            list_docs_tree,
            read_doc,
            write_doc,
            delete_doc,
            create_doc_folder,
            rename_doc_folder,
            delete_doc_folder,
            move_doc,
            rename_doc,
            list_notes,
            read_note,
            write_note,
            delete_note,
            rename_note,
            import_file,
            export_project,
            reset_all_data_cmd,
            write_snapshot,
            // Context Search & Inspector (0.1.2.5)
            context_search,
            context_inspect,
            context_health,
            context_refresh,
            // Desktop Continuity Surfaces (0.1.13)
            continuity_desktop::continuity_pending,
            continuity_desktop::continuity_digest,
            continuity_desktop::continuity_hub_summary,
            continuity_desktop::mesh_list_peers,
            continuity_desktop::mesh_list_envelopes,
            continuity_desktop::mesh_query_peer,
            continuity_desktop::relay_list_audit,
            continuity_desktop::online_proxy_status,
            continuity_desktop::online_proxy_init,
            continuity_desktop::online_proxy_ask,
            // Team Workspace Foundation (0.1.15)
            continuity_desktop::team_workspace_status,
            continuity_desktop::team_list_members,
            // Team Cloud Beta (0.1.16)
            continuity_desktop::team_cloud_status,
            continuity_desktop::team_cloud_sync_scaffold,
            // Trust Admin Beta (0.1.17)
            continuity_desktop::team_trust_policy_status,
            continuity_desktop::team_trust_audit_list,
            // Connector Layer (0.1.18)
            continuity_desktop::connector_list,
            continuity_desktop::org_graph_show,
            continuity_desktop::pilot_status,
            continuity_desktop::rc_status,
            // LAN Relay + Live Ask (0.1.22)
            continuity_desktop::lan_serve_start,
            continuity_desktop::lan_serve_stop,
            continuity_desktop::lan_serve_status,
            continuity_desktop::lan_discover,
            continuity_desktop::lan_list_last_peers,
            continuity_desktop::lan_list_approved_packages,
            continuity_desktop::lan_send_package,
            continuity_desktop::lan_ask_peer,
            // Agent Engine + Tool Loop (0.1.23)
            agent_engine_desktop::agent_secret_status,
            agent_engine_desktop::agent_secret_set,
            agent_engine_desktop::agent_secret_clear,
            agent_engine_desktop::agent_provider_test,
            agent_engine_desktop::agent_engine_turn,
            agent_engine_desktop::agent_workspace_tool,
            agent_engine_desktop::agent_patch_get,
            agent_engine_desktop::agent_patch_apply,
            agent_engine_desktop::agent_patch_reject,
            agent_engine_desktop::agent_patch_rollback,
            agent_engine_desktop::agent_patch_summary,
            agent_engine_desktop::agent_recipe_list,
            agent_engine_desktop::agent_recipe_run,
            agent_engine_desktop::agent_recipe_cancel,
            agent_engine_desktop::agent_recipe_get,
            agent_engine_desktop::agent_delegate_brief,
            agent_engine_desktop::agent_runs_recent,
            agent_engine_desktop::agent_handoff_approve,
            // Skills / Hooks / Plugins (local marketplace MVP)
            extensions_desktop::extensions_list,
            extensions_desktop::extensions_catalog,
            extensions_desktop::extensions_set_enabled,
            extensions_desktop::extensions_install,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn context_refresh(project_path: String) -> Result<context_service::RefreshResult, String> {
    context_service::refresh_project_context(&project_path).map_err(|e| e.to_string())
}

#[tauri::command]
fn context_search(
    project_path: String,
    query: String,
    kinds: Option<Vec<String>>,
    limit: Option<usize>,
) -> Result<Vec<context_service::ContextSearchResult>, String> {
    context_service::search_project_context(&project_path, &query, kinds, limit)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn context_inspect(
    project_path: String,
    document_id: String,
) -> Result<Option<context_service::ContextInspection>, String> {
    context_service::inspect_context_document(&project_path, &document_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn context_health(project_path: String) -> Result<context_service::ContextHealth, String> {
    context_service::get_context_index_health(&project_path).map_err(|e| e.to_string())
}
