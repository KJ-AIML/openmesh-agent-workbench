//! Dev Track 0.1.3.4 Checkpoint E — integration boundary guards and API smoke tests.
//!
//! Proves the WorkEvent ledger remains a pure `openmesh-core` surface with no
//! CLI/Tauri/Desktop/Reporter leakage and no promotion/correlation modules.

use openmesh_core::domain::{validate_event_semantics, EvidenceAttachment, EvidenceRef, WorkEvent};
use openmesh_core::events::{
    append_event, effective_summary, get_event, ledger_dir, list_corrections_for, list_events,
    validate_ledger,
};
use std::fs;
use std::path::{Path, PathBuf};

const LEDGER_FORBIDDEN_TERMS: &[&str] = &[
    "openmesh_core::events",
    "events::append_event",
    "list_corrections_for",
    "effective_summary",
    "validate_ledger",
    "classify_ledger_record",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_if_exists(path: &Path) -> Option<String> {
    path.exists()
        .then(|| fs::read_to_string(path))
        .transpose()
        .ok()
        .flatten()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn assert_files_exclude_terms(paths: &[PathBuf], terms: &[&str], label: &str) {
    for path in paths {
        let Some(content) = read_if_exists(path) else {
            continue;
        };
        for term in terms {
            assert!(
                !content.contains(term),
                "{label} must not reference ledger API `{term}`: {}",
                path.display()
            );
        }
    }
}

#[test]
fn cli_crate_has_no_work_event_ledger_surface() {
    let cli_src = workspace_root().join("crates/openmesh-cli/src");
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    assert_files_exclude_terms(&files, LEDGER_FORBIDDEN_TERMS, "CLI");
}

#[test]
fn tauri_crate_has_no_work_event_ledger_commands() {
    let tauri_lib = workspace_root().join("src-tauri/src/lib.rs");
    let content = read_if_exists(&tauri_lib).expect("src-tauri/src/lib.rs");
    for term in LEDGER_FORBIDDEN_TERMS {
        assert!(
            !content.contains(term),
            "Tauri lib must not reference ledger API `{term}`"
        );
    }
}

#[test]
fn tauri_command_count_remains_52() {
    let tauri_lib = workspace_root().join("src-tauri/src/lib.rs");
    let content = read_if_exists(&tauri_lib).expect("src-tauri/src/lib.rs");
    let count = content.matches("#[tauri::command]").count();
    assert_eq!(count, 52, "unexpected Tauri command surface expansion");
}

#[test]
fn desktop_frontend_has_no_work_event_ledger_hooks() {
    let frontend_src = workspace_root().join("src");
    let mut files = Vec::new();
    collect_rs_files(&frontend_src, &mut files);
    let mut ts_files = Vec::new();
    collect_ts_files(&frontend_src, &mut ts_files);
    files.extend(ts_files);
    assert_files_exclude_terms(&files, LEDGER_FORBIDDEN_TERMS, "Desktop frontend");
}

fn collect_ts_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_ts_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "ts" || ext == "tsx")
        {
            out.push(path);
        }
    }
}

#[test]
fn openmesh_core_has_no_promotion_or_correlation_modules() {
    let core_src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    for forbidden in [
        "promotion.rs",
        "correlation.rs",
        "suppression.rs",
        "current_state.rs",
    ] {
        assert!(
            !core_src.join(forbidden).exists(),
            "forbidden module leaked into openmesh-core: {forbidden}"
        );
    }
}

#[test]
fn signals_module_does_not_append_work_events() {
    let signals_rs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/signals.rs");
    let content = fs::read_to_string(signals_rs).expect("signals.rs");
    for term in ["append_event", "list_corrections_for", "effective_summary"] {
        assert!(
            !content.contains(term),
            "signals must not process into WorkEvent ledger (`{term}`)"
        );
    }
}

#[test]
fn evidence_ref_variants_remain_file_path_and_producer_signal_only() {
    let domain_rs = include_str!("../src/domain.rs");
    let enum_body = domain_rs
        .split("pub enum EvidenceRef")
        .nth(1)
        .and_then(|rest| rest.split('}').next())
        .expect("EvidenceRef enum");
    assert!(enum_body.contains("FilePath"));
    assert!(enum_body.contains("ProducerSignal"));
    assert!(!enum_body.contains("Git"));
    assert!(!enum_body.contains("Heli"));
}

#[test]
fn openmesh_core_does_not_depend_on_openmesh_cli() {
    let manifest = include_str!("../Cargo.toml");
    assert!(
        !manifest.contains("openmesh-cli"),
        "openmesh-core must not depend on openmesh-cli"
    );
}

#[test]
fn ledger_public_api_smoke_test() {
    let dir = std::env::temp_dir().join(format!(
        "openmesh-ledger-smoke-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = fs::remove_dir_all(&dir);
    let project_dir = dir.join("smoke-project");
    fs::create_dir_all(project_dir.join(".openmesh")).unwrap();

    let project_id = "proj-smoke-ledger";
    let now = "2026-07-08T00:00:00.000Z";
    let project_json = serde_json::json!({
        "id": project_id,
        "name": "Smoke",
        "folderPath": project_dir.to_str().unwrap(),
        "repoUrl": null,
        "defaultBranch": "main",
        "sprintSource": "none",
        "docsFolder": null,
        "terminalDir": null,
        "defaultAgentCli": null,
        "notes": null,
        "status": "active",
        "createdAt": now,
        "updatedAt": now,
    });
    fs::write(
        project_dir.join(".openmesh/project.json"),
        serde_json::to_string_pretty(&project_json).unwrap(),
    )
    .unwrap();

    let project_path = project_dir.to_string_lossy().into_owned();

    let original = WorkEvent::new(
        "evt-smoke-original",
        project_id,
        "work.completed",
        "original smoke summary",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/smoke.md".into()),
            observed_at: None,
        }],
        "2026-07-15T07:00:00Z",
    );
    validate_event_semantics(&original).expect("valid event");
    append_event(&project_path, &original).expect("append original");

    let restored = get_event(&project_path, "evt-smoke-original")
        .expect("get")
        .expect("present");
    assert_eq!(restored.summary, "original smoke summary");

    let listed = list_events(&project_path).expect("list");
    assert_eq!(listed.len(), 1);

    let report = validate_ledger(&project_path).expect("validate");
    assert_eq!(report.valid.len(), 1);

    assert_eq!(
        effective_summary(&project_path, "evt-smoke-original")
            .expect("effective")
            .as_deref(),
        Some("original smoke summary")
    );
    assert!(list_corrections_for(&project_path, "evt-smoke-original")
        .expect("corrections")
        .is_empty());

    let mut correction = WorkEvent::new(
        "evt-smoke-corr",
        project_id,
        "work.completed",
        "corrected smoke summary",
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/smoke.md".into()),
            observed_at: None,
        }],
        "2026-07-15T08:00:00Z",
    );
    correction.corrects_event_id = Some("evt-smoke-original".into());
    append_event(&project_path, &correction).expect("append correction");

    assert_eq!(
        list_events(&project_path)
            .expect("list after correction")
            .len(),
        2
    );
    assert_eq!(
        list_corrections_for(&project_path, "evt-smoke-original")
            .expect("corrections after")
            .len(),
        1
    );
    assert_eq!(
        effective_summary(&project_path, "evt-smoke-original")
            .expect("effective after")
            .as_deref(),
        Some("corrected smoke summary")
    );
    assert!(ledger_dir(&project_path).exists());
}
