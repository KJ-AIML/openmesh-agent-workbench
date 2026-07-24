//! Dev Track 0.1.6 Checkpoint E — Ask My Proxy compatibility and boundary guards.

#![allow(non_snake_case)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const APPROVED_AXGA_REVISION: &str = "f47ebba523a0b59754e3ba2eb200e55b2e7d5d35";

const FROZEN_CORE_FILE_HASHES: &[(&str, &str)] = &[
    (
        "crates/openmesh-core/src/proxy_runtime_axga.rs",
        "c248d42549a9c0e7f86f3ca140c309d9046c52556d14c6d41d71e6326f5bacc0",
    ),
    (
        "crates/openmesh-core/src/proxy_ask.rs",
        "534071703f5893a5e3fbf7de1e06e522a4286770d2110f1df63968349273b50d",
    ),
    (
        "crates/openmesh-core/src/proxy_draft_safety.rs",
        "361b28588f0e5c9da4e67afbbd3742d4c69efff094a4b5fefc81cddfcc18f34b",
    ),
    (
        "crates/openmesh-core/src/proxy_prompt.rs",
        "a14a24f6c8ee48d7ee0de5d672a22b02e94b1645418ef719270542d9914eb5b7",
    ),
    (
        "crates/openmesh-core/src/proxy_prompt_context.rs",
        "c52b66d2d6332d06761456d484bc71e284b80fcc7b1ba33ba561a09be3c6ad54",
    ),
    (
        "crates/openmesh-core/src/proxy_question.rs",
        "4fac5f59e0a8bd39f0b46ff9113e4c642e8b4cf50bef91feb8434369e563746d",
    ),
    (
        "crates/openmesh-core/src/proxy_runtime.rs",
        "8af4fbfe5755aa503c3104bb45643363e4d21a51687d5217daebe7aa8d3f42aa",
    ),
    (
        "crates/openmesh-core/src/domain.rs",
        "ba1a9bb79d21b897c75ccd59eb39ee4f5a2cd2bd536a6db3aefdd0b8cced6500",
    ),
    (
        "crates/openmesh-core/src/lib.rs",
        "29b13f9c8aef14d927a80f11e76906e35562f80589dd32f4323fa6a67acb745d",
    ),
    (
        "Cargo.lock",
        "aab2d38142da35262bdba180bf7aa5c61752fcf6f3f8a497bd03f8268fea7248",
    ),
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn sha256_bytes_powershell(bytes: &[u8]) -> String {
    let tmp = std::env::temp_dir().join(format!(
        "openmesh-sha256-{}-{}.bin",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::write(&tmp, bytes).expect("write temp hash file");
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-FileHash -Algorithm SHA256 '{}').Hash.ToLower()",
                tmp.display()
            ),
        ])
        .output()
        .expect("hash bytes");
    let _ = fs::remove_file(&tmp);
    assert!(
        output.status.success(),
        "hash command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn sha256_file(path: &Path) -> String {
    let root = workspace_root();
    if let Ok(relative_path) = path.strip_prefix(&root) {
        let relative = relative_path.to_string_lossy().replace('\\', "/");
        let canonical = Command::new("git")
            .args(["cat-file", "-p", &format!("HEAD:{relative}")])
            .current_dir(&root)
            .output();
        if let Ok(output) = canonical {
            if output.status.success() {
                return sha256_bytes_powershell(&output.stdout);
            }
        }
    }

    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "(Get-FileHash -Algorithm SHA256 '{}').Hash.ToLower()",
                path.display()
            ),
        ])
        .output()
        .expect("hash file");
    assert!(
        output.status.success(),
        "hash command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn read_if_exists(path: &Path) -> Option<String> {
    path.exists()
        .then(|| fs::read_to_string(path))
        .transpose()
        .ok()
        .flatten()
}

#[test]
fn adapter_module_remains_byte_identical() {
    let path = workspace_root().join("crates/openmesh-core/src/proxy_runtime_axga.rs");
    let actual = sha256_file(&path);
    assert_eq!(
        actual,
        FROZEN_CORE_FILE_HASHES
            .iter()
            .find(|(name, _)| *name == "crates/openmesh-core/src/proxy_runtime_axga.rs")
            .map(|(_, hash)| *hash)
            .unwrap()
    );
}

#[test]
fn ask_service_remains_byte_identical() {
    let path = workspace_root().join("crates/openmesh-core/src/proxy_ask.rs");
    assert_eq!(
        sha256_file(&path),
        "534071703f5893a5e3fbf7de1e06e522a4286770d2110f1df63968349273b50d"
    );
}

#[test]
fn checkpoint_a_d_frozen_files_remain_byte_identical() {
    let root = workspace_root();
    for (relative, expected) in FROZEN_CORE_FILE_HASHES {
        if relative.contains("proxy_runtime_axga") || relative.contains("proxy_ask.rs") {
            continue;
        }
        let path = root.join(relative);
        assert_eq!(
            sha256_file(&path),
            *expected,
            "hash mismatch for {relative}"
        );
    }
}

#[test]
fn approved_AXGA_revision_remains_unchanged() {
    let lock = fs::read_to_string(workspace_root().join("Cargo.lock")).expect("Cargo.lock");
    assert!(lock.contains(APPROVED_AXGA_REVISION));
    let core_manifest =
        fs::read_to_string(workspace_root().join("crates/openmesh-core/Cargo.toml")).expect("core");
    assert!(core_manifest.contains(APPROVED_AXGA_REVISION));
}

#[test]
fn no_axga_core_dependency() {
    let lock = fs::read_to_string(workspace_root().join("Cargo.lock")).expect("Cargo.lock");
    assert!(!lock.contains("name = \"axga-core\""));
}

#[test]
fn checkpoint_e_product_files_exist() {
    let root = workspace_root();
    assert!(root.join("crates/openmesh-cli/src/proxy.rs").exists());
    assert!(root
        .join("crates/openmesh-cli/src/proxy_runtime_factory.rs")
        .exists());
    let main_rs =
        fs::read_to_string(root.join("crates/openmesh-cli/src/main.rs")).expect("main.rs");
    assert!(main_rs.contains("mod proxy;"));
    assert!(main_rs.contains("mod proxy_runtime_factory;"));
    assert!(main_rs.contains("Proxy(proxy::ProxyCommand)"));
}

#[test]
fn openmesh_cli_has_no_direct_axga_ai_dependency() {
    let manifest =
        fs::read_to_string(workspace_root().join("crates/openmesh-cli/Cargo.toml")).expect("cli");
    assert!(!manifest.contains("axga-ai"));
    assert!(!manifest.contains("axga_ai"));
}

#[test]
fn only_runtime_factory_constructs_AxgaAiProxyDraftRuntime() {
    let cli_src = workspace_root().join("crates/openmesh-cli/src");
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    let mut constructors = Vec::new();
    for path in files {
        let Some(content) = read_if_exists(&path) else {
            continue;
        };
        if content.contains("AxgaAiProxyDraftRuntime::new") {
            constructors.push(path);
        }
    }
    assert_eq!(constructors.len(), 1);
    assert!(constructors[0]
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "proxy_runtime_factory.rs"));
}

#[test]
fn only_runtime_factory_reads_OpenMesh_proxy_selector_environment() {
    let cli_src = workspace_root().join("crates/openmesh-cli/src");
    let mut files = Vec::new();
    collect_rs_files(&cli_src, &mut files);
    let mut readers = Vec::new();
    for path in files {
        let Some(content) = read_if_exists(&path) else {
            continue;
        };
        if content.contains("OPENMESH_PROXY_PROVIDER") || content.contains("OPENMESH_PROXY_MODEL") {
            readers.push(path);
        }
    }
    assert_eq!(readers.len(), 1);
    assert!(readers[0]
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == "proxy_runtime_factory.rs"));
}

#[test]
fn command_module_does_not_read_API_keys_directly() {
    let proxy = fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs"))
        .expect("proxy.rs");
    for forbidden in [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "DEEPSEEK_API_KEY",
        "std::env::var",
    ] {
        assert!(!proxy.contains(forbidden), "proxy reads `{forbidden}`");
    }
}

#[test]
fn adapter_still_reads_no_environment_variables() {
    let adapter =
        fs::read_to_string(workspace_root().join("crates/openmesh-core/src/proxy_runtime_axga.rs"))
            .expect("adapter");
    assert!(!adapter.contains("std::env::var"));
    assert!(!adapter.contains("env::var"));
}

#[test]
fn production_factory_never_selects_stub() {
    let factory = fs::read_to_string(
        workspace_root().join("crates/openmesh-cli/src/proxy_runtime_factory.rs"),
    )
    .expect("factory");
    let production = factory.split("#[cfg(test)]").next().unwrap_or(&factory);
    assert!(!production.contains("DeterministicStubProxyDraftRuntime"));
}

#[test]
fn no_stub_environment_value_exists() {
    let sources = [
        fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs")).unwrap(),
        fs::read_to_string(
            workspace_root().join("crates/openmesh-cli/src/proxy_runtime_factory.rs"),
        )
        .unwrap(),
    ];
    for source in sources {
        let lower = source.to_ascii_lowercase();
        for forbidden in ["openmesh_proxy_stub", "test_runtime", "use_stub"] {
            assert!(!lower.contains(forbidden), "stub env `{forbidden}`");
        }
    }
}

#[test]
fn no_runtime_fallback_exists() {
    let factory = fs::read_to_string(
        workspace_root().join("crates/openmesh-cli/src/proxy_runtime_factory.rs"),
    )
    .expect("factory");
    for forbidden in ["fallback", "or_else", "unwrap_or_else"] {
        assert!(
            !factory.contains(forbidden),
            "factory contains fallback pattern `{forbidden}`"
        );
    }
}

#[test]
fn no_response_persistence_exists() {
    let proxy = fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs"))
        .expect("proxy");
    for forbidden in [
        "write_proxy",
        "persist_draft",
        "response_history",
        "fs::write",
    ] {
        assert!(!proxy.contains(forbidden), "persistence `{forbidden}`");
    }
}

#[test]
fn no_conversation_history_exists() {
    let proxy = fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs"))
        .expect("proxy");
    assert!(!proxy.contains("--history"));
    assert!(!proxy.to_ascii_lowercase().contains("conversation_history"));
}

#[test]
fn no_authority_resolver_exists() {
    let proxy = fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs"))
        .expect("proxy");
    assert!(!proxy.contains("resolve_profile_authority"));
}

#[test]
fn no_claims_or_citations_are_added() {
    let proxy = fs::read_to_string(workspace_root().join("crates/openmesh-cli/src/proxy.rs"))
        .expect("proxy");
    for forbidden in [
        "claims[]",
        "citations[]",
        "claim_citation",
        "verifiedAnswer",
    ] {
        assert!(!proxy.contains(forbidden), "0.1.7 field `{forbidden}`");
    }
}

#[test]
fn no_Tauri_command_is_added() {
    let tauri_lib = workspace_root().join("src-tauri/src/lib.rs");
    let content = fs::read_to_string(&tauri_lib).expect("tauri lib");
    let lower = content.to_ascii_lowercase();
    for forbidden in ["proxy_ask", "ask_my_proxy", "proxy draft"] {
        assert!(!lower.contains(forbidden), "tauri added `{forbidden}`");
    }
}

#[test]
fn no_frontend_behavior_is_added() {
    let frontend_src = workspace_root().join("src");
    let mut files = Vec::new();
    collect_rs_files(&frontend_src, &mut files);
    for path in files {
        let Some(content) = read_if_exists(&path) else {
            continue;
        };
        let lower = content.to_ascii_lowercase();
        for forbidden in ["proxy ask", "askmyproxy", "proxy draft"] {
            assert!(
                !lower.contains(forbidden),
                "frontend added `{forbidden}` in {}",
                path.display()
            );
        }
    }
}

#[test]
fn Tauri_command_count_remains_52() {
    let tauri_lib = fs::read_to_string(workspace_root().join("src-tauri/src/lib.rs")).unwrap();
    let count = tauri_lib.matches("#[tauri::command]").count();
    assert_eq!(count, 52);
}

fn harness_reports_dir() -> PathBuf {
    let worktree_root = workspace_root();
    let local = worktree_root.join(".heli-harness/state/reports");
    if local.join("openmesh-0.1.6-proxy-dogfood-gate.md").exists() {
        return local;
    }
    worktree_root
        .join("../../.heli-harness/state/reports")
        .canonicalize()
        .unwrap_or(local)
}

#[test]
fn checkpoint_g_remains_incomplete_pending_gb_live() {
    let reports = harness_reports_dir();
    let gate_path = reports.join("openmesh-0.1.6-proxy-dogfood-gate.md");
    assert!(
        gate_path.exists(),
        "Checkpoint G gate report must exist: {}",
        gate_path.display()
    );
    let gate = fs::read_to_string(&gate_path).expect("read checkpoint G gate report");
    assert!(
        gate.contains("G-A") && gate.contains("PASS"),
        "Checkpoint G must record G-A PASS"
    );
    assert!(
        gate.contains("NEEDS PATCH") || gate.contains("provider-failure"),
        "Checkpoint G must remain incomplete pending successful G-B live"
    );
    assert!(
        gate.contains("0 successful `ProxyDraft`") || gate.contains("NOT EXECUTED (successful)"),
        "gate must record absence of successful live ProxyDraft"
    );

    let evidence = reports.join("openmesh-0.1.6-dogfood-evidence");
    for forbidden_success in [
        "gb-success-proxy-draft.json",
        "gb-live-success-summary.txt",
        "gb-success-summary.txt",
    ] {
        assert!(
            !evidence.join(forbidden_success).exists(),
            "successful G-B evidence must not exist: {forbidden_success}"
        );
    }

    let ga_manifest = evidence.join("ga-manifest.json");
    let ga_runner = evidence.join("run-0.1.6-proxy-dogfood-ga.ps1");
    assert!(
        ga_manifest.exists() || ga_runner.exists(),
        "G-A evidence must remain on record"
    );

    assert!(!workspace_root()
        .join("crates/openmesh-cli/src/proxy_dogfood.rs")
        .exists());
    let cli_tests = workspace_root().join("crates/openmesh-cli/tests");
    for forbidden in ["proxy_live_dogfood.rs", "proxy_provider_dogfood.rs"] {
        assert!(
            !cli_tests.join(forbidden).exists(),
            "production live dogfood test file must not exist: {forbidden}"
        );
    }
    for forbidden in ["openmesh-0.1.6-checkpoint-g.md"] {
        assert!(
            !reports.join(forbidden).exists(),
            "premature full Checkpoint G closure artifact: {forbidden}"
        );
    }
}

#[test]
fn checkpoint_h_has_not_started() {
    let root = workspace_root();
    let reports = root.join(".heli-harness/state/reports");
    for forbidden in [
        "openmesh-0.1.6-checkpoint-h.md",
        "openmesh-0.1.6-final-human-review.md",
        "openmesh-0.1.6-closeout-report.md",
    ] {
        assert!(
            !reports.join(forbidden).exists(),
            "Checkpoint H artifact exists: {forbidden}"
        );
    }
    let ledger =
        fs::read_to_string(root.join("docs/development/execution-ledger.md")).expect("ledger");
    assert!(!ledger.contains("Dev Track 0.1.6 — PASS"));
    assert!(!ledger.contains("0.1.6 Checkpoint H — PASS"));
}

#[test]
fn OpenMesh_0_1_7_has_not_started() {
    let root = workspace_root();
    for forbidden in [
        "crates/openmesh-core/src/proxy_claims.rs",
        "crates/openmesh-core/src/proxy_citations.rs",
        "crates/openmesh-cli/src/proxy_verify.rs",
    ] {
        assert!(
            !root.join(forbidden).exists(),
            "0.1.7 file exists: {forbidden}"
        );
    }
}

#[test]
fn execution_ledger_is_unchanged() {
    let ledger =
        fs::read_to_string(workspace_root().join("docs/development/execution-ledger.md")).unwrap();
    assert!(!ledger.contains("0.1.6 Checkpoint E — PASS"));
    assert!(!ledger.contains("Dev Track 0.1.6 — PASS"));
}

#[test]
fn no_test_is_ignored() {
    let output = Command::new(env!("CARGO"))
        .current_dir(workspace_root())
        .args([
            "test",
            "-p",
            "openmesh-cli",
            "--test",
            "proxy_boundary",
            "--test",
            "proxy_workflow",
            "--test",
            "proxy_compatibility",
            "--",
            "--list",
        ])
        .output()
        .expect("cargo test --list");
    let listing = String::from_utf8_lossy(&output.stdout);
    for line in listing.lines() {
        if line.contains(": test") {
            assert!(!line.contains(", ignored"), "ignored test: {line}");
        }
    }
}
