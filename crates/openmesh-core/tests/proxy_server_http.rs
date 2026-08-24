use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use openmesh_core::oauth::OAuthProviderId;
use openmesh_core::proxy_server::{
    ProxyModelConfig, ProxyOAuthCredential, ProxyOAuthCredentialResolver, ProxyProviderProtocol,
    ProxyRoutingStrategy, ProxyServer, ProxyServerConfig, ProxyUpstreamConfig,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

#[derive(Clone)]
struct StaticOAuthResolver;

#[async_trait::async_trait]
impl ProxyOAuthCredentialResolver for StaticOAuthResolver {
    async fn resolve(
        &self,
        _provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<ProxyOAuthCredential>, String> {
        assert_eq!(account_id, "account-1");
        Ok(Some(ProxyOAuthCredential {
            access_token: "oauth-access".to_string(),
            token_type: "Bearer".to_string(),
            metadata: Default::default(),
        }))
    }
}

#[derive(Clone)]
struct KimiOAuthResolver;

#[async_trait::async_trait]
impl ProxyOAuthCredentialResolver for KimiOAuthResolver {
    async fn resolve(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<ProxyOAuthCredential>, String> {
        assert_eq!(provider, OAuthProviderId::Kimi);
        assert_eq!(account_id, "account-1");
        Ok(Some(ProxyOAuthCredential {
            access_token: "kimi-oauth-access".to_string(),
            token_type: "Bearer".to_string(),
            metadata: BTreeMap::from([("device_id".to_string(), "kimi-device-1".to_string())]),
        }))
    }
}

#[derive(Clone, Default)]
struct OAuthUpstreamState {
    authorization: Arc<Mutex<Option<String>>>,
    anthropic_beta: Arc<Mutex<Option<String>>>,
    browser_access: Arc<Mutex<Option<String>>>,
    x_app: Arc<Mutex<Option<String>>>,
    user_agent: Arc<Mutex<Option<String>>>,
    kimi_device_id: Arc<Mutex<Option<String>>>,
    kimi_platform: Arc<Mutex<Option<String>>>,
    kimi_version: Arc<Mutex<Option<String>>>,
}

async fn oauth_upstream_chat(
    State(state): State<OAuthUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.authorization.lock().expect("OAuth auth lock") = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    (
        StatusCode::OK,
        Json(json!({
            "id": "oauth-chat",
            "object": "chat.completion",
            "model": payload["model"],
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "oauth reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })),
    )
}

async fn oauth_codex_responses(
    State(state): State<OAuthUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.authorization.lock().expect("Codex auth lock") = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    assert_eq!(payload["stream"], true);
    let body = concat!(
        "data: {\"type\":\"response.created\",\"response\":{\"id\":\"resp-codex\",\"model\":\"codex-model\",\"created_at\":1}}\n\n",
        "data: {\"type\":\"response.output_text.delta\",\"delta\":\"hello from codex\"}\n\n",
        "data: {\"type\":\"response.completed\",\"response\":{\"id\":\"resp-codex\",\"model\":\"codex-model\",\"created_at\":1,\"status\":\"completed\",\"output\":[{\"type\":\"message\",\"content\":[{\"type\":\"output_text\",\"text\":\"hello from codex\"}]}],\"usage\":{\"input_tokens\":3,\"output_tokens\":4,\"total_tokens\":7}}}\n\n",
        "data: [DONE]\n\n"
    );
    (
        StatusCode::OK,
        [("content-type", "text/event-stream")],
        body,
    )
}

async fn oauth_anthropic_messages(
    State(state): State<OAuthUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.authorization.lock().expect("Claude auth lock") = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.anthropic_beta.lock().expect("Claude beta lock") = headers
        .get("anthropic-beta")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state
        .browser_access
        .lock()
        .expect("Claude browser header lock") = headers
        .get("anthropic-dangerous-direct-browser-access")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.x_app.lock().expect("Claude app header lock") = headers
        .get("x-app")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.user_agent.lock().expect("Claude user-agent lock") = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    (
        StatusCode::OK,
        Json(json!({
            "id": "oauth-claude-message",
            "type": "message",
            "role": "assistant",
            "model": payload["model"],
            "content": [{"type": "text", "text": "oauth claude reply"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 2, "output_tokens": 3}
        })),
    )
}

async fn oauth_kimi_chat(
    State(state): State<OAuthUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.authorization.lock().expect("Kimi auth lock") = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.kimi_device_id.lock().expect("Kimi device lock") = headers
        .get("x-msh-device-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.kimi_platform.lock().expect("Kimi platform lock") = headers
        .get("x-msh-platform")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    *state.kimi_version.lock().expect("Kimi version lock") = headers
        .get("x-msh-version")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    (
        StatusCode::OK,
        Json(json!({
            "id": "kimi-chat",
            "object": "chat.completion",
            "model": payload["model"],
            "choices": [{"index": 0, "message": {"role": "assistant", "content": "kimi reply"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })),
    )
}

#[tokio::test]
async fn built_in_proxy_sends_kimi_oauth_device_headers() {
    let upstream_state = OAuthUpstreamState::default();
    let upstream_router = Router::new()
        .route("/v1/chat/completions", post(oauth_kimi_chat))
        .with_state(upstream_state.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Kimi upstream listener");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("Kimi upstream address");
    let (upstream_stop_tx, upstream_stop_rx) = oneshot::channel::<()>();
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_router)
            .with_graceful_shutdown(async {
                let _ = upstream_stop_rx.await;
            })
            .await
            .expect("Kimi upstream server");
    });

    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: 1,
        upstreams: vec![ProxyUpstreamConfig {
            id: "kimi-oauth".to_string(),
            base_url: format!("http://{upstream_address}/v1"),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: None,
            enabled: true,
            priority: 0,
            account_id: Some("account-1".to_string()),
            oauth_provider: Some("kimi".to_string()),
            models: vec![ProxyModelConfig {
                id: "kimi-model".to_string(),
                owned_by: "kimi".to_string(),
                capabilities: vec!["chat".to_string()],
            }],
        }],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    let proxy = ProxyServer::new(config).expect("Kimi OAuth proxy config");
    proxy.set_oauth_credential_resolver(Arc::new(KimiOAuthResolver));
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Kimi proxy listener");
    let proxy_address = proxy_listener.local_addr().expect("Kimi proxy address");
    let (proxy_stop_tx, proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        proxy
            .serve(proxy_listener, async {
                let _ = proxy_stop_rx.await;
            })
            .await
            .expect("Kimi proxy server");
    });

    let response = reqwest::Client::new()
        .post(format!("http://{proxy_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "kimi-model",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("Kimi proxy response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        upstream_state.authorization.lock().unwrap().as_deref(),
        Some("Bearer kimi-oauth-access")
    );
    assert_eq!(
        upstream_state.kimi_device_id.lock().unwrap().as_deref(),
        Some("kimi-device-1")
    );
    assert_eq!(
        upstream_state.kimi_platform.lock().unwrap().as_deref(),
        Some("OpenMesh")
    );
    assert_eq!(
        upstream_state.kimi_version.lock().unwrap().as_deref(),
        Some(env!("CARGO_PKG_VERSION"))
    );

    let _ = proxy_stop_tx.send(());
    let _ = upstream_stop_tx.send(());
    proxy_task.await.expect("Kimi proxy join");
    upstream_task.await.expect("Kimi upstream join");
}

#[tokio::test]
async fn built_in_proxy_sends_claude_oauth_provider_headers() {
    let upstream_state = OAuthUpstreamState::default();
    let upstream_router = Router::new()
        .route("/v1/messages", post(oauth_anthropic_messages))
        .with_state(upstream_state.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Claude upstream listener");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("Claude upstream address");
    let (upstream_stop_tx, upstream_stop_rx) = oneshot::channel::<()>();
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_router)
            .with_graceful_shutdown(async {
                let _ = upstream_stop_rx.await;
            })
            .await
            .expect("Claude upstream server");
    });

    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: 1,
        upstreams: vec![ProxyUpstreamConfig {
            id: "claude-oauth".to_string(),
            base_url: format!("http://{upstream_address}/v1"),
            protocol: ProxyProviderProtocol::Anthropic,
            api_key: None,
            enabled: true,
            priority: 0,
            account_id: Some("account-1".to_string()),
            oauth_provider: Some("claude".to_string()),
            models: vec![ProxyModelConfig {
                id: "claude-model".to_string(),
                owned_by: "claude".to_string(),
                capabilities: vec!["messages".to_string()],
            }],
        }],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    let proxy = ProxyServer::new(config).expect("Claude OAuth proxy config");
    proxy.set_oauth_credential_resolver(Arc::new(StaticOAuthResolver));
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Claude proxy listener");
    let proxy_address = proxy_listener.local_addr().expect("Claude proxy address");
    let (proxy_stop_tx, proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        proxy
            .serve(proxy_listener, async {
                let _ = proxy_stop_rx.await;
            })
            .await
            .expect("Claude proxy server");
    });

    let response = reqwest::Client::new()
        .post(format!("http://{proxy_address}/v1/messages"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "claude-model",
            "max_tokens": 32,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("Claude proxy response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        upstream_state.authorization.lock().unwrap().as_deref(),
        Some("Bearer oauth-access")
    );
    assert_eq!(
        upstream_state.anthropic_beta.lock().unwrap().as_deref(),
        Some("oauth-2025-04-20")
    );
    assert_eq!(
        upstream_state.browser_access.lock().unwrap().as_deref(),
        Some("true")
    );
    assert_eq!(upstream_state.x_app.lock().unwrap().as_deref(), Some("cli"));
    assert!(upstream_state
        .user_agent
        .lock()
        .unwrap()
        .as_deref()
        .is_some_and(|value| value.starts_with("claude-cli/")));

    let accounts: Value = reqwest::Client::new()
        .get(format!("http://{proxy_address}/v0/management/accounts"))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("Claude accounts response")
        .json()
        .await
        .expect("Claude accounts JSON");
    assert_eq!(accounts["data"][0]["credentialStatus"], "ready");
    assert_eq!(accounts["data"][0]["oauthProvider"], "claude");

    let _ = proxy_stop_tx.send(());
    let _ = upstream_stop_tx.send(());
    proxy_task.await.expect("Claude proxy join");
    upstream_task.await.expect("Claude upstream join");
}

#[tokio::test]
async fn built_in_proxy_resolves_oauth_credentials_without_serializing_tokens() {
    let upstream_state = OAuthUpstreamState::default();
    let upstream_router = Router::new()
        .route("/v1/chat/completions", post(oauth_upstream_chat))
        .with_state(upstream_state.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("upstream listener");
    let upstream_address = upstream_listener.local_addr().expect("upstream address");
    let (upstream_stop_tx, upstream_stop_rx) = oneshot::channel::<()>();
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_router)
            .with_graceful_shutdown(async {
                let _ = upstream_stop_rx.await;
            })
            .await
            .expect("OAuth upstream server");
    });

    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: 1,
        upstreams: vec![ProxyUpstreamConfig {
            id: "codex-account".to_string(),
            base_url: format!("http://{upstream_address}/v1"),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: None,
            enabled: true,
            priority: 0,
            account_id: Some("account-1".to_string()),
            oauth_provider: Some("claude".to_string()),
            models: vec![ProxyModelConfig {
                id: "codex-model".to_string(),
                owned_by: "codex".to_string(),
                capabilities: vec!["chat".to_string()],
            }],
        }],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    let proxy = ProxyServer::new(config).expect("OAuth proxy config");
    proxy.set_oauth_credential_resolver(Arc::new(StaticOAuthResolver));
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("proxy listener");
    let proxy_address = proxy_listener.local_addr().expect("proxy address");
    let (proxy_stop_tx, proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        proxy
            .serve(proxy_listener, async {
                let _ = proxy_stop_rx.await;
            })
            .await
            .expect("OAuth proxy server");
    });

    let response = reqwest::Client::new()
        .post(format!("http://{proxy_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({"model": "codex-model", "messages": [{"role": "user", "content": "hello"}]}))
        .send()
        .await
        .expect("OAuth proxy response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        upstream_state.authorization.lock().unwrap().as_deref(),
        Some("Bearer oauth-access")
    );

    let _ = proxy_stop_tx.send(());
    let _ = upstream_stop_tx.send(());
    proxy_task.await.expect("OAuth proxy join");
    upstream_task.await.expect("OAuth upstream join");
}

#[tokio::test]
async fn built_in_proxy_translates_codex_responses_to_chat_completions() {
    let upstream_state = OAuthUpstreamState::default();
    let upstream_router = Router::new()
        .route("/v1/responses", post(oauth_codex_responses))
        .with_state(upstream_state.clone());
    let upstream_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Codex upstream listener");
    let upstream_address = upstream_listener
        .local_addr()
        .expect("Codex upstream address");
    let (upstream_stop_tx, upstream_stop_rx) = oneshot::channel::<()>();
    let upstream_task = tokio::spawn(async move {
        axum::serve(upstream_listener, upstream_router)
            .with_graceful_shutdown(async {
                let _ = upstream_stop_rx.await;
            })
            .await
            .expect("Codex upstream server");
    });
    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: 1,
        upstreams: vec![ProxyUpstreamConfig {
            id: "codex-native".to_string(),
            base_url: format!("http://{upstream_address}/v1"),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: None,
            enabled: true,
            priority: 0,
            account_id: Some("account-1".to_string()),
            oauth_provider: Some("codex".to_string()),
            models: vec![ProxyModelConfig {
                id: "codex-model".to_string(),
                owned_by: "codex".to_string(),
                capabilities: vec!["chat".to_string(), "responses".to_string()],
            }],
        }],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    let proxy = ProxyServer::new(config).expect("Codex proxy config");
    proxy.set_oauth_credential_resolver(Arc::new(StaticOAuthResolver));
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Codex proxy listener");
    let proxy_address = proxy_listener.local_addr().expect("Codex proxy address");
    let (proxy_stop_tx, proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        proxy
            .serve(proxy_listener, async {
                let _ = proxy_stop_rx.await;
            })
            .await
            .expect("Codex proxy server");
    });
    let response = reqwest::Client::new()
        .post(format!("http://{proxy_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({"model":"codex-model","messages":[{"role":"user","content":"hello"}]}))
        .send()
        .await
        .expect("Codex proxy response");
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("Codex response JSON");
    assert_eq!(body["choices"][0]["message"]["content"], "hello from codex");
    assert_eq!(body["usage"]["total_tokens"], 7);
    assert_eq!(
        upstream_state.authorization.lock().unwrap().as_deref(),
        Some("Bearer oauth-access")
    );
    let _ = proxy_stop_tx.send(());
    let _ = upstream_stop_tx.send(());
    proxy_task.await.expect("Codex proxy join");
    upstream_task.await.expect("Codex upstream join");
}

#[derive(Clone, Default)]
struct MockUpstreamState {
    last_request: Arc<Mutex<Option<Value>>>,
    saw_expected_auth: Arc<Mutex<bool>>,
}

#[tokio::test]
async fn built_in_proxy_handles_auth_alias_responses_and_sse_without_sidecar() {
    let mock_state = MockUpstreamState::default();
    let mock_router = Router::new()
        .route("/v1/chat/completions", post(mock_chat_completions))
        .route("/v1/messages", post(mock_anthropic_messages))
        .route(
            "/v1beta/models/gemini-model:generateContent",
            post(mock_gemini_generate_content),
        )
        .with_state(mock_state.clone());
    let mock_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("mock listener");
    let mock_address = mock_listener.local_addr().expect("mock address");
    let (mock_stop_tx, mock_stop_rx) = oneshot::channel::<()>();
    let mock_task = tokio::spawn(async move {
        axum::serve(mock_listener, mock_router)
            .with_graceful_shutdown(async {
                let _ = mock_stop_rx.await;
            })
            .await
            .expect("mock server");
    });

    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: openmesh_core::proxy_server::DEFAULT_MAX_RETRIES,
        upstreams: vec![ProxyUpstreamConfig {
            id: "mock".to_string(),
            base_url: format!("http://{mock_address}/v1"),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: Some("provider-key".to_string()),
            enabled: true,
            priority: 0,
            account_id: None,
            oauth_provider: None,
            models: vec![ProxyModelConfig {
                id: "provider-model".to_string(),
                owned_by: "mock".to_string(),
                capabilities: vec!["chat".to_string(), "responses".to_string()],
            }],
        }],
        model_aliases: [("friendly-model".to_string(), "provider-model".to_string())]
            .into_iter()
            .collect(),
        model_fallbacks: Default::default(),
    };
    let proxy = ProxyServer::new(config).expect("proxy config");
    let proxy_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("proxy listener");
    let proxy_address = proxy_listener.local_addr().expect("proxy address");
    let (proxy_stop_tx, proxy_stop_rx) = oneshot::channel::<()>();
    let proxy_task = tokio::spawn(async move {
        proxy
            .serve(proxy_listener, async {
                let _ = proxy_stop_rx.await;
            })
            .await
            .expect("proxy server");
    });

    let client = reqwest::Client::new();
    let models_url = format!("http://{proxy_address}/v1/models");
    let unauthorized = client
        .get(&models_url)
        .send()
        .await
        .expect("unauthorized response");
    assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);
    assert!(unauthorized.headers().get("x-request-id").is_some());

    let models = client
        .get(&models_url)
        .bearer_auth("client-key")
        .send()
        .await
        .expect("models response");
    assert_eq!(models.status(), StatusCode::OK);
    let models_json: Value = models.json().await.expect("models JSON");
    let model_ids = models_json["data"]
        .as_array()
        .expect("model list")
        .iter()
        .filter_map(|model| model["id"].as_str())
        .collect::<Vec<_>>();
    assert!(model_ids.contains(&"friendly-model"));
    assert!(model_ids.contains(&"provider-model"));

    let management_url = format!("http://{proxy_address}/v0/management/config");
    let management_unauthorized = client
        .get(&management_url)
        .send()
        .await
        .expect("management unauthorized response");
    assert_eq!(management_unauthorized.status(), StatusCode::UNAUTHORIZED);
    let management = client
        .get(&management_url)
        .bearer_auth("client-key")
        .send()
        .await
        .expect("management config response");
    assert_eq!(management.status(), StatusCode::OK);
    let management_body = management.text().await.expect("management body");
    assert!(management_body.contains("provider-model"));
    assert!(management_body.contains("apiKeyConfigured"));
    assert!(!management_body.contains("provider-key"));

    let management_update = client
        .put(&management_url)
        .bearer_auth("client-key")
        .json(&json!({
            "modelAliases": {
                "friendly-model": "provider-model",
                "second-friendly-model": "provider-model"
            }
        }))
        .send()
        .await
        .expect("management update response");
    assert_eq!(management_update.status(), StatusCode::OK);
    let management_update_body = management_update
        .text()
        .await
        .expect("management update body");
    assert!(management_update_body.contains("second-friendly-model"));
    assert!(!management_update_body.contains("provider-key"));

    let providers_url = format!("http://{proxy_address}/v0/management/providers");
    let providers: Value = client
        .get(&providers_url)
        .bearer_auth("client-key")
        .send()
        .await
        .expect("providers response")
        .json()
        .await
        .expect("providers JSON");
    assert_eq!(providers["providers"][0]["id"], "mock");
    assert!(!providers.to_string().contains("provider-key"));

    let providers_update = client
        .put(&providers_url)
        .bearer_auth("client-key")
        .json(&json!({ "routingStrategy": "fill-first" }))
        .send()
        .await
        .expect("providers update response");
    assert_eq!(providers_update.status(), StatusCode::OK);
    let providers_update_json: Value = providers_update
        .json()
        .await
        .expect("providers update JSON");
    assert_eq!(providers_update_json["routingStrategy"], "fill-first");

    let created_provider = client
        .post(&providers_url)
        .bearer_auth("client-key")
        .json(&json!({
            "id": "created",
            "baseUrl": format!("http://{mock_address}/v1"),
            "protocol": "open-ai-compatible",
            "apiKey": "created-provider-key",
            "enabled": true,
            "priority": 1,
            "accountId": "account-2",
            "models": [{"id": "created-model", "ownedBy": "mock", "capabilities": ["chat"]}]
        }))
        .send()
        .await
        .expect("provider create response");
    assert_eq!(created_provider.status(), StatusCode::CREATED);
    let created_provider_json: Value = created_provider.json().await.expect("provider create JSON");
    assert_eq!(created_provider_json["providers"][1]["id"], "created");
    assert_eq!(created_provider_json["providers"][1]["keyConfigured"], true);
    assert!(!created_provider_json
        .to_string()
        .contains("created-provider-key"));

    let activated = client
        .post(format!(
            "http://{proxy_address}/v0/management/accounts/active"
        ))
        .bearer_auth("client-key")
        .json(&json!({"accountId": "account-2"}))
        .send()
        .await
        .expect("account activation response");
    assert_eq!(activated.status(), StatusCode::OK);
    let activated_json: Value = activated.json().await.expect("account activation JSON");
    assert_eq!(activated_json["accountId"], "account-2");
    assert_eq!(activated_json["status"], "active");

    let deleted_provider = client
        .delete(format!(
            "http://{proxy_address}/v0/management/providers/created"
        ))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("provider delete response");
    assert_eq!(deleted_provider.status(), StatusCode::OK);
    let deleted_provider_json: Value = deleted_provider.json().await.expect("provider delete JSON");
    assert!(deleted_provider_json["providers"]
        .as_array()
        .is_some_and(|providers| providers.iter().all(|provider| provider["id"] != "created")));

    let accounts: Value = client
        .get(format!("http://{proxy_address}/v0/management/accounts"))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("accounts response")
        .json()
        .await
        .expect("accounts JSON");
    assert_eq!(accounts["data"][0]["id"], "mock");

    let quotas: Value = client
        .get(format!("http://{proxy_address}/v0/management/quotas"))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("quotas response")
        .json()
        .await
        .expect("quotas JSON");
    assert_eq!(quotas["supported"], false);

    let provider_models: Value = client
        .get(format!(
            "http://{proxy_address}/v0/management/models/openai"
        ))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("provider models response")
        .json()
        .await
        .expect("provider models JSON");
    assert_eq!(provider_models["data"][0]["id"], "provider-model");

    let updated_models = client
        .get(&models_url)
        .bearer_auth("client-key")
        .send()
        .await
        .expect("updated models response");
    let updated_models_json: Value = updated_models.json().await.expect("updated models JSON");
    let updated_model_ids = updated_models_json["data"]
        .as_array()
        .expect("updated model list")
        .iter()
        .filter_map(|model| model["id"].as_str())
        .collect::<Vec<_>>();
    assert!(updated_model_ids.contains(&"second-friendly-model"));

    let restart_required = client
        .put(&management_url)
        .bearer_auth("client-key")
        .json(&json!({ "port": 9001 }))
        .send()
        .await
        .expect("restart-required response");
    assert_eq!(restart_required.status(), StatusCode::CONFLICT);
    let restart_required_json: Value = restart_required
        .json()
        .await
        .expect("restart-required JSON");
    assert_eq!(restart_required_json["error"]["code"], "restart_required");

    let usage = client
        .get(format!("http://{proxy_address}/v0/management/usage"))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("usage response");
    assert_eq!(usage.status(), StatusCode::OK);
    let usage_json: Value = usage.json().await.expect("usage JSON");
    assert!(usage_json["totalRequests"].as_u64().unwrap_or(0) >= 3);
    assert!(usage_json["logs"]
        .as_array()
        .is_some_and(|logs| !logs.is_empty()));

    let chat = client
        .post(format!("http://{proxy_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "friendly-model",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("chat response");
    assert_eq!(chat.status(), StatusCode::OK);
    let chat_input_tokens = chat
        .headers()
        .get("x-openmesh-input-tokens")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let chat_json: Value = chat.json().await.expect("chat JSON");
    assert_eq!(chat_json["choices"][0]["message"]["content"], "mock reply");
    assert_eq!(
        mock_state
            .last_request
            .lock()
            .expect("mock request lock")
            .as_ref()
            .expect("mock request")["model"],
        "provider-model"
    );
    assert!(*mock_state.saw_expected_auth.lock().expect("mock auth lock"));
    assert_eq!(chat_input_tokens.as_deref(), Some("2"));

    let usage_after_chat: Value = client
        .get(format!("http://{proxy_address}/v0/management/usage"))
        .bearer_auth("client-key")
        .send()
        .await
        .expect("usage after chat response")
        .json()
        .await
        .expect("usage after chat JSON");
    assert!(usage_after_chat["logs"].as_array().is_some_and(|logs| {
        logs.iter().any(|log| {
            log["path"] == "/v1/chat/completions"
                && log["model"] == "provider-model"
                && log["totalTokens"] == 4
        })
    }));

    let response = client
        .post(format!("http://{proxy_address}/v1/responses"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "friendly-model",
            "input": "stream this",
            "stream": true
        }))
        .send()
        .await
        .expect("responses response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let response_body = response.text().await.expect("responses body");
    assert!(response_body.contains("response.created"));
    assert!(response_body.contains("response.output_text.delta"));
    assert!(response_body.contains("response.completed"));
    assert!(response_body.contains("data: [DONE]"));

    let config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: Default::default(),
        max_retries: openmesh_core::proxy_server::DEFAULT_MAX_RETRIES,
        upstreams: vec![
            ProxyUpstreamConfig {
                id: "openai".to_string(),
                base_url: format!("http://{mock_address}/v1"),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 0,
                account_id: None,
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "provider-model".to_string(),
                    owned_by: "openai".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
            ProxyUpstreamConfig {
                id: "anthropic".to_string(),
                base_url: format!("http://{mock_address}/v1"),
                protocol: ProxyProviderProtocol::Anthropic,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 1,
                account_id: Some("claude-account".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "claude-model".to_string(),
                    owned_by: "anthropic".to_string(),
                    capabilities: vec!["chat".to_string(), "messages".to_string()],
                }],
            },
            ProxyUpstreamConfig {
                id: "gemini".to_string(),
                base_url: format!("http://{mock_address}/v1beta"),
                protocol: ProxyProviderProtocol::Gemini,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 2,
                account_id: Some("gemini-account".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "gemini-model".to_string(),
                    owned_by: "google".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
        ],
        model_aliases: Default::default(),
        model_fallbacks: Default::default(),
    };
    let second_proxy = ProxyServer::new(config.clone()).expect("provider adapter config");
    let second_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("second proxy listener");
    let second_address = second_listener.local_addr().expect("second proxy address");
    let (second_stop_tx, second_stop_rx) = oneshot::channel::<()>();
    let second_task = tokio::spawn(async move {
        second_proxy
            .serve(second_listener, async {
                let _ = second_stop_rx.await;
            })
            .await
            .expect("second proxy server");
    });

    let claude = client
        .post(format!("http://{second_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "claude-model",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("anthropic translated response");
    assert_eq!(claude.status(), StatusCode::OK);
    let claude_json: Value = claude.json().await.expect("anthropic translated JSON");
    assert_eq!(
        claude_json["choices"][0]["message"]["content"],
        "anthropic reply"
    );

    let direct_messages = client
        .post(format!("http://{second_address}/v1/messages"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "claude-model",
            "max_tokens": 32,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("native messages response");
    assert_eq!(direct_messages.status(), StatusCode::OK);
    let direct_messages_json: Value = direct_messages.json().await.expect("native messages JSON");
    assert_eq!(direct_messages_json["type"], "message");
    assert_eq!(
        direct_messages_json["content"][0]["text"],
        "anthropic reply"
    );

    let messages = client
        .post(format!("http://{second_address}/v1/messages"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "provider-model",
            "max_tokens": 32,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("messages translated response");
    assert_eq!(messages.status(), StatusCode::OK);
    let messages_json: Value = messages.json().await.expect("messages JSON");
    assert_eq!(messages_json["type"], "message");
    assert_eq!(messages_json["content"][0]["text"], "mock reply");

    let gemini = client
        .post(format!("http://{second_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "gemini-model",
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await
        .expect("gemini translated response");
    assert_eq!(gemini.status(), StatusCode::OK);
    let gemini_json: Value = gemini.json().await.expect("gemini translated JSON");
    assert_eq!(
        gemini_json["choices"][0]["message"]["content"],
        "gemini reply"
    );

    let routing_config = ProxyServerConfig {
        bind_host: "127.0.0.1".to_string(),
        port: 8317,
        api_keys: vec!["client-key".to_string()],
        allow_unauthenticated: false,
        request_timeout_secs: 10,
        routing_strategy: ProxyRoutingStrategy::RoundRobin,
        max_retries: 2,
        upstreams: vec![
            ProxyUpstreamConfig {
                id: "primary-failing-account".to_string(),
                base_url: "http://127.0.0.1:1/v1".to_string(),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: None,
                enabled: true,
                priority: 0,
                account_id: Some("primary-account".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "primary-model".to_string(),
                    owned_by: "mock".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
            ProxyUpstreamConfig {
                id: "round-robin-a".to_string(),
                base_url: format!("http://{mock_address}/v1"),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 0,
                account_id: Some("account-a".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "round-robin-model".to_string(),
                    owned_by: "mock".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
            ProxyUpstreamConfig {
                id: "round-robin-b".to_string(),
                base_url: format!("http://{mock_address}/v1"),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 1,
                account_id: Some("account-b".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "round-robin-model".to_string(),
                    owned_by: "mock".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
            ProxyUpstreamConfig {
                id: "fallback-account".to_string(),
                base_url: format!("http://{mock_address}/v1"),
                protocol: ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 0,
                account_id: Some("fallback-account".to_string()),
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "fallback-model".to_string(),
                    owned_by: "mock".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            },
        ],
        model_aliases: Default::default(),
        model_fallbacks: [(
            "primary-model".to_string(),
            vec!["fallback-model".to_string()],
        )]
        .into_iter()
        .collect(),
    };
    let routing_proxy = ProxyServer::new(routing_config).expect("routing config");
    let routing_listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("routing listener");
    let routing_address = routing_listener.local_addr().expect("routing address");
    let (routing_stop_tx, routing_stop_rx) = oneshot::channel::<()>();
    let routing_task = tokio::spawn(async move {
        routing_proxy
            .serve(routing_listener, async {
                let _ = routing_stop_rx.await;
            })
            .await
            .expect("routing server");
    });

    let fallback = client
        .post(format!("http://{routing_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "primary-model",
            "messages": [{"role": "user", "content": "fallback"}]
        }))
        .send()
        .await
        .expect("fallback response");
    assert_eq!(fallback.status(), StatusCode::OK);
    assert_eq!(
        fallback
            .headers()
            .get("x-openmesh-upstream")
            .and_then(|value| value.to_str().ok()),
        Some("fallback-account")
    );

    let round_robin_a = client
        .post(format!("http://{routing_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "round-robin-model",
            "messages": [{"role": "user", "content": "round robin"}]
        }))
        .send()
        .await
        .expect("first round-robin response");
    let round_robin_b = client
        .post(format!("http://{routing_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .json(&json!({
            "model": "round-robin-model",
            "messages": [{"role": "user", "content": "round robin"}]
        }))
        .send()
        .await
        .expect("second round-robin response");
    assert_eq!(round_robin_a.status(), StatusCode::OK);
    assert_eq!(round_robin_b.status(), StatusCode::OK);
    assert_ne!(
        round_robin_a.headers().get("x-openmesh-upstream"),
        round_robin_b.headers().get("x-openmesh-upstream")
    );

    let malformed = client
        .post(format!("http://{proxy_address}/v1/chat/completions"))
        .bearer_auth("client-key")
        .header("content-type", "application/json")
        .body("not-json")
        .send()
        .await
        .expect("malformed response");
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    let malformed_json: Value = malformed.json().await.expect("malformed JSON error");
    assert_eq!(malformed_json["error"]["code"], "invalid_request");

    let _ = proxy_stop_tx.send(());
    let _ = second_stop_tx.send(());
    let _ = routing_stop_tx.send(());
    let _ = mock_stop_tx.send(());
    proxy_task.await.expect("proxy join");
    second_task.await.expect("second proxy join");
    routing_task.await.expect("routing join");
    mock_task.await.expect("mock join");
}

async fn mock_anthropic_messages(
    State(state): State<MockUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.last_request.lock().expect("mock request lock") = Some(payload);
    *state.saw_expected_auth.lock().expect("mock auth lock") = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok())
        == Some("provider-key");
    (
        StatusCode::OK,
        Json(json!({
            "id": "msg-mock",
            "type": "message",
            "role": "assistant",
            "model": "claude-model",
            "content": [{"type": "text", "text": "anthropic reply"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 2, "output_tokens": 2}
        })),
    )
}

async fn mock_gemini_generate_content(
    State(state): State<MockUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.last_request.lock().expect("mock request lock") = Some(payload);
    *state.saw_expected_auth.lock().expect("mock auth lock") = headers
        .get("x-goog-api-key")
        .and_then(|value| value.to_str().ok())
        == Some("provider-key");
    (
        StatusCode::OK,
        Json(json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "gemini reply"}]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {"promptTokenCount": 2, "candidatesTokenCount": 2, "totalTokenCount": 4}
        })),
    )
}

async fn mock_chat_completions(
    State(state): State<MockUpstreamState>,
    headers: HeaderMap,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    *state.last_request.lock().expect("mock request lock") = Some(payload.clone());
    *state.saw_expected_auth.lock().expect("mock auth lock") = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        == Some("Bearer provider-key");
    (
        StatusCode::OK,
        Json(json!({
            "id": "chatcmpl-mock",
            "object": "chat.completion",
            "model": payload["model"],
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "mock reply"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 2, "completion_tokens": 2, "total_tokens": 4}
        })),
    )
}
