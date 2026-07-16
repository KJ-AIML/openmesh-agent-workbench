//! Read-only local Git evidence reader via system `git` subprocess (0.1.3.6 Checkpoint B).

use std::path::Path;
use std::process::Command;

use crate::domain::{
    bound_git_changed_paths, validate_git_state, GitProducerError, GitProducerResult, GitSnapshot,
    SignalValidationError, MAX_GIT_STATE_REPO_ID_BYTES,
};

const STDERR_SNIPPET_MAX_BYTES: usize = 256;

/// Collect a bounded Git repository snapshot from `workspace_path`.
///
/// Uses read-only `git` subprocess invocations only. Never writes WorkSignals or
/// mutates Git state.
pub fn read_git_snapshot(workspace_path: &Path) -> GitProducerResult {
    if !git_executable_available() {
        return GitProducerResult::Err(GitProducerError::GitNotAvailable);
    }

    let cwd = workspace_path;
    let toplevel = match git_output(cwd, &["rev-parse", "--show-toplevel"]) {
        Ok(path) => path,
        Err(err) if is_not_a_repository(&err) => {
            return GitProducerResult::Err(GitProducerError::NotARepository);
        }
        Err(err) => return GitProducerResult::Err(err),
    };

    let head = match git_output(cwd, &["rev-parse", "HEAD"]) {
        Ok(sha) => sha,
        Err(err) if is_unborn_repository(&err) => {
            return GitProducerResult::Err(GitProducerError::ReadFailed(
                "repository has no commits".into(),
            ));
        }
        Err(err) => return GitProducerResult::Err(err),
    };

    let branch = git_output(cwd, &["branch", "--show-current"]).unwrap_or_default();
    let porcelain = match git_output(cwd, &["status", "--porcelain=v1"]) {
        Ok(status) => status,
        Err(err) => return GitProducerResult::Err(err),
    };

    let parsed = parse_porcelain_status(&porcelain);
    let upstream = read_upstream(cwd);
    let observed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let snapshot = GitSnapshot {
        repo_id: derive_repo_id(&toplevel),
        branch,
        head,
        dirty: parsed.dirty,
        staged_count: parsed.staged_count,
        unstaged_count: parsed.unstaged_count,
        untracked_count: parsed.untracked_count,
        changed_paths: parsed.changed_paths,
        observed_at,
        ahead: upstream.as_ref().map(|u| u.ahead),
        behind: upstream.as_ref().map(|u| u.behind),
        base_ref: upstream.map(|u| u.base_ref),
        worktree_root: Some(normalize_path_separators(&toplevel)),
    };

    if let Err(err) = validate_git_state(&snapshot) {
        return GitProducerResult::Err(GitProducerError::ReadFailed(format_validation_error(err)));
    }

    GitProducerResult::Snapshot(snapshot)
}

struct PorcelainParse {
    dirty: bool,
    staged_count: u32,
    unstaged_count: u32,
    untracked_count: u32,
    changed_paths: Vec<String>,
}

struct UpstreamCounts {
    base_ref: String,
    ahead: u32,
    behind: u32,
}

fn git_executable_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_output(cwd: &Path, args: &[&str]) -> Result<String, GitProducerError> {
    let output = Command::new("git")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|err| {
            if err.kind() == std::io::ErrorKind::NotFound {
                GitProducerError::GitNotAvailable
            } else {
                GitProducerError::ReadFailed(bound_stderr(&err.to_string()))
            }
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(GitProducerError::ReadFailed(bound_stderr(&stderr)))
    }
}

fn is_unborn_repository(err: &GitProducerError) -> bool {
    matches!(
        err,
        GitProducerError::ReadFailed(msg)
            if msg.contains("unborn")
                || msg.contains("does not have any commits")
                || msg.contains("unknown revision")
                || msg.contains("HEAD resolve")
    )
}

fn is_not_a_repository(err: &GitProducerError) -> bool {
    matches!(
        err,
        GitProducerError::ReadFailed(msg)
            if msg.contains("not a git repository")
                || msg.contains("Not a git repository")
                || msg.contains("fatal: not a git repository")
    )
}

fn read_upstream(cwd: &Path) -> Option<UpstreamCounts> {
    let base_ref = git_output(cwd, &["rev-parse", "--abbrev-ref", "@{upstream}"]).ok()?;
    if base_ref.is_empty() || base_ref == "@{upstream}" {
        return None;
    }

    let counts = git_output(
        cwd,
        &["rev-list", "--left-right", "--count", "@{upstream}...HEAD"],
    )
    .ok()?;
    let (behind, ahead) = parse_ahead_behind(&counts)?;

    Some(UpstreamCounts {
        base_ref,
        ahead,
        behind,
    })
}

fn parse_ahead_behind(raw: &str) -> Option<(u32, u32)> {
    let mut parts = raw.split_whitespace();
    let behind = parts.next()?.parse().ok()?;
    let ahead = parts.next()?.parse().ok()?;
    Some((behind, ahead))
}

fn parse_porcelain_status(porcelain: &str) -> PorcelainParse {
    let mut staged_paths = std::collections::BTreeSet::new();
    let mut unstaged_paths = std::collections::BTreeSet::new();
    let mut untracked_paths = std::collections::BTreeSet::new();

    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let bytes = line.as_bytes();
        if bytes[2] != b' ' {
            continue;
        }
        let index_status = bytes[0] as char;
        let worktree_status = bytes[1] as char;
        let path_part = unquote_git_path(line[3..].trim());

        if index_status == '?' {
            if let Some(path) = normalize_repo_relative_path(&path_part) {
                untracked_paths.insert(path);
            }
            continue;
        }

        let path = extract_path_identity(&path_part);
        let Some(path) = path else {
            continue;
        };

        if index_status != ' ' {
            staged_paths.insert(path.clone());
        }
        if worktree_status != ' ' {
            unstaged_paths.insert(path);
        }
    }

    let staged_count = u32::try_from(staged_paths.len()).unwrap_or(u32::MAX);
    let unstaged_count = u32::try_from(unstaged_paths.len()).unwrap_or(u32::MAX);
    let untracked_count = u32::try_from(untracked_paths.len()).unwrap_or(u32::MAX);

    let mut changed_paths: Vec<String> = staged_paths
        .into_iter()
        .chain(unstaged_paths)
        .chain(untracked_paths)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    changed_paths = bound_git_changed_paths(changed_paths);

    let dirty = staged_count > 0 || unstaged_count > 0 || untracked_count > 0;

    PorcelainParse {
        dirty,
        staged_count,
        unstaged_count,
        untracked_count,
        changed_paths,
    }
}

fn extract_path_identity(path_part: &str) -> Option<String> {
    let path = if let Some((_old, new)) = path_part.rsplit_once(" -> ") {
        new.trim()
    } else {
        path_part
    };
    normalize_repo_relative_path(path)
}

/// Strip Git porcelain double-quotes and minimal escape sequences.
fn unquote_git_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        let inner = &trimmed[1..trimmed.len() - 1];
        return inner.replace("\\\"", "\"").replace("\\\\", "\\");
    }
    trimmed.to_string()
}

fn normalize_repo_relative_path(path: &str) -> Option<String> {
    let normalized = normalize_path_separators(path.trim());
    if normalized.is_empty()
        || normalized.starts_with('/')
        || normalized.starts_with("../")
        || normalized.contains("/../")
    {
        return None;
    }
    Some(normalized)
}

fn normalize_path_separators(path: &str) -> String {
    path.replace('\\', "/")
}

fn derive_repo_id(toplevel: &str) -> String {
    let normalized = normalize_path_separators(toplevel);
    let hash = fnv1a_hex(&normalized);
    let prefix = "fnv1a-";
    let max_suffix = MAX_GIT_STATE_REPO_ID_BYTES.saturating_sub(prefix.len());
    let suffix = &hash[..hash.len().min(max_suffix)];
    format!("{prefix}{suffix}")
}

fn fnv1a_hex(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in input.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", hash)
}

fn bound_stderr(stderr: &str) -> String {
    if stderr.len() <= STDERR_SNIPPET_MAX_BYTES {
        return stderr.to_string();
    }
    stderr[..STDERR_SNIPPET_MAX_BYTES].to_string()
}

fn format_validation_error(err: SignalValidationError) -> String {
    err.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn porcelain_parser_counts_staged_unstaged_untracked_and_renames() {
        let porcelain = concat!(
            " M tracked-unstaged.txt\n",
            "M  tracked-staged.txt\n",
            "MM both.txt\n",
            "?? untracked.txt\n",
            "R  old-name.txt -> new-name.txt\n",
        );
        let parsed = parse_porcelain_status(porcelain);
        assert!(parsed.dirty);
        assert_eq!(parsed.staged_count, 3);
        assert_eq!(parsed.unstaged_count, 2);
        assert_eq!(parsed.untracked_count, 1);
        assert_eq!(
            parsed.changed_paths,
            vec![
                "both.txt".to_string(),
                "new-name.txt".to_string(),
                "tracked-staged.txt".to_string(),
                "tracked-unstaged.txt".to_string(),
                "untracked.txt".to_string(),
            ]
        );
    }

    #[test]
    fn porcelain_parser_rename_uses_new_path_identity() {
        let parsed = parse_porcelain_status("R  old-name.txt -> new-name.txt\n");
        assert_eq!(parsed.staged_count, 1);
        assert_eq!(parsed.changed_paths, vec!["new-name.txt".to_string()]);
    }

    #[test]
    fn porcelain_parser_clean_repo_is_not_dirty() {
        let parsed = parse_porcelain_status("");
        assert!(!parsed.dirty);
        assert_eq!(parsed.staged_count, 0);
        assert_eq!(parsed.unstaged_count, 0);
        assert_eq!(parsed.untracked_count, 0);
        assert!(parsed.changed_paths.is_empty());
    }

    #[test]
    fn porcelain_parser_handles_quoted_paths_with_spaces() {
        let parsed = parse_porcelain_status("?? \"path with spaces.txt\"\n");
        assert_eq!(parsed.untracked_count, 1);
        assert_eq!(
            parsed.changed_paths,
            vec!["path with spaces.txt".to_string()]
        );
    }

    #[test]
    fn porcelain_parser_rename_resolves_last_arrow_separator() {
        let parsed = parse_porcelain_status("R  weird -> name.txt -> final-name.txt\n");
        assert_eq!(parsed.changed_paths, vec!["final-name.txt".to_string()]);
    }

    #[test]
    fn unquote_git_path_strips_quotes_and_escapes() {
        assert_eq!(unquote_git_path("\"a b\""), "a b");
        assert_eq!(unquote_git_path("\"a\\\"b\""), "a\"b");
    }

    #[test]
    fn derive_repo_id_is_bounded_and_prefixed() {
        let repo_id = derive_repo_id(r"C:\repo\worktree");
        assert!(repo_id.starts_with("fnv1a-"));
        assert!(repo_id.len() <= MAX_GIT_STATE_REPO_ID_BYTES);
    }
}
