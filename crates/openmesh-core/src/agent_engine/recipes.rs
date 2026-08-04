//! Approved verify recipes (Phase 3) — no shell expansion.

use super::patch::append_run;
use crate::storage::{atomic_write, get_project_dir, now_iso};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub title: String,
    pub argv: Vec<String>,
    #[serde(default)]
    pub cwd_rel: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    120_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunResult {
    pub recipe_id: String,
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub cancelled: bool,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub run_id: String,
}

fn recipes_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join("agent").join("recipes")
}

pub fn ensure_default_recipes(project_path: &str) -> Result<(), String> {
    let dir = recipes_dir(project_path);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let defaults = [
        Recipe {
            id: "cargo-test-core".into(),
            title: "cargo test openmesh-core lib".into(),
            argv: vec![
                "cargo".into(),
                "test".into(),
                "-p".into(),
                "openmesh-core".into(),
                "--lib".into(),
            ],
            cwd_rel: String::new(),
            timeout_ms: 180_000,
        },
        Recipe {
            id: "npm-test".into(),
            title: "npm test".into(),
            argv: vec!["npm".into(), "test".into()],
            cwd_rel: String::new(),
            timeout_ms: 180_000,
        },
        Recipe {
            id: "npm-typecheck".into(),
            title: "npm run typecheck".into(),
            argv: vec!["npm".into(), "run".into(), "typecheck".into()],
            cwd_rel: String::new(),
            timeout_ms: 120_000,
        },
    ];
    for r in defaults {
        let path = dir.join(format!("{}.json", r.id));
        if !path.exists() {
            let json = serde_json::to_string_pretty(&r).map_err(|e| e.to_string())?;
            atomic_write(&path, &json)?;
        }
    }
    Ok(())
}

pub fn list_recipes(project_path: &str) -> Result<Vec<Recipe>, String> {
    ensure_default_recipes(project_path)?;
    let dir = recipes_dir(project_path);
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        if let Ok(r) = serde_json::from_str::<Recipe>(&text) {
            out.push(r);
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

pub fn get_recipe(project_path: &str, id: &str) -> Result<Recipe, String> {
    ensure_default_recipes(project_path)?;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("invalid recipe id".into());
    }
    let path = recipes_dir(project_path).join(format!("{id}.json"));
    let text = fs::read_to_string(&path).map_err(|_| format!("recipe not found: {id}"))?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn validate_argv(argv: &[String]) -> Result<(), String> {
    if argv.is_empty() {
        return Err("recipe argv is empty".into());
    }
    for a in argv {
        if a.contains('\0') || a.contains('$') || a.contains('`') || a.contains('|') || a.contains(';')
        {
            return Err("recipe argv contains forbidden characters".into());
        }
    }
    // Block obvious destructive / remote commands in defaults usage.
    let joined = argv.join(" ").to_lowercase();
    for bad in ["git push", "rm -rf", "curl ", "wget ", "npm publish", "cargo publish"] {
        if joined.contains(bad) {
            return Err(format!("recipe blocked: contains '{bad}'"));
        }
    }
    Ok(())
}

fn cancel_map() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    static MAP: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();
    MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn cancel_recipe_run(run_key: &str) -> bool {
    if let Ok(g) = cancel_map().lock() {
        if let Some(f) = g.get(run_key) {
            f.store(true, Ordering::SeqCst);
            return true;
        }
    }
    false
}

fn register_cancel(run_key: &str) -> Arc<AtomicBool> {
    let flag = Arc::new(AtomicBool::new(false));
    if let Ok(mut g) = cancel_map().lock() {
        g.insert(run_key.to_string(), flag.clone());
    }
    flag
}

fn remove_cancel(run_key: &str) {
    if let Ok(mut g) = cancel_map().lock() {
        g.remove(run_key);
    }
}

pub type LogCallback = Arc<dyn Fn(String) + Send + Sync>;

pub fn run_recipe(
    project_path: &str,
    recipe_id: &str,
    run_key: &str,
    on_log: Option<LogCallback>,
) -> Result<RecipeRunResult, String> {
    let recipe = get_recipe(project_path, recipe_id)?;
    validate_argv(&recipe.argv)?;

    let cwd = if recipe.cwd_rel.trim().is_empty() {
        PathBuf::from(project_path)
    } else {
        let root = PathBuf::from(project_path);
        let joined = crate::storage::safe_child_path(&root, recipe.cwd_rel.trim())?;
        if !joined.is_dir() {
            return Err("recipe cwd is not a directory".into());
        }
        joined
    };

    let cancel = register_cancel(run_key);
    let started = Instant::now();
    let mut cmd = Command::new(&recipe.argv[0]);
    if recipe.argv.len() > 1 {
        cmd.args(&recipe.argv[1..]);
    }
    cmd.current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| format!("failed to spawn recipe: {e}"))?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_buf = Arc::new(Mutex::new(String::new()));
    let err_buf = Arc::new(Mutex::new(String::new()));

    fn pump_stream<R: Read + Send + 'static>(
        stream: Option<R>,
        buf: Arc<Mutex<String>>,
        on_log: Option<LogCallback>,
        prefix: &'static str,
    ) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let Some(stream) = stream else { return };
            let reader = BufReader::new(stream);
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if let Ok(mut g) = buf.lock() {
                    g.push_str(&line);
                    g.push('\n');
                }
                if let Some(cb) = &on_log {
                    cb(format!("{prefix}{line}"));
                }
            }
        })
    }

    let t_out = pump_stream(stdout, out_buf.clone(), on_log.clone(), "");
    let t_err = pump_stream(stderr, err_buf.clone(), on_log.clone(), "[err] ");

    let timeout = Duration::from_millis(recipe.timeout_ms.max(1_000));
    let mut timed_out = false;
    let mut cancelled = false;
    let exit_code = loop {
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            cancelled = true;
            break None;
        }
        if started.elapsed() > timeout {
            let _ = child.kill();
            timed_out = true;
            break None;
        }
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(e) => {
                remove_cancel(run_key);
                return Err(e.to_string());
            }
        }
    };

    let _ = t_out.join();
    let _ = t_err.join();
    remove_cancel(run_key);

    let stdout = out_buf.lock().map(|g| g.clone()).unwrap_or_default();
    let stderr = err_buf.lock().map(|g| g.clone()).unwrap_or_default();
    let duration_ms = started.elapsed().as_millis() as u64;
    let ok = !timed_out && !cancelled && exit_code == Some(0);

    let run = append_run(
        project_path,
        "verify_recipe",
        if ok {
            "ok"
        } else if cancelled {
            "cancelled"
        } else if timed_out {
            "timeout"
        } else {
            "failed"
        },
        json!({
            "recipeId": recipe_id,
            "exitCode": exit_code,
            "durationMs": duration_ms,
        }),
    )?;

    // Clip stored logs in ledger detail already small; full text returned to caller.
    let _ = now_iso();

    Ok(RecipeRunResult {
        recipe_id: recipe_id.into(),
        ok,
        exit_code,
        timed_out,
        cancelled,
        stdout: clip(&stdout, 48_000),
        stderr: clip(&stderr, 24_000),
        duration_ms,
        run_id: run.id,
    })
}

fn clip(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…(truncated)", &s[..max])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::init_project;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_project() -> String {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "openmesh-recipes-{}-{}",
            std::process::id(),
            n
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.to_string_lossy().to_string();
        init_project(&path).unwrap();
        path
    }

    #[test]
    fn seeds_and_lists_defaults() {
        let project = temp_project();
        let list = list_recipes(&project).unwrap();
        assert!(list.iter().any(|r| r.id == "npm-typecheck"));
        let _ = fs::remove_dir_all(&project);
    }

    #[test]
    fn runs_echo_recipe() {
        let project = temp_project();
        let dir = recipes_dir(&project);
        fs::create_dir_all(&dir).unwrap();
        let recipe = Recipe {
            id: "echo-hi".into(),
            title: "echo".into(),
            argv: vec!["echo".into(), "hello-openmesh".into()],
            cwd_rel: String::new(),
            timeout_ms: 5_000,
        };
        atomic_write(
            &dir.join("echo-hi.json"),
            &serde_json::to_string_pretty(&recipe).unwrap(),
        )
        .unwrap();
        let result = run_recipe(&project, "echo-hi", "test-run-1", None).unwrap();
        assert!(result.ok, "{result:?}");
        assert!(result.stdout.contains("hello-openmesh"), "{:?}", result.stdout);
        let _ = fs::remove_dir_all(&project);
    }
}
