//! Dev Track 0.1.5 Checkpoint F — Proxy Context Pack no-answer / no-authority boundary proofs.

use openmesh_core::context_pack::{
    build_proxy_context_pack, compose_proxy_context_pack, ProxyContextPackBuildOptions,
    ProxyContextPackComposeInputs,
};
use openmesh_core::continuity::{
    build_catch_up_view, build_current_state_projection, load_continuity_input_snapshot,
};
use openmesh_core::domain::{
    default_work_proxy_profile, validate_proxy_context_pack, AuthorityRule, ProxyAuthorityLevel,
    ProxyContextPack, CONTEXT_PACK_EXECUTION_BOUNDARY, PROXY_CONTEXT_PACK_PROTOCOL_VERSION,
};
use openmesh_core::events::append_event;
use openmesh_core::profile::{read_work_proxy_profile, write_work_proxy_profile};
use openmesh_core::storage::init_project;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const FIXTURE: &str = include_str!("fixtures/context/proxy-context-pack-valid.json");
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";
const WINDOW_UNTIL: &str = "2026-07-18T00:00:00Z";
const GENERATED_AT: &str = "2026-07-18T04:00:00Z";

const FORBIDDEN_QA_WIRE_KEYS: &[&str] = &[
    "question",
    "query",
    "prompt",
    "answer",
    "response",
    "generatedResponse",
    "draftContent",
    "suggestionContent",
    "conversation",
    "chatHistory",
    "messages",
    "chat",
];

const FORBIDDEN_RUNTIME_WIRE_KEYS: &[&str] = &[
    "model",
    "provider",
    "temperature",
    "toolCalls",
    "toolCall",
    "tools",
    "approval",
    "approvals",
    "delegation",
    "tokenCount",
    "maxTokens",
];

fn fixture_pack() -> ProxyContextPack {
    serde_json::from_str(FIXTURE).expect("fixture pack")
}

fn collect_wire_keys(value: &Value, prefix: &str, out: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                out.insert(path.clone());
                collect_wire_keys(child, &path, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_wire_keys(child, &format!("{prefix}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn leaf_key(path: &str) -> &str {
    path.rsplit('.')
        .next()
        .unwrap_or(path)
        .split('[')
        .next()
        .unwrap_or(path)
}

fn wire_keys_for_pack(pack: &ProxyContextPack) -> BTreeSet<String> {
    let value = serde_json::to_value(pack).expect("serialize pack");
    let mut keys = BTreeSet::new();
    collect_wire_keys(&value, "", &mut keys);
    keys
}

fn assert_no_exact_wire_keys(keys: &BTreeSet<String>, forbidden: &[&str]) {
    for path in keys {
        let leaf = leaf_key(path);
        for banned in forbidden {
            assert_ne!(
                leaf, *banned,
                "forbidden wire key `{banned}` found at `{path}`"
            );
        }
    }
}

fn context_module_sources() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    [
        "context_pack.rs",
        "context_pack_selection.rs",
        "context_pack_validation.rs",
        "context_pack_storage.rs",
    ]
    .into_iter()
    .map(|name| root.join(name))
    .collect()
}

fn read_sources(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| fs::read_to_string(path).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

fn temp_project(label: &str) -> (PathBuf, String) {
    let dir = std::env::temp_dir().join(format!(
        "openmesh-context-pack-boundary-{label}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    let project_path = dir.to_string_lossy().to_string();
    (dir, project_path)
}

fn seed_profile_and_event(project_path: &str, workspace_id: &str) {
    let profile = default_work_proxy_profile(
        workspace_id,
        format!("profile-{workspace_id}"),
        "Boundary Owner",
        "Boundary Role",
        "2026-07-17T08:00:00Z",
    );
    write_work_proxy_profile(project_path, &profile).expect("profile");
    let event = openmesh_core::domain::WorkEvent::new(
        "evt-boundary-seed",
        workspace_id,
        "work.completed",
        "boundary seed",
        vec![openmesh_core::domain::EvidenceAttachment {
            evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    );
    append_event(project_path, &event).expect("event");
}

fn built_pack_with_authority(level: ProxyAuthorityLevel) -> ProxyContextPack {
    let (_dir, project_path) = temp_project("authority-pack");
    let workspace_id = fs::read_to_string(Path::new(&project_path).join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    let mut profile = default_work_proxy_profile(
        &workspace_id,
        format!("profile-{workspace_id}"),
        "Authority Owner",
        "Authority Role",
        "2026-07-17T08:00:00Z",
    );
    profile.authority_rules = vec![AuthorityRule {
        rule_id: "rule-scope".into(),
        scope: "work.progress".into(),
        authority: level,
        description: Some("boundary authority rule".into()),
        conditions: vec![],
        evidence_required: true,
        human_confirmation_required: matches!(
            level,
            ProxyAuthorityLevel::MustAskHuman | ProxyAuthorityLevel::CannotAnswer
        ),
        limitations: vec![],
    }];
    write_work_proxy_profile(&project_path, &profile).expect("write profile");
    let event = openmesh_core::domain::WorkEvent::new(
        "evt-authority-seed",
        &workspace_id,
        "work.completed",
        "authority seed",
        vec![openmesh_core::domain::EvidenceAttachment {
            evidence_ref: openmesh_core::domain::EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    );
    append_event(&project_path, &event).expect("event");
    let window = openmesh_core::domain::CatchUpWindow {
        since: WINDOW_SINCE.into(),
        until: WINDOW_UNTIL.into(),
    };
    build_proxy_context_pack(
        &project_path,
        window,
        ProxyContextPackBuildOptions {
            generated_at: GENERATED_AT.into(),
            selection: Default::default(),
        },
    )
    .expect("build pack")
}

#[test]
fn context_pack_wire_surface_contains_no_question_or_answer_fields() {
    let pack = fixture_pack();
    validate_proxy_context_pack(&pack).expect("fixture validates");
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, FORBIDDEN_QA_WIRE_KEYS);

    let mut value: Value = serde_json::from_str(FIXTURE).expect("parse fixture json");
    value
        .as_object_mut()
        .expect("object")
        .insert("answer".into(), json!("must be rejected"));
    let rejected = serde_json::from_value::<ProxyContextPack>(value);
    assert!(
        rejected.is_err(),
        "deny_unknown_fields must reject answer field"
    );
}

#[test]
fn context_pack_contains_no_prompt_model_or_tool_execution_contract() {
    let pack = fixture_pack();
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, FORBIDDEN_RUNTIME_WIRE_KEYS);

    for banned in FORBIDDEN_RUNTIME_WIRE_KEYS {
        let mut value: Value = serde_json::from_str(FIXTURE).expect("parse fixture json");
        value
            .as_object_mut()
            .expect("object")
            .insert((*banned).into(), json!("injected"));
        let rejected = serde_json::from_value::<ProxyContextPack>(value);
        assert!(
            rejected.is_err(),
            "deny_unknown_fields must reject injected `{banned}`"
        );
    }
}

#[test]
fn authority_summary_is_declarative_only() {
    let pack = fixture_pack();
    assert!(pack
        .authority_summary
        .execution_boundary
        .to_ascii_lowercase()
        .contains("metadata"));
    assert_eq!(
        pack.authority_summary.execution_boundary,
        CONTEXT_PACK_EXECUTION_BOUNDARY
    );
    assert!(!pack.authority_summary.ladder_levels.is_empty());
}

#[test]
fn can_answer_does_not_generate_answer() {
    let pack = built_pack_with_authority(ProxyAuthorityLevel::CanAnswer);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, &["answer", "response", "generatedResponse"]);
}

#[test]
fn can_draft_does_not_generate_draft() {
    let pack = built_pack_with_authority(ProxyAuthorityLevel::CanDraft);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, &["draftContent", "draft", "response"]);
}

#[test]
fn can_suggest_does_not_generate_suggestion() {
    let pack = built_pack_with_authority(ProxyAuthorityLevel::CanSuggest);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, &["suggestionContent", "suggestion", "response"]);
}

#[test]
fn must_ask_human_does_not_simulate_human_confirmation() {
    let pack = built_pack_with_authority(ProxyAuthorityLevel::MustAskHuman);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(
        &keys,
        &[
            "humanConfirmation",
            "humanResponse",
            "approval",
            "approved",
            "delegation",
        ],
    );
    let serialized = serde_json::to_string(&pack).expect("serialize");
    let lowered = serialized.to_ascii_lowercase();
    for phrase in ["human confirmed", "owner approved", "asked the human"] {
        assert!(!lowered.contains(phrase));
    }
}

#[test]
fn context_commands_do_not_call_question_authority_resolution() {
    let sources = read_sources(&context_module_sources());
    assert!(!sources.contains("resolve_profile_authority"));
    assert!(!sources.contains("ProfileEvaluationContext"));
}

#[test]
fn owner_identity_remains_metadata_not_impersonation() {
    let pack = fixture_pack();
    assert!(!pack.owner_identity.owner_label.is_empty());
    assert!(!pack.owner_identity.role_label.is_empty());
    let serialized = serde_json::to_string(&pack).expect("serialize");
    let lowered = serialized.to_ascii_lowercase();
    for phrase in [
        "i am the owner",
        "i am fixture owner",
        "speaking as the owner",
    ] {
        assert!(!lowered.contains(phrase));
    }
}

#[test]
fn context_pack_preserves_no_impersonation_refusal() {
    let pack = fixture_pack();
    assert!(pack
        .authority_summary
        .default_refusal_rules
        .iter()
        .any(|rule| rule.rule_id == "refusal-no-impersonation"));
    assert!(pack
        .authority_summary
        .default_refusal_rules
        .iter()
        .any(|rule| rule.statement.to_ascii_lowercase().contains("impersonate")));
}

#[test]
fn context_modules_invoke_no_llm_axga_or_model_runtime() {
    let sources = read_sources(&context_module_sources()).to_ascii_lowercase();
    for forbidden in [
        "openai",
        "anthropic",
        "axga",
        "llm",
        "model_runtime",
        "reqwest",
        "http://",
        "https://",
        "tokio::spawn",
    ] {
        assert!(
            !sources.contains(forbidden),
            "context modules must not reference {forbidden}"
        );
    }
}

#[test]
fn context_modules_do_not_activate_intelligence_runtime() {
    let sources = read_sources(&context_module_sources());
    assert!(!sources.contains("ContinuityIntelligence"));
    assert!(!sources.contains("resolve_ambiguous_with_intelligence"));
    assert!(!sources.contains("NoopContinuityIntelligence"));
    assert!(!sources.contains("intelligence::"));
}

#[test]
fn context_pack_compose_does_not_execute_authority() {
    let (_dir, project_path) = temp_project("compose-boundary");
    let workspace_id = fs::read_to_string(Path::new(&project_path).join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    seed_profile_and_event(&project_path, &workspace_id);
    let profile = read_work_proxy_profile(&project_path).expect("profile");
    let snapshot = load_continuity_input_snapshot(&project_path).expect("snapshot");
    let current_state = build_current_state_projection(&snapshot).expect("state");
    let window = openmesh_core::domain::CatchUpWindow {
        since: WINDOW_SINCE.into(),
        until: WINDOW_UNTIL.into(),
    };
    let catch_up = build_catch_up_view(&snapshot, &current_state, &window).expect("catch-up");
    let pack = compose_proxy_context_pack(
        &ProxyContextPackComposeInputs {
            profile,
            snapshot,
            current_state,
            catch_up,
            window,
            generated_at: GENERATED_AT.into(),
        },
        &ProxyContextPackBuildOptions {
            generated_at: GENERATED_AT.into(),
            selection: Default::default(),
        },
    )
    .expect("compose");
    assert_eq!(pack.protocol_version, PROXY_CONTEXT_PACK_PROTOCOL_VERSION);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, FORBIDDEN_QA_WIRE_KEYS);
}

#[test]
fn cannot_answer_authority_remains_declarative_metadata() {
    let pack = built_pack_with_authority(ProxyAuthorityLevel::CannotAnswer);
    let keys = wire_keys_for_pack(&pack);
    assert_no_exact_wire_keys(&keys, &["answer", "response", "refusalText"]);
    assert!(pack
        .authority_summary
        .execution_boundary
        .contains("metadata"));
}
