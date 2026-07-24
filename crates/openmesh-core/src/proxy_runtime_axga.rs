//! Dev Track 0.1.6 DG — thin OpenMesh adapter over the approved `axga-ai` runtime.

use crate::domain::{
    validate_proxy_runtime_output, validate_proxy_runtime_request, ProxyPromptBundle,
    ProxyRuntimeOutput, ProxyRuntimeRequest,
};
use crate::proxy_runtime::{ProxyDraftRuntime, ProxyDraftRuntimeError};
use axga_ai::providers::anthropic::AnthropicProvider;
use axga_ai::providers::deepseek::DeepSeekProvider;
use axga_ai::providers::openai::OpenAiProvider;
use axga_ai::providers::Provider;
use axga_ai::request::RequestBuilder;
use axga_ai::stream::SseStream;
use axga_shared::error::AxgaError;
use axga_shared::types::{AgentMessage, StreamEvent};
use futures::future::FutureExt;
use futures::StreamExt;
use reqwest::Client;
use serde_json::Value;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Frozen reqwest client timeout inside axga-ai providers.
pub const AXGA_CLIENT_TIMEOUT_MS: u64 = 120_000;

/// Safety margin so the OpenMesh whole-operation deadline expires before AXGA's client cap.
pub const AXGA_TIMEOUT_SAFETY_MARGIN_MS: u64 = 1_000;

/// Maximum effective OpenMesh deadline — always below `AXGA_CLIENT_TIMEOUT_MS`.
pub const AXGA_MAX_EFFECTIVE_TIMEOUT_MS: u64 =
    AXGA_CLIENT_TIMEOUT_MS - AXGA_TIMEOUT_SAFETY_MARGIN_MS;

/// Adapter-declared runtime kind for OpenAI-backed AXGA calls.
pub const PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI: &str = "axga-openai";

/// Adapter-declared runtime kind for Anthropic-backed AXGA calls.
pub const PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC: &str = "axga-anthropic";

/// Adapter-declared runtime kind for DeepSeek-backed AXGA calls.
pub const PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK: &str = "axga-deepseek";

const OPENAI_PROVIDER_ID: &str = "openai";
const ANTHROPIC_PROVIDER_ID: &str = "anthropic";
const DEEPSEEK_PROVIDER_ID: &str = "deepseek";

const ADAPTER_THREAD_NAME: &str = "openmesh-axga-adapter";

/// Truthful HTTP User-Agent for OpenMesh-owned DashScope Coding Plan transport only.
pub const OPENMESH_AXGA_HTTP_USER_AGENT: &str = "OpenMesh/0.1.6";

/// Exact DashScope Coding Plan OpenAI-compatible host (ASCII case-insensitive match only).
pub const DASHSCOPE_CODING_PLAN_HOST: &str = "coding-intl.dashscope.aliyuncs.com";

/// Resolved live-provider route for adapter configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveProviderRoute {
    AxgaAnthropic,
    AxgaDeepSeek,
    AxgaOpenAi,
    OpenMeshDashScopeCodingPlan,
}

/// Returns which backend transport the adapter selects for `config`.
pub fn resolve_live_provider_route(config: &AxgaAiProxyDraftRuntimeConfig) -> LiveProviderRoute {
    match config.provider {
        AxgaAiProviderKind::Anthropic => LiveProviderRoute::AxgaAnthropic,
        AxgaAiProviderKind::DeepSeek => LiveProviderRoute::AxgaDeepSeek,
        AxgaAiProviderKind::OpenAi => {
            if config
                .openai_compatible_base_url
                .as_deref()
                .and_then(|base_url| extract_host_from_base_url(base_url).ok())
                .as_deref()
                .is_some_and(is_dashscope_coding_plan_host)
            {
                LiveProviderRoute::OpenMeshDashScopeCodingPlan
            } else {
                LiveProviderRoute::AxgaOpenAi
            }
        }
    }
}

/// Per-runtime adapter-thread lifecycle counters for integration tests.
#[derive(Default)]
pub struct AdapterThreadLifecycleRecorder {
    thread_started: AtomicUsize,
    operation_completed: AtomicUsize,
    thread_joined: AtomicUsize,
    thread_alive: AtomicBool,
}

/// Observed adapter-thread lifecycle counters for integration tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AdapterThreadLifecycleSnapshot {
    pub thread_started: usize,
    pub operation_completed: usize,
    pub thread_joined: usize,
    pub thread_alive_after_return: bool,
}

impl AdapterThreadLifecycleRecorder {
    pub fn snapshot(&self) -> AdapterThreadLifecycleSnapshot {
        AdapterThreadLifecycleSnapshot {
            thread_started: self.thread_started.load(Ordering::SeqCst),
            operation_completed: self.operation_completed.load(Ordering::SeqCst),
            thread_joined: self.thread_joined.load(Ordering::SeqCst),
            thread_alive_after_return: self.thread_alive.load(Ordering::SeqCst),
        }
    }
}

/// Supported AXGA provider backends for the OpenMesh adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxgaAiProviderKind {
    OpenAi,
    Anthropic,
    DeepSeek,
}

/// Explicit adapter configuration. Credentials are supplied by the caller only.
pub struct AxgaAiProxyDraftRuntimeConfig {
    pub provider: AxgaAiProviderKind,
    pub model_id: String,
    api_key: String,
    openai_compatible_base_url: Option<String>,
}

impl AxgaAiProxyDraftRuntimeConfig {
    pub fn new(
        provider: AxgaAiProviderKind,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
        openai_compatible_base_url: Option<String>,
    ) -> Result<Self, AxgaAiRuntimeConfigError> {
        let model_id = model_id.into();
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(AxgaAiRuntimeConfigError::EmptyApiKey);
        }
        if model_id.trim().is_empty() {
            return Err(AxgaAiRuntimeConfigError::EmptyModelId);
        }
        if let Some(ref base_url) = openai_compatible_base_url {
            validate_openai_compatible_base_url(base_url)?;
        }
        if matches!(provider, AxgaAiProviderKind::Anthropic) && openai_compatible_base_url.is_some()
        {
            return Err(AxgaAiRuntimeConfigError::UnsupportedBaseUrlForProvider);
        }
        Ok(Self {
            provider,
            model_id,
            api_key,
            openai_compatible_base_url,
        })
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    pub fn openai_compatible_base_url(&self) -> Option<&str> {
        self.openai_compatible_base_url.as_deref()
    }
}

impl fmt::Debug for AxgaAiProxyDraftRuntimeConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AxgaAiProxyDraftRuntimeConfig")
            .field("provider", &self.provider)
            .field("model_id", &self.model_id)
            .field("api_key", &"<redacted>")
            .field(
                "openai_compatible_base_url",
                &self.openai_compatible_base_url,
            )
            .finish()
    }
}

/// Secret-safe configuration validation errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AxgaAiRuntimeConfigError {
    #[error("axga adapter api key must not be empty")]
    EmptyApiKey,
    #[error("axga adapter model id must not be empty")]
    EmptyModelId,
    #[error("axga adapter base url is invalid")]
    InvalidBaseUrl,
    #[error("axga adapter base url is unsupported for this provider")]
    UnsupportedBaseUrlForProvider,
    #[error("axga adapter provider construction failed")]
    ProviderConstructionFailed,
}

type AxgaEventStream = Pin<Box<dyn futures::Stream<Item = Result<StreamEvent, AxgaError>> + Send>>;

type AxgaStreamFuture = Pin<Box<dyn Future<Output = Result<AxgaEventStream, AxgaError>> + Send>>;

/// Internal stream backend seam for tests and live AXGA providers.
pub trait AxgaChatBackend: Send + Sync {
    fn begin_stream(&self, request: RequestBuilder) -> AxgaStreamFuture;
}

struct LiveAxgaChatBackend {
    inner: LiveProvider,
}

enum LiveProvider {
    OpenAi(OpenAiProvider),
    Anthropic(AnthropicProvider),
    DeepSeek(DeepSeekProvider),
    DashScopeCodingPlan(OpenMeshDashScopeCodingPlanClient),
}

#[doc(hidden)]
#[derive(Clone)]
pub struct OpenMeshDashScopeCodingPlanClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenMeshDashScopeCodingPlanClient {
    fn new(api_key: String, base_url: String) -> Result<Self, AxgaAiRuntimeConfigError> {
        Self::new_with_client(
            api_key,
            base_url,
            Client::builder()
                .user_agent(OPENMESH_AXGA_HTTP_USER_AGENT)
                .pool_max_idle_per_host(2)
                .timeout(Duration::from_millis(AXGA_CLIENT_TIMEOUT_MS))
                .build()
                .map_err(|_| AxgaAiRuntimeConfigError::ProviderConstructionFailed)?,
        )
    }

    fn new_with_client(
        api_key: String,
        base_url: String,
        client: Client,
    ) -> Result<Self, AxgaAiRuntimeConfigError> {
        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }

    /// Loopback-only constructor: resolves the DashScope host to a local socket for tests.
    #[doc(hidden)]
    pub fn new_for_loopback_test(
        api_key: String,
        loopback_socket: std::net::SocketAddr,
    ) -> Result<Self, AxgaAiRuntimeConfigError> {
        let base_url = format!("http://{DASHSCOPE_CODING_PLAN_HOST}/v1");
        let client = Client::builder()
            .user_agent(OPENMESH_AXGA_HTTP_USER_AGENT)
            .resolve(DASHSCOPE_CODING_PLAN_HOST, loopback_socket)
            .pool_max_idle_per_host(2)
            .timeout(Duration::from_millis(AXGA_CLIENT_TIMEOUT_MS))
            .build()
            .map_err(|_| AxgaAiRuntimeConfigError::ProviderConstructionFailed)?;
        Self::new_with_client(api_key, base_url, client)
    }

    async fn stream_chat(&self, request: &RequestBuilder) -> Result<AxgaEventStream, AxgaError> {
        let body = prepare_dashscope_coding_plan_body(request);
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AxgaError::Network(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            let _ = response.bytes().await;
            if status.as_u16() == 429 {
                return Err(AxgaError::RateLimited(String::new()));
            }
            return Err(AxgaError::Http {
                status: status.as_u16(),
                body: String::new(),
            });
        }

        Ok(Box::pin(SseStream {
            inner: response.bytes_stream(),
            buffer: String::with_capacity(4096),
            done: false,
        }))
    }
}

fn prepare_dashscope_coding_plan_body(request: &RequestBuilder) -> Value {
    let mut body = request.build_openai_body();
    if let Some(obj) = body.as_object_mut() {
        obj.insert("enable_thinking".to_string(), Value::Bool(false));
    }
    body
}

fn extract_host_from_base_url(base_url: &str) -> Result<String, AxgaAiRuntimeConfigError> {
    let trimmed = base_url.trim();
    let authority = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .and_then(|rest| rest.split('/').next())
        .filter(|segment| !segment.is_empty())
        .ok_or(AxgaAiRuntimeConfigError::InvalidBaseUrl)?;

    if authority.starts_with('[') {
        let end = authority
            .find(']')
            .ok_or(AxgaAiRuntimeConfigError::InvalidBaseUrl)?;
        return Ok(authority[1..end].to_string());
    }

    if let Some((host, port)) = authority.rsplit_once(':') {
        if !host.is_empty() && port.chars().all(|ch| ch.is_ascii_digit()) {
            return Ok(host.to_string());
        }
    }

    Ok(authority.to_string())
}

fn is_dashscope_coding_plan_host(host: &str) -> bool {
    host.eq_ignore_ascii_case(DASHSCOPE_CODING_PLAN_HOST)
}

impl AxgaChatBackend for OpenMeshDashScopeCodingPlanClient {
    fn begin_stream(&self, request: RequestBuilder) -> AxgaStreamFuture {
        let client = self.clone();
        async move { client.stream_chat(&request).await }.boxed()
    }
}

impl AxgaChatBackend for LiveAxgaChatBackend {
    fn begin_stream(&self, request: RequestBuilder) -> AxgaStreamFuture {
        match &self.inner {
            LiveProvider::OpenAi(provider) => {
                let provider = provider.clone();
                async move { provider.stream_chat(&request).await }.boxed()
            }
            LiveProvider::Anthropic(provider) => {
                let provider = provider.clone();
                async move { provider.stream_chat(&request).await }.boxed()
            }
            LiveProvider::DeepSeek(provider) => {
                let provider = provider.clone();
                async move { provider.stream_chat(&request).await }.boxed()
            }
            LiveProvider::DashScopeCodingPlan(client) => {
                let client = client.clone();
                async move { client.stream_chat(&request).await }.boxed()
            }
        }
    }
}

/// OpenMesh-owned synchronous adapter over approved `axga-ai` streaming providers.
///
/// Blocking policy: when no Tokio runtime is active on the calling thread, each
/// `generate_draft` call spawns one named OS thread with a private current-thread
/// Tokio runtime, runs the full async provider operation there, and joins before
/// returning. When a Tokio runtime is already active, the adapter returns
/// `RuntimeUnavailable` rather than nesting `block_on`.
pub struct AxgaAiProxyDraftRuntime {
    config: AxgaAiProxyDraftRuntimeConfig,
    backend: Arc<dyn AxgaChatBackend>,
    lifecycle: Arc<AdapterThreadLifecycleRecorder>,
}

impl AxgaAiProxyDraftRuntime {
    pub fn new(config: AxgaAiProxyDraftRuntimeConfig) -> Result<Self, AxgaAiRuntimeConfigError> {
        let backend = Arc::new(LiveAxgaChatBackend {
            inner: build_live_provider(&config)?,
        });
        Ok(Self {
            config,
            backend,
            lifecycle: Arc::new(AdapterThreadLifecycleRecorder::default()),
        })
    }

    pub fn with_chat_backend_for_tests<B: AxgaChatBackend + 'static>(
        config: AxgaAiProxyDraftRuntimeConfig,
        backend: Arc<B>,
    ) -> Result<Self, AxgaAiRuntimeConfigError> {
        Ok(Self {
            config,
            backend,
            lifecycle: Arc::new(AdapterThreadLifecycleRecorder::default()),
        })
    }

    pub fn thread_lifecycle(&self) -> AdapterThreadLifecycleSnapshot {
        self.lifecycle.snapshot()
    }

    pub fn config(&self) -> &AxgaAiProxyDraftRuntimeConfig {
        &self.config
    }

    pub fn chat_backend(&self) -> Arc<dyn AxgaChatBackend> {
        Arc::clone(&self.backend)
    }
}

impl ProxyDraftRuntime for AxgaAiProxyDraftRuntime {
    fn runtime_kind(&self) -> &'static str {
        match self.config.provider {
            AxgaAiProviderKind::OpenAi => PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI,
            AxgaAiProviderKind::Anthropic => PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC,
            AxgaAiProviderKind::DeepSeek => PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK,
        }
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        validate_proxy_runtime_request(request)
            .map_err(|_| ProxyDraftRuntimeError::InvalidRequest)?;

        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(ProxyDraftRuntimeError::RuntimeUnavailable);
        }

        let config = self.config.clone_for_invoke();
        let backend = Arc::clone(&self.backend);
        let request = request.clone();
        let started = Instant::now();

        run_on_adapter_thread(Arc::clone(&self.lifecycle), move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|_| ProxyDraftRuntimeError::RuntimeUnavailable)?;

            runtime.block_on(async move {
                execute_provider_operation(&config, backend.as_ref(), &request, started).await
            })
        })
    }
}

impl AxgaAiProxyDraftRuntimeConfig {
    fn clone_for_invoke(&self) -> Self {
        Self {
            provider: self.provider,
            model_id: self.model_id.clone(),
            api_key: self.api_key.clone(),
            openai_compatible_base_url: self.openai_compatible_base_url.clone(),
        }
    }
}

fn run_on_adapter_thread<T>(
    lifecycle: Arc<AdapterThreadLifecycleRecorder>,
    operation: impl FnOnce() -> Result<T, ProxyDraftRuntimeError> + Send + 'static,
) -> Result<T, ProxyDraftRuntimeError>
where
    T: Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::sync_channel(1);
    let lifecycle_for_thread = Arc::clone(&lifecycle);
    let handle: JoinHandle<()> = std::thread::Builder::new()
        .name(ADAPTER_THREAD_NAME.to_string())
        .spawn(move || {
            lifecycle_for_thread
                .thread_started
                .fetch_add(1, Ordering::SeqCst);
            lifecycle_for_thread
                .thread_alive
                .store(true, Ordering::SeqCst);
            struct ThreadAliveGuard(Arc<AdapterThreadLifecycleRecorder>);
            impl Drop for ThreadAliveGuard {
                fn drop(&mut self) {
                    self.0.thread_alive.store(false, Ordering::SeqCst);
                }
            }
            let _alive = ThreadAliveGuard(Arc::clone(&lifecycle_for_thread));
            let result = operation();
            lifecycle_for_thread
                .operation_completed
                .fetch_add(1, Ordering::SeqCst);
            let _ = tx.send(result);
        })
        .map_err(|_| ProxyDraftRuntimeError::RuntimeUnavailable)?;

    let result = match rx.recv() {
        Ok(result) => result,
        Err(_) => {
            let _ = handle.join();
            lifecycle.thread_joined.fetch_add(1, Ordering::SeqCst);
            return Err(ProxyDraftRuntimeError::RuntimeUnavailable);
        }
    };

    handle
        .join()
        .map_err(|_| ProxyDraftRuntimeError::RuntimeUnavailable)?;

    lifecycle.thread_joined.fetch_add(1, Ordering::SeqCst);
    result
}

/// Compute the whole-operation deadline used by the adapter.
pub fn effective_operation_timeout_ms(request_timeout_ms: u64) -> u64 {
    request_timeout_ms.min(AXGA_MAX_EFFECTIVE_TIMEOUT_MS)
}

/// Build the AXGA request for inspection and provider invocation.
pub fn build_axga_request_builder(
    bundle: &ProxyPromptBundle,
    model_id: &str,
    max_output_bytes: u32,
) -> RequestBuilder {
    let messages = vec![
        AgentMessage::User {
            content: bundle.context_json.clone(),
        },
        AgentMessage::User {
            content: bundle.user_message.clone(),
        },
    ];
    RequestBuilder::new(model_id, &messages)
        .with_system_prompt(&bundle.system_message)
        .with_max_tokens(max_output_bytes.max(1))
}

async fn execute_provider_operation(
    config: &AxgaAiProxyDraftRuntimeConfig,
    backend: &dyn AxgaChatBackend,
    request: &ProxyRuntimeRequest,
    started: Instant,
) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
    let effective_timeout_ms = effective_operation_timeout_ms(request.timeout_ms);
    let deadline = Duration::from_millis(effective_timeout_ms);

    let axga_request =
        build_axga_request_builder(&request.prompt, &config.model_id, request.max_output_bytes);

    let operation = async {
        let stream = backend
            .begin_stream(axga_request)
            .await
            .map_err(map_axga_error_to_runtime_error)?;
        let draft_text = aggregate_stream_events(stream, request.max_output_bytes).await?;
        let output = ProxyRuntimeOutput {
            draft_text,
            provider_id: provider_id_for_kind(config.provider).to_string(),
            model_id: config.model_id.clone(),
            network_used: true,
            duration_ms: started.elapsed().as_millis() as u64,
        };
        validate_proxy_runtime_output(&output)
            .map_err(|_| ProxyDraftRuntimeError::InvalidOutput)?;
        Ok(output)
    };

    match tokio::time::timeout(deadline, operation).await {
        Ok(result) => result,
        Err(_) => Err(ProxyDraftRuntimeError::Timeout),
    }
}

async fn aggregate_stream_events(
    mut stream: AxgaEventStream,
    max_output_bytes: u32,
) -> Result<String, ProxyDraftRuntimeError> {
    let mut output = String::new();
    while let Some(item) = stream.next().await {
        let event = item.map_err(map_axga_error_to_runtime_error)?;
        match event {
            StreamEvent::TextDelta { text } => {
                if would_exceed_byte_bound(&output, text.len(), max_output_bytes) {
                    return Err(ProxyDraftRuntimeError::InvalidOutput);
                }
                output.push_str(&text);
            }
            StreamEvent::ToolCallDelta { .. } | StreamEvent::ThinkingDelta { .. } => {
                return Err(ProxyDraftRuntimeError::InvalidOutput);
            }
            StreamEvent::Usage { .. } => {}
            StreamEvent::Done | StreamEvent::Stop { .. } => break,
            StreamEvent::Error { .. } => return Err(ProxyDraftRuntimeError::ProviderFailure),
        }
    }

    if output.trim().is_empty() {
        return Err(ProxyDraftRuntimeError::InvalidOutput);
    }
    Ok(output)
}

fn would_exceed_byte_bound(current: &str, incoming_len: usize, max_output_bytes: u32) -> bool {
    current.len().saturating_add(incoming_len) > max_output_bytes as usize
}

fn provider_id_for_kind(provider: AxgaAiProviderKind) -> &'static str {
    match provider {
        AxgaAiProviderKind::OpenAi => OPENAI_PROVIDER_ID,
        AxgaAiProviderKind::Anthropic => ANTHROPIC_PROVIDER_ID,
        AxgaAiProviderKind::DeepSeek => DEEPSEEK_PROVIDER_ID,
    }
}

fn build_live_provider(
    config: &AxgaAiProxyDraftRuntimeConfig,
) -> Result<LiveProvider, AxgaAiRuntimeConfigError> {
    match resolve_live_provider_route(config) {
        LiveProviderRoute::AxgaAnthropic => AnthropicProvider::new(Some(config.api_key.clone()))
            .map(LiveProvider::Anthropic)
            .map_err(|_| AxgaAiRuntimeConfigError::ProviderConstructionFailed),
        LiveProviderRoute::AxgaDeepSeek => DeepSeekProvider::new(
            Some(config.api_key.clone()),
            config.openai_compatible_base_url.clone(),
        )
        .map(LiveProvider::DeepSeek)
        .map_err(|_| AxgaAiRuntimeConfigError::ProviderConstructionFailed),
        LiveProviderRoute::AxgaOpenAi => OpenAiProvider::new(
            Some(config.api_key.clone()),
            config.openai_compatible_base_url.clone(),
        )
        .map(LiveProvider::OpenAi)
        .map_err(|_| AxgaAiRuntimeConfigError::ProviderConstructionFailed),
        LiveProviderRoute::OpenMeshDashScopeCodingPlan => {
            let base_url = config
                .openai_compatible_base_url
                .clone()
                .expect("dashscope route requires custom base url");
            OpenMeshDashScopeCodingPlanClient::new(config.api_key.clone(), base_url)
                .map(LiveProvider::DashScopeCodingPlan)
        }
    }
}

fn validate_openai_compatible_base_url(base_url: &str) -> Result<(), AxgaAiRuntimeConfigError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty()
        || !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        || trimmed.contains(char::is_whitespace)
    {
        return Err(AxgaAiRuntimeConfigError::InvalidBaseUrl);
    }
    Ok(())
}

fn map_axga_error_to_runtime_error(_err: AxgaError) -> ProxyDraftRuntimeError {
    match _err {
        AxgaError::RateLimited(_) | AxgaError::Http { .. } | AxgaError::LlmProvider(_) => {
            ProxyDraftRuntimeError::ProviderFailure
        }
        AxgaError::Network(_) | AxgaError::Io(_) => ProxyDraftRuntimeError::RuntimeUnavailable,
        AxgaError::Aborted | AxgaError::Unsupported(_) => {
            ProxyDraftRuntimeError::RuntimeUnavailable
        }
        AxgaError::Config(_) | AxgaError::Serialization(_) => {
            ProxyDraftRuntimeError::ProviderFailure
        }
        AxgaError::TokenLimitExceeded { .. }
        | AxgaError::ToolError { .. }
        | AxgaError::FileTooLarge { .. }
        | AxgaError::FileNotFound(_)
        | AxgaError::AccessDenied(_) => ProxyDraftRuntimeError::ProviderFailure,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn effective_timeout_is_capped_below_client_limit() {
        assert_eq!(effective_operation_timeout_ms(30_000), 30_000);
        assert_eq!(
            effective_operation_timeout_ms(AXGA_CLIENT_TIMEOUT_MS),
            AXGA_MAX_EFFECTIVE_TIMEOUT_MS
        );
        assert_eq!(
            effective_operation_timeout_ms(AXGA_CLIENT_TIMEOUT_MS + 1),
            AXGA_MAX_EFFECTIVE_TIMEOUT_MS
        );
    }

    #[test]
    fn would_exceed_detects_incremental_bound() {
        assert!(!would_exceed_byte_bound("abc", 1, 4));
        assert!(would_exceed_byte_bound("abc", 2, 4));
    }

    #[test]
    fn dashscope_compat_injects_enable_thinking_only_for_dashscope_transport() {
        let request = build_axga_request_builder(
            &ProxyPromptBundle {
                protocol_version: "1.0".into(),
                system_message: "sys".into(),
                context_json: "{}".into(),
                user_message: "hi".into(),
            },
            "qwen3.7-plus",
            256,
        );
        let with_compat = prepare_dashscope_coding_plan_body(&request);
        assert_eq!(with_compat["enable_thinking"], Value::Bool(false));
    }

    #[test]
    fn dashscope_host_match_is_exact_ascii_case_insensitive() {
        assert!(is_dashscope_coding_plan_host(DASHSCOPE_CODING_PLAN_HOST));
        assert!(is_dashscope_coding_plan_host(
            "CODING-INTL.DASHSCOPE.ALIYUNCS.COM"
        ));
        assert!(!is_dashscope_coding_plan_host(
            "evil-coding-intl.dashscope.aliyuncs.com"
        ));
        assert!(!is_dashscope_coding_plan_host(
            "coding-intl.dashscope.aliyuncs.com.evil.com"
        ));
        assert!(!is_dashscope_coding_plan_host("dashscope.aliyuncs.com"));
        assert!(!is_dashscope_coding_plan_host("api.openai.com"));
    }

    #[test]
    fn resolve_live_provider_route_matches_required_matrix() {
        let anthropic = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::Anthropic,
            "claude",
            "key",
            None,
        )
        .expect("anthropic");
        assert_eq!(
            resolve_live_provider_route(&anthropic),
            LiveProviderRoute::AxgaAnthropic
        );

        let deepseek = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::DeepSeek,
            "deepseek-chat",
            "key",
            Some("https://api.deepseek.com/v1".into()),
        )
        .expect("deepseek");
        assert_eq!(
            resolve_live_provider_route(&deepseek),
            LiveProviderRoute::AxgaDeepSeek
        );

        let openai_default = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::OpenAi,
            "gpt-4o-mini",
            "key",
            None,
        )
        .expect("openai default");
        assert_eq!(
            resolve_live_provider_route(&openai_default),
            LiveProviderRoute::AxgaOpenAi
        );

        let openai_custom = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::OpenAi,
            "gpt-4o-mini",
            "key",
            Some("https://api.example.com/v1".into()),
        )
        .expect("openai custom");
        assert_eq!(
            resolve_live_provider_route(&openai_custom),
            LiveProviderRoute::AxgaOpenAi
        );

        let dashscope = AxgaAiProxyDraftRuntimeConfig::new(
            AxgaAiProviderKind::OpenAi,
            "qwen3.7-plus",
            "key",
            Some(format!("https://{DASHSCOPE_CODING_PLAN_HOST}/v1")),
        )
        .expect("dashscope");
        assert_eq!(
            resolve_live_provider_route(&dashscope),
            LiveProviderRoute::OpenMeshDashScopeCodingPlan
        );
    }
}
