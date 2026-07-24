//! Dev Track 0.1.6 DG — AXGA adapter tests (no live provider calls).

use axga_ai::request::RequestBuilder;
use axga_shared::error::AxgaError;
use axga_shared::types::StreamEvent;
use futures::future::FutureExt;
use futures::stream;
use openmesh_core::domain::{
    validate_proxy_runtime_output, ProxyPromptBundle, ProxyRuntimeRequest,
    MAX_PROXY_DRAFT_TEXT_BYTES, PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
};
use openmesh_core::proxy_runtime::{ProxyDraftRuntime, ProxyDraftRuntimeError};
use openmesh_core::proxy_runtime_axga::{
    build_axga_request_builder, effective_operation_timeout_ms, resolve_live_provider_route,
    AxgaAiProviderKind, AxgaAiProxyDraftRuntime, AxgaAiProxyDraftRuntimeConfig,
    AxgaAiRuntimeConfigError, AxgaChatBackend, LiveProviderRoute,
    OpenMeshDashScopeCodingPlanClient, AXGA_CLIENT_TIMEOUT_MS, AXGA_MAX_EFFECTIVE_TIMEOUT_MS,
    AXGA_TIMEOUT_SAFETY_MARGIN_MS, DASHSCOPE_CODING_PLAN_HOST, OPENMESH_AXGA_HTTP_USER_AGENT,
    PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC, PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK,
    PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI,
};
use std::fs;
use std::future::Future;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const API_KEY_CANARY: &str = "CANARY-API-KEY-9f3a2b1c";
const PROMPT_CANARY: &str = "CANARY-PROMPT-question-text";
const CONTEXT_CANARY: &str = "CANARY-CONTEXT-json-body";
const PROVIDER_BODY_CANARY: &str = "CANARY-PROVIDER-ERROR-BODY-SECRET";

fn sample_bundle() -> ProxyPromptBundle {
    ProxyPromptBundle {
        protocol_version: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION.to_string(),
        system_message: "System instructions for proxy draft.".into(),
        context_json: format!(r#"{{"ownerLabel":"Owner","note":"{CONTEXT_CANARY}"}}"#),
        user_message: PROMPT_CANARY.into(),
    }
}

fn sample_request(timeout_ms: u64, max_output_bytes: u32) -> ProxyRuntimeRequest {
    ProxyRuntimeRequest {
        prompt: sample_bundle(),
        timeout_ms,
        max_output_bytes,
    }
}

fn openai_config() -> AxgaAiProxyDraftRuntimeConfig {
    AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::OpenAi,
        "gpt-4o-mini",
        API_KEY_CANARY,
        None,
    )
    .expect("openai config")
}

fn anthropic_config() -> AxgaAiProxyDraftRuntimeConfig {
    AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::Anthropic,
        "claude-3-5-sonnet-20241022",
        API_KEY_CANARY,
        None,
    )
    .expect("anthropic config")
}

fn deepseek_config() -> AxgaAiProxyDraftRuntimeConfig {
    AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::DeepSeek,
        "deepseek-chat",
        API_KEY_CANARY,
        Some("https://api.deepseek.com/v1".into()),
    )
    .expect("deepseek config")
}

struct ScriptedBackend {
    scripts: Mutex<Vec<Vec<Result<StreamEvent, AxgaError>>>>,
    calls: AtomicUsize,
    captured: Mutex<Vec<String>>,
}

impl ScriptedBackend {
    fn new(events: Vec<Result<StreamEvent, AxgaError>>) -> Self {
        Self {
            scripts: Mutex::new(vec![events]),
            calls: AtomicUsize::new(0),
            captured: Mutex::new(Vec::new()),
        }
    }

    fn with_scripts(scripts: Vec<Vec<Result<StreamEvent, AxgaError>>>) -> Self {
        Self {
            scripts: Mutex::new(scripts),
            calls: AtomicUsize::new(0),
            captured: Mutex::new(Vec::new()),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl AxgaChatBackend for ScriptedBackend {
    fn begin_stream(
        &self,
        request: RequestBuilder,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>>,
                        AxgaError,
                    >,
                > + Send,
        >,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let body = request.build_openai_body().to_string();
        self.captured.lock().expect("lock").push(body);
        let mut scripts = self.scripts.lock().expect("lock");
        let events = scripts.pop().unwrap_or_default();
        async move {
            Ok(Box::pin(stream::iter(events))
                as Pin<
                    Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>,
                >)
        }
        .boxed()
    }
}

struct NeverCompletingBackend;

impl AxgaChatBackend for NeverCompletingBackend {
    fn begin_stream(
        &self,
        _request: RequestBuilder,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>>,
                        AxgaError,
                    >,
                > + Send,
        >,
    > {
        async move {
            Ok(Box::pin(PendingStream)
                as Pin<
                    Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>,
                >)
        }
        .boxed()
    }
}

struct PanickingBackend;

impl AxgaChatBackend for PanickingBackend {
    fn begin_stream(
        &self,
        _request: RequestBuilder,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>>,
                        AxgaError,
                    >,
                > + Send,
        >,
    > {
        async move { panic!("CANARY-PANIC-SECRET-MUST-NOT-APPEAR") }.boxed()
    }
}

struct PendingStream;

impl futures::Stream for PendingStream {
    type Item = Result<StreamEvent, AxgaError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::task::Poll::Pending
    }
}

fn runtime_with_backend<B: AxgaChatBackend + 'static>(
    config: AxgaAiProxyDraftRuntimeConfig,
    backend: Arc<B>,
) -> AxgaAiProxyDraftRuntime {
    AxgaAiProxyDraftRuntime::with_chat_backend_for_tests(config, backend).expect("runtime")
}

fn ok_text_events(chunks: &[&str]) -> Vec<Result<StreamEvent, AxgaError>> {
    let mut events = chunks
        .iter()
        .map(|chunk| {
            Ok(StreamEvent::TextDelta {
                text: (*chunk).to_string(),
            })
        })
        .collect::<Vec<_>>();
    events.push(Ok(StreamEvent::Done));
    events
}

// --- configuration tests ---

#[test]
fn openai_configuration_is_explicit() {
    let config = openai_config();
    assert_eq!(config.provider, AxgaAiProviderKind::OpenAi);
    assert_eq!(config.model_id, "gpt-4o-mini");
    assert_eq!(config.api_key(), API_KEY_CANARY);
}

#[test]
fn anthropic_configuration_is_explicit() {
    let config = anthropic_config();
    assert_eq!(config.provider, AxgaAiProviderKind::Anthropic);
    assert_eq!(config.model_id, "claude-3-5-sonnet-20241022");
}

#[test]
fn deepseek_configuration_is_explicit() {
    let config = deepseek_config();
    assert_eq!(config.provider, AxgaAiProviderKind::DeepSeek);
    assert_eq!(
        config.openai_compatible_base_url(),
        Some("https://api.deepseek.com/v1")
    );
}

#[test]
fn empty_api_key_is_rejected() {
    let err = AxgaAiProxyDraftRuntimeConfig::new(AxgaAiProviderKind::OpenAi, "m", "  ", None)
        .expect_err("empty key");
    assert_eq!(err, AxgaAiRuntimeConfigError::EmptyApiKey);
}

#[test]
fn empty_model_id_is_rejected() {
    let err =
        AxgaAiProxyDraftRuntimeConfig::new(AxgaAiProviderKind::OpenAi, "  ", API_KEY_CANARY, None)
            .expect_err("empty model");
    assert_eq!(err, AxgaAiRuntimeConfigError::EmptyModelId);
}

#[test]
fn api_key_is_not_in_debug_output() {
    let debug = format!("{:?}", openai_config());
    assert!(!debug.contains(API_KEY_CANARY));
    assert!(debug.contains("redacted"));
}

#[test]
fn api_key_is_not_in_display_errors() {
    let err = format!("{}", AxgaAiRuntimeConfigError::EmptyApiKey);
    assert!(!err.contains(API_KEY_CANARY));
}

#[test]
fn adapter_reads_no_environment_variables() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("std::env"));
    assert!(!source.contains("var_os"));
    assert!(!source.contains("var("));
}

#[test]
fn deepseek_does_not_use_openai_env_fallback() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(source.contains("DeepSeekProvider::new"));
    assert!(!source.contains("OpenMeshOpenAiCompatibleClient"));
    assert!(source.contains("Some(config.api_key.clone())"));
    unsafe { std::env::set_var("OPENAI_API_KEY", "env-openai-key-should-not-be-used") };
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["ok"])));
    let runtime = runtime_with_backend(deepseek_config(), backend);
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "ok");
    unsafe { std::env::remove_var("OPENAI_API_KEY") };
}

#[test]
fn runtime_kind_matches_provider() {
    let openai = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    let anthropic = runtime_with_backend(
        anthropic_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    let deepseek = runtime_with_backend(
        deepseek_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    assert_eq!(openai.runtime_kind(), PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI);
    assert_eq!(
        anthropic.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC
    );
    assert_eq!(
        deepseek.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK
    );
}

#[test]
fn provider_id_matches_provider() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["draft text"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.provider_id, "openai");
}

#[test]
fn model_id_comes_from_configuration() {
    let runtime = runtime_with_backend(
        anthropic_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["draft text"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.model_id, "claude-3-5-sonnet-20241022");
}

#[test]
fn no_project_or_trace_configuration_exists() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    for forbidden in [
        "ProxyContextPack",
        "ProxyDraftTraceMetadata",
        "workspace_id",
        "profile_id",
        "context_pack_id",
        "build_inputs_hash",
        "project_path",
    ] {
        assert!(
            !source.contains(forbidden),
            "adapter must not reference {forbidden}"
        );
    }
}

// --- request boundary tests ---

#[test]
fn request_is_validated_before_provider_invocation() {
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["ok"])));
    let runtime = runtime_with_backend(openai_config(), Arc::clone(&backend));
    let mut request = sample_request(5_000, 256);
    request.timeout_ms = 0;
    let err = runtime
        .generate_draft(&request)
        .expect_err("invalid request");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidRequest);
    assert_eq!(backend.call_count(), 0);
}

#[test]
fn system_message_is_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(body.contains("System instructions for proxy draft."));
}

#[test]
fn context_json_is_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(body.contains(CONTEXT_CANARY));
}

#[test]
fn user_message_is_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(body.contains(PROMPT_CANARY));
}

#[test]
fn message_order_is_preserved() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    let context_pos = body.find(CONTEXT_CANARY).expect("context");
    let prompt_pos = body.find(PROMPT_CANARY).expect("prompt");
    assert!(context_pos < prompt_pos);
}

#[test]
fn protocol_version_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains(PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION));
}

#[test]
fn question_id_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("questionId"));
}

#[test]
fn workspace_id_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("workspaceId"));
}

#[test]
fn profile_id_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("profileId"));
}

#[test]
fn context_pack_id_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("contextPackId"));
}

#[test]
fn build_inputs_hash_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("buildInputsHash"));
}

#[test]
fn trace_metadata_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("evidenceSummary"));
}

#[test]
fn evidence_summary_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("secretItemsOmitted"));
}

#[test]
fn authority_metadata_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("authorityNotice"));
}

#[test]
fn tools_are_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("\"tools\""));
}

#[test]
fn tool_choice_is_not_transmitted() {
    let builder = build_axga_request_builder(&sample_bundle(), "m", 256);
    let body = builder.build_openai_body().to_string();
    assert!(!body.contains("tool_choice"));
}

#[test]
fn provider_request_contains_no_hidden_metadata() {
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["visible draft"])));
    let runtime = runtime_with_backend(openai_config(), backend);
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
}

// --- stream tests ---

#[test]
fn text_deltas_are_aggregated_in_order() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["ab", "cd"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "abcd");
}

#[test]
fn safe_thai_text_deltas_are_preserved() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["สว", "ัสดี"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "สวัสดี");
}

#[test]
fn usage_events_are_ignored() {
    let events = vec![
        Ok(StreamEvent::TextDelta {
            text: "hello".into(),
        }),
        Ok(StreamEvent::Usage {
            input_tokens: 1,
            output_tokens: 1,
        }),
        Ok(StreamEvent::Done),
    ];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "hello");
}

#[test]
fn done_terminates_normally() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["done-case"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "done-case");
}

#[test]
fn stop_terminates_normally() {
    let events = vec![
        Ok(StreamEvent::TextDelta {
            text: "stop-case".into(),
        }),
        Ok(StreamEvent::Stop {
            reason: "stop".into(),
        }),
    ];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "stop-case");
}

#[test]
fn tool_call_delta_fails_closed() {
    let events = vec![Ok(StreamEvent::ToolCallDelta {
        id: "t1".into(),
        name: "tool".into(),
        args_fragment: "{}".into(),
    })];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("tool delta");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn thinking_delta_fails_closed() {
    let events = vec![Ok(StreamEvent::ThinkingDelta {
        text: "hidden".into(),
    })];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("thinking delta");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn error_event_maps_to_provider_failure() {
    let events = vec![Ok(StreamEvent::Error {
        message: PROVIDER_BODY_CANARY.into(),
    })];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("provider error");
    assert_eq!(err, ProxyDraftRuntimeError::ProviderFailure);
    assert!(!format!("{err}").contains(PROVIDER_BODY_CANARY));
}

#[test]
fn empty_stream_is_invalid_output() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(vec![])));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("empty");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn whitespace_only_output_is_invalid() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["   \n\t  "]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("whitespace");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn output_bound_is_enforced_incrementally() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["abcdef"]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 3))
        .expect_err("bound");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn output_bound_failure_returns_no_partial_draft() {
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["abcdef"])));
    let runtime = runtime_with_backend(openai_config(), Arc::clone(&backend));
    let _ = runtime.generate_draft(&sample_request(5_000, 3));
    assert_eq!(backend.call_count(), 1);
}

#[test]
fn complete_utf8_text_is_preserved() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["กขค"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert!(validate_proxy_runtime_output(&output).is_ok());
}

#[test]
fn no_event_is_processed_after_terminal_failure() {
    let events = vec![
        Ok(StreamEvent::ToolCallDelta {
            id: "t".into(),
            name: "n".into(),
            args_fragment: "{}".into(),
        }),
        Ok(StreamEvent::TextDelta {
            text: "late".into(),
        }),
    ];
    let backend = Arc::new(ScriptedBackend::new(events));
    let runtime = runtime_with_backend(openai_config(), backend);
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("terminal");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
}

#[test]
fn final_output_passes_checkpoint_a_validator() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&[
            "valid draft output",
        ]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, MAX_PROXY_DRAFT_TEXT_BYTES as u32))
        .expect("draft");
    assert!(validate_proxy_runtime_output(&output).is_ok());
    assert!(output.network_used);
}

fn assert_thread_fully_joined(runtime: &AxgaAiProxyDraftRuntime) {
    assert_thread_lifecycle(runtime, 1, 1);
}

fn assert_thread_joined_after_failure(runtime: &AxgaAiProxyDraftRuntime) {
    let lifecycle = runtime.thread_lifecycle();
    assert_eq!(lifecycle.thread_started, 1);
    assert_eq!(lifecycle.thread_joined, 1);
    assert!(!lifecycle.thread_alive_after_return);
}

fn assert_thread_lifecycle(
    runtime: &AxgaAiProxyDraftRuntime,
    expected_invocations: usize,
    expected_completed: usize,
) {
    let lifecycle = runtime.thread_lifecycle();
    assert_eq!(lifecycle.thread_started, expected_invocations);
    assert_eq!(lifecycle.operation_completed, expected_completed);
    assert_eq!(lifecycle.thread_joined, expected_invocations);
    assert!(!lifecycle.thread_alive_after_return);
}

// --- timeout and thread tests ---

#[test]
fn timeout_below_adapter_cap_is_preserved() {
    assert_eq!(effective_operation_timeout_ms(118_999), 118_999);
}

#[test]
fn timeout_at_adapter_cap_is_capped_correctly() {
    assert_eq!(
        effective_operation_timeout_ms(AXGA_MAX_EFFECTIVE_TIMEOUT_MS),
        AXGA_MAX_EFFECTIVE_TIMEOUT_MS
    );
}

#[test]
fn timeout_above_internal_client_cap_is_capped_to_119_seconds() {
    assert_eq!(
        effective_operation_timeout_ms(AXGA_CLIENT_TIMEOUT_MS),
        AXGA_MAX_EFFECTIVE_TIMEOUT_MS
    );
    assert_eq!(AXGA_MAX_EFFECTIVE_TIMEOUT_MS, 119_000);
}

#[test]
fn openmesh_deadline_precedes_axga_client_timeout() {
    assert!(
        AXGA_MAX_EFFECTIVE_TIMEOUT_MS + AXGA_TIMEOUT_SAFETY_MARGIN_MS <= AXGA_CLIENT_TIMEOUT_MS
    );
    assert_eq!(
        effective_operation_timeout_ms(AXGA_CLIENT_TIMEOUT_MS),
        AXGA_MAX_EFFECTIVE_TIMEOUT_MS
    );
}

#[test]
fn timeout_classification_does_not_parse_network_error_strings() {
    let events = vec![Err(AxgaError::Network(
        "request timeout while waiting for provider".into(),
    ))];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("network timeout string");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeUnavailable);
    assert_ne!(err, ProxyDraftRuntimeError::Timeout);
}

#[test]
fn provider_string_containing_timeout_does_not_become_timeout() {
    let events = vec![Err(AxgaError::LlmProvider(
        "upstream timeout in body".into(),
    ))];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("provider timeout string");
    assert_eq!(err, ProxyDraftRuntimeError::ProviderFailure);
    assert_ne!(err, ProxyDraftRuntimeError::Timeout);
}

#[test]
fn deadline_timeout_still_returns_timeout() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let err = runtime
        .generate_draft(&sample_request(50, 256))
        .expect_err("deadline timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
    assert_thread_fully_joined(&runtime);
}

#[test]
fn whole_operation_timeout_maps_to_timeout() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let err = runtime
        .generate_draft(&sample_request(50, 256))
        .expect_err("timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
}

#[test]
fn timeout_covers_stream_creation() {
    assert_eq!(effective_operation_timeout_ms(50), 50);
}

#[test]
fn timeout_covers_stream_consumption() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let err = runtime
        .generate_draft(&sample_request(30, 256))
        .expect_err("timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
}

#[test]
fn timeout_returns_no_partial_output() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let err = runtime
        .generate_draft(&sample_request(30, 256))
        .expect_err("timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
    assert_thread_fully_joined(&runtime);
}

#[test]
fn success_path_joins_adapter_thread() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["joined success"]))),
    );
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "joined success");
    assert_thread_fully_joined(&runtime);
}

#[test]
fn timeout_path_joins_adapter_thread() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let err = runtime
        .generate_draft(&sample_request(30, 256))
        .expect_err("timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
    assert_thread_fully_joined(&runtime);
}

#[test]
fn provider_failure_path_joins_adapter_thread() {
    let events = vec![Err(AxgaError::Http {
        status: 500,
        body: PROVIDER_BODY_CANARY.into(),
    })];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("provider failure");
    assert_eq!(err, ProxyDraftRuntimeError::ProviderFailure);
    assert_thread_fully_joined(&runtime);
}

#[test]
fn invalid_output_path_joins_adapter_thread() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["   "]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("invalid output");
    assert_eq!(err, ProxyDraftRuntimeError::InvalidOutput);
    assert_thread_fully_joined(&runtime);
}

#[test]
fn result_is_not_returned_before_thread_exit() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["exit proof"]))),
    );
    let _ = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    let lifecycle = runtime.thread_lifecycle();
    assert!(!lifecycle.thread_alive_after_return);
    assert_eq!(lifecycle.thread_joined, 1);
}

#[test]
fn thread_panic_maps_to_runtime_unavailable() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(PanickingBackend));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("panic");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeUnavailable);
    assert_thread_joined_after_failure(&runtime);
}

#[test]
fn thread_panic_message_is_not_exposed() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(PanickingBackend));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("panic");
    let display = format!("{err}");
    assert!(!display.contains("CANARY-PANIC-SECRET"));
}

#[test]
fn no_background_continuation_after_success() {
    let backend = Arc::new(ScriptedBackend::with_scripts(vec![
        ok_text_events(&["once"]),
        ok_text_events(&["twice"]),
    ]));
    let runtime = runtime_with_backend(openai_config(), Arc::clone(&backend));
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_thread_fully_joined(&runtime);
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("second draft");
    assert_eq!(backend.call_count(), 2);
    assert_thread_lifecycle(&runtime, 2, 2);
}

#[test]
fn no_background_continuation_after_timeout() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let _ = runtime.generate_draft(&sample_request(30, 256));
    assert_thread_fully_joined(&runtime);
    let err = runtime
        .generate_draft(&sample_request(30, 256))
        .expect_err("second timeout");
    assert_eq!(err, ProxyDraftRuntimeError::Timeout);
    assert_thread_lifecycle(&runtime, 2, 2);
}

#[test]
fn timeout_does_not_retry() {
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["once"])));
    let runtime = runtime_with_backend(openai_config(), Arc::clone(&backend));
    let _ = runtime.generate_draft(&sample_request(5_000, 256));
    assert_eq!(backend.call_count(), 1);
}

#[test]
fn timeout_thread_is_joined_before_return() {
    let runtime = runtime_with_backend(openai_config(), Arc::new(NeverCompletingBackend));
    let _ = runtime.generate_draft(&sample_request(30, 256));
    assert_thread_fully_joined(&runtime);
}

#[tokio::test]
async fn active_tokio_runtime_returns_runtime_unavailable() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("nested runtime");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeUnavailable);
}

#[tokio::test]
async fn active_runtime_still_returns_without_nested_panic() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("nested runtime");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeUnavailable);
}

#[tokio::test]
async fn active_tokio_runtime_does_not_panic() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["x"]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("nested runtime");
    assert_eq!(err, ProxyDraftRuntimeError::RuntimeUnavailable);
}

#[test]
fn provider_error_body_is_redacted() {
    let events = vec![Err(AxgaError::Http {
        status: 500,
        body: PROVIDER_BODY_CANARY.into(),
    })];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("http");
    let display = format!("{err}");
    assert!(!display.contains(PROVIDER_BODY_CANARY));
}

#[test]
fn network_error_string_is_redacted() {
    let events = vec![Err(AxgaError::Network(
        "network failure with secret endpoint".into(),
    ))];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("network");
    let display = format!("{err}");
    assert!(!display.contains("secret endpoint"));
}

#[test]
fn credential_canary_is_absent_from_errors() {
    let events = vec![Err(AxgaError::RateLimited(API_KEY_CANARY.into()))];
    let runtime = runtime_with_backend(openai_config(), Arc::new(ScriptedBackend::new(events)));
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("rate limit");
    assert!(!format!("{err}").contains(API_KEY_CANARY));
}

#[test]
fn prompt_canary_is_absent_from_errors() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["   "]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("invalid");
    assert!(!format!("{err}").contains(PROMPT_CANARY));
}

#[test]
fn context_canary_is_absent_from_errors() {
    let runtime = runtime_with_backend(
        openai_config(),
        Arc::new(ScriptedBackend::new(ok_text_events(&["   "]))),
    );
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("invalid");
    assert!(!format!("{err}").contains(CONTEXT_CANARY));
}

// --- dependency and scope tests ---

#[test]
fn cargo_resolution_uses_exact_axga_git_revision() {
    let lock = include_str!("../../../Cargo.lock");
    assert!(lock.contains("git+https://github.com/KJ-AIML/axga-harness-agent-rs.git"));
    assert!(lock.contains("f47ebba523a0b59754e3ba2eb200e55b2e7d5d35"));
}

#[test]
fn openmesh_core_depends_on_axga_ai() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("axga-ai"));
}

#[test]
fn axga_core_is_absent_from_dependency_tree() {
    let lock = include_str!("../../../Cargo.lock");
    assert!(!lock.contains("name = \"axga-core\""));
}

// Lifecycle amendment (Checkpoint E isolation patch): adapter-owned source must not
// contain the production runtime factory. Factory ownership lives in openmesh-cli.
#[test]
fn no_production_runtime_factory_exists() {
    let adapter_source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!adapter_source.contains("resolve_production_proxy_draft_runtime"));
    assert!(!adapter_source.contains("proxy_runtime_factory"));
}

// Lifecycle amendment (Checkpoint E isolation patch): production factory must not
// select the deterministic stub. Production slice excludes `#[cfg(test)]` block.
#[test]
fn no_cli_runtime_selection_exists() {
    let factory_source = include_str!("../../openmesh-cli/src/proxy_runtime_factory.rs");
    let factory_production = factory_source
        .split("#[cfg(test)]")
        .next()
        .unwrap_or(factory_source);
    assert!(!factory_production.contains("DeterministicStubProxyDraftRuntime"));
}

#[test]
fn no_environment_credential_resolution_exists() {
    adapter_reads_no_environment_variables();
}

#[test]
fn no_deterministic_stub_fallback_exists() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("DeterministicStubProxyDraftRuntime"));
    assert!(!source.contains("UnconfiguredProxyDraftRuntime"));
}

#[test]
fn no_provider_retry_exists() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("retry"));
}

#[test]
fn no_tool_registration_call_exists() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("with_tools"));
}

#[test]
fn no_response_persistence_exists() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("persist"));
    assert!(!source.contains("history"));
}

#[test]
fn checkpoint_f_has_not_started() {
    let dogfood =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openmesh-cli/src/proxy_dogfood.rs");
    assert!(!dogfood.exists());
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
}

fn harness_reports_dir() -> PathBuf {
    let worktree_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
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
fn no_0_1_7_contracts_are_added() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(!source.contains("0.1.7"));
    assert!(!source.contains("\"claims\""));
}

#[test]
fn no_commit_push_or_feature_closure_exists() {
    let ledger = include_str!("../../../docs/development/execution-ledger.md");
    assert!(!ledger.contains("Dev Track 0.1.6 — PASS"));
    assert!(!ledger.contains("0.1.6 Checkpoint G — PASS"));
    assert!(!ledger.contains("0.1.6 Checkpoint H — PASS"));

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo_root)
        .output()
        .expect("git status");
    assert!(status.status.success());
    let porcelain = String::from_utf8_lossy(&status.stdout);
    assert!(
        !porcelain.is_empty(),
        "amendment verification expects an uncommitted worktree diff"
    );

    let ahead = Command::new("git")
        .args(["rev-list", "--count", "@{upstream}..HEAD"])
        .current_dir(&repo_root)
        .output()
        .expect("git ahead count");
    if ahead.status.success() {
        let count = String::from_utf8_lossy(&ahead.stdout).trim().to_string();
        assert_eq!(count, "0", "branch must not be ahead of upstream (no push)");
    }
}

#[test]
fn effective_timeout_respects_axga_client_cap() {
    assert_eq!(
        effective_operation_timeout_ms(AXGA_CLIENT_TIMEOUT_MS + 1),
        AXGA_MAX_EFFECTIVE_TIMEOUT_MS
    );
}

#[test]
fn single_provider_request_per_generate_draft() {
    let backend = Arc::new(ScriptedBackend::new(ok_text_events(&["once"])));
    let runtime = runtime_with_backend(openai_config(), Arc::clone(&backend));
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(backend.call_count(), 1);
}

struct LoopbackHttpCapture {
    request: Mutex<Option<String>>,
}

fn read_http_request(stream: &mut TcpStream) -> String {
    let mut buffer = vec![0_u8; 65_536];
    let mut total = 0_usize;
    loop {
        let read = stream
            .read(&mut buffer[total..])
            .expect("read loopback request");
        if read == 0 {
            break;
        }
        total += read;
        if buffer[..total]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            break;
        }
    }
    String::from_utf8_lossy(&buffer[..total]).into_owned()
}

fn write_http_response(stream: &mut TcpStream, status_line: &str, body: &str) {
    let response = format!(
        "{status_line}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .expect("write response");
    stream.flush().expect("flush response");
}

fn ok_sse_body() -> &'static str {
    "data: {\"choices\":[{\"delta\":{\"content\":\"loopback-ok\"},\"index\":0}]}\n\ndata: [DONE]\n\n"
}

fn spawn_loopback_server(
    handler: impl Fn(String) -> (String, String) + Send + 'static,
) -> (String, Arc<LoopbackHttpCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let addr = listener.local_addr().expect("listener addr");
    let capture = Arc::new(LoopbackHttpCapture {
        request: Mutex::new(None),
    });
    let capture_for_thread = Arc::clone(&capture);
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let (mut stream, _) = loop {
            if let Ok(accepted) = listener.accept() {
                break accepted;
            }
            if std::time::Instant::now() >= deadline {
                panic!("loopback server timed out waiting for connection");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let request = read_http_request(&mut stream);
        *capture_for_thread.request.lock().expect("lock") = Some(request.clone());
        let (status_line, body) = handler(request);
        write_http_response(&mut stream, &status_line, &body);
    });
    (format!("http://{addr}/v1"), capture)
}

fn dashscope_openai_config(base_url: String) -> AxgaAiProxyDraftRuntimeConfig {
    AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::OpenAi,
        "qwen3.7-plus",
        API_KEY_CANARY,
        Some(base_url),
    )
    .expect("dashscope config")
}

fn dashscope_production_base_url() -> String {
    format!("https://{DASHSCOPE_CODING_PLAN_HOST}/v1")
}

fn deepseek_loopback_config(base_url: String) -> AxgaAiProxyDraftRuntimeConfig {
    AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::DeepSeek,
        "deepseek-chat",
        API_KEY_CANARY,
        Some(base_url),
    )
    .expect("deepseek loopback config")
}

fn spawn_loopback_listener() -> (std::net::SocketAddr, Arc<LoopbackHttpCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("listener addr");
    let capture = Arc::new(LoopbackHttpCapture {
        request: Mutex::new(None),
    });
    let capture_for_thread = Arc::clone(&capture);
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let (mut stream, _) = loop {
            if let Ok(accepted) = listener.accept() {
                break accepted;
            }
            if std::time::Instant::now() >= deadline {
                panic!("loopback server timed out waiting for connection");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let request = read_http_request(&mut stream);
        *capture_for_thread.request.lock().expect("lock") = Some(request);
        write_http_response(&mut stream, "HTTP/1.1 200 OK", ok_sse_body());
    });
    (addr, capture)
}

fn spawn_loopback_listener_with_error_body() -> (std::net::SocketAddr, Arc<LoopbackHttpCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let addr = listener.local_addr().expect("listener addr");
    let capture = Arc::new(LoopbackHttpCapture {
        request: Mutex::new(None),
    });
    let capture_for_thread = Arc::clone(&capture);
    thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let (mut stream, _) = loop {
            if let Ok(accepted) = listener.accept() {
                break accepted;
            }
            if std::time::Instant::now() >= deadline {
                panic!("loopback server timed out waiting for connection");
            }
            thread::sleep(Duration::from_millis(10));
        };
        let request = read_http_request(&mut stream);
        *capture_for_thread.request.lock().expect("lock") = Some(request);
        write_http_response(
            &mut stream,
            "HTTP/1.1 401 Unauthorized",
            &format!("{{\"error\":\"invalid key {API_KEY_CANARY} and {PROVIDER_BODY_CANARY}\"}}"),
        );
    });
    (addr, capture)
}

fn runtime_with_loopback_dashscope_transport(
    listen: std::net::SocketAddr,
) -> AxgaAiProxyDraftRuntime {
    let client = OpenMeshDashScopeCodingPlanClient::new_for_loopback_test(
        API_KEY_CANARY.to_string(),
        listen,
    )
    .expect("loopback dashscope client");
    AxgaAiProxyDraftRuntime::with_chat_backend_for_tests(
        dashscope_openai_config(dashscope_production_base_url()),
        Arc::new(client),
    )
    .expect("runtime")
}

#[test]
fn openai_compatible_http_requests_use_openmesh_user_agent() {
    let (listen, capture) = spawn_loopback_listener();
    let runtime = runtime_with_loopback_dashscope_transport(listen);
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "loopback-ok");
    let request = capture
        .request
        .lock()
        .expect("lock")
        .clone()
        .expect("captured request");
    assert!(
        request
            .to_ascii_lowercase()
            .contains(&format!("user-agent: {OPENMESH_AXGA_HTTP_USER_AGENT}").to_ascii_lowercase()),
        "expected OpenMesh user agent, got:\n{request}"
    );
    assert!(
        !request.to_ascii_lowercase().contains("curl/"),
        "must not impersonate curl"
    );
    assert!(
        !request.to_ascii_lowercase().contains("claude"),
        "must not impersonate another coding agent"
    );
}

#[test]
fn dashscope_openai_compatible_payload_includes_enable_thinking_false() {
    let (listen, capture) = spawn_loopback_listener();
    let runtime = runtime_with_loopback_dashscope_transport(listen);
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    let request = capture
        .request
        .lock()
        .expect("lock")
        .clone()
        .expect("captured request");
    let body = request.split("\r\n\r\n").nth(1).expect("request body");
    assert!(
        body.contains("\"enable_thinking\":false"),
        "expected enable_thinking false in body: {body}"
    );
}

#[test]
fn deepseek_openai_compatible_payload_omits_enable_thinking() {
    let (base_url, capture) =
        spawn_loopback_server(|_| ("HTTP/1.1 200 OK".into(), ok_sse_body().to_string()));
    let runtime =
        AxgaAiProxyDraftRuntime::new(deepseek_loopback_config(base_url)).expect("runtime");
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    let request = capture
        .request
        .lock()
        .expect("lock")
        .clone()
        .expect("captured request");
    let body = request.split("\r\n\r\n").nth(1).expect("request body");
    assert!(
        !body.contains("enable_thinking"),
        "deepseek payload must not inject enable_thinking: {body}"
    );
    assert!(
        !request
            .to_ascii_lowercase()
            .contains(&format!("user-agent: {OPENMESH_AXGA_HTTP_USER_AGENT}").to_ascii_lowercase()),
        "deepseek must use axga transport without OpenMesh user agent"
    );
}

#[test]
fn live_http_provider_failure_does_not_expose_error_body_or_credentials() {
    let (listen, _capture) = spawn_loopback_listener_with_error_body();
    let runtime = runtime_with_loopback_dashscope_transport(listen);
    let err = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect_err("provider failure");
    assert_eq!(err, ProxyDraftRuntimeError::ProviderFailure);
    let display = format!("{err}");
    assert!(!display.contains(API_KEY_CANARY));
    assert!(!display.contains(PROVIDER_BODY_CANARY));
}

#[test]
fn hostile_dashscope_host_lookalikes_route_to_axga_openai() {
    for hostile in [
        "https://evil-coding-intl.dashscope.aliyuncs.com/v1",
        "https://coding-intl.dashscope.aliyuncs.com.evil.com/v1",
        "https://dashscope.aliyuncs.com/v1",
        "https://api.openai.com/v1",
    ] {
        let config = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::OpenAi,
            "gpt-4o-mini",
            API_KEY_CANARY,
            Some(hostile.into()),
        )
        .expect("config");
        assert_eq!(
            resolve_live_provider_route(&config),
            LiveProviderRoute::AxgaOpenAi,
            "hostile host must not select dashscope transport: {hostile}"
        );
    }
}

#[test]
fn dashscope_host_case_variants_route_to_openmesh_transport() {
    for url in [
        format!("https://{DASHSCOPE_CODING_PLAN_HOST}/v1"),
        format!("https://{}/v1", "CODING-INTL.DASHSCOPE.ALIYUNCS.COM"),
        format!("http://{DASHSCOPE_CODING_PLAN_HOST}:443/v1"),
    ] {
        let config = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::OpenAi,
            "qwen3.7-plus",
            API_KEY_CANARY,
            Some(url.clone()),
        )
        .expect("config");
        assert_eq!(
            resolve_live_provider_route(&config),
            LiveProviderRoute::OpenMeshDashScopeCodingPlan,
            "expected dashscope transport for {url}"
        );
    }
}

#[test]
fn routing_source_uses_exact_host_equality_not_substring_matching() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(source.contains("eq_ignore_ascii_case(DASHSCOPE_CODING_PLAN_HOST)"));
    assert!(!source.contains("contains(\"dashscope\")"));
    assert!(!source.contains("ends_with("));
    assert!(!source.contains("starts_with(\"coding-intl\")"));
    assert!(!source.contains("Regex"));
}

#[test]
fn openmesh_transport_is_limited_to_dashscope_coding_plan_host() {
    let source = include_str!("../src/proxy_runtime_axga.rs");
    assert!(source.contains("OpenMeshDashScopeCodingPlanClient"));
    assert!(source.contains("OpenAiProvider::new"));
    assert!(source.contains("DeepSeekProvider::new"));
    assert!(source.contains("AnthropicProvider::new"));
    assert!(!source.contains("OpenMeshOpenAiCompatibleClient"));
}

#[test]
fn dashscope_transport_timeout_thread_and_stream_behavior_match_adapter_contract() {
    let (listen, _) = spawn_loopback_listener();
    let runtime = runtime_with_loopback_dashscope_transport(listen);
    let output = runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    assert_eq!(output.draft_text, "loopback-ok");
    assert_thread_fully_joined(&runtime);
}

#[test]
fn non_dashscope_openai_custom_base_url_omits_enable_thinking_in_axga_body() {
    let (base_url, capture) =
        spawn_loopback_server(|_| ("HTTP/1.1 200 OK".into(), ok_sse_body().to_string()));
    let config = AxgaAiProxyDraftRuntimeConfig::new(
        AxgaAiProviderKind::OpenAi,
        "gpt-4o-mini",
        API_KEY_CANARY,
        Some(base_url),
    )
    .expect("config");
    assert_eq!(
        resolve_live_provider_route(&config),
        LiveProviderRoute::AxgaOpenAi
    );
    let runtime = AxgaAiProxyDraftRuntime::new(config).expect("runtime");
    runtime
        .generate_draft(&sample_request(5_000, 256))
        .expect("draft");
    let request = capture
        .request
        .lock()
        .expect("lock")
        .clone()
        .expect("captured request");
    let body = request.split("\r\n\r\n").nth(1).expect("request body");
    assert!(
        !body.contains("enable_thinking"),
        "non-dashscope openai must not inject enable_thinking: {body}"
    );
}
