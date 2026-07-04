use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::io::Write;

// ============================================================================
// Constants
// ============================================================================

pub const SCHEMA_VERSION: &str = "1.0.0";

// ============================================================================
// Types — mirror TypeScript types in src/types.ts
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub folder_path: String,
    pub repo_url: Option<String>,
    pub default_branch: String,
    pub sprint_source: String,
    pub docs_folder: Option<String>,
    pub terminal_dir: Option<String>,
    pub default_agent_cli: Option<String>,
    pub notes: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocSource {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub connected_path: Option<String>,
    pub is_connected: bool,
    pub file_count: Option<u32>,
    pub agent_context_enabled: bool,
    pub last_indexed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sprint {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub status: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub sprint_id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub owner: Option<String>,
    pub next_action: Option<String>,
    pub notes: Option<String>,
    pub linked_doc_ids: Vec<String>,
    pub linked_session_ids: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentItem {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub project_id: Option<String>,
    pub source_id: Option<String>,
    pub source_path: Option<String>,
    pub last_opened_at: String,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    pub id: String,
    pub tool: String,
    pub title: String,
    pub project_id: Option<String>,
    pub source_path: Option<String>,
    pub status: String,
    pub summary: Option<String>,
    pub started_at: String,
    pub last_active_at: String,
    pub ended_at: Option<String>,
    pub changed_files: Option<Vec<String>>,
    pub linked_task_id: Option<String>,
    pub is_important: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPreset {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub risk_level: String,
    pub cwd: Option<String>,
    pub description: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub workspace: WorkspaceSettings,
    pub provider: ProviderSettings,
    pub models: ModelsSettings,
    pub server: ServerSettings,
    pub agent_clis: AgentClisSettings,
    pub session_dirs: SessionDirsSettings,
    pub local_paths: LocalPathsSettings,
    pub appearance: AppearanceSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSettings {
    pub name: Option<String>,
    pub default_project_id: Option<String>,
    pub theme: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSettings {
    pub name: Option<String>,
    pub api_key_configured: bool,
    pub default_model: Option<String>,
    pub fallback_model: Option<String>,
    pub usage_tracking_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelsSettings {
    pub coding_model: Option<String>,
    pub research_model: Option<String>,
    pub summarization_model: Option<String>,
    pub local_model_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSettings {
    pub mode: String,
    pub api_base_url: String,
    pub health_status: String,
    pub sync_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentClisSettings {
    pub codex_path: Option<String>,
    pub claude_code_path: Option<String>,
    pub opencode_path: Option<String>,
    pub axga_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDirsSettings {
    pub codex_dir: Option<String>,
    pub codex_enabled: bool,
    pub claude_code_dir: Option<String>,
    pub claude_code_enabled: bool,
    pub opencode_dir: Option<String>,
    pub opencode_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPathsSettings {
    pub default_projects_dir: Option<String>,
    pub default_terminal_dir: Option<String>,
    pub data_storage_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: String,
    pub font_size: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub current_project_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}

/// Tree node for docs folder structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocTreeNode {
    pub name: String,
    /// Relative path from docs/ root (e.g. "architecture/overview.md")
    pub path: String,
    pub node_type: String, // "file" or "folder"
    pub children: Option<Vec<DocTreeNode>>,
    // File-only fields
    pub size: Option<u64>,
    pub modified_at: Option<String>,
}

// ============================================================================
// Global Storage (~/.openmesh/)
// ============================================================================

pub fn get_global_dir() -> PathBuf {
    let home = dirs::home_dir().expect("Could not determine home directory");
    let dir = home.join(".openmesh");
    if !dir.exists() {
        fs::create_dir_all(&dir).expect("Failed to create ~/.openmesh directory");
    }
    dir
}

pub fn read_global<T: serde::de::DeserializeOwned>(filename: &str) -> Option<T> {
    let path = get_global_dir().join(filename);
    read_with_recovery(&path)
}

pub fn write_global<T: serde::Serialize>(filename: &str, data: &T) -> Result<(), String> {
    let path = get_global_dir().join(filename);
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    atomic_write(&path, &content)
}

// ============================================================================
// Project Storage (<project>/.openmesh/)
// ============================================================================

pub fn get_project_dir(project_path: &str) -> PathBuf {
    PathBuf::from(project_path).join(".openmesh")
}

pub fn init_project(project_path: &str) -> Result<(), String> {
    let dir = get_project_dir(project_path);

    // Create .openmesh/ and subdirectories
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("docs")).map_err(|e| e.to_string())?;
    fs::create_dir_all(dir.join("notes")).map_err(|e| e.to_string())?;

    // Add .openmesh/ to Git exclude if this is a Git repository
    if let Err(e) = add_to_git_exclude(project_path) {
        eprintln!("Warning: Failed to add .openmesh/ to Git exclude: {}", e);
    }

    // Create default empty files if they don't exist
    let default_files = [
        "sessions.json",
        "sprint.json",
        "tasks.json",
        "presets.json",
        "recent.json",
    ];

    for file in &default_files {
        let path = dir.join(file);
        if !path.exists() {
            fs::write(&path, "[]").map_err(|e| e.to_string())?;
        }
    }

    // Create project.json with defaults
    let project_json_path = dir.join("project.json");
    if !project_json_path.exists() {
        let now = chrono::Utc::now().to_rfc3339();
        let project = Project {
            id: generate_id(),
            name: PathBuf::from(project_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string()),
            folder_path: project_path.to_string(),
            repo_url: None,
            default_branch: "main".to_string(),
            sprint_source: "none".to_string(),
            docs_folder: None,
            terminal_dir: None,
            default_agent_cli: None,
            notes: None,
            status: "active".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        write_project(project_path, "project.json", &project)?;
    }

    Ok(())
}

pub fn read_project<T: serde::de::DeserializeOwned>(
    project_path: &str,
    filename: &str,
) -> Option<T> {
    let path = get_project_dir(project_path).join(filename);
    read_with_recovery(&path)
}

pub fn write_project<T: serde::Serialize>(
    project_path: &str,
    filename: &str,
    data: &T,
) -> Result<(), String> {
    let dir = get_project_dir(project_path);
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    let path = dir.join(filename);
    let content = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    atomic_write(&path, &content)
}

pub fn delete_project_data(project_path: &str) -> Result<(), String> {
    let dir = get_project_dir(project_path);
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ============================================================================
// File Operations (for docs/notes)
// ============================================================================

pub fn list_files(dir_path: &Path, extensions: &[&str]) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    if !dir_path.exists() {
        return entries;
    }

    if let Ok(dir_entries) = fs::read_dir(dir_path) {
        for entry in dir_entries.flatten() {
            let path = entry.path();
            let metadata = match fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let name = entry
                .file_name()
                .to_string_lossy()
                .to_string();

            // Filter by extension if specified
            if !extensions.is_empty() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if !extensions.contains(&ext) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            let modified_at = metadata
                .modified()
                .ok()
                .and_then(|t| {
                    chrono::DateTime::<chrono::Utc>::from_timestamp(
                        t.duration_since(std::time::UNIX_EPOCH)
                            .ok()?
                            .as_secs() as i64,
                        0,
                    )
                    .map(|dt| dt.to_rfc3339())
                });

            entries.push(FileEntry {
                name,
                path: path.to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                size: if metadata.is_file() {
                    Some(metadata.len())
                } else {
                    None
                },
                modified_at,
            });
        }
    }

    // Sort by modified time, newest first
    entries.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    entries
}

pub fn read_file_content(path: &str) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| e.to_string())
}

pub fn write_file_content(path: &str, content: &str) -> Result<(), String> {
    // Ensure parent directory exists
    if let Some(parent) = Path::new(path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(path, content).map_err(|e| e.to_string())
}

pub fn delete_file(path: &str) -> Result<(), String> {
    if Path::new(path).exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn safe_child_path(base: &Path, relative: &str) -> Result<PathBuf, String> {
    let relative = relative.trim();
    if relative.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let mut out = base.to_path_buf();
    for component in Path::new(relative).components() {
        match component {
            Component::Normal(part) => out.push(part),
            _ => return Err("Invalid path".to_string()),
        }
    }
    Ok(out)
}

// ============================================================================
// Docs Tree (nested folder support)
// ============================================================================

/// Recursively build a tree of docs/ directory.
pub fn list_docs_tree_fn(dir_path: &Path, base_path: &str) -> Vec<DocTreeNode> {
    let mut nodes = Vec::new();

    if !dir_path.exists() {
        return nodes;
    }

    let mut entries: Vec<_> = match fs::read_dir(dir_path) {
        Ok(e) => e.filter_map(|e| e.ok()).collect(),
        Err(_) => return nodes,
    };

    // Sort: folders first, then files; alphabetical within each group
    entries.sort_by(|a, b| {
        let a_is_dir = a.path().is_dir();
        let b_is_dir = b.path().is_dir();
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for entry in entries {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip hidden files/folders
        if name.starts_with('.') {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| {
                chrono::DateTime::<chrono::Utc>::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH)
                        .ok()?
                        .as_secs() as i64,
                    0,
                )
                .map(|dt| dt.to_rfc3339())
            });

        if metadata.is_dir() {
            let child_base = if base_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", base_path, name)
            };
            let children = list_docs_tree_fn(&path, &child_base);
            nodes.push(DocTreeNode {
                name: name.clone(),
                path: child_base,
                node_type: "folder".to_string(),
                children: Some(children),
                size: None,
                modified_at,
            });
        } else {
            // Only include markdown and text files
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !["md", "txt", "markdown"].contains(&ext) {
                continue;
            }

            let relative_path = if base_path.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", base_path, name)
            };

            nodes.push(DocTreeNode {
                name: name.clone(),
                path: relative_path,
                node_type: "file".to_string(),
                children: None,
                size: Some(metadata.len()),
                modified_at,
            });
        }
    }

    nodes
}

/// Create a folder inside docs/.
pub fn create_docs_folder(project_path: &str, folder_name: &str) -> Result<(), String> {
    // Sanitize: reject path traversal attempts
    if folder_name.contains("..") || folder_name.contains('/') || folder_name.contains('\\') {
        return Err("Invalid folder name".to_string());
    }
    if folder_name.trim().is_empty() {
        return Err("Folder name cannot be empty".to_string());
    }

    let docs_dir = get_project_dir(project_path).join("docs");
    let folder_path = docs_dir.join(folder_name);

    if folder_path.exists() {
        return Err("Folder already exists".to_string());
    }

    fs::create_dir_all(&folder_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Rename a folder inside docs/.
pub fn rename_docs_folder(project_path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    let docs_dir = get_project_dir(project_path).join("docs");
    let old_path = safe_child_path(&docs_dir, old_name)?;
    let new_path = safe_child_path(&docs_dir, new_name)?;

    if !old_path.exists() {
        return Err("Folder not found".to_string());
    }
    if new_path.exists() {
        return Err("A folder with that name already exists".to_string());
    }

    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a folder inside docs/ (recursively).
pub fn delete_docs_folder(project_path: &str, folder_name: &str) -> Result<(), String> {
    let docs_dir = get_project_dir(project_path).join("docs");
    let folder_path = safe_child_path(&docs_dir, folder_name)?;

    if !folder_path.exists() {
        return Err("Folder not found".to_string());
    }

    fs::remove_dir_all(&folder_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Move a doc file to a different folder within docs/.
pub fn move_doc_fn(project_path: &str, filename: &str, target_folder: &str) -> Result<(), String> {
    let docs_dir = get_project_dir(project_path).join("docs");
    let src_path = safe_child_path(&docs_dir, filename)?;
    let dst_dir = if target_folder.is_empty() {
        docs_dir.clone()
    } else {
        safe_child_path(&docs_dir, target_folder)?
    };

    if !src_path.exists() {
        return Err("File not found".to_string());
    }

    if !dst_dir.exists() {
        fs::create_dir_all(&dst_dir).map_err(|e| e.to_string())?;
    }

    let file_name = src_path.file_name().ok_or("Invalid filename")?;
    let dst_path = dst_dir.join(file_name);

    if dst_path.exists() {
        return Err("A file with that name already exists in the target folder".to_string());
    }

    fs::rename(&src_path, &dst_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Rename a doc file within docs/.
pub fn rename_doc_fn(project_path: &str, old_filename: &str, new_filename: &str) -> Result<(), String> {
    if new_filename.trim().is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    let docs_dir = get_project_dir(project_path).join("docs");
    let old_path = safe_child_path(&docs_dir, old_filename)?;
    let new_path = safe_child_path(&docs_dir, new_filename)?;

    if !old_path.exists() {
        return Err("File not found".to_string());
    }
    if new_path.exists() {
        return Err("A file with that name already exists".to_string());
    }

    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn rename_note_fn(project_path: &str, old_filename: &str, new_filename: &str) -> Result<(), String> {
    if new_filename.trim().is_empty() {
        return Err("Filename cannot be empty".to_string());
    }

    let notes_dir = get_project_dir(project_path).join("notes");
    let old_path = safe_child_path(&notes_dir, old_filename)?;
    let new_path = safe_child_path(&notes_dir, new_filename)?;

    if !old_path.exists() {
        return Err("Note not found".to_string());
    }
    if new_path.exists() {
        return Err("A note with that name already exists".to_string());
    }

    fs::rename(&old_path, &new_path).map_err(|e| e.to_string())?;
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

pub fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{}-{}", timestamp, rand_suffix())
}

fn rand_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    format!("{:x}", seed % 0xFFFFFF)
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

// ============================================================================
// Atomic Write Helper
// ============================================================================

/// Write data to a file atomically by writing to a temp file first, then renaming.
/// This prevents corruption if the app crashes mid-write.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let temp_path = path.with_extension("tmp");
    
    // Write to temp file
    let mut file = fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp file: {}", e))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write temp file: {}", e))?;
    file.flush()
        .map_err(|e| format!("Failed to flush temp file: {}", e))?;
    
    // Rename temp file to final path (atomic on most filesystems)
    fs::rename(&temp_path, path)
        .map_err(|e| format!("Failed to rename temp file: {}", e))?;
    
    Ok(())
}

// ============================================================================
// Corrupt File Recovery
// ============================================================================

/// Read a JSON file with corrupt file recovery.
/// If the file is corrupt, backs it up and returns None.
pub fn read_with_recovery<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    if !path.exists() {
        return None;
    }
    
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            backup_corrupt_file(path);
            return None;
        }
    };
    
    match serde_json::from_str::<T>(&content) {
        Ok(data) => Some(data),
        Err(e) => {
            eprintln!("Failed to parse {}: {}", path.display(), e);
            backup_corrupt_file(path);
            None
        }
    }
}

/// Backup a corrupt file with a timestamp suffix.
fn backup_corrupt_file(path: &Path) {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let backup_path = path.with_extension(format!("corrupt-{}.bak", timestamp));
    
    if let Err(e) = fs::rename(path, &backup_path) {
        eprintln!("Failed to backup corrupt file {}: {}", path.display(), e);
    } else {
        eprintln!("Backed up corrupt file to: {}", backup_path.display());
    }
}

// ============================================================================
// Git Safety
// ============================================================================

/// Add .openmesh/ to .git/info/exclude if the project is a Git repository.
/// This prevents accidental commits of Openmesh metadata.
pub fn add_to_git_exclude(project_path: &str) -> Result<(), String> {
    let git_dir = PathBuf::from(project_path).join(".git");
    
    // Only proceed if this is a Git repository
    if !git_dir.exists() {
        return Ok(()); // Not a Git repo, nothing to do
    }
    
    let exclude_file = git_dir.join("info").join("exclude");
    
    // Create .git/info directory if it doesn't exist
    if let Some(parent) = exclude_file.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create .git/info directory: {}", e))?;
    }
    
    // Read existing exclude file
    let existing_content = fs::read_to_string(&exclude_file).unwrap_or_default();
    
    // Check if .openmesh/ is already excluded
    if existing_content.lines().any(|line| line.trim() == ".openmesh/") {
        return Ok(()); // Already excluded
    }
    
    // Append .openmesh/ to exclude file
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&exclude_file)
        .map_err(|e| format!("Failed to open exclude file: {}", e))?;
    
    writeln!(file, "\n# Openmesh metadata (auto-added by Openmesh app)")
        .map_err(|e| format!("Failed to write to exclude file: {}", e))?;
    writeln!(file, ".openmesh/")
        .map_err(|e| format!("Failed to write to exclude file: {}", e))?;
    
    Ok(())
}

// ============================================================================
// Default factories
// ============================================================================

pub fn default_settings() -> Settings {
    Settings {
        workspace: WorkspaceSettings {
            name: None,
            default_project_id: None,
            theme: "dark".to_string(),
        },
        provider: ProviderSettings {
            name: None,
            api_key_configured: false,
            default_model: None,
            fallback_model: None,
            usage_tracking_enabled: false,
        },
        models: ModelsSettings {
            coding_model: None,
            research_model: None,
            summarization_model: None,
            local_model_enabled: false,
        },
        server: ServerSettings {
            mode: "local".to_string(),
            api_base_url: "http://localhost:3000".to_string(),
            health_status: "unknown".to_string(),
            sync_status: "unknown".to_string(),
        },
        agent_clis: AgentClisSettings {
            codex_path: None,
            claude_code_path: None,
            opencode_path: None,
            axga_path: None,
        },
        session_dirs: SessionDirsSettings {
            codex_dir: None,
            codex_enabled: false,
            claude_code_dir: None,
            claude_code_enabled: false,
            opencode_dir: None,
            opencode_enabled: false,
        },
        local_paths: LocalPathsSettings {
            default_projects_dir: None,
            default_terminal_dir: None,
            data_storage_dir: None,
        },
        appearance: AppearanceSettings {
            theme: "dark".to_string(),
            font_size: "medium".to_string(),
        },
    }
}

pub fn default_app_state() -> AppState {
    AppState {
        current_project_id: None,
    }
}

// ============================================================================
// Reset All Data
// ============================================================================

/// Delete all Openmesh data from disk.
/// This deletes ~/.openmesh/ and all <project>/.openmesh/ folders.
pub fn reset_all_data(project_paths: &[String]) -> Result<(), String> {
    // Delete global ~/.openmesh/ directory
    let global_dir = get_global_dir();
    if global_dir.exists() {
        fs::remove_dir_all(&global_dir)
            .map_err(|e| format!("Failed to delete global directory: {}", e))?;
    }

    // Delete all project .openmesh/ directories
    for project_path in project_paths {
        let project_dir = get_project_dir(project_path);
        if project_dir.exists() {
            if let Err(e) = fs::remove_dir_all(&project_dir) {
                eprintln!("Warning: Failed to delete project directory {}: {}", project_path, e);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_project(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "openmesh-storage-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(path.join(".openmesh").join("docs").join("folder")).unwrap();
        path
    }

    #[test]
    fn rename_doc_keeps_nested_relative_path_inside_docs() {
        let project = temp_project("rename-doc");
        fs::write(
            project.join(".openmesh").join("docs").join("folder").join("old.md"),
            "hello",
        )
        .unwrap();

        let project_path = project.to_string_lossy().to_string();
        let result = rename_doc_fn(&project_path, "folder/old.md", "folder/new.md");

        assert!(result.is_ok(), "{result:?}");
        assert!(project.join(".openmesh/docs/folder/new.md").exists());
        assert!(!project.join(".openmesh/docs/folder/old.md").exists());

        let _ = fs::remove_dir_all(project);
    }
}
