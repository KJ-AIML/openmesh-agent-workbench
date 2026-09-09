//! A5.2: Agent Engine blocking runtime and HTTP-proxy async adapter share one
//! OpenAI-compatible executor (no localhost OpenMesh proxy hop).

use axum::{routing::post, Json, Router};
use openmesh_core::agent_engine::{
    build_request_body, parse_chat_completion, ChatMessage, ChatRole, LlmRuntime,
    OpenAiCompatibleProvider, ProviderConfig,
};
use openmesh_core::llm_runtime::{complete_openai_chat_async, ProviderRuntimeSpec};
use openmesh_core::proxy_server::{ProxyModelConfig, ProxyProviderProtocol, ProxyUpstreamConfig};
use serde_json::{json, Value};
use std::net::SocketAddr;

async fn completions(Json(_body): Json<Value>) -> Json<Value> {
    Json(json!({
        "choices": [{
            "message": { "content": "parity-ok", "tool_calls": null }
        }],
        "usage": { "prompt_tokens": 3, "completion_tokens": 1, "total_tokens": 4 }
    }))
}

async fn spawn_mock() -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let app = Router::new().route("/v1/chat/completions", post(completions));
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    addr
}

fn sample_upstream(base: &str) -> ProxyUpstreamConfig {
    ProxyUpstreamConfig {
        id: "gateway".into(),
        base_url: base.into(),
        protocol: ProxyProviderProtocol::OpenAiCompatible,
        api_key: Some("sk-test".into()),
        enabled: true,
        priority: 0,
        account_id: None,
        oauth_provider: None,
        models: vec![ProxyModelConfig {
            id: "mock-model".into(),
            owned_by: "openmesh".into(),
            capabilities: vec!["chat".into()],
        }],
    }
}

#[tokio::test]
async fn agent_spec_and_proxy_upstream_describe_the_same_runtime() {
    let config = ProviderConfig {
        api_key: "sk-test".into(),
        model: "mock-model".into(),
        base_url: "https://api.example.com/v1".into(),
    };
    let from_agent = ProviderRuntimeSpec::from_agent_config("gateway", &config);
    let from_proxy = ProviderRuntimeSpec::from_proxy_upstream(
        &sample_upstream("https://api.example.com/v1"),
        "",
    )
    .expect("proxy spec");
    assert_eq!(from_agent.base_url, from_proxy.base_url);
    assert_eq!(from_agent.model, from_proxy.model);
    assert_eq!(from_agent.api_key, from_proxy.api_key);
    assert_eq!(from_agent.protocol, from_proxy.protocol);
}

#[tokio::test]
async fn blocking_engine_and_async_proxy_adapter_see_the_same_completion() {
    let addr = spawn_mock().await;
    let base = format!("http://{addr}/v1");
    let config = ProviderConfig {
        api_key: "sk-test".into(),
        model: "mock-model".into(),
        base_url: base.clone(),
    };
    let messages = [ChatMessage {
        role: ChatRole::User,
        content: "ping".into(),
        tool_call_id: None,
        name: None,
        tool_calls: vec![],
    }];
    let config_for_block = config.clone();
    let messages_for_block = messages.clone();
    let blocking_turn = tokio::task::spawn_blocking(move || {
        let blocking = OpenAiCompatibleProvider::new(config_for_block).expect("client");
        blocking
            .complete(&messages_for_block, &[])
            .expect("blocking")
    })
    .await
    .expect("join");

    let spec = ProviderRuntimeSpec::from_proxy_upstream(&sample_upstream(&base), "mock-model")
        .expect("spec");
    let async_client = reqwest::Client::new();
    let body = build_request_body(&spec.model, &messages, &[]);
    let raw = complete_openai_chat_async(&async_client, &spec, body)
        .await
        .expect("async");
    let async_turn = parse_chat_completion(&raw).expect("parse");

    assert_eq!(blocking_turn.content, "parity-ok");
    assert_eq!(async_turn.content, blocking_turn.content);
    assert_eq!(
        blocking_turn.usage.as_ref().and_then(|u| u.total_tokens),
        Some(4)
    );
    assert_eq!(async_turn.usage, blocking_turn.usage);
}

#[test]
fn oauth_only_upstream_does_not_invent_a_bearer() {
    let mut upstream = sample_upstream("https://api.example.com/v1");
    upstream.api_key = None;
    upstream.oauth_provider = Some("codex".into());
    let err = ProviderRuntimeSpec::from_proxy_upstream(&upstream, "mock-model").unwrap_err();
    assert_eq!(err.code(), "missing_credentials");
}
