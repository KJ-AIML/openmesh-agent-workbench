//! Project init — create `.openmesh/` so CLI workflows can run without Desktop.

use clap::Args;
use openmesh_core::storage::{get_project_dir, init_project};
use serde_json::json;
use std::path::Path;

#[derive(Args, Debug, Clone)]
pub struct InitArgs {
    /// Project directory to initialize. Defaults to current working directory.
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_init(args: &InitArgs, cwd: &Path) -> i32 {
    let project_path = args
        .project
        .as_deref()
        .map(Path::new)
        .map(|p| {
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            }
        })
        .unwrap_or_else(|| cwd.to_path_buf());

    let project_str = project_path.to_string_lossy().to_string();
    if let Err(err) = std::fs::create_dir_all(&project_path) {
        return print_init_error(
            &format!("failed to create project directory: {err}"),
            args.json,
        );
    }

    match init_project(&project_str) {
        Ok(()) => {
            let marker = get_project_dir(&project_str);
            if args.json {
                println!(
                    "{}",
                    json!({
                        "status": "ok",
                        "project": project_str,
                        "openmeshDir": marker.to_string_lossy(),
                    })
                );
            } else {
                println!("Initialized OpenMesh project at {project_str}");
                println!("marker={}", marker.display());
            }
            0
        }
        Err(err) => print_init_error(&err, args.json),
    }
}

fn print_init_error(message: &str, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "category": "init-failed", "message": message})
        );
    } else {
        eprintln!("ERROR init-failed: {message}");
    }
    3
}
