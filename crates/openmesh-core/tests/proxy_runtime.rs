//! Dev Track 0.1.6 Checkpoint C — proxy draft runtime contract and isolation tests.

use openmesh_core::domain::{
    validate_proxy_runtime_output, validate_proxy_runtime_request, ProxyPromptBundle,
    ProxyRuntimeOutput, ProxyRuntimeRequest, MAX_PROXY_DRAFT_TEXT_BYTES,
    PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
};
use openmesh_core::proxy_runtime::{
    DeterministicStubProxyDraftRuntime, ProxyDraftRuntime, ProxyDraftRuntimeError,
    UnconfiguredProxyDraftRuntime, DETERMINISTIC_STUB_DURATION_MS, DETERMINISTIC_STUB_MODEL_ID,
    DETERMINISTIC_STUB_PROVIDER_ID, PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB,
    PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED,
};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

fn sample_prompt_bundle() -> ProxyPromptBundle {
    ProxyPromptBundle {
        protocol_version: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION.to_string(),
        system_message: "You are a local proxy draft assistant.".into(),
        context_json: r#"{"ownerLabel":"Fixture Owner","limitations":["metadata only"]}"#.into(),
        user_message: "What is the current status?".into(),
    }
}

fn sample_runtime_request() -> ProxyRuntimeRequest {
    ProxyRuntimeRequest {
        prompt: sample_prompt_bundle(),
        timeout_ms: 30_000,
        max_output_bytes: MAX_PROXY_DRAFT_TEXT_BYTES as u32,
    }
}

fn assert_object_safe(_runtime: &dyn ProxyDraftRuntime) {}

fn assert_send_sync<T: Send + Sync>() {}

struct ImmediateTimeoutRuntime;

impl ProxyDraftRuntime for ImmediateTimeoutRuntime {
    fn runtime_kind(&self) -> &'static str {
        "timeout-fake"
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_proxy_runtime_request(request)
            .map_err(|_| ProxyDraftRuntimeError::InvalidRequest)?;
        Err(ProxyDraftRuntimeError::Timeout)
    }
}

struct ImmediateUnavailableRuntime;

impl ProxyDraftRuntime for ImmediateUnavailableRuntime {
    fn runtime_kind(&self) -> &'static str {
        "unavailable-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::RuntimeUnavailable)
    }
}

struct ImmediateProviderFailureRuntime;

impl ProxyDraftRuntime for ImmediateProviderFailureRuntime {
    fn runtime_kind(&self) -> &'static str {
        "provider-failure-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::ProviderFailure)
    }
}

struct ImmediateInvalidOutputRuntime;

impl ProxyDraftRuntime for ImmediateInvalidOutputRuntime {
    fn runtime_kind(&self) -> &'static str {
        "invalid-output-fake"
    }

    fn generate_draft(
        &self,
        _request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        Err(ProxyDraftRuntimeError::InvalidOutput)
    }
}

#[test]
fn proxy_draft_runtime_is_object_safe() {
    let unconfigured = UnconfiguredProxyDraftRuntime::new();
    let stub = DeterministicStubProxyDraftRuntime::new_for_tests();
    assert_object_safe(&unconfigured);
    assert_object_safe(&stub);
}

#[test]
fn proxy_draft_runtime_is_send_and_sync() {
    assert_send_sync::<UnconfiguredProxyDraftRuntime>();
    assert_send_sync::<DeterministicStubProxyDraftRuntime>();
    assert_send_sync::<Box<dyn ProxyDraftRuntime>>();
}

#[test]
fn runtime_kind_is_adapter_declared() {
    let unconfigured = UnconfiguredProxyDraftRuntime::new();
    let stub = DeterministicStubProxyDraftRuntime::new_for_tests();
    assert_eq!(
        unconfigured.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
    );
    assert_eq!(
        stub.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_DETERMINISTIC_STUB
    );
    assert_ne!(unconfigured.runtime_kind(), stub.runtime_kind());
}

#[test]
fn runtime_trait_accepts_only_proxy_runtime_request() {
    fn accepts_request(
        runtime: &dyn ProxyDraftRuntime,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        runtime.generate_draft(request)
    }
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let request = sample_runtime_request();
    let _ = accepts_request(&runtime, &request).expect("typed request accepted");
}

#[test]
fn unconfigured_runtime_kind_is_exact() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    assert_eq!(runtime.runtime_kind(), "unconfigured");
}

#[test]
fn unconfigured_runtime_returns_runtime_not_configured() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    let err = runtime
        .generate_draft(&request)
        .expect_err("must not succeed");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeNotConfigured);
}

#[test]
fn unconfigured_runtime_never_returns_placeholder_output() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    assert!(runtime.generate_draft(&request).is_err());
}

#[test]
fn unconfigured_runtime_rejects_invalid_request() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let mut request = sample_runtime_request();
    request.timeout_ms = 0;
    let err = runtime
        .generate_draft(&request)
        .expect_err("invalid request");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidRequest);
}

#[test]
fn unconfigured_error_does_not_echo_prompt() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    let err = runtime
        .generate_draft(&request)
        .expect_err("not configured");
    let message = err.to_string();
    assert!(!message.contains(&request.prompt.user_message));
    assert!(!message.contains(&request.prompt.context_json));
    assert!(!message.contains(&request.prompt.system_message));
}

#[test]
fn unconfigured_runtime_performs_no_io_or_network() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    let started = Instant::now();
    let err = runtime
        .generate_draft(&request)
        .expect_err("not configured");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeNotConfigured);
    assert!(started.elapsed().as_millis() < 100);
}

#[test]
fn deterministic_stub_kind_is_exact() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    assert_eq!(runtime.runtime_kind(), "deterministic-stub");
}

#[test]
fn deterministic_stub_constructor_is_test_oriented() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(source.contains("new_for_tests"));
    assert!(source.contains("CI injection only"));
    assert!(!source.contains("impl Default for DeterministicStubProxyDraftRuntime"));
}

#[test]
fn identical_request_produces_identical_output() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let request = sample_runtime_request();
    let first = runtime.generate_draft(&request).expect("first");
    let second = runtime.generate_draft(&request).expect("second");
    assert_eq!(first, second);
}

#[test]
fn repeated_calls_produce_identical_output() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let request = sample_runtime_request();
    let baseline = runtime.generate_draft(&request).expect("baseline");
    for _ in 0..8 {
        assert_eq!(runtime.generate_draft(&request).expect("repeat"), baseline);
    }
}

#[test]
fn separate_stub_instances_produce_identical_output() {
    let request = sample_runtime_request();
    let first = DeterministicStubProxyDraftRuntime::new_for_tests()
        .generate_draft(&request)
        .expect("first");
    let second = DeterministicStubProxyDraftRuntime::new_for_tests()
        .generate_draft(&request)
        .expect("second");
    assert_eq!(first, second);
}

#[test]
fn stub_output_passes_checkpoint_a_validation() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    validate_proxy_runtime_output(&output).expect("structural validation");
}

#[test]
fn stub_provider_metadata_is_fixed() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    assert_eq!(output.provider_id, DETERMINISTIC_STUB_PROVIDER_ID);
    assert_eq!(output.model_id, DETERMINISTIC_STUB_MODEL_ID);
}

#[test]
fn stub_network_used_is_false() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    assert!(!output.network_used);
}

#[test]
fn stub_duration_is_deterministic() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    assert_eq!(output.duration_ms, DETERMINISTIC_STUB_DURATION_MS);
}

#[test]
fn stub_respects_request_output_bound() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let mut request = sample_runtime_request();
    request.max_output_bytes = 128;
    let output = runtime.generate_draft(&request).expect("bounded output");
    assert!(output.draft_text.len() <= 128);
    assert!(!output.draft_text.is_empty());
}

#[test]
fn stub_never_exceeds_global_draft_bound() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    assert!(output.draft_text.len() <= MAX_PROXY_DRAFT_TEXT_BYTES);
}

#[test]
fn stub_preserves_valid_utf8() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    assert!(std::str::from_utf8(output.draft_text.as_bytes()).is_ok());
}

#[test]
fn stub_handles_thai_question_utf8() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let mut request = sample_runtime_request();
    request.prompt.user_message = "สถานะปัจจุบันคืออะไร?".into();
    let output = runtime.generate_draft(&request).expect("thai output");
    assert!(output.draft_text.contains("สถานะปัจจุบันคืออะไร?"));
    assert!(std::str::from_utf8(output.draft_text.as_bytes()).is_ok());
}

#[test]
fn stub_removes_only_complete_utf8_characters() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let mut request = sample_runtime_request();
    request.prompt.user_message = "กขค".into();
    request.max_output_bytes = 8;
    let output = runtime.generate_draft(&request).expect("truncated output");
    assert!(std::str::from_utf8(output.draft_text.as_bytes()).is_ok());
    assert!(output.draft_text.len() <= 8);
}

#[test]
fn stub_rejects_invalid_request() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let mut request = sample_runtime_request();
    request.max_output_bytes = 0;
    let err = runtime.generate_draft(&request).expect_err("invalid");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidRequest);
}

#[test]
fn stub_does_not_mutate_request() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let request = sample_runtime_request();
    let before = serde_json::to_string(&request).expect("serialize");
    let _ = runtime.generate_draft(&request).expect("generate");
    let after = serde_json::to_string(&request).expect("serialize");
    assert_eq!(before, after);
}

#[test]
fn stub_does_not_read_project_or_context_pack() {
    let decoy_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/proxy/proxy-draft-valid.json");
    let decoy = fs::read_to_string(&decoy_path).expect("read decoy fixture");
    let unique_marker = "context-pack-fnv1a-6dd176ff3e7276a3";
    assert!(decoy.contains(unique_marker));

    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let request = sample_runtime_request();
    let output = runtime.generate_draft(&request).expect("stub output");
    assert!(!output.draft_text.contains(unique_marker));
    assert!(!output.draft_text.contains("context-pack-fnv1a"));
}

#[test]
fn stub_does_not_invoke_network() {
    let source = include_str!("../src/proxy_runtime.rs");
    for forbidden in [
        "reqwest",
        "ureq",
        "hyper",
        "http::",
        "TcpStream",
        "std::net",
    ] {
        assert!(
            !source.contains(forbidden),
            "forbidden network symbol present: {forbidden}"
        );
    }
}

#[test]
fn stub_contains_no_trace_metadata() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    for forbidden in [
        "workspaceId",
        "profileId",
        "contextPackId",
        "buildInputsHash",
        "evidenceSummary",
        "trace",
    ] {
        assert!(!output.draft_text.contains(forbidden));
    }
}

#[test]
fn stub_contains_no_authority_metadata() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    for forbidden in [
        "authorityNotice",
        "executionBoundary",
        "classification",
        "mustAskHuman",
        "authorityExecution",
    ] {
        assert!(!output.draft_text.contains(forbidden));
    }
}

#[test]
fn stub_contains_no_evidence_summary() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    for forbidden in ["evidenceIndex", "sourceCounts", "secretItemsOmitted"] {
        assert!(!output.draft_text.contains(forbidden));
    }
}

#[test]
fn stub_contains_no_tool_or_action_structure() {
    let runtime = DeterministicStubProxyDraftRuntime::new_for_tests();
    let output = runtime
        .generate_draft(&sample_runtime_request())
        .expect("stub output");
    for forbidden in ["tool_calls", "function_call", "executeTool", "\"action\""] {
        assert!(!output.draft_text.contains(forbidden));
    }
}

#[test]
fn timeout_fake_returns_timeout_without_sleep() {
    let runtime = ImmediateTimeoutRuntime;
    let started = Instant::now();
    let err = runtime
        .generate_draft(&sample_runtime_request())
        .expect_err("timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
    assert!(started.elapsed().as_millis() < 100);
}

#[test]
fn runtime_unavailable_error_is_secret_safe() {
    let runtime = ImmediateUnavailableRuntime;
    let err = runtime
        .generate_draft(&sample_runtime_request())
        .expect_err("unavailable");
    let message = err.to_string();
    assert_eq!(message, "proxy draft runtime is unavailable");
    assert!(!message.contains("api"));
    assert!(!message.contains("key"));
}

#[test]
fn provider_failure_error_is_secret_safe() {
    let runtime = ImmediateProviderFailureRuntime;
    let err = runtime
        .generate_draft(&sample_runtime_request())
        .expect_err("provider failure");
    let message = err.to_string();
    assert_eq!(message, "proxy draft runtime provider failed");
}

#[test]
fn invalid_output_error_is_secret_safe() {
    let runtime = ImmediateInvalidOutputRuntime;
    let err = runtime
        .generate_draft(&sample_runtime_request())
        .expect_err("invalid output");
    let message = err.to_string();
    assert_eq!(message, "proxy draft runtime produced invalid output");
}

#[test]
fn error_messages_do_not_echo_question() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    let message = runtime
        .generate_draft(&request)
        .expect_err("error")
        .to_string();
    assert!(!message.contains(&request.prompt.user_message));
}

#[test]
fn error_messages_do_not_echo_context_json() {
    let runtime = UnconfiguredProxyDraftRuntime::new();
    let request = sample_runtime_request();
    let message = runtime
        .generate_draft(&request)
        .expect_err("error")
        .to_string();
    assert!(!message.contains(&request.prompt.context_json));
}

#[test]
fn empty_fake_output_fails_structural_validation() {
    let output = ProxyRuntimeOutput {
        draft_text: String::new(),
        provider_id: DETERMINISTIC_STUB_PROVIDER_ID.into(),
        model_id: DETERMINISTIC_STUB_MODEL_ID.into(),
        network_used: false,
        duration_ms: 0,
    };
    assert!(validate_proxy_runtime_output(&output).is_err());
}

#[test]
fn oversized_fake_output_fails_structural_validation() {
    let output = ProxyRuntimeOutput {
        draft_text: "x".repeat(MAX_PROXY_DRAFT_TEXT_BYTES + 1),
        provider_id: DETERMINISTIC_STUB_PROVIDER_ID.into(),
        model_id: DETERMINISTIC_STUB_MODEL_ID.into(),
        network_used: false,
        duration_ms: 0,
    };
    assert!(validate_proxy_runtime_output(&output).is_err());
}

#[test]
fn malformed_provider_id_fails_structural_validation() {
    let output = ProxyRuntimeOutput {
        draft_text: "valid draft".into(),
        provider_id: String::new(),
        model_id: DETERMINISTIC_STUB_MODEL_ID.into(),
        network_used: false,
        duration_ms: 0,
    };
    assert!(validate_proxy_runtime_output(&output).is_err());
}

#[test]
fn malformed_model_id_fails_structural_validation() {
    let output = ProxyRuntimeOutput {
        draft_text: "valid draft".into(),
        provider_id: DETERMINISTIC_STUB_PROVIDER_ID.into(),
        model_id: String::new(),
        network_used: false,
        duration_ms: 0,
    };
    assert!(validate_proxy_runtime_output(&output).is_err());
}

#[test]
fn checkpoint_c_adds_no_production_runtime_factory() {
    let lib_source = include_str!("../src/lib.rs");
    let core_sources = [
        include_str!("../src/proxy_runtime.rs"),
        include_str!("../src/proxy_question.rs"),
        include_str!("../src/proxy_prompt.rs"),
        include_str!("../src/proxy_prompt_context.rs"),
    ];
    assert!(!lib_source.contains("resolve_production_proxy_draft_runtime"));
    for source in core_sources {
        assert!(!source.contains("resolve_production_proxy_draft_runtime"));
        assert!(!source.contains("proxy_runtime_factory"));
    }
}

#[test]
fn checkpoint_c_adds_no_environment_runtime_selection() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(!source.contains("std::env"));
    assert!(!source.contains("var_os"));
    assert!(!source.contains("var("));
}

#[test]
fn checkpoint_c_adds_no_stub_fallback() {
    let request = sample_runtime_request();
    assert_eq!(
        UnconfiguredProxyDraftRuntime::new()
            .generate_draft(&request)
            .expect_err("not configured"),
        ProxyDraftRuntimeError::RuntimeNotConfigured
    );
    assert!(DeterministicStubProxyDraftRuntime::new_for_tests()
        .generate_draft(&request)
        .is_ok());
}

// Lifecycle amendment (Checkpoint E isolation patch): C-owned runtime must not embed
// CLI proxy command modules. Existence of `proxy.rs` is proven by Checkpoint E tests.
#[test]
fn checkpoint_c_adds_no_cli_command() {
    let runtime_source = include_str!("../src/proxy_runtime.rs");
    assert!(
        !runtime_source.contains("mod proxy"),
        "C-owned runtime must not own CLI proxy command"
    );
}

#[test]
fn checkpoint_c_adds_no_tauri_command() {
    let tauri_lib = include_str!("../../../src-tauri/src/lib.rs");
    assert!(!tauri_lib.contains("proxy ask"));
    assert!(!tauri_lib.contains("ask_my_proxy"));
}

// Lifecycle amendment (DG compatibility guard patch): `openmesh-core/Cargo.toml` is a
// shared later-checkpoint dependency surface — post-G DashScope compatibility legitimately
// owns direct `reqwest` in `proxy_runtime_axga.rs`. Checkpoint C ownership remains
// `proxy_runtime.rs`: no HTTP client, provider integration, or network I/O.
#[test]
fn checkpoint_c_runtime_remains_network_dependency_free() {
    let source = include_str!("../src/proxy_runtime.rs");
    for forbidden in [
        "reqwest",
        "ureq",
        "hyper",
        "axum",
        "http::",
        "TcpStream",
        "std::net",
        "OpenAiProvider",
        "AnthropicProvider",
        "DeepSeekProvider",
        "stream_chat",
    ] {
        assert!(
            !source.contains(forbidden),
            "Checkpoint C runtime must remain network- and provider-integration-free: {forbidden}"
        );
    }
}

#[test]
fn checkpoint_c_adds_no_runtime_thread_or_sleep() {
    let source = include_str!("../src/proxy_runtime.rs");
    for forbidden in ["thread::spawn", "sleep(", "tokio", "async fn"] {
        assert!(!source.contains(forbidden));
    }
}

#[test]
fn checkpoint_c_does_not_construct_proxy_draft() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(!source.contains("ProxyDraft {"));
    assert!(!source.contains("build_proxy_draft"));
}

#[test]
fn checkpoint_c_does_not_populate_trace_metadata() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(!source.contains("build_proxy_draft_trace_metadata"));
    assert!(!source.contains("ProxyDraftEvidenceSummary"));
}

// Lifecycle amendment (Checkpoint D isolation patch): after authorized D `lib.rs`
// wiring adds `mod proxy_ask` / `mod proxy_draft_safety`, scanning shared `lib.rs`
// would false-fail while C runtime behavior remains isolated. Guard C-owned source.
#[test]
fn checkpoint_c_does_not_start_checkpoint_d() {
    let runtime_source = include_str!("../src/proxy_runtime.rs");
    for forbidden in [
        "ask_my_proxy_local",
        "build_proxy_draft_trace_metadata",
        "proxy_ask.rs",
    ] {
        assert!(
            !runtime_source.contains(forbidden),
            "checkpoint C runtime source must not start D symbol {forbidden}"
        );
    }
}

#[test]
fn checkpoint_c_does_not_execute_dg() {
    // Lifecycle guard amendment: Checkpoint C owns `proxy_runtime.rs`. DG adapter wiring
    // in shared `lib.rs` is authorized post-DG; C product source must remain AXGA-free.
    let runtime_source = include_str!("../src/proxy_runtime.rs");
    for forbidden in [
        "AxgaAiProxyDraftRuntime",
        "proxy_runtime_axga",
        "axga-ai",
        "axga_ai",
        "resolve_production_proxy_draft_runtime",
        "proxy_runtime_factory",
        "std::env",
        "var_os",
        "var(",
    ] {
        assert!(
            !runtime_source.contains(forbidden),
            "checkpoint C runtime source must not execute DG adapter symbol `{forbidden}`"
        );
    }
}

#[test]
fn checkpoint_c_does_not_start_0_1_7() {
    let source = include_str!("../src/proxy_runtime.rs");
    assert!(!source.contains("0.1.7"));
    assert!(!source.contains("citation"));
}
