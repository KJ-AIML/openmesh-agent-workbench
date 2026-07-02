use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Normalize an agent tool identifier.
///
/// The app's data model (Vue types, stored sessions, project defaults) uses
/// `"claude-code"` while earlier backend code used the bare `"claude"`. Both
/// forms appear in real callers, so we accept either and canonicalize to the
/// short form used for binary lookup. This avoids any data migration.
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
        if let Ok(_status) = Command::new("wt").arg("-d").arg(&cwd).spawn() {
            return TerminalLaunchResult {
                success: true,
                error: None,
            };
        }

        // Fallback to cmd
        match Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .arg("/K")
            .arg("cd")
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
fn open_agent_cli(tool: String, cwd: String, cli_path: Option<String>) -> AgentCliLaunchResult {
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
    let command = cli_path.unwrap_or_else(|| canonical_tool.to_string());

    // Launch the agent CLI
    match Command::new(&command).current_dir(&cwd).spawn() {
        Ok(_) => AgentCliLaunchResult {
            success: true,
            error: None,
        },
        Err(e) => AgentCliLaunchResult {
            success: false,
            error: Some(format!("Failed to launch {}: {}", canonical_tool, e)),
        },
    }
}

// Phase 6: Agent Session Scanner
#[derive(Serialize, Clone)]
struct ScannedSession {
    id: String,
    tool_name: String,
    title: String,
    session_path: String,
    file_name: String,
    created_at: String,
    last_active_at: String,
    file_size_bytes: u64,
    summary_preview: Option<String>,
    project_hint: Option<String>,
}

#[derive(Serialize)]
struct ScanAgentSessionsResult {
    success: bool,
    sessions: Vec<ScannedSession>,
    error: Option<String>,
}

#[tauri::command]
fn scan_agent_sessions(
    tool: String,
    directory_path: String,
    limit: Option<u32>,
) -> ScanAgentSessionsResult {
    let dir_path = PathBuf::from(&directory_path);

    // Validate directory exists
    match std::fs::metadata(&dir_path) {
        Ok(metadata) => {
            if !metadata.is_dir() {
                return ScanAgentSessionsResult {
                    success: false,
                    sessions: vec![],
                    error: Some("Path is not a directory".to_string()),
                };
            }
        }
        Err(e) => {
            return ScanAgentSessionsResult {
                success: false,
                sessions: vec![],
                error: Some(format!("Directory does not exist: {}", e)),
            };
        }
    }

    // Validate tool is in allowlist (accepts both "claude" and "claude-code")
    let canonical_tool = match normalize_tool(&tool) {
        Some(t) => t,
        None => {
            return ScanAgentSessionsResult {
                success: false,
                sessions: vec![],
                error: Some(format!(
                    "Tool '{}' is not in the allowlist. Allowed: codex, claude, opencode",
                    tool
                )),
            };
        }
    };

    let max_limit = limit.unwrap_or(100) as usize;
    let allowed_extensions = ["json", "jsonl", "md", "txt", "log"];
    let mut sessions = Vec::new();

    // Read directory (non-recursive, top-level only)
    if let Ok(entries) = std::fs::read_dir(&dir_path) {
        for entry in entries.flatten() {
            if sessions.len() >= max_limit {
                break;
            }

            let path = entry.path();

            // Use symlink_metadata so symlinks are detected instead of followed.
            // std::fs::metadata follows symlinks, which would make is_symlink()
            // return false and defeat the protection. Directories are skipped too.
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if metadata.is_dir() || metadata.file_type().is_symlink() {
                continue;
            }

            // Check file extension
            let ext = match path.extension().and_then(|e| e.to_str()) {
                Some(e) if allowed_extensions.contains(&e) => e,
                _ => continue,
            };
            let _ = ext; // extension used only for filtering

            // Extract file info. Follow the (now-confirmed-regular) file path for
            // size/timestamps via metadata() so symlink targets are not measured.
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let file_size = metadata.len();
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let created = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(modified);

            // Read first 500 bytes for preview, then redact common secret patterns
            // so tokens are not surfaced in the UI. This is best-effort, not a
            // security boundary; the files are local and user-owned.
            let preview = std::fs::read_to_string(&path)
                .ok()
                .map(|content| {
                    redact_secrets(content.chars().take(500).collect::<String>().as_str())
                });

            // Generate title from filename
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&file_name)
                .replace(['_', '-'], " ");

            sessions.push(ScannedSession {
                id: format!("{}_{}", canonical_tool, file_name),
                tool_name: canonical_tool.to_string(),
                title: title.chars().take(100).collect(),
                session_path: path.to_string_lossy().to_string(),
                file_name,
                created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(created as i64, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string()),
                last_active_at: chrono::DateTime::<chrono::Utc>::from_timestamp(
                    modified as i64,
                    0,
                )
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "unknown".to_string()),
                file_size_bytes: file_size,
                summary_preview: preview,
                project_hint: None,
            });
        }
    }

    ScanAgentSessionsResult {
        success: true,
        sessions,
        error: None,
    }
}

/// Best-effort redaction of common secret patterns from a session preview.
///
/// Replaces the secret value with `[REDACTED]` for a small set of common
/// token shapes. This is NOT a security boundary — the underlying files are
/// local and user-owned — but it prevents accidental exposure of credentials
/// in the UI's preview pane. Implemented without the regex crate to avoid
/// adding a dependency.
fn redact_secrets(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out: Vec<char> = Vec::with_capacity(n);
    let mut i = 0;

    while i < n {
        // Try to match a secret pattern starting at i. Each arm returns the
        // number of input chars consumed; if none match we copy one char.
        let consumed = try_redact_at(&chars, i, &mut out);
        if consumed == 0 {
            out.push(chars[i]);
            i += 1;
        } else {
            i += consumed;
        }
    }

    out.into_iter().collect()
}

/// Attempt to redact a secret beginning at `start`. Returns chars consumed.
/// On success pushes "[REDACTED]" to `out`; on no-match returns 0 and pushes nothing.
fn try_redact_at(chars: &[char], start: usize, out: &mut Vec<char>) -> usize {
    // Helper: does chars[pos..] start with `needle` (ASCII, case-insensitive)?
    let starts = |pos: usize, needle: &[u8]| -> bool {
        let lower = |c: char| c.to_ascii_lowercase() as u8;
        if pos + needle.len() > chars.len() {
            return false;
        }
        needle
            .iter()
            .enumerate()
            .all(|(k, &b)| lower(chars[pos + k]) == b)
    };

    // Assignable token prefixes (sk-, ghp_, gho_, ghu_, ghs_, gpat_, github_pat_,
    // AKIA..., AIza...). Redact the token run that follows.
    let value_prefixes: &[&[u8]] = &[
        b"sk-", b"ghp_", b"gho_", b"ghu_", b"ghs_", b"ghr_", b"aiza",
    ];
    for &prefix in value_prefixes {
        if starts(start, prefix) {
            // Consume the prefix plus a run of token chars.
            let mut j = start + prefix.len();
            while j < chars.len() && is_token_char(chars[j]) {
                j += 1;
            }
            // Only treat as a secret if at least a few chars followed the prefix,
            // to avoid false positives on stray "sk-" in prose.
            if j - (start + prefix.len()) >= 6 {
                out.extend("[REDACTED]".chars());
                return j - start;
            }
        }
    }

    // AWS access key id: AKIA followed by 12+ uppercase alphanumerics.
    if starts(start, b"akia") {
        let mut j = start + 4;
        while j < chars.len() && (chars[j].is_ascii_alphanumeric() && chars[j].is_ascii_uppercase())
        {
            j += 1;
        }
        if j - (start + 4) >= 12 {
            out.extend("[REDACTED]".chars());
            return j - start;
        }
    }

    // "Bearer <token>" — redact the token run after the keyword.
    if starts(start, b"bearer") && (start + 6 >= chars.len() || is_space_or_sep(chars[start + 6])) {
        let mut j = start + 6;
        // Skip whitespace/separators
        while j < chars.len() && is_space_or_sep(chars[j]) {
            j += 1;
        }
        let token_start = j;
        while j < chars.len() && is_token_char(chars[j]) {
            j += 1;
        }
        if j > token_start {
            out.extend("bearer ".chars());
            out.extend("[REDACTED]".chars());
            return j - start;
        }
    }

    // "key=value" assignments for common secret key names.
    // Matches: token=, api_key=, apikey=, secret=, password=, passwd=, access_key=, accesskey=
    let key_assignments: &[&[u8]] = &[
        b"token",
        b"api_key",
        b"apikey",
        b"secret",
        b"password",
        b"passwd",
        b"access_key",
        b"accesskey",
    ];
    for &key in key_assignments {
        if starts(start, key) {
            let after_key = start + key.len();
            // Optional surrounding whitespace, then '=' or ':'
            let mut j = after_key;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '=' || chars[j] == ':') {
                j += 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                let val_start = j;
                // Redact the quoted or bare value.
                if j < chars.len() && (chars[j] == '"' || chars[j] == '\'') {
                    let quote = chars[j];
                    j += 1;
                    let inner_start = j;
                    while j < chars.len() && chars[j] != quote && chars[j] != '\n' {
                        j += 1;
                    }
                    if j > inner_start {
                        out.extend_from_slice(&chars[start..val_start]);
                        out.push(chars[val_start]); // the quote
                        out.extend("[REDACTED]".chars());
                        if j < chars.len() {
                            out.push(chars[j]); // closing quote
                            j += 1;
                        }
                        return j - start;
                    }
                } else {
                    while j < chars.len() && !chars[j].is_whitespace() && chars[j] != '\n' {
                        j += 1;
                    }
                    if j > val_start {
                        out.extend_from_slice(&chars[start..val_start]);
                        out.extend("[REDACTED]".chars());
                        return j - start;
                    }
                }
            }
        }
    }

    0
}

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.'
}

fn is_space_or_sep(c: char) -> bool {
    c.is_whitespace() || c == ':'
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            validate_path,
            open_folder,
            get_git_status,
            open_terminal,
            open_agent_cli,
            scan_agent_sessions,
            run_command_preset
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
