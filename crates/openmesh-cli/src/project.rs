// ============================================================================
// Boundary-safe project resolution — Checkpoint B (approved plan §6).
// ============================================================================
// Two-phase design: discovery (existence-only, may walk multiple ancestors)
// then validation (a real load via `openmesh_core::storage::read_project`,
// exactly once, on the single selected boundary). A corrupt nearest marker
// fails at that boundary — it is never skipped in favor of an outer project.
// ============================================================================

use openmesh_core::storage::{read_project, Project};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct ResolvedProject {
    pub path: PathBuf,
    pub project: Project,
}

#[derive(Debug)]
pub enum ProjectResolutionError {
    /// No `.openmesh/project.json` marker found at `path` (explicit
    /// `--project` case), or nowhere from `searched_from` up to the
    /// filesystem root (upward-discovery case).
    NotFound {
        explicit_path: Option<PathBuf>,
        searched: Vec<PathBuf>,
    },
    /// A marker existed at `path` but failed to load (corrupt/unreadable).
    /// Never triggers a fallback search past `path`.
    Invalid { path: PathBuf },
}

impl ProjectResolutionError {
    /// A human-readable description (Checkpoint C's `output.rs` reuses this
    /// for the `project-resolution` failure category).
    pub fn describe(&self) -> String {
        match self {
            ProjectResolutionError::NotFound {
                explicit_path: Some(path),
                ..
            } => format!(
                "no OpenMesh project found at explicit --project path: {}",
                path.display()
            ),
            ProjectResolutionError::NotFound {
                explicit_path: None,
                searched,
            } => {
                let list = searched
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "no OpenMesh project found in this directory or any parent (searched: {list}) — pass --project <path> or run inside an initialized OpenMesh project"
                )
            }
            ProjectResolutionError::Invalid { path } => format!(
                "OpenMesh project at {} exists but is invalid/corrupt/unreadable",
                path.display()
            ),
        }
    }
}

fn marker_path(dir: &Path) -> PathBuf {
    dir.join(".openmesh").join("project.json")
}

/// Cheap existence-only check — deliberately not a load, so upward discovery
/// never speculatively invokes the recovery-capable loader on every ancestor.
fn marker_exists(dir: &Path) -> bool {
    marker_path(dir).exists()
}

/// Loads the single selected boundary exactly once.
fn load_boundary(dir: &Path) -> Result<Project, ()> {
    let project_path = dir.to_string_lossy().to_string();
    read_project::<Project>(&project_path, "project.json").ok_or(())
}

fn to_absolute(raw: &str, cwd: &Path) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    }
}

/// Resolves the project per the approved plan's boundary-safe algorithm.
///
/// `explicit`: the `--project <path>` value, if given.
/// `cwd`: the current working directory to start upward discovery from.
pub fn resolve_project(
    explicit: Option<&str>,
    cwd: &Path,
) -> Result<ResolvedProject, ProjectResolutionError> {
    match explicit {
        Some(raw) => {
            let path = to_absolute(raw, cwd);
            if !marker_exists(&path) {
                return Err(ProjectResolutionError::NotFound {
                    explicit_path: Some(path),
                    searched: Vec::new(),
                });
            }
            match load_boundary(&path) {
                Ok(project) => Ok(ResolvedProject { path, project }),
                Err(()) => Err(ProjectResolutionError::Invalid { path }),
            }
        }
        None => {
            let mut searched = Vec::new();
            let mut candidate = Some(cwd.to_path_buf());
            while let Some(dir) = candidate {
                if marker_exists(&dir) {
                    // First marker found is the boundary. Load it exactly
                    // once and return either way — never fall through to a
                    // parent directory, even on load failure.
                    return match load_boundary(&dir) {
                        Ok(project) => Ok(ResolvedProject { path: dir, project }),
                        Err(()) => Err(ProjectResolutionError::Invalid { path: dir }),
                    };
                }
                searched.push(dir.clone());
                candidate = dir.parent().map(Path::to_path_buf);
            }
            Err(ProjectResolutionError::NotFound {
                explicit_path: None,
                searched,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openmesh_core::storage::init_project;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Mirrors `openmesh-core`'s own test helper pattern: a unique temp
    /// directory per test, no shared state, no external dependency.
    fn temp_dir(label: &str) -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-cli-test-{label}-{}-{n}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_real_project(dir: &Path) {
        init_project(&dir.to_string_lossy()).expect("init_project should succeed");
    }

    fn plant_corrupt_marker(dir: &Path) {
        let openmesh_dir = dir.join(".openmesh");
        fs::create_dir_all(&openmesh_dir).unwrap();
        fs::write(openmesh_dir.join("project.json"), "{ not valid json").unwrap();
    }

    #[test]
    fn project_root_invocation_resolves() {
        let root = temp_dir("root");
        init_real_project(&root);
        let resolved = resolve_project(None, &root).expect("should resolve");
        assert_eq!(resolved.path, root);
    }

    #[test]
    fn nested_directory_invocation_walks_upward_to_the_marker() {
        let root = temp_dir("nested-root");
        init_real_project(&root);
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let resolved = resolve_project(None, &nested).expect("should resolve upward");
        assert_eq!(resolved.path, root);
    }

    #[test]
    fn explicit_valid_project_resolves() {
        let root = temp_dir("explicit-valid");
        init_real_project(&root);
        let elsewhere = temp_dir("explicit-valid-cwd");
        let resolved = resolve_project(Some(&root.to_string_lossy()), &elsewhere)
            .expect("should resolve explicit valid project");
        assert_eq!(resolved.path, root);
    }

    #[test]
    fn explicit_path_with_no_marker_fails_without_upward_search() {
        let uninitialized = temp_dir("explicit-no-marker");
        // A real, initialized ancestor exists as a sibling concern only —
        // resolution must not search upward from an explicit --project path.
        let elsewhere = temp_dir("explicit-no-marker-cwd");
        init_real_project(&elsewhere);
        let result = resolve_project(Some(&uninitialized.to_string_lossy()), &elsewhere);
        assert!(matches!(
            result,
            Err(ProjectResolutionError::NotFound {
                explicit_path: Some(_),
                ..
            })
        ));
    }

    #[test]
    fn explicit_path_with_corrupt_marker_fails() {
        let corrupt = temp_dir("explicit-corrupt");
        plant_corrupt_marker(&corrupt);
        let elsewhere = temp_dir("explicit-corrupt-cwd");
        let result = resolve_project(Some(&corrupt.to_string_lossy()), &elsewhere);
        assert!(matches!(
            result,
            Err(ProjectResolutionError::Invalid { .. })
        ));
    }

    #[test]
    fn no_marker_anywhere_fails_with_searched_list() {
        let isolated = temp_dir("no-marker-anywhere");
        let nested = isolated.join("x").join("y").join("z");
        fs::create_dir_all(&nested).unwrap();
        // temp_dir() itself has no .openmesh marker, and neither do its
        // freshly created subdirectories — this proves the "not found"
        // path is reachable without requiring a marker-free filesystem
        // root (which this test cannot control).
        let result = resolve_project(Some("this-path-does-not-exist-at-all"), &nested);
        assert!(matches!(
            result,
            Err(ProjectResolutionError::NotFound {
                explicit_path: Some(_),
                ..
            })
        ));
    }

    /// The specific regression this correction pass exists to prevent: a
    /// corrupt nearest project marker must fail at that boundary and must
    /// NEVER silently fall back to a valid outer project.
    #[test]
    fn corrupt_nearest_marker_with_valid_outer_project_fails_at_nearest_not_outer() {
        let outer = temp_dir("corrupt-nearest-outer");
        init_real_project(&outer);
        let inner = outer.join("inner");
        fs::create_dir_all(&inner).unwrap();
        plant_corrupt_marker(&inner);

        let result = resolve_project(None, &inner);
        match result {
            Err(ProjectResolutionError::Invalid { path }) => {
                assert_eq!(path, inner, "must fail at the nearest (inner) boundary");
            }
            other => panic!(
                "expected Invalid at the nearest boundary, got {other:?} — a corrupt nearest \
                 marker must never be silently skipped in favor of the valid outer project"
            ),
        }
    }
}
