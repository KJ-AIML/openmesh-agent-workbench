//! Dev Track 0.1.3.6 Checkpoint B — Git evidence reader integration tests.
//!
//! Uses temporary Git repositories only. Skips gracefully when system `git` is
//! unavailable; does not fake coverage.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use openmesh_core::domain::{
    validate_git_state, GitProducerError, GitProducerResult, MAX_GIT_STATE_CHANGED_PATHS,
};
use openmesh_core::producers::read_git_snapshot;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new() -> Option<Self> {
        if !system_git_available() {
            return None;
        }
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("openmesh-git-reader-{id}"));
        fs::create_dir_all(&base).expect("temp dir");
        run_git(&base, &["init"]).expect("git init");
        run_git(&base, &["config", "user.email", "git-reader@test.openmesh"]).expect("email");
        run_git(&base, &["config", "user.name", "Git Reader Test"]).expect("name");
        Some(Self { path: base })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write_file(&self, rel: &str, content: &str) {
        let file = self.path.join(rel);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).expect("parent dirs");
        }
        fs::write(file, content).expect("write file");
    }

    fn add(&self, rel: &str) {
        run_git(&self.path, &["add", rel]).expect("git add");
    }

    fn commit(&self, message: &str) {
        run_git(&self.path, &["commit", "-m", message]).expect("git commit");
    }

    fn head(&self) -> String {
        run_git(&self.path, &["rev-parse", "HEAD"]).expect("head")
    }

    fn branch(&self, name: &str) {
        run_git(&self.path, &["branch", name]).expect("branch");
    }

    fn checkout(&self, name: &str) {
        run_git(&self.path, &["checkout", name]).expect("checkout");
    }

    fn checkout_new_branch(&self, name: &str) {
        run_git(&self.path, &["checkout", "-b", name]).expect("checkout -b");
    }

    fn rename_branch(&self, name: &str) {
        run_git(&self.path, &["branch", "-M", name]).expect("branch -M");
    }

    fn set_upstream(&self, upstream: &str) {
        run_git(&self.path, &["branch", "--set-upstream-to", upstream]).expect("set upstream");
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn system_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

fn require_git_repo() -> TempGitRepo {
    TempGitRepo::new().expect("system git required for this test")
}

fn snapshot_from(result: GitProducerResult) -> openmesh_core::domain::GitSnapshot {
    match result {
        GitProducerResult::Snapshot(snapshot) => snapshot,
        other => panic!("expected snapshot, got {other:?}"),
    }
}

#[test]
fn git_reader_returns_snapshot_for_clean_repo() {
    let repo = match TempGitRepo::new() {
        Some(repo) => repo,
        None => {
            eprintln!("SKIP git_reader_returns_snapshot_for_clean_repo: git unavailable");
            return;
        }
    };
    repo.write_file("README.md", "clean\n");
    repo.add("README.md");
    repo.commit("initial");

    let result = read_git_snapshot(repo.path());
    let snapshot = snapshot_from(result);
    assert!(!snapshot.dirty);
    assert_eq!(snapshot.staged_count, 0);
    assert_eq!(snapshot.unstaged_count, 0);
    assert_eq!(snapshot.untracked_count, 0);
    assert!(snapshot.changed_paths.is_empty());
    assert_eq!(snapshot.head, repo.head());
    validate_git_state(&snapshot).expect("valid snapshot");
}

#[test]
fn git_reader_detects_dirty_repo() {
    let repo = require_git_repo();
    repo.write_file("tracked.txt", "v1\n");
    repo.add("tracked.txt");
    repo.commit("initial");
    repo.write_file("tracked.txt", "v2-different-content\n");
    repo.write_file("untracked-dirty.txt", "new\n");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert!(snapshot.dirty);
    assert!(
        snapshot.staged_count + snapshot.unstaged_count + snapshot.untracked_count > 0,
        "dirty repo should report at least one changed path category"
    );
    assert!(!snapshot.changed_paths.is_empty());
}

#[test]
fn git_reader_detects_staged_unstaged_and_untracked_counts() {
    let repo = require_git_repo();
    repo.write_file("tracked.txt", "v1\n");
    repo.add("tracked.txt");
    repo.commit("initial");

    repo.write_file("tracked.txt", "v2\n");
    repo.write_file("staged.txt", "new\n");
    repo.add("staged.txt");
    repo.write_file("untracked.txt", "ghost\n");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert!(snapshot.dirty);
    assert_eq!(snapshot.staged_count, 1);
    assert_eq!(snapshot.unstaged_count, 1);
    assert_eq!(snapshot.untracked_count, 1);
    assert!(snapshot.changed_paths.contains(&"staged.txt".to_string()));
    assert!(snapshot.changed_paths.contains(&"tracked.txt".to_string()));
    assert!(snapshot
        .changed_paths
        .contains(&"untracked.txt".to_string()));
}

#[test]
fn git_reader_bounds_changed_paths_to_64() {
    let repo = require_git_repo();
    repo.write_file("seed.txt", "seed\n");
    repo.add("seed.txt");
    repo.commit("initial");

    for i in 0..80 {
        repo.write_file(&format!("file-{i:03}.txt"), "x\n");
    }

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert_eq!(snapshot.changed_paths.len(), MAX_GIT_STATE_CHANGED_PATHS);
    assert_eq!(snapshot.untracked_count, 80);
    validate_git_state(&snapshot).expect("bounded snapshot validates");
}

#[test]
fn git_reader_sorts_changed_paths_deterministically() {
    let repo = require_git_repo();
    repo.write_file("seed.txt", "seed\n");
    repo.add("seed.txt");
    repo.commit("initial");

    for name in ["z-last.txt", "a-first.txt", "m-middle.txt"] {
        repo.write_file(name, "x\n");
    }

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    let mut sorted = snapshot.changed_paths.clone();
    sorted.sort();
    assert_eq!(snapshot.changed_paths, sorted);
    assert_eq!(
        snapshot.changed_paths,
        vec![
            "a-first.txt".to_string(),
            "m-middle.txt".to_string(),
            "z-last.txt".to_string(),
        ]
    );
}

#[test]
fn git_reader_reports_branch_and_head() {
    let repo = require_git_repo();
    repo.write_file("README.md", "clean\n");
    repo.add("README.md");
    repo.commit("initial");
    repo.checkout_new_branch("feature/git-reader");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert_eq!(snapshot.branch, "feature/git-reader");
    assert_eq!(snapshot.head.len(), 40);
    assert_eq!(snapshot.head, repo.head());
}

#[test]
fn git_reader_handles_non_git_workspace_without_panic() {
    let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("openmesh-non-git-{id}"));
    fs::create_dir_all(&path).expect("temp dir");

    let result = read_git_snapshot(&path);
    let _ = fs::remove_dir_all(&path);

    if !system_git_available() {
        assert!(matches!(
            result,
            GitProducerResult::Err(GitProducerError::GitNotAvailable)
        ));
        return;
    }

    assert!(matches!(
        result,
        GitProducerResult::Err(GitProducerError::NotARepository)
    ));
}

#[test]
fn git_reader_handles_missing_upstream_without_failure() {
    let repo = require_git_repo();
    repo.write_file("README.md", "clean\n");
    repo.add("README.md");
    repo.commit("initial");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert!(snapshot.ahead.is_none());
    assert!(snapshot.behind.is_none());
    assert!(snapshot.base_ref.is_none());
}

#[test]
fn git_reader_reports_ahead_behind_when_upstream_exists_if_feasible_in_temp_repo() {
    let repo = require_git_repo();
    repo.write_file("README.md", "v1\n");
    repo.add("README.md");
    repo.commit("initial on main");
    repo.rename_branch("main");

    repo.write_file("README.md", "v2\n");
    repo.commit_all("second on main");

    repo.checkout_new_branch("feature/upstream-test");
    repo.reset_hard("HEAD~1");

    repo.set_upstream("main");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    assert_eq!(snapshot.base_ref.as_deref(), Some("main"));
    assert_eq!(snapshot.behind, Some(1));
    assert_eq!(snapshot.ahead, Some(0));
}

#[test]
fn git_reader_does_not_persist_full_diff_or_source_content() {
    let repo = require_git_repo();
    let secret = "SECRET_SOURCE_CONTENT_SHOULD_NOT_APPEAR";
    repo.write_file("secret.rs", &format!("fn main() {{ {secret} }}\n"));
    repo.add("secret.rs");
    repo.commit("add secret");

    let snapshot = snapshot_from(read_git_snapshot(repo.path()));
    let json = serde_json::to_string(&snapshot).expect("serialize snapshot");
    assert!(!json.contains(secret));
    assert!(!json.contains("diffBody"));
    assert!(!json.contains("patch"));
    assert!(!json.contains("diff"));
}

#[test]
fn git_reader_does_not_write_signal_inbox() {
    let repo = require_git_repo();
    repo.write_file("README.md", "clean\n");
    repo.add("README.md");
    repo.commit("initial");

    let inbox = repo.path().join(".openmesh/signals/pending");
    fs::create_dir_all(&inbox).expect("inbox dir");
    let before = fs::read_dir(&inbox)
        .map(|entries| entries.count())
        .unwrap_or(0);

    let _ = read_git_snapshot(repo.path());

    let after = fs::read_dir(&inbox)
        .map(|entries| entries.count())
        .unwrap_or(0);
    assert_eq!(before, after);
}

#[test]
fn git_reader_is_read_only_and_does_not_mutate_git_state() {
    let repo = require_git_repo();
    repo.write_file("README.md", "v1\n");
    repo.add("README.md");
    repo.commit("initial");
    repo.write_file("dirty.txt", "pending\n");

    let head_before = repo.head();
    let status_before = run_git(repo.path(), &["status", "--porcelain=v1"]).expect("status");

    let _ = read_git_snapshot(repo.path());

    let head_after = repo.head();
    let status_after = run_git(repo.path(), &["status", "--porcelain=v1"]).expect("status");
    assert_eq!(head_before, head_after);
    assert_eq!(status_before, status_after);
}

trait TempGitRepoExt {
    fn commit_all(&self, message: &str);
    fn reset_hard(&self, rev: &str);
}

impl TempGitRepoExt for TempGitRepo {
    fn commit_all(&self, message: &str) {
        run_git(&self.path, &["commit", "-a", "-m", message]).expect("commit -a");
    }

    fn reset_hard(&self, rev: &str) {
        run_git(&self.path, &["reset", "--hard", rev]).expect("reset --hard");
    }
}
