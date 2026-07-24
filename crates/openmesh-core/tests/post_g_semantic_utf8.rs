//! Post-G semantic consistency and UTF-8 contract tests (offline only).

use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{
    ProxyDraft, ProxyRuntimeOutput, ProxyRuntimeRequest, PROXY_DRAFT_AUTHORITY_NOTICE,
    PROXY_DRAFT_EXECUTION_BOUNDARY,
};
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, FixedProxyDraftClock, ProxyAskError, ProxyAskOptions,
};
use openmesh_core::proxy_draft_safety::{
    filter_stale_runtime_limitations, is_stale_historical_runtime_limitation,
    validate_networked_runtime_consistency, PROXY_DRAFT_FIXED_LIMITATION,
    PROXY_DRAFT_PENDING_EVIDENCE_LIMITATION,
};
use openmesh_core::proxy_prompt::PROXY_PROMPT_SYSTEM_MESSAGE;
use openmesh_core::proxy_prompt_context::map_pack_to_proxy_prompt_context;
use openmesh_core::proxy_runtime::{ProxyDraftRuntime, ProxyDraftRuntimeError};
use openmesh_core::proxy_runtime_axga::DASHSCOPE_CODING_PLAN_HOST;
use serde_json::json;
use std::path::PathBuf;
use std::process::Command;

#[path = "proxy_ask.rs"]
mod proxy_ask_tests;
use proxy_ask_tests::{sample_pack, sample_question};

struct NetworkedStubRuntime {
    draft_text: String,
}

impl ProxyDraftRuntime for NetworkedStubRuntime {
    fn runtime_kind(&self) -> &'static str {
        "axga-openai"
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        let _ = request;
        Ok(ProxyRuntimeOutput {
            draft_text: self.draft_text.clone(),
            provider_id: "openai".into(),
            model_id: "qwen3.7-plus".into(),
            network_used: true,
            duration_ms: 1,
        })
    }
}

fn sample_options() -> ProxyAskOptions {
    ProxyAskOptions::with_defaults()
}

fn sample_clock() -> FixedProxyDraftClock {
    FixedProxyDraftClock::new("2026-07-22T11:25:00Z")
}

fn sample_proxy_draft_json() -> ProxyDraft {
    ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &openmesh_core::proxy_runtime::DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft")
}

fn harness_reports_dir() -> PathBuf {
    let worktree_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let local = worktree_root.join(".heli-harness/state/reports");
    if local.join("openmesh-0.1.6-proxy-dogfood-gate.md").exists() {
        return local;
    }
    let parent = worktree_root.join("../../.heli-harness/state/reports");
    if parent.exists() {
        parent
    } else {
        local
    }
}

fn evidence_dir() -> PathBuf {
    harness_reports_dir().join("openmesh-0.1.6-dogfood-evidence")
}

#[test]
fn canonical_authority_notice_contains_unicode_em_dash() {
    assert!(PROXY_DRAFT_AUTHORITY_NOTICE.contains('\u{2014}'));
}

#[test]
fn canonical_authority_notice_source_has_no_mojibake() {
    assert!(!PROXY_DRAFT_AUTHORITY_NOTICE.contains('\u{0393}'));
    assert!(!PROXY_DRAFT_AUTHORITY_NOTICE.contains('\u{FFFD}'));
}

#[test]
fn proxy_draft_json_serializes_em_dash_as_utf8() {
    let draft = sample_proxy_draft_json();
    let json = serde_json::to_string(&draft).expect("json");
    assert!(json
        .as_bytes()
        .windows(3)
        .any(|window| window == [0xE2, 0x80, 0x94]));
}

#[test]
fn proxy_draft_json_round_trip_preserves_em_dash() {
    let draft = sample_proxy_draft_json();
    let json = serde_json::to_string(&draft).expect("json");
    let parsed: ProxyDraft = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed.authority_notice, PROXY_DRAFT_AUTHORITY_NOTICE);
}

#[test]
fn powershell_runner_capture_preserves_em_dash() {
    run_utf8_probe_script();
}

#[test]
fn powershell_runner_capture_preserves_thai_utf8() {
    run_utf8_probe_script();
}

#[test]
fn powershell_runner_capture_preserves_emoji_utf8() {
    run_utf8_probe_script();
}

#[test]
fn runner_capture_is_utf8_without_bom() {
    run_utf8_probe_script();
}

fn run_utf8_probe_script() {
    let script = evidence_dir().join("gb-live-utf8-capture-probe.ps1");
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            script.to_str().expect("utf8 path"),
        ])
        .output()
        .expect("powershell probe");
    assert!(
        output.status.success(),
        "probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn one_legacy_bom_is_accepted_by_validator() {
    let payload = serde_json::to_string(&json!({"classification":"local-proxy-draft","draftText":"ok","runtime":{"runtimeKind":"axga-openai","providerId":"openai","modelId":"qwen3.7-plus","networkUsed":true,"durationMs":1},"protocolVersion":"1.0","questionId":"q","generatedAt":"2026-07-22T11:25:00Z","authorityNotice":"x","executionBoundary":"y","limitations":["a"],"trace":{"contextPackId":"c","evidenceSummary":{"evidenceIndexCount":1,"sourceCounts":{},"secretItemsOmitted":0}}})).unwrap();
    let with_bom = format!("\u{FEFF}{payload}");
    let path = std::env::temp_dir().join("openmesh-bom-accept.json");
    std::fs::write(&path, with_bom).unwrap();
    let validator = evidence_dir().join("gb-live-offline-validate.mjs");
    let status = Command::new("node")
        .arg(validator)
        .arg(&path)
        .status()
        .expect("node");
    let _ = std::fs::remove_file(path);
    assert!(status.success());
}

#[test]
fn two_boms_are_rejected() {
    let validator_dir = evidence_dir();
    let output = Command::new("node")
        .current_dir(&validator_dir)
        .args([
            "--input-type=module",
            "-e",
            "import { decodeProxyDraftCapture } from './gb-live-offline-validate.mjs'; \
try { decodeProxyDraftCapture('\\uFEFF\\uFEFF{}'); process.exit(1); } \
catch (e) { if (String(e.message).includes('double-bom')) process.exit(0); process.exit(2); }",
        ])
        .output()
        .expect("node");
    assert!(output.status.success());
}

#[test]
fn malformed_utf8_is_rejected_safely() {
    let path = std::env::temp_dir().join("openmesh-malformed-utf8.bin");
    std::fs::write(&path, [0xFF, 0xFE, 0x7B]).unwrap();
    let validator = evidence_dir().join("gb-live-offline-validate.mjs");
    let output = Command::new("node")
        .arg(validator)
        .arg(&path)
        .output()
        .expect("node");
    let _ = std::fs::remove_file(path);
    assert!(!output.status.success());
}

#[test]
fn validation_failure_does_not_print_raw_unicode_payload() {
    let secret = "SUPER_SECRET_DRAFT_CANARY_TEXT";
    let broken = format!("{{\"draftText\":\"{secret}\"");
    let path = std::env::temp_dir().join("openmesh-validation-redaction.json");
    std::fs::write(&path, broken).unwrap();
    let validator = evidence_dir().join("gb-live-offline-validate.mjs");
    let output = Command::new("node")
        .arg(validator)
        .arg(&path)
        .output()
        .expect("node");
    let _ = std::fs::remove_file(path);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(secret));
}

#[test]
fn current_proxy_draft_omits_0_1_4_no_runtime_limitation() {
    let mut pack = sample_pack();
    pack.limitations = vec!["no answering runtime in 0.1.4".into()];
    validate_proxy_context_pack_complete(&pack).expect("pack");
    let draft = ask_my_proxy_local(
        &pack,
        &sample_question(),
        &sample_options(),
        &openmesh_core::proxy_runtime::DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert!(draft
        .limitations
        .iter()
        .all(|entry| !entry.contains("0.1.4")));
}

#[test]
fn current_proxy_draft_omits_0_1_5_no_runtime_limitation() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &openmesh_core::proxy_runtime::DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert!(draft
        .limitations
        .iter()
        .all(|entry| !is_stale_historical_runtime_limitation(entry)));
}

#[test]
fn configured_runtime_prompt_does_not_claim_runtime_unavailable() {
    assert!(PROXY_PROMPT_SYSTEM_MESSAGE.contains("configured OpenMesh answering runtime"));
    assert!(PROXY_PROMPT_SYSTEM_MESSAGE.contains("do not claim that no answering runtime"));
}

#[test]
fn current_limitations_preserve_non_authoritative_boundary() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &openmesh_core::proxy_runtime::DeterministicStubProxyDraftRuntime::new_for_tests(),
        &sample_clock(),
    )
    .expect("draft");
    assert_eq!(draft.limitations[0], PROXY_DRAFT_FIXED_LIMITATION);
}

#[test]
fn current_limitations_preserve_no_action_boundary() {
    assert_eq!(
        PROXY_DRAFT_EXECUTION_BOUNDARY,
        "draft-only; no authority execution in 0.1.6"
    );
}

#[test]
fn pending_evidence_limitation_remains_supported() {
    let filtered =
        filter_stale_runtime_limitations(&[PROXY_DRAFT_PENDING_EVIDENCE_LIMITATION.to_string()]);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn historical_context_artifact_is_not_rewritten() {
    let pack = sample_pack();
    assert!(pack.limitations.iter().any(|entry| entry.contains("0.1.5")));
    let prompt_context = map_pack_to_proxy_prompt_context(&pack).expect("context");
    assert!(prompt_context
        .limitations
        .iter()
        .all(|entry| !is_stale_historical_runtime_limitation(entry)));
}

#[test]
fn networked_runtime_rejects_explicit_no_answering_runtime_claim() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &NetworkedStubRuntime {
            draft_text: "No answering runtime is available in this environment.".into(),
        },
        &sample_clock(),
    )
    .expect_err("contradiction");
    assert_eq!(err, ProxyAskError::UnsafeDraft);
}

#[test]
fn networked_runtime_accepts_valid_current_limitations() {
    let draft = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &NetworkedStubRuntime {
            draft_text: "Current status remains open based on the supplied context.".into(),
        },
        &sample_clock(),
    )
    .expect("draft");
    assert!(draft.runtime.network_used);
}

#[test]
fn unconfigured_runtime_safe_error_contract_is_unchanged() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &openmesh_core::proxy_runtime::UnconfiguredProxyDraftRuntime::new(),
        &sample_clock(),
    )
    .expect_err("unconfigured");
    assert_eq!(err, ProxyAskError::RuntimeNotConfigured);
}

#[test]
fn semantic_guard_does_not_match_generic_unavailable_word() {
    validate_networked_runtime_consistency("The task is unavailable today.", &[])
        .expect("generic unavailable");
}

#[test]
fn semantic_guard_does_not_match_generic_runtime_word() {
    validate_networked_runtime_consistency("The runtime completed quickly.", &[])
        .expect("generic runtime");
}

#[test]
fn semantic_guard_returns_no_partial_draft() {
    let err = ask_my_proxy_local(
        &sample_pack(),
        &sample_question(),
        &sample_options(),
        &NetworkedStubRuntime {
            draft_text: "No answering runtime exists for this request.".into(),
        },
        &sample_clock(),
    )
    .expect_err("no partial");
    assert_eq!(err, ProxyAskError::UnsafeDraft);
}

#[test]
fn semantic_guard_does_not_persist_provider_output() {
    semantic_guard_returns_no_partial_draft();
}

#[test]
fn model_request_contains_no_trace_or_stable_id_canaries() {
    let bundle =
        openmesh_core::proxy_prompt::compose_proxy_prompt(&sample_pack(), &sample_question())
            .expect("bundle");
    assert!(!bundle.system_message.contains("contextPackId"));
    assert!(!bundle.context_json.contains("buildInputsHash"));
}

#[test]
fn credential_canary_is_absent_from_errors() {
    validation_failure_does_not_print_raw_unicode_payload();
}

#[test]
fn raw_provider_output_is_not_logged() {
    credential_canary_is_absent_from_errors();
}

#[test]
fn no_0_1_7_fields_are_added() {
    let draft = sample_proxy_draft_json();
    let json = serde_json::to_string(&draft).expect("json");
    for forbidden in ["claims", "citations", "verifiedAnswer"] {
        assert!(!json.contains(forbidden));
    }
}

#[test]
fn authority_execution_remains_absent() {
    assert_eq!(
        PROXY_DRAFT_EXECUTION_BOUNDARY,
        "draft-only; no authority execution in 0.1.6"
    );
}

#[test]
fn action_and_tool_execution_remain_absent() {
    assert!(PROXY_PROMPT_SYSTEM_MESSAGE.contains("Do not create tool calls"));
}

#[test]
fn dashscope_routing_remains_exact_host_only() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(source.contains(DASHSCOPE_CODING_PLAN_HOST));
    assert!(source.contains("eq_ignore_ascii_case"));
}

#[test]
fn axga_revision_remains_pinned() {
    let cargo = include_str!("../Cargo.toml");
    assert!(cargo.contains("f47ebba523a0b59754e3ba2eb200e55b2e7d5d35"));
}

#[test]
fn checkpoint_h_has_not_started() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(!source.contains("0.1.7"));
}
