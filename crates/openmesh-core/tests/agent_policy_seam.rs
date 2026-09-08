//! v0.2 A3 — Agent Engine entry paths must go through the shared policy seam.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn engine_loop_requires_authorized_turn() {
    let src = read(&workspace_root().join("crates/openmesh-core/src/agent_engine/engine_loop.rs"));
    assert!(
        src.contains("auth: &AuthorizedAgentTurn"),
        "run_agent_turn* must take AuthorizedAgentTurn"
    );
    assert!(
        src.contains("apply_authorized_policy"),
        "engine loop must apply policy allowlist/budget from the token"
    );
}

#[test]
fn production_entry_paths_select_an_origin() {
    let desktop = read(&workspace_root().join("src-tauri/src/agent_engine_desktop.rs"));
    assert!(desktop.contains("authorize_agent_turn"));
    assert!(desktop.contains("AgentOrigin::LocalChat"));
    assert!(desktop.contains("AgentOrigin::LocalDelegate"));

    let cli = read(&workspace_root().join("crates/openmesh-cli/src/agent.rs"));
    assert!(cli.contains("authorize_agent_turn"));
    assert!(cli.contains("local_cli"));

    let lan = read(&workspace_root().join("crates/openmesh-core/src/lan/ask.rs"));
    assert!(lan.contains("AgentOrigin::LanPeer"));

    let continuity = read(&workspace_root().join("crates/openmesh-core/src/online_proxy/ask.rs"));
    assert!(continuity.contains("AgentOrigin::ContinuityQuery"));

    let live = read(&workspace_root().join("crates/openmesh-core/src/agent_engine/live_ask.rs"));
    assert!(live.contains("authorize_agent_turn"));
    assert!(live.contains("is_live_ask"));
}

#[test]
fn live_ask_rejects_local_origins_in_source() {
    let live = read(&workspace_root().join("crates/openmesh-core/src/agent_engine/live_ask.rs"));
    assert!(live.contains("InvalidOrigin"));
}
