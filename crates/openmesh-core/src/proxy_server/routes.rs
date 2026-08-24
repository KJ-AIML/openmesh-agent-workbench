use super::{
    config::MAX_PROXY_BODY_BYTES, ProxyProviderProtocol, ProxyRoutingStrategy, ProxyServerState,
};
use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::Request;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{delete, get, post};
use axum::Router;
use base64::Engine;
use chrono::Utc;
use futures::StreamExt;
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) fn router(state: Arc<ProxyServerState>) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/v1/models", get(list_models).post(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(responses))
        .route("/v1/embeddings", post(embeddings))
        .route("/v1/messages", post(messages))
        .route("/v0/management/status", get(management_status))
        .route(
            "/v0/management/config",
            get(management_config).put(management_config_update),
        )
        .route(
            "/v0/management/providers",
            get(management_providers)
                .put(management_providers_update)
                .post(management_provider_create),
        )
        .route(
            "/v0/management/providers/:id",
            delete(management_provider_delete),
        )
        .route("/v0/management/accounts", get(management_accounts))
        .route(
            "/v0/management/accounts/active",
            post(management_account_activate),
        )
        .route("/v0/management/quotas", get(management_quotas))
        .route("/v0/management/models/:provider", get(management_models))
        .route("/v0/management/usage", get(management_usage))
        .layer(middleware::from_fn_with_state(state.clone(), record_usage))
        .with_state(state)
}

async fn record_usage(
    State(state): State<Arc<ProxyServerState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let path = request.uri().path().to_string();
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let request_id = response
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let upstream_id = response
        .headers()
        .get("x-openmesh-upstream")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let account_id = response
        .headers()
        .get("x-openmesh-account-id")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let model = response
        .headers()
        .get("x-openmesh-model")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let input_tokens = response
        .headers()
        .get("x-openmesh-input-tokens")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let output_tokens = response
        .headers()
        .get("x-openmesh-output-tokens")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    let total_tokens = response
        .headers()
        .get("x-openmesh-total-tokens")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    state.usage.record(
        request_id,
        path,
        response.status().as_u16(),
        started.elapsed().as_millis() as u64,
        model,
        input_tokens,
        output_tokens,
        total_tokens,
        upstream_id,
        account_id,
    );
    response
}

async fn health(State(state): State<Arc<ProxyServerState>>) -> Response {
    let payload = json!({
        "status": "ok",
        "runtime": "openmesh-built-in",
        "bindHost": state.config().bind_host,
        "port": state.config().port,
    });
    json_response(StatusCode::OK, &payload, &state.next_request_id())
}

async fn list_models(State(state): State<Arc<ProxyServerState>>, headers: HeaderMap) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }

    let config = state.config();
    let mut models = BTreeMap::<String, Value>::new();
    for upstream in config.upstreams.iter().filter(|upstream| upstream.enabled) {
        for model in &upstream.models {
            models.entry(model.id.clone()).or_insert_with(|| {
                json!({
                    "id": model.id,
                    "object": "model",
                    "created": 0,
                    "owned_by": model.owned_by,
                    "capabilities": model.capabilities,
                })
            });
        }
    }

    // Aliases are first-class model IDs. Clients can discover the name they
    // send while OpenMesh resolves it to the configured upstream model.
    for (alias, target) in &config.model_aliases {
        if let Some(base) = models.get(target).cloned() {
            let mut alias_model = base;
            if let Some(object) = alias_model.as_object_mut() {
                object.insert("id".to_string(), Value::String(alias.clone()));
                object.insert(
                    "owned_by".to_string(),
                    Value::String("openmesh".to_string()),
                );
            }
            models.insert(alias.clone(), alias_model);
        }
    }

    let payload = json!({
        "object": "list",
        "data": models.into_values().collect::<Vec<_>>(),
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_status(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let config = state.config();
    let payload = json!({
        "mode": "managed",
        "ownership": "built-in",
        "managementEndpoint": management_base_endpoint(&config),
        "dataPlaneEndpoint": data_plane_base_endpoint(&config),
        "managementStatus": "ready",
        "dataPlaneStatus": "ready",
        "capabilities": {
            "chat": "available",
            "responses": "available",
            "embeddings": "available",
            "claudeMessages": "available",
            "geminiTranslation": "available",
            "oauth": {
                "status": "partial",
                "nativeProviders": ["codex", "claude", "antigravity", "grok", "kimi"],
                "tokenStore": "os-credential-manager"
            },
            "accounts": "available",
            "quotas": "deferred",
            "usage": "in-memory"
        }
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_config(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let config = state.config();
    let payload = management_config_payload(&config);
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_config_update(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let payload = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let current = state.config();
    let candidate = match merge_management_config(&current, payload, &request_id) {
        Ok(config) => config,
        Err(response) => return response,
    };
    if candidate.bind_host != current.bind_host
        || candidate.port != current.port
        || candidate.request_timeout_secs != current.request_timeout_secs
    {
        return ProxyRouteError::RestartRequired.response(&request_id);
    }
    if state.persist_config_value(&candidate).is_err() {
        return ProxyRouteError::ConfigurationPersistence.response(&request_id);
    }
    if state.replace_config(candidate).is_err() {
        return ProxyRouteError::InvalidRequest("proxy configuration is invalid")
            .response(&request_id);
    }
    let config = state.config();
    let payload = management_config_payload(&config);
    json_response(StatusCode::OK, &payload, &request_id)
}

fn management_config_payload(config: &super::ProxyServerConfig) -> Value {
    json!({
        "bindHost": config.bind_host,
        "port": config.port,
        "allowUnauthenticated": config.allow_unauthenticated,
        "requestTimeoutSecs": config.request_timeout_secs,
        "routingStrategy": config.routing_strategy,
        "maxRetries": config.max_retries,
        "apiKeyCount": config.api_keys.len(),
        "upstreamCount": config.upstreams.len(),
        "modelAliasCount": config.model_aliases.len(),
        "upstreams": config.upstreams.iter().map(|upstream| json!({
            "id": upstream.id,
            "baseUrl": upstream.base_url,
            "protocol": upstream.protocol,
            "apiKeyConfigured": upstream.api_key.as_ref().is_some_and(|key| !key.trim().is_empty()),
            "enabled": upstream.enabled,
            "priority": upstream.priority,
            "accountId": upstream.account_id,
            "oauthProvider": upstream.oauth_provider,
            "models": upstream.models.clone(),
        })).collect::<Vec<_>>(),
        "modelAliases": config.model_aliases.clone(),
        "modelFallbacks": config.model_fallbacks.clone(),
    })
}

fn merge_management_config(
    current: &super::ProxyServerConfig,
    requested: Value,
    request_id: &str,
) -> Result<super::ProxyServerConfig, Response> {
    let mut requested = requested.as_object().cloned().ok_or_else(|| {
        ProxyRouteError::InvalidRequest("proxy configuration must be a JSON object")
            .response(request_id)
    })?;

    // The GET representation intentionally includes safe read-only metadata.
    // Ignore that metadata on PUT so clients can edit and submit the redacted
    // document without accidentally trying to treat counts as configuration.
    for key in ["apiKeyCount", "upstreamCount", "modelAliasCount"] {
        requested.remove(key);
    }

    let mut merged = serde_json::to_value(current).map_err(|_| {
        ProxyRouteError::InvalidRequest("proxy configuration could not be serialized")
            .response(request_id)
    })?;
    let Some(merged_object) = merged.as_object_mut() else {
        return Err(
            ProxyRouteError::InvalidRequest("proxy configuration could not be serialized")
                .response(request_id),
        );
    };
    for (key, value) in requested {
        merged_object.insert(key, value);
    }

    // Secrets are omitted by the GET representation. Preserve an existing
    // upstream credential when its editable entry does not include apiKey;
    // explicit null still removes it.
    if let Some(upstreams) = merged_object
        .get_mut("upstreams")
        .and_then(Value::as_array_mut)
    {
        for upstream in upstreams {
            let Some(upstream_object) = upstream.as_object_mut() else {
                continue;
            };
            upstream_object.remove("apiKeyConfigured");
            if upstream_object.contains_key("apiKey") {
                continue;
            }
            let Some(id) = upstream_object.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(existing) = current.upstreams.iter().find(|item| item.id == id) {
                if let Some(api_key) = existing.api_key.as_ref() {
                    upstream_object.insert("apiKey".to_string(), Value::String(api_key.clone()));
                }
            }
        }
    }

    serde_json::from_value(merged).map_err(|_| {
        ProxyRouteError::InvalidRequest("proxy configuration is invalid").response(request_id)
    })
}

async fn management_providers(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let config = state.config();
    json_response(
        StatusCode::OK,
        &management_providers_payload(&config),
        &request_id,
    )
}

async fn management_providers_update(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let requested = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let current = state.config();
    let candidate = match merge_management_config(&current, requested, &request_id) {
        Ok(config) => config,
        Err(response) => return response,
    };
    if candidate.bind_host != current.bind_host
        || candidate.port != current.port
        || candidate.request_timeout_secs != current.request_timeout_secs
    {
        return ProxyRouteError::RestartRequired.response(&request_id);
    }
    if state.persist_config_value(&candidate).is_err() {
        return ProxyRouteError::ConfigurationPersistence.response(&request_id);
    }
    if state.replace_config(candidate).is_err() {
        return ProxyRouteError::InvalidRequest("proxy configuration is invalid")
            .response(&request_id);
    }
    let config = state.config();
    json_response(
        StatusCode::OK,
        &management_providers_payload(&config),
        &request_id,
    )
}

async fn management_provider_create(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let requested = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let mut provider = requested.get("provider").cloned().unwrap_or(requested);
    if let Some(object) = provider.as_object_mut() {
        for key in [
            "apiKeyConfigured",
            "endpointConfigured",
            "keyConfigured",
            "keyCount",
        ] {
            object.remove(key);
        }
    }
    let upstream = match serde_json::from_value::<super::ProxyUpstreamConfig>(provider) {
        Ok(upstream) => upstream,
        Err(_) => {
            return ProxyRouteError::InvalidRequest(
                "provider payload must contain a valid OpenMesh upstream",
            )
            .response(&request_id)
        }
    };
    let mut candidate = state.config();
    if candidate
        .upstreams
        .iter()
        .any(|existing| existing.id == upstream.id)
    {
        return ProxyRouteError::InvalidRequest("provider id is already configured")
            .response(&request_id);
    }
    candidate.upstreams.push(upstream);
    if candidate.validate().is_err() {
        return ProxyRouteError::InvalidRequest("provider configuration is invalid")
            .response(&request_id);
    }
    if state.persist_config_value(&candidate).is_err() {
        return ProxyRouteError::ConfigurationPersistence.response(&request_id);
    }
    if state.replace_config(candidate).is_err() {
        return ProxyRouteError::InvalidRequest("provider configuration is invalid")
            .response(&request_id);
    }
    let config = state.config();
    json_response(
        StatusCode::CREATED,
        &management_providers_payload(&config),
        &request_id,
    )
}

async fn management_provider_delete(
    State(state): State<Arc<ProxyServerState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let mut candidate = state.config();
    let before = candidate.upstreams.len();
    candidate.upstreams.retain(|upstream| upstream.id != id);
    if candidate.upstreams.len() == before {
        return ProxyRouteError::ProviderNotFound.response(&request_id);
    }
    if candidate.upstreams.is_empty() {
        return ProxyRouteError::InvalidRequest("at least one provider must remain configured")
            .response(&request_id);
    }
    if state.persist_config_value(&candidate).is_err() {
        return ProxyRouteError::ConfigurationPersistence.response(&request_id);
    }
    if state.replace_config(candidate).is_err() {
        return ProxyRouteError::InvalidRequest("provider configuration is invalid")
            .response(&request_id);
    }
    let config = state.config();
    json_response(
        StatusCode::OK,
        &management_providers_payload(&config),
        &request_id,
    )
}

async fn management_account_activate(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let requested = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let Some(account_id) = requested
        .get("accountId")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|account_id| !account_id.is_empty())
    else {
        return ProxyRouteError::InvalidRequest("accountId is required").response(&request_id);
    };
    let mut candidate = state.config();
    let matching = candidate
        .upstreams
        .iter()
        .filter(|upstream| upstream.account_id.as_deref() == Some(account_id))
        .count();
    if matching == 0 {
        return ProxyRouteError::AccountNotFound.response(&request_id);
    }
    let preferred_priority = candidate
        .upstreams
        .iter()
        .map(|upstream| upstream.priority)
        .min()
        .unwrap_or_default()
        .saturating_sub(1);
    for upstream in &mut candidate.upstreams {
        if upstream.account_id.as_deref() == Some(account_id) {
            upstream.priority = preferred_priority;
        }
    }
    if state.persist_config_value(&candidate).is_err() {
        return ProxyRouteError::ConfigurationPersistence.response(&request_id);
    }
    if state.replace_config(candidate).is_err() {
        return ProxyRouteError::InvalidRequest(
            "account activation produced invalid configuration",
        )
        .response(&request_id);
    }
    json_response(
        StatusCode::OK,
        &json!({"accountId": account_id, "status": "active"}),
        &request_id,
    )
}

fn management_providers_payload(config: &super::ProxyServerConfig) -> Value {
    json!({
        "revision": "openmesh-static-1",
        "routingStrategy": config.routing_strategy,
        "providers": config.upstreams.iter().map(|upstream| json!({
            "id": upstream.id,
            "name": upstream.id,
            "baseUrl": upstream.base_url,
            "protocol": upstream.protocol,
            "endpointConfigured": true,
            "keyConfigured": upstream.api_key.as_ref().is_some_and(|key| !key.trim().is_empty()),
            "keyCount": u8::from(upstream.api_key.is_some()),
            "enabled": upstream.enabled,
            "models": upstream.models.clone(),
            "priority": upstream.priority,
            "accountId": upstream.account_id,
            "oauthProvider": upstream.oauth_provider,
        })).collect::<Vec<_>>(),
    })
}

async fn management_accounts(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let config = state.config();
    let mut accounts = BTreeMap::<String, Value>::new();
    for upstream in &config.upstreams {
        let account_id = upstream
            .account_id
            .clone()
            .unwrap_or_else(|| upstream.id.clone());
        let credential_status = if !upstream.enabled {
            "disabled"
        } else if let (Some(provider), Some(account_id)) = (
            upstream.oauth_provider.as_deref(),
            upstream.account_id.as_deref(),
        ) {
            match provider.parse::<crate::oauth::OAuthProviderId>() {
                Ok(provider) => match state.oauth_resolver() {
                    Some(resolver) => match resolver.resolve(provider, account_id).await {
                        Ok(Some(_)) => "ready",
                        Ok(None) => "missing",
                        Err(_) => "error",
                    },
                    None => "credential-unavailable",
                },
                Err(_) => "invalid",
            }
        } else if upstream
            .api_key
            .as_deref()
            .is_some_and(|api_key| !api_key.trim().is_empty())
        {
            "configured"
        } else {
            "missing"
        };
        let entry = accounts.entry(account_id.clone()).or_insert_with(|| {
            json!({
                "id": account_id,
                "status": if upstream.enabled { "enabled" } else { "disabled" },
                "upstreamIds": [],
                "quotaStatus": "unavailable",
                "credentialStatus": credential_status,
                "oauthProvider": upstream.oauth_provider,
            })
        });
        if let Some(ids) = entry.get_mut("upstreamIds").and_then(Value::as_array_mut) {
            ids.push(Value::String(upstream.id.clone()));
        }
    }
    let payload = json!({
        "object": "list",
        "data": accounts.into_values().collect::<Vec<_>>(),
        "quotaSupport": "deferred",
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_quotas(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let config = state.config();
    let payload = json!({
        "object": "list",
        "data": config.upstreams.iter().map(|upstream| json!({
            "provider": upstream.id,
            "accountId": upstream.account_id,
            "status": "unavailable",
            "remaining": null,
            "reason": "provider quota polling is not implemented",
        })).collect::<Vec<_>>(),
        "supported": false,
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_models(
    State(state): State<Arc<ProxyServerState>>,
    Path(provider): Path<String>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let provider = provider.to_ascii_lowercase();
    let protocol = match provider.as_str() {
        "openai" | "codex" | "open-ai-compatible" => Some(ProxyProviderProtocol::OpenAiCompatible),
        "claude" | "anthropic" => Some(ProxyProviderProtocol::Anthropic),
        "gemini" | "google" => Some(ProxyProviderProtocol::Gemini),
        _ => None,
    };
    let config = state.config();
    let models = config
        .upstreams
        .iter()
        .filter(|upstream| {
            protocol.is_some_and(|expected| upstream.protocol == expected)
                || upstream.id.eq_ignore_ascii_case(&provider)
        })
        .flat_map(|upstream| upstream.models.clone())
        .collect::<Vec<_>>();
    let payload = json!({
        "provider": provider,
        "object": "list",
        "data": models,
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn management_usage(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let snapshot = state.usage.snapshot(100);
    let payload = serde_json::to_value(snapshot).unwrap_or_else(|_| {
        json!({
            "totalRequests": 0,
            "successfulRequests": 0,
            "failedRequests": 0,
            "logs": []
        })
    });
    json_response(StatusCode::OK, &payload, &request_id)
}

async fn chat_completions(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let payload = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_model = match required_model(&payload, &request_id) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let selections = match select_upstreams(&state, &requested_model, &request_id) {
        Ok(selections) => selections,
        Err(response) => return response,
    };
    let stream = payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    match execute_openai_chat_candidates(&state, &selections, payload, stream, &request_id).await {
        Ok(response) => response,
        Err(error) => error.response(&request_id),
    }
}

async fn responses(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let payload = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_model = match required_model(&payload, &request_id) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let selections = match select_upstreams(&state, &requested_model, &request_id) {
        Ok(selections) => selections,
        Err(response) => return response,
    };
    // Keep native Responses execution when every candidate is Codex. If the
    // fallback chain mixes Codex with another provider, use the general chat
    // compatibility path so each candidate is dispatched by its own executor.
    if uses_native_codex_responses(selections.as_slice()) {
        let wants_stream = payload
            .get("stream")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        return match execute_codex_responses_candidates(
            &state,
            &selections,
            payload,
            wants_stream,
            &request_id,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => error.response(&request_id),
        };
    }
    let wants_stream = payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let chat_payload =
        match responses_to_chat_payload(&payload, &selections[0].resolved_model, &request_id) {
            Ok(value) => value,
            Err(response) => return response,
        };

    // The first compatibility slice translates Responses into the ubiquitous
    // Chat Completions upstream. A streamed Responses request is emitted as a
    // valid SSE sequence after the upstream has completed; later provider
    // adapters can replace this with token-level native streaming.
    match execute_openai_chat_candidates(&state, &selections, chat_payload, false, &request_id)
        .await
    {
        Ok(upstream_response) => {
            let status = upstream_response.status();
            let internal_headers = internal_headers(&upstream_response);
            let bytes =
                match axum::body::to_bytes(upstream_response.into_body(), MAX_PROXY_BODY_BYTES)
                    .await
                {
                    Ok(bytes) => bytes,
                    Err(_) => {
                        return ProxyRouteError::UpstreamInvalidResponse.response(&request_id)
                    }
                };
            if !status.is_success() {
                return ProxyRouteError::UpstreamStatus(status.as_u16()).response(&request_id);
            }
            let translated = match translate_chat_response(&bytes, &requested_model, &request_id) {
                Ok(value) => value,
                Err(error) => return error.response(&request_id),
            };
            if wants_stream {
                let mut response = responses_sse_response(&translated, &request_id);
                copy_internal_headers(&mut response, &internal_headers);
                annotate_usage(&mut response, &translated);
                response
            } else {
                let mut response = json_response(StatusCode::OK, &translated, &request_id);
                copy_internal_headers(&mut response, &internal_headers);
                annotate_usage(&mut response, &translated);
                response
            }
        }
        Err(error) => error.response(&request_id),
    }
}

async fn embeddings(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let payload = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_model = match required_model(&payload, &request_id) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let selections = match select_upstreams(&state, &requested_model, &request_id) {
        Ok(selections) => selections,
        Err(response) => return response,
    };
    match forward_json_candidates(
        &state,
        &selections,
        "embeddings",
        payload,
        false,
        &request_id,
    )
    .await
    {
        Ok(response) => response,
        Err(error) => error.response(&request_id),
    }
}

async fn messages(
    State(state): State<Arc<ProxyServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let request_id = request_id(&headers, &state);
    if let Some(response) = require_auth(&state, &headers, &request_id) {
        return response;
    }
    let payload = match parse_json_request(&body, &request_id) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let requested_model = match required_model(&payload, &request_id) {
        Ok(model) => model,
        Err(response) => return response,
    };
    let selections = match select_upstreams(&state, &requested_model, &request_id) {
        Ok(selections) => selections,
        Err(response) => return response,
    };
    let wants_stream = payload
        .get("stream")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    // Direct Anthropic forwarding preserves the native messages contract. A
    // mixed-protocol fallback chain must use the general executor so a later
    // OpenAI/Gemini candidate is not filtered out before it can be tried.
    if selections
        .iter()
        .all(|selection| selection.protocol == ProxyProviderProtocol::Anthropic)
    {
        let anthropic_selections = selections
            .iter()
            .filter(|selection| selection.protocol == ProxyProviderProtocol::Anthropic)
            .cloned()
            .collect::<Vec<_>>();
        return match forward_json_candidates(
            &state,
            &anthropic_selections,
            "messages",
            payload,
            wants_stream,
            &request_id,
        )
        .await
        {
            Ok(response) => response,
            Err(error) => error.response(&request_id),
        };
    }

    let chat_payload =
        match messages_to_chat_payload(&payload, &selections[0].resolved_model, &request_id) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let upstream_response =
        match execute_openai_chat_candidates(&state, &selections, chat_payload, false, &request_id)
            .await
        {
            Ok(response) => response,
            Err(error) => return error.response(&request_id),
        };
    let internal_headers = internal_headers(&upstream_response);
    let bytes =
        match axum::body::to_bytes(upstream_response.into_body(), MAX_PROXY_BODY_BYTES).await {
            Ok(bytes) => bytes,
            Err(_) => return ProxyRouteError::UpstreamInvalidResponse.response(&request_id),
        };
    let anthropic =
        match translate_openai_response_to_anthropic(&bytes, &requested_model, &request_id) {
            Ok(value) => value,
            Err(error) => return error.response(&request_id),
        };
    if wants_stream {
        let mut response = anthropic_sse_response(&anthropic, &request_id);
        copy_internal_headers(&mut response, &internal_headers);
        annotate_usage(&mut response, &anthropic);
        response
    } else {
        let mut response = json_response(StatusCode::OK, &anthropic, &request_id);
        copy_internal_headers(&mut response, &internal_headers);
        annotate_usage(&mut response, &anthropic);
        response
    }
}

#[derive(Clone, Debug)]
struct UpstreamSelection {
    upstream_id: String,
    account_id: Option<String>,
    base_url: String,
    api_key: Option<String>,
    oauth_provider: Option<String>,
    protocol: ProxyProviderProtocol,
    resolved_model: String,
}

async fn forward_json(
    state: &ProxyServerState,
    selection: &UpstreamSelection,
    path: &str,
    payload: Value,
    stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let endpoint = format!("{}/{}", selection.base_url.trim_end_matches('/'), path);
    let mut request = state
        .client()
        .post(endpoint)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-request-id", request_id)
        .json(&payload);
    let oauth_credential = if let Some(provider) = selection.oauth_provider.as_deref() {
        let provider = provider
            .parse::<crate::oauth::OAuthProviderId>()
            .map_err(|_| ProxyRouteError::OAuthCredentialUnavailable)?;
        let account_id = selection
            .account_id
            .as_deref()
            .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
        let resolver = state
            .oauth_resolver()
            .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
        resolver
            .resolve(provider, account_id)
            .await
            .map_err(|_| ProxyRouteError::OAuthCredentialUnavailable)?
            .ok_or(ProxyRouteError::OAuthCredentialUnavailable)
            .map(Some)?
    } else {
        None
    };
    if let Some(credential) = oauth_credential {
        let authorization = credential
            .authorization_header()
            .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
        request = request.header(header::AUTHORIZATION, authorization);
        if selection.oauth_provider.as_deref() == Some("claude")
            && selection.protocol == ProxyProviderProtocol::Anthropic
        {
            // Claude Code OAuth requests are distinguished from API-key
            // requests by this beta and client profile. Keep this in the
            // provider executor boundary rather than leaking it into routes.
            request = request
                .header("anthropic-version", "2023-06-01")
                .header("anthropic-beta", "oauth-2025-04-20")
                .header("anthropic-dangerous-direct-browser-access", "true")
                .header("x-app", "cli")
                .header("user-agent", "claude-cli/2.1.220 (external, cli)")
                .header("x-client-request-id", request_id);
        } else if selection.oauth_provider.as_deref() == Some("grok") {
            request = request
                .header("x-xai-token-auth", "xai-grok-cli")
                .header("x-grok-client-version", "0.2.120")
                .header("x-grok-client-identifier", "grok-shell")
                .header("x-authenticateresponse", "authenticate-response")
                .header("user-agent", "xai-grok-workspace/0.2.120");
        } else if selection.oauth_provider.as_deref() == Some("kimi") {
            request = request
                .header("x-msh-platform", "OpenMesh")
                .header("x-msh-version", env!("CARGO_PKG_VERSION"))
                .header("x-msh-device-name", "openmesh")
                .header("x-msh-device-model", std::env::consts::OS);
            if let Some(device_id) = credential.metadata.get("device_id") {
                request = request.header("x-msh-device-id", device_id);
            }
        } else if selection.oauth_provider.as_deref() == Some("antigravity") {
            request = request
                .header("user-agent", "antigravity/hub/2.9.1 darwin/arm64")
                .header("x-goog-api-client", "gl-node/22.21.1");
            if let Some(project_id) = credential.metadata.get("project_id") {
                request = request.header("x-goog-user-project", project_id);
            }
        }
    } else if let Some(api_key) = selection.api_key.as_deref() {
        request = match selection.protocol {
            ProxyProviderProtocol::OpenAiCompatible => request.bearer_auth(api_key),
            ProxyProviderProtocol::Anthropic => request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01"),
            ProxyProviderProtocol::Gemini => request.header("x-goog-api-key", api_key),
        };
    }
    if stream {
        request = request.header(header::ACCEPT, "text/event-stream");
    }

    let upstream = request
        .send()
        .await
        .map_err(|_| ProxyRouteError::UpstreamUnavailable)?;
    let status = upstream.status();
    if !status.is_success() {
        // Consume the body so the connection can be reused, but never reflect
        // provider text that could contain credentials or internal URLs.
        let _ = upstream.bytes().await;
        return Err(ProxyRouteError::UpstreamStatus(status.as_u16()));
    }

    if stream {
        let content_type = upstream
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("text/event-stream")
            .to_string();
        let stream = upstream.bytes_stream().map(|chunk| {
            chunk.map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
        });
        let mut response = Response::new(Body::from_stream(stream));
        *response.status_mut() = status;
        if let Ok(value) = HeaderValue::from_str(&content_type) {
            response.headers_mut().insert(header::CONTENT_TYPE, value);
        }
        annotate_upstream(&mut response, selection);
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(request_id).expect("request id is validated"),
        );
        Ok(response)
    } else {
        let bytes = upstream
            .bytes()
            .await
            .map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
        let value: Value =
            serde_json::from_slice(&bytes).map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
        let mut response = json_response(status, &value, request_id);
        annotate_upstream(&mut response, selection);
        annotate_usage(&mut response, &value);
        Ok(response)
    }
}

async fn forward_json_candidates(
    state: &ProxyServerState,
    selections: &[UpstreamSelection],
    path: &str,
    payload: Value,
    stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let max_attempts = state.config().max_retries.saturating_add(1) as usize;
    let mut last_error = ProxyRouteError::UpstreamUnavailable;
    for selection in selections.iter().take(max_attempts.max(1)) {
        let mut attempt_payload = payload.clone();
        if let Some(object) = attempt_payload.as_object_mut() {
            object.insert(
                "model".to_string(),
                Value::String(selection.resolved_model.clone()),
            );
        }
        match forward_json(state, selection, path, attempt_payload, stream, request_id).await {
            Ok(response) => return Ok(response),
            Err(error) if error.retryable() => {
                if matches!(error, ProxyRouteError::UpstreamStatus(429)) {
                    state.mark_rate_limited(&selection.upstream_id);
                }
                last_error = error;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

async fn execute_openai_chat(
    state: &ProxyServerState,
    selection: &UpstreamSelection,
    payload: Value,
    stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    if selection.oauth_provider.as_deref() == Some("codex") {
        return execute_codex_chat(state, selection, payload, stream, request_id).await;
    }
    if selection.protocol == ProxyProviderProtocol::OpenAiCompatible {
        return forward_json(
            state,
            selection,
            "chat/completions",
            payload,
            stream,
            request_id,
        )
        .await;
    }

    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or(&selection.resolved_model);
    let provider_payload = match selection.protocol {
        ProxyProviderProtocol::Anthropic => openai_to_anthropic_payload(&payload, model),
        ProxyProviderProtocol::Gemini => openai_to_gemini_payload(&payload, model),
        ProxyProviderProtocol::OpenAiCompatible => unreachable!(),
    }?;
    let path = match selection.protocol {
        ProxyProviderProtocol::Anthropic => "messages".to_string(),
        ProxyProviderProtocol::Gemini => gemini_generate_content_path(model)?,
        ProxyProviderProtocol::OpenAiCompatible => unreachable!(),
    };
    let upstream_response =
        forward_json(state, selection, &path, provider_payload, false, request_id).await?;
    let internal_headers = internal_headers(&upstream_response);
    let bytes = axum::body::to_bytes(upstream_response.into_body(), MAX_PROXY_BODY_BYTES)
        .await
        .map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
    let translated = provider_to_openai_response(&bytes, selection.protocol, model)?;
    if stream {
        let mut response = openai_sse_response(&translated, request_id);
        copy_internal_headers(&mut response, &internal_headers);
        annotate_usage(&mut response, &translated);
        annotate_upstream(&mut response, selection);
        Ok(response)
    } else {
        let mut response = json_response(StatusCode::OK, &translated, request_id);
        copy_internal_headers(&mut response, &internal_headers);
        annotate_usage(&mut response, &translated);
        annotate_upstream(&mut response, selection);
        Ok(response)
    }
}

async fn execute_codex_chat(
    state: &ProxyServerState,
    selection: &UpstreamSelection,
    payload: Value,
    wants_stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let codex_payload = openai_chat_to_codex_responses(&payload);
    let (status, _upstream_headers, bytes) =
        send_codex_request(state, selection, codex_payload, request_id).await?;
    if !status.is_success() {
        return Err(ProxyRouteError::UpstreamStatus(status.as_u16()));
    }
    let terminal = codex_terminal_event(&bytes).ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let translated = codex_terminal_to_openai_chat(
        terminal.get("response").unwrap_or(&terminal),
        payload
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("codex"),
    );
    let mut response = if wants_stream {
        openai_sse_response(&translated, request_id)
    } else {
        json_response(StatusCode::OK, &translated, request_id)
    };
    annotate_upstream(&mut response, selection);
    annotate_usage(&mut response, &translated);
    Ok(response)
}

async fn execute_codex_responses(
    state: &ProxyServerState,
    selection: &UpstreamSelection,
    payload: Value,
    wants_stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let codex_payload = normalize_codex_responses_payload(&payload);
    let (status, _upstream_headers, bytes) =
        send_codex_request(state, selection, codex_payload, request_id).await?;
    if !status.is_success() {
        return Err(ProxyRouteError::UpstreamStatus(status.as_u16()));
    }
    let terminal = codex_terminal_event(&bytes).ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let mut response = if wants_stream {
        let mut response = Response::new(Body::from(bytes));
        *response.status_mut() = StatusCode::OK;
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        response.headers_mut().insert(
            "x-request-id",
            HeaderValue::from_str(request_id).expect("request id is validated"),
        );
        response
    } else {
        let payload = terminal.get("response").cloned().unwrap_or(terminal);
        let mut response = json_response(StatusCode::OK, &payload, request_id);
        annotate_usage(&mut response, &payload);
        response
    };
    annotate_upstream(&mut response, selection);
    Ok(response)
}

async fn execute_codex_responses_candidates(
    state: &ProxyServerState,
    selections: &[UpstreamSelection],
    payload: Value,
    wants_stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let max_attempts = state.config().max_retries.saturating_add(1) as usize;
    let mut last_error = ProxyRouteError::UpstreamUnavailable;
    for selection in selections.iter().take(max_attempts.max(1)) {
        let mut attempt_payload = payload.clone();
        if let Some(object) = attempt_payload.as_object_mut() {
            object.insert(
                "model".to_string(),
                Value::String(selection.resolved_model.clone()),
            );
        }
        match execute_codex_responses(state, selection, attempt_payload, wants_stream, request_id)
            .await
        {
            Ok(response) => return Ok(response),
            Err(error) if error.retryable() => {
                if matches!(error, ProxyRouteError::UpstreamStatus(429)) {
                    state.mark_rate_limited(&selection.upstream_id);
                }
                last_error = error;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

async fn send_codex_request(
    state: &ProxyServerState,
    selection: &UpstreamSelection,
    payload: Value,
    request_id: &str,
) -> Result<(StatusCode, HeaderMap, Bytes), ProxyRouteError> {
    let provider = selection
        .oauth_provider
        .as_deref()
        .and_then(|value| value.parse::<crate::oauth::OAuthProviderId>().ok())
        .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
    let account_id = selection
        .account_id
        .as_deref()
        .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
    let resolver = state
        .oauth_resolver()
        .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
    let credential = resolver
        .resolve(provider, account_id)
        .await
        .map_err(|_| ProxyRouteError::OAuthCredentialUnavailable)?
        .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
    let authorization = credential
        .authorization_header()
        .ok_or(ProxyRouteError::OAuthCredentialUnavailable)?;
    let endpoint = format!("{}/responses", selection.base_url.trim_end_matches('/'));
    let mut request = state
        .client()
        .post(endpoint)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "text/event-stream")
        .header("authorization", authorization)
        .header("originator", "codex-tui")
        .header("user-agent", "codex-tui/0.146.0")
        .header("x-client-request-id", request_id)
        .header("x-request-id", request_id)
        .json(&payload);
    if let Some(account_id) = selection.account_id.as_deref() {
        request = request.header("chatgpt-account-id", account_id);
    }
    let response = request
        .send()
        .await
        .map_err(|_| ProxyRouteError::UpstreamUnavailable)?;
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response
        .bytes()
        .await
        .map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
    Ok((status, headers, bytes))
}

fn normalize_codex_responses_payload(payload: &Value) -> Value {
    let mut normalized = payload.clone();
    let Some(object) = normalized.as_object_mut() else {
        return normalized;
    };
    if let Some(input) = object.get("input").cloned() {
        if let Some(text) = input.as_str() {
            object.insert(
                "input".to_string(),
                json!([{"type":"message","role":"user","content":[{"type":"input_text","text":text}]}]),
            );
        }
    }
    object.insert("stream".to_string(), Value::Bool(true));
    object.insert("store".to_string(), Value::Bool(false));
    object.insert("parallel_tool_calls".to_string(), Value::Bool(true));
    object.insert(
        "include".to_string(),
        json!(["reasoning.encrypted_content"]),
    );
    for field in [
        "max_output_tokens",
        "max_completion_tokens",
        "temperature",
        "top_p",
        "truncation",
        "prompt_cache_options",
        "prompt_cache_retention",
        "user",
        "context_management",
    ] {
        object.remove(field);
    }
    if let Some(input) = object.get_mut("input").and_then(Value::as_array_mut) {
        for item in input {
            if item.get("role").and_then(Value::as_str) == Some("system") {
                if let Some(item) = item.as_object_mut() {
                    item.insert("role".to_string(), Value::String("developer".to_string()));
                }
            }
        }
    }
    normalized
}

fn openai_chat_to_codex_responses(payload: &Value) -> Value {
    let model = payload.get("model").cloned().unwrap_or(Value::Null);
    let mut input = Vec::new();
    let mut instructions = Vec::new();
    for message in payload
        .get("messages")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let role = message
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user");
        if matches!(role, "system" | "developer") {
            if let Some(text) = message_text(message) {
                instructions.push(text);
            }
            continue;
        }
        if role == "tool" {
            input.push(json!({
                "type": "function_call_output",
                "call_id": message.get("tool_call_id").cloned().unwrap_or(Value::String("tool-call".to_string())),
                "output": message.get("content").cloned().unwrap_or(Value::String(String::new())),
            }));
            continue;
        }
        if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
            for tool_call in tool_calls {
                let function = tool_call.get("function").cloned().unwrap_or(Value::Null);
                input.push(json!({
                    "type": "function_call",
                    "call_id": tool_call.get("id").cloned().unwrap_or(Value::String("tool-call".to_string())),
                    "name": function.get("name").cloned().unwrap_or(Value::String("function".to_string())),
                    "arguments": function.get("arguments").cloned().unwrap_or(Value::String("{}".to_string())),
                }));
            }
        }
        let content = codex_message_content(message, role == "assistant");
        input.push(json!({"type":"message","role":role,"content":content}));
    }
    let mut output = json!({
        "model": model,
        "input": input,
        "instructions": instructions.join("\n\n"),
        "stream": true,
        "store": false,
        "parallel_tool_calls": true,
        "include": ["reasoning.encrypted_content"],
    });
    if let Some(tools) = payload.get("tools") {
        output["tools"] = tools.clone();
    }
    if let Some(tool_choice) = payload.get("tool_choice") {
        output["tool_choice"] = tool_choice.clone();
    }
    if let Some(reasoning) = payload.get("reasoning") {
        output["reasoning"] = reasoning.clone();
    } else if let Some(effort) = payload.get("reasoning_effort") {
        output["reasoning"] = json!({"effort": effort});
    }
    output
}

fn message_text(message: &Value) -> Option<String> {
    match message.get("content") {
        Some(Value::String(text)) => Some(text.clone()),
        Some(Value::Array(parts)) => Some(
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        _ => None,
    }
}

fn codex_message_content(message: &Value, assistant: bool) -> Vec<Value> {
    match message.get("content") {
        Some(Value::String(text)) => vec![json!({
            "type": if assistant { "output_text" } else { "input_text" },
            "text": text,
        })],
        Some(Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| match part.get("type").and_then(Value::as_str) {
                Some("text") => Some(json!({
                    "type": if assistant { "output_text" } else { "input_text" },
                    "text": part.get("text").cloned().unwrap_or(Value::String(String::new())),
                })),
                Some("image_url") if !assistant => Some(json!({
                    "type": "input_image",
                    "image_url": part.get("image_url").and_then(|image| image.get("url")).cloned().unwrap_or(Value::Null),
                })),
                _ => None,
            })
            .collect(),
        _ => vec![json!({
            "type": if assistant { "output_text" } else { "input_text" },
            "text": "",
        })],
    }
}

fn codex_terminal_event(bytes: &[u8]) -> Option<Value> {
    let mut terminal = None;
    for line in bytes.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(data) = line.strip_prefix(b"data:") else {
            continue;
        };
        let data = std::str::from_utf8(data).ok()?.trim();
        if data == "[DONE]" {
            continue;
        }
        let value = serde_json::from_str::<Value>(data).ok()?;
        if matches!(
            value.get("type").and_then(Value::as_str),
            Some("response.completed" | "response.incomplete")
        ) {
            terminal = Some(value);
        }
    }
    terminal
}

fn codex_terminal_to_openai_chat(response: &Value, requested_model: &str) -> Value {
    let mut content = String::new();
    let mut tool_calls = Vec::new();
    if let Some(output) = response.get("output").and_then(Value::as_array) {
        for item in output {
            match item.get("type").and_then(Value::as_str) {
                Some("message") => {
                    if let Some(parts) = item.get("content").and_then(Value::as_array) {
                        for part in parts {
                            if part.get("type").and_then(Value::as_str) == Some("output_text") {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    content.push_str(text);
                                }
                            }
                        }
                    }
                }
                Some("function_call") | Some("custom_tool_call") => tool_calls.push(json!({
                    "id": item.get("call_id").cloned().unwrap_or(Value::String("tool-call".to_string())),
                    "type": "function",
                    "function": {
                        "name": item.get("name").cloned().unwrap_or(Value::String("function".to_string())),
                        "arguments": item.get("arguments").or_else(|| item.get("input")).cloned().unwrap_or(Value::String("{}".to_string())),
                    }
                })),
                _ => {}
            }
        }
    }
    let mut message = json!({"role":"assistant","content": if content.is_empty() { Value::Null } else { Value::String(content) }});
    if !tool_calls.is_empty() {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    let finish_reason = if message.get("tool_calls").is_some() {
        "tool_calls"
    } else {
        "stop"
    };
    let mut output = json!({
        "id": response.get("id").cloned().unwrap_or(Value::String(format!("chatcmpl-{requested_model}"))),
        "object": "chat.completion",
        "created": response.get("created_at").cloned().unwrap_or_else(|| Value::Number(chrono::Utc::now().timestamp().into())),
        "model": response.get("model").cloned().unwrap_or_else(|| Value::String(requested_model.to_string())),
        "choices": [{"index":0,"message":message,"finish_reason":finish_reason}],
    });
    if let Some(usage) = response.get("usage") {
        output["usage"] = json!({
            "prompt_tokens": usage.get("input_tokens").cloned().unwrap_or(Value::Number(0.into())),
            "completion_tokens": usage.get("output_tokens").cloned().unwrap_or(Value::Number(0.into())),
            "total_tokens": usage.get("total_tokens").cloned().unwrap_or(Value::Number(0.into())),
        });
    }
    output
}

fn annotate_upstream(response: &mut Response, selection: &UpstreamSelection) {
    if let Ok(value) = HeaderValue::from_str(&selection.upstream_id) {
        response.headers_mut().insert("x-openmesh-upstream", value);
    }
    if let Some(account_id) = selection.account_id.as_deref() {
        if let Ok(value) = HeaderValue::from_str(account_id) {
            response
                .headers_mut()
                .insert("x-openmesh-account-id", value);
        }
    }
}

fn internal_headers(response: &Response) -> HeaderMap {
    let mut headers = HeaderMap::new();
    for name in ["x-openmesh-upstream", "x-openmesh-account-id"] {
        if let Some(value) = response.headers().get(name) {
            headers.insert(name, value.clone());
        }
    }
    headers
}

fn copy_internal_headers(response: &mut Response, headers: &HeaderMap) {
    for name in ["x-openmesh-upstream", "x-openmesh-account-id"] {
        if let Some(value) = headers.get(name) {
            response.headers_mut().insert(name, value.clone());
        }
    }
}

fn annotate_usage(response: &mut Response, payload: &Value) {
    if let Some(model) = payload.get("model").and_then(Value::as_str) {
        if let Ok(value) = HeaderValue::from_str(model) {
            response.headers_mut().insert("x-openmesh-model", value);
        }
    }
    let usage = payload
        .get("usage")
        .or_else(|| payload.get("usageMetadata"));
    let Some(usage) = usage.and_then(Value::as_object) else {
        return;
    };
    let input_tokens = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .or_else(|| usage.get("promptTokenCount"))
        .and_then(Value::as_u64);
    let output_tokens = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .or_else(|| usage.get("candidatesTokenCount"))
        .and_then(Value::as_u64);
    let total_tokens = usage
        .get("total_tokens")
        .or_else(|| usage.get("totalTokenCount"))
        .and_then(Value::as_u64)
        .or_else(|| {
            input_tokens
                .zip(output_tokens)
                .map(|(input, output)| input + output)
        });
    for (name, value) in [
        ("x-openmesh-input-tokens", input_tokens),
        ("x-openmesh-output-tokens", output_tokens),
        ("x-openmesh-total-tokens", total_tokens),
    ] {
        if let Some(value) = value.and_then(|value| HeaderValue::from_str(&value.to_string()).ok())
        {
            response.headers_mut().insert(name, value);
        }
    }
}

async fn execute_openai_chat_candidates(
    state: &ProxyServerState,
    selections: &[UpstreamSelection],
    payload: Value,
    stream: bool,
    request_id: &str,
) -> Result<Response, ProxyRouteError> {
    let max_attempts = state.config().max_retries.saturating_add(1) as usize;
    let mut last_error = ProxyRouteError::UpstreamUnavailable;
    for selection in selections.iter().take(max_attempts.max(1)) {
        let mut attempt_payload = payload.clone();
        if let Some(object) = attempt_payload.as_object_mut() {
            object.insert(
                "model".to_string(),
                Value::String(selection.resolved_model.clone()),
            );
        }
        match execute_openai_chat(state, selection, attempt_payload, stream, request_id).await {
            Ok(response) => return Ok(response),
            Err(error) if error.retryable() => {
                if matches!(error, ProxyRouteError::UpstreamStatus(429)) {
                    state.mark_rate_limited(&selection.upstream_id);
                }
                last_error = error;
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error)
}

fn messages_to_chat_payload(
    payload: &Value,
    resolved_model: &str,
    request_id: &str,
) -> Result<Value, Response> {
    let object = payload.as_object().ok_or_else(|| {
        ProxyRouteError::InvalidRequest("messages request body must be a JSON object")
            .response(request_id)
    })?;
    let messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProxyRouteError::InvalidRequest("messages must be an array").response(request_id)
        })?;
    if messages.is_empty() {
        return Err(
            ProxyRouteError::InvalidRequest("messages must not be empty").response(request_id),
        );
    }
    let mut chat_messages = Vec::with_capacity(messages.len() + 1);
    if let Some(system) = object.get("system") {
        if let Some(text) = message_content_text(Some(system)) {
            chat_messages.push(json!({ "role": "system", "content": text }));
        }
    }
    chat_messages.extend(messages.iter().cloned());
    let mut chat = Map::new();
    chat.insert(
        "model".to_string(),
        Value::String(resolved_model.to_string()),
    );
    chat.insert("messages".to_string(), Value::Array(chat_messages));
    for (source, target) in [
        ("max_tokens", "max_tokens"),
        ("temperature", "temperature"),
        ("top_p", "top_p"),
        ("tools", "tools"),
        ("tool_choice", "tool_choice"),
        ("stop_sequences", "stop"),
    ] {
        if let Some(value) = object.get(source) {
            chat.insert(target.to_string(), value.clone());
        }
    }
    chat.insert("stream".to_string(), Value::Bool(false));
    Ok(Value::Object(chat))
}

fn data_url_parts(url: &str) -> Option<(&str, &str)> {
    let (metadata, data) = url.strip_prefix("data:")?.split_once(',')?;
    let media_type = metadata.split(';').next()?.trim();
    metadata
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"))
        .then_some((media_type, data))
}

fn openai_image_url_to_anthropic(url: &str) -> Result<Value, ProxyRouteError> {
    if let Some((media_type, encoded)) = data_url_parts(url) {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| ProxyRouteError::InvalidRequest("image data URL is not valid base64"))?;
        return Ok(json!({
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": media_type,
                "data": base64::engine::general_purpose::STANDARD.encode(decoded),
            }
        }));
    }
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| ProxyRouteError::InvalidRequest("image URL must be a valid HTTP(S) URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ProxyRouteError::InvalidRequest(
            "image URL must be a valid HTTP(S) URL",
        ));
    }
    Ok(json!({
        "type": "image",
        "source": { "type": "url", "url": url }
    }))
}

fn openai_content_to_anthropic(value: &Value) -> Result<Value, ProxyRouteError> {
    let Some(parts) = value.as_array() else {
        return Ok(value.clone());
    };
    let mut blocks = Vec::with_capacity(parts.len());
    for part in parts {
        let object = part.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "multimodal content parts must be objects",
        ))?;
        match object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "text" => {
                let text = object.get("text").and_then(Value::as_str).ok_or(
                    ProxyRouteError::InvalidRequest("text content parts must contain text"),
                )?;
                blocks.push(json!({ "type": "text", "text": text }));
            }
            "image_url" => {
                let url = object
                    .get("image_url")
                    .and_then(|image| image.get("url"))
                    .and_then(Value::as_str)
                    .ok_or(ProxyRouteError::InvalidRequest(
                        "image content parts must contain image_url.url",
                    ))?;
                blocks.push(openai_image_url_to_anthropic(url)?);
            }
            _ => {
                return Err(ProxyRouteError::InvalidRequest(
                    "unsupported OpenAI content part",
                ))
            }
        }
    }
    Ok(Value::Array(blocks))
}

fn openai_tools_to_anthropic(value: Option<&Value>) -> Result<Option<Value>, ProxyRouteError> {
    let Some(tools) = value else { return Ok(None) };
    let tools = tools
        .as_array()
        .ok_or(ProxyRouteError::InvalidRequest("tools must be an array"))?;
    let mut translated = Vec::with_capacity(tools.len());
    for tool in tools {
        let object = tool.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "tool definitions must be objects",
        ))?;
        let function = object.get("function").unwrap_or(tool);
        let function = function.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "tool function definitions must be objects",
        ))?;
        let name = function
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .ok_or(ProxyRouteError::InvalidRequest(
                "tool function name is required",
            ))?;
        translated.push(json!({
            "name": name,
            "description": function.get("description").cloned().unwrap_or_else(|| json!("")),
            "input_schema": function.get("parameters").cloned().unwrap_or_else(|| json!({
                "type": "object",
                "properties": {}
            })),
        }));
    }
    Ok(Some(Value::Array(translated)))
}

fn openai_tool_choice_to_anthropic(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    match value {
        Value::String(choice) => Some(match choice.as_str() {
            "none" => json!("none"),
            "required" => json!("any"),
            _ => json!("auto"),
        }),
        Value::Object(object) => object
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
            .map(|name| json!({ "type": "tool", "name": name })),
        _ => None,
    }
}

fn openai_to_anthropic_payload(payload: &Value, model: &str) -> Result<Value, ProxyRouteError> {
    let object = payload.as_object().ok_or(ProxyRouteError::InvalidRequest(
        "request body must be a JSON object",
    ))?;
    let source_messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or(ProxyRouteError::InvalidRequest("messages must be an array"))?;
    let mut messages = Vec::new();
    let mut system_parts = Vec::new();
    for message in source_messages {
        let Some(message_object) = message.as_object() else {
            return Err(ProxyRouteError::InvalidRequest(
                "messages must contain objects",
            ));
        };
        let role = message_object
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user");
        if role == "system" {
            let content = message_object
                .get("content")
                .and_then(|value| message_content_text(Some(value)))
                .ok_or(ProxyRouteError::InvalidRequest(
                    "system content must contain text",
                ))?;
            system_parts.push(content);
        } else if role == "tool" {
            let tool_use_id = message_object
                .get("tool_call_id")
                .and_then(Value::as_str)
                .ok_or(ProxyRouteError::InvalidRequest(
                    "tool messages must contain tool_call_id",
                ))?;
            let content = message_object
                .get("content")
                .and_then(|value| message_content_text(Some(value)))
                .unwrap_or_default();
            messages.push(json!({
                "role": "user",
                "content": [{
                    "type": "tool_result",
                    "tool_use_id": tool_use_id,
                    "content": content,
                }]
            }));
        } else {
            let role = if role == "assistant" {
                "assistant"
            } else {
                "user"
            };
            let mut content_blocks = Vec::new();
            if let Some(content) = message_object.get("content") {
                match openai_content_to_anthropic(content)? {
                    Value::Array(blocks) => content_blocks.extend(blocks),
                    value => content_blocks.push(json!({ "type": "text", "text": value })),
                }
            }
            if let Some(tool_calls) = message_object.get("tool_calls").and_then(Value::as_array) {
                for tool_call in tool_calls {
                    let function = tool_call.get("function").and_then(Value::as_object).ok_or(
                        ProxyRouteError::InvalidRequest("tool calls must contain a function"),
                    )?;
                    let name = function
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .ok_or(ProxyRouteError::InvalidRequest(
                            "tool call function name is required",
                        ))?;
                    let input = function
                        .get("arguments")
                        .and_then(Value::as_str)
                        .and_then(|arguments| serde_json::from_str::<Value>(arguments).ok())
                        .unwrap_or_else(|| json!({}));
                    content_blocks.push(json!({
                        "type": "tool_use",
                        "id": tool_call.get("id").and_then(Value::as_str).unwrap_or("call_openmesh"),
                        "name": name,
                        "input": input,
                    }));
                }
            }
            if content_blocks.is_empty() {
                return Err(ProxyRouteError::InvalidRequest(
                    "message content must contain text, image, or tool content",
                ));
            }
            messages.push(json!({ "role": role, "content": content_blocks }));
        }
    }
    if messages.is_empty() {
        return Err(ProxyRouteError::InvalidRequest(
            "messages must contain a user or assistant turn",
        ));
    }
    let mut anthropic = json!({
        "model": model,
        "max_tokens": object.get("max_tokens").cloned().unwrap_or_else(|| json!(1024)),
        "messages": messages,
    });
    if !system_parts.is_empty() {
        anthropic["system"] = Value::String(system_parts.join("\n\n"));
    }
    for (source, target) in [("temperature", "temperature"), ("top_p", "top_p")] {
        if let Some(value) = object.get(source) {
            anthropic[target] = value.clone();
        }
    }
    if let Some(tools) = openai_tools_to_anthropic(object.get("tools"))? {
        anthropic["tools"] = tools;
    }
    if let Some(tool_choice) = openai_tool_choice_to_anthropic(object.get("tool_choice")) {
        anthropic["tool_choice"] = tool_choice;
    }
    Ok(anthropic)
}

fn openai_image_url_to_gemini_part(url: &str) -> Result<Value, ProxyRouteError> {
    if let Some((media_type, encoded)) = data_url_parts(url) {
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| ProxyRouteError::InvalidRequest("image data URL is not valid base64"))?;
        return Ok(json!({
            "inlineData": {
                "mimeType": media_type,
                "data": base64::engine::general_purpose::STANDARD.encode(decoded),
            }
        }));
    }
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| ProxyRouteError::InvalidRequest("image URL must be a valid HTTP(S) URL"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(ProxyRouteError::InvalidRequest(
            "image URL must be a valid HTTP(S) URL",
        ));
    }
    Ok(json!({ "fileData": { "fileUri": url } }))
}

fn openai_content_to_gemini_parts(value: &Value) -> Result<Vec<Value>, ProxyRouteError> {
    let Some(parts) = value.as_array() else {
        return Ok(vec![json!({ "text": value.as_str().unwrap_or_default() })]);
    };
    let mut translated = Vec::with_capacity(parts.len());
    for part in parts {
        let object = part.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "multimodal content parts must be objects",
        ))?;
        match object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default()
        {
            "text" => translated.push(json!({
                "text": object.get("text").and_then(Value::as_str).ok_or(
                    ProxyRouteError::InvalidRequest("text content parts must contain text"),
                )?
            })),
            "image_url" => {
                let url = object
                    .get("image_url")
                    .and_then(|image| image.get("url"))
                    .and_then(Value::as_str)
                    .ok_or(ProxyRouteError::InvalidRequest(
                        "image content parts must contain image_url.url",
                    ))?;
                translated.push(openai_image_url_to_gemini_part(url)?);
            }
            _ => {
                return Err(ProxyRouteError::InvalidRequest(
                    "unsupported OpenAI content part",
                ))
            }
        }
    }
    Ok(translated)
}

fn openai_tools_to_gemini(value: Option<&Value>) -> Result<Option<Value>, ProxyRouteError> {
    let Some(tools) = value else { return Ok(None) };
    let tools = tools
        .as_array()
        .ok_or(ProxyRouteError::InvalidRequest("tools must be an array"))?;
    let mut declarations = Vec::with_capacity(tools.len());
    for tool in tools {
        let object = tool.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "tool definitions must be objects",
        ))?;
        let function = object.get("function").unwrap_or(tool);
        let function = function.as_object().ok_or(ProxyRouteError::InvalidRequest(
            "tool function definitions must be objects",
        ))?;
        let name = function
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.trim().is_empty())
            .ok_or(ProxyRouteError::InvalidRequest(
                "tool function name is required",
            ))?;
        declarations.push(json!({
            "name": name,
            "description": function.get("description").cloned().unwrap_or_else(|| json!("")),
            "parameters": function.get("parameters").cloned().unwrap_or_else(|| json!({
                "type": "object",
                "properties": {}
            })),
        }));
    }
    Ok(Some(json!([{ "functionDeclarations": declarations }])))
}

fn openai_tool_choice_to_gemini(value: Option<&Value>) -> Option<Value> {
    let value = value?;
    let mode = match value {
        Value::String(choice) => match choice.as_str() {
            "none" => "NONE",
            "required" => "ANY",
            _ => "AUTO",
        },
        Value::Object(_) => "ANY",
        _ => return None,
    };
    let mut calling = Map::new();
    calling.insert("mode".to_string(), Value::String(mode.to_string()));
    if let Value::Object(object) = value {
        if let Some(name) = object
            .get("function")
            .and_then(|function| function.get("name"))
            .and_then(Value::as_str)
        {
            calling.insert("allowedFunctionNames".to_string(), json!([name]));
        }
    }
    Some(Value::Object(Map::from_iter([(
        "functionCallingConfig".to_string(),
        Value::Object(calling),
    )])))
}

fn openai_to_gemini_payload(payload: &Value, _model: &str) -> Result<Value, ProxyRouteError> {
    let object = payload.as_object().ok_or(ProxyRouteError::InvalidRequest(
        "request body must be a JSON object",
    ))?;
    let source_messages = object
        .get("messages")
        .and_then(Value::as_array)
        .ok_or(ProxyRouteError::InvalidRequest("messages must be an array"))?;
    let mut contents = Vec::new();
    let mut system_parts = Vec::new();
    for message in source_messages {
        let Some(message_object) = message.as_object() else {
            return Err(ProxyRouteError::InvalidRequest(
                "messages must contain objects",
            ));
        };
        let role = message_object
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or("user");
        if role == "system" {
            let text = message_object
                .get("content")
                .and_then(|value| message_content_text(Some(value)))
                .ok_or(ProxyRouteError::InvalidRequest(
                    "system content must contain text",
                ))?;
            system_parts.push(text);
        } else {
            let mut parts = Vec::new();
            if let Some(content) = message_object.get("content") {
                parts.extend(openai_content_to_gemini_parts(content)?);
            }
            if let Some(tool_calls) = message_object.get("tool_calls").and_then(Value::as_array) {
                for tool_call in tool_calls {
                    let function = tool_call.get("function").and_then(Value::as_object).ok_or(
                        ProxyRouteError::InvalidRequest("tool calls must contain a function"),
                    )?;
                    let name = function
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.trim().is_empty())
                        .ok_or(ProxyRouteError::InvalidRequest(
                            "tool call function name is required",
                        ))?;
                    let args = function
                        .get("arguments")
                        .and_then(Value::as_str)
                        .and_then(|arguments| serde_json::from_str::<Value>(arguments).ok())
                        .unwrap_or_else(|| json!({}));
                    parts.push(json!({ "functionCall": { "name": name, "args": args } }));
                }
            }
            if role == "tool" {
                let name = message_object
                    .get("name")
                    .and_then(Value::as_str)
                    .or_else(|| message_object.get("tool_call_id").and_then(Value::as_str))
                    .unwrap_or("tool");
                let response = message_object
                    .get("content")
                    .and_then(|value| message_content_text(Some(value)))
                    .unwrap_or_default();
                parts = vec![json!({
                    "functionResponse": { "name": name, "response": { "content": response } }
                })];
            }
            if parts.is_empty() {
                return Err(ProxyRouteError::InvalidRequest(
                    "message content must contain text, image, or tool content",
                ));
            }
            contents.push(json!({
                "role": if role == "assistant" { "model" } else { "user" },
                "parts": parts,
            }));
        }
    }
    if contents.is_empty() {
        return Err(ProxyRouteError::InvalidRequest(
            "messages must contain a user or assistant turn",
        ));
    }
    let mut result = json!({ "contents": contents });
    if !system_parts.is_empty() {
        result["systemInstruction"] = json!({ "parts": [{ "text": system_parts.join("\n\n") }] });
    }
    let mut generation = Map::new();
    for (source, target) in [
        ("max_tokens", "maxOutputTokens"),
        ("temperature", "temperature"),
        ("top_p", "topP"),
    ] {
        if let Some(value) = object.get(source) {
            generation.insert(target.to_string(), value.clone());
        }
    }
    if let Some(stop) = object.get("stop") {
        generation.insert("stopSequences".to_string(), stop.clone());
    }
    if !generation.is_empty() {
        result["generationConfig"] = Value::Object(generation);
    }
    if let Some(tools) = openai_tools_to_gemini(object.get("tools"))? {
        result["tools"] = tools;
    }
    if let Some(tool_config) = openai_tool_choice_to_gemini(object.get("tool_choice")) {
        result["toolConfig"] = tool_config;
    }
    Ok(result)
}

fn gemini_generate_content_path(model: &str) -> Result<String, ProxyRouteError> {
    if model.is_empty()
        || model.bytes().any(|byte| {
            !(byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
        })
    {
        return Err(ProxyRouteError::InvalidRequest(
            "Gemini model id contains unsafe path characters",
        ));
    }
    Ok(format!("models/{model}:generateContent"))
}

fn provider_to_openai_response(
    bytes: &Bytes,
    protocol: ProxyProviderProtocol,
    model: &str,
) -> Result<Value, ProxyRouteError> {
    let upstream: Value =
        serde_json::from_slice(bytes).map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
    let (text, finish_reason, usage, tool_calls) = match protocol {
        ProxyProviderProtocol::Anthropic => {
            let parts = upstream
                .get("content")
                .and_then(Value::as_array)
                .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>();
            let tool_calls = parts
                .iter()
                .filter(|part| part.get("type").and_then(Value::as_str) == Some("tool_use"))
                .enumerate()
                .filter_map(|(index, part)| {
                    Some(json!({
                        "id": part.get("id").and_then(Value::as_str).unwrap_or_else(|| {
                            if index == 0 { "call_openmesh" } else { "call_openmesh_extra" }
                        }),
                        "type": "function",
                        "function": {
                            "name": part.get("name")?.as_str()?,
                            "arguments": serde_json::to_string(part.get("input")?).ok()?,
                        }
                    }))
                })
                .collect::<Vec<_>>();
            if text.is_empty() && tool_calls.is_empty() {
                return Err(ProxyRouteError::UpstreamInvalidResponse);
            }
            let usage = upstream.get("usage").map(|value| {
                json!({
                    "prompt_tokens": value.get("input_tokens").and_then(Value::as_u64).unwrap_or(0),
                    "completion_tokens": value.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
                    "total_tokens": value.get("input_tokens").and_then(Value::as_u64).unwrap_or(0)
                        + value.get("output_tokens").and_then(Value::as_u64).unwrap_or(0),
                })
            });
            (
                text,
                if tool_calls.is_empty() {
                    "stop"
                } else {
                    "tool_calls"
                },
                usage,
                (!tool_calls.is_empty()).then_some(tool_calls),
            )
        }
        ProxyProviderProtocol::Gemini => {
            let candidate = upstream
                .get("candidates")
                .and_then(Value::as_array)
                .and_then(|candidates| candidates.first())
                .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
            let parts = candidate
                .get("content")
                .and_then(|content| content.get("parts"))
                .and_then(Value::as_array)
                .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
            let text = parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>();
            let tool_calls = parts
                .iter()
                .filter_map(|part| part.get("functionCall"))
                .enumerate()
                .filter_map(|(index, call)| {
                    Some(json!({
                        "id": format!("call_openmesh_{index}"),
                        "type": "function",
                        "function": {
                            "name": call.get("name")?.as_str()?,
                            "arguments": serde_json::to_string(call.get("args")?).ok()?,
                        }
                    }))
                })
                .collect::<Vec<_>>();
            if text.is_empty() && tool_calls.is_empty() {
                return Err(ProxyRouteError::UpstreamInvalidResponse);
            }
            let usage = upstream.get("usageMetadata").map(|value| {
                json!({
                    "prompt_tokens": value.get("promptTokenCount").and_then(Value::as_u64).unwrap_or(0),
                    "completion_tokens": value.get("candidatesTokenCount").and_then(Value::as_u64).unwrap_or(0),
                    "total_tokens": value.get("totalTokenCount").and_then(Value::as_u64).unwrap_or(0),
                })
            });
            (
                text,
                if tool_calls.is_empty() {
                    "stop"
                } else {
                    "tool_calls"
                },
                usage,
                (!tool_calls.is_empty()).then_some(tool_calls),
            )
        }
        ProxyProviderProtocol::OpenAiCompatible => {
            return Err(ProxyRouteError::UpstreamInvalidResponse)
        }
    };
    let mut message = json!({
        "role": "assistant",
        "content": if text.is_empty() { Value::Null } else { Value::String(text) },
    });
    if let Some(tool_calls) = tool_calls {
        message["tool_calls"] = Value::Array(tool_calls);
    }
    let mut response = json!({
        "id": "chatcmpl-openmesh",
        "object": "chat.completion",
        "created": Utc::now().timestamp(),
        "model": model,
        "choices": [{
            "index": 0,
            "message": message,
            "finish_reason": finish_reason,
        }]
    });
    if let Some(usage) = usage {
        response["usage"] = usage;
    }
    Ok(response)
}

fn translate_openai_response_to_anthropic(
    bytes: &Bytes,
    model: &str,
    request_id: &str,
) -> Result<Value, ProxyRouteError> {
    let upstream: Value =
        serde_json::from_slice(bytes).map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
    let choice = upstream
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let message = choice
        .get("message")
        .and_then(Value::as_object)
        .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let mut content = Vec::new();
    if let Some(text) = message_content_text(message.get("content")) {
        content.push(json!({ "type": "text", "text": text }));
    }
    if let Some(tool_calls) = message.get("tool_calls").and_then(Value::as_array) {
        for tool_call in tool_calls {
            let function = tool_call
                .get("function")
                .and_then(Value::as_object)
                .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
            let input = function
                .get("arguments")
                .and_then(Value::as_str)
                .and_then(|arguments| serde_json::from_str::<Value>(arguments).ok())
                .unwrap_or_else(|| json!({}));
            content.push(json!({
                "type": "tool_use",
                "id": tool_call.get("id").and_then(Value::as_str).unwrap_or("call_openmesh"),
                "name": name,
                "input": input,
            }));
        }
    }
    if content.is_empty() {
        return Err(ProxyRouteError::UpstreamInvalidResponse);
    }
    let mut usage = json!({});
    if let Some(source) = upstream.get("usage") {
        usage["input_tokens"] = source
            .get("prompt_tokens")
            .cloned()
            .unwrap_or_else(|| json!(0));
        usage["output_tokens"] = source
            .get("completion_tokens")
            .cloned()
            .unwrap_or_else(|| json!(0));
    }
    Ok(json!({
        "id": format!("msg_{request_id}"),
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": if message.get("tool_calls").and_then(Value::as_array).is_some_and(|calls| !calls.is_empty()) { "tool_use" } else { "end_turn" },
        "stop_sequence": null,
        "usage": usage,
    }))
}

fn openai_sse_response(payload: &Value, request_id: &str) -> Response {
    let message = &payload["choices"][0]["message"];
    let text = message["content"].as_str().unwrap_or("");
    let model = payload.get("model").and_then(Value::as_str).unwrap_or("");
    let tool_calls = message.get("tool_calls").cloned();
    let mut delta = json!({ "role": "assistant" });
    if !text.is_empty() {
        delta["content"] = Value::String(text.to_string());
    }
    if let Some(tool_calls) = tool_calls.clone() {
        delta["tool_calls"] = tool_calls;
    }
    let first = json!({
        "id": format!("chatcmpl_{request_id}"),
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{ "index": 0, "delta": delta, "finish_reason": null }]
    });
    let last = json!({
        "id": format!("chatcmpl_{request_id}"),
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{ "index": 0, "delta": {}, "finish_reason": if tool_calls.is_some() { "tool_calls" } else { "stop" } }]
    });
    sse_response(&[first, last], request_id)
}

fn anthropic_sse_response(payload: &Value, request_id: &str) -> Response {
    let model = payload.get("model").and_then(Value::as_str).unwrap_or("");
    let mut events = vec![json!({
        "type": "message_start",
        "message": {
            "id": payload["id"],
            "type": "message",
            "role": "assistant",
            "model": model,
            "content": [],
            "stop_reason": null,
            "stop_sequence": null,
            "usage": { "input_tokens": 0, "output_tokens": 0 }
        }
    })];
    if let Some(content) = payload.get("content").and_then(Value::as_array) {
        for (index, block) in content.iter().enumerate() {
            match block.get("type").and_then(Value::as_str) {
                Some("tool_use") => {
                    events.push(json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": {
                            "type": "tool_use",
                            "id": block.get("id"),
                            "name": block.get("name"),
                            "input": {}
                        }
                    }));
                    events.push(json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": {
                            "type": "input_json_delta",
                            "partial_json": serde_json::to_string(block.get("input").unwrap_or(&Value::Object(Map::new()))).unwrap_or_else(|_| "{}".to_string())
                        }
                    }));
                }
                _ => {
                    events.push(json!({
                        "type": "content_block_start",
                        "index": index,
                        "content_block": { "type": "text", "text": "" }
                    }));
                    events.push(json!({
                        "type": "content_block_delta",
                        "index": index,
                        "delta": { "type": "text_delta", "text": block.get("text").and_then(Value::as_str).unwrap_or("") }
                    }));
                }
            }
            events.push(json!({ "type": "content_block_stop", "index": index }));
        }
    }
    events.push(json!({
        "type": "message_delta",
        "delta": { "stop_reason": payload.get("stop_reason").cloned().unwrap_or_else(|| json!("end_turn")), "stop_sequence": null },
        "usage": payload["usage"]
    }));
    events.push(json!({ "type": "message_stop" }));
    sse_response(&events, request_id)
}

fn sse_response(events: &[Value], request_id: &str) -> Response {
    let mut body = String::new();
    for event in events {
        body.push_str("data: ");
        body.push_str(&event.to_string());
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("request id is validated"),
    );
    response
}

fn responses_to_chat_payload(
    payload: &Value,
    resolved_model: &str,
    request_id: &str,
) -> Result<Value, Response> {
    let object = payload.as_object().ok_or_else(|| {
        ProxyRouteError::InvalidRequest("request body must be a JSON object").response(request_id)
    })?;
    let mut chat = Map::new();
    chat.insert(
        "model".to_string(),
        Value::String(resolved_model.to_string()),
    );

    let mut messages = Vec::new();
    if let Some(instructions) = object.get("instructions").and_then(Value::as_str) {
        if !instructions.trim().is_empty() {
            messages.push(json!({ "role": "system", "content": instructions }));
        }
    }
    if let Some(input) = object.get("input") {
        match input {
            Value::String(text) => messages.push(json!({ "role": "user", "content": text })),
            Value::Array(items) => {
                for item in items {
                    if let Some(message) = response_input_item_to_chat(item) {
                        messages.push(message);
                    } else {
                        return Err(ProxyRouteError::InvalidRequest(
                            "responses input items must contain a role and content",
                        )
                        .response(request_id));
                    }
                }
            }
            _ => {
                return Err(ProxyRouteError::InvalidRequest(
                    "responses input must be a string or array",
                )
                .response(request_id))
            }
        }
    } else if let Some(existing) = object.get("messages") {
        messages = existing.as_array().cloned().ok_or_else(|| {
            ProxyRouteError::InvalidRequest("messages must be an array").response(request_id)
        })?;
    } else {
        return Err(
            ProxyRouteError::InvalidRequest("responses input is required").response(request_id),
        );
    }
    chat.insert("messages".to_string(), Value::Array(messages));

    for key in [
        "temperature",
        "top_p",
        "max_output_tokens",
        "tools",
        "tool_choice",
        "parallel_tool_calls",
        "stop",
    ] {
        if let Some(value) = object.get(key) {
            let target = if key == "max_output_tokens" {
                "max_tokens"
            } else {
                key
            };
            chat.insert(target.to_string(), value.clone());
        }
    }
    chat.insert("stream".to_string(), Value::Bool(false));
    Ok(Value::Object(chat))
}

fn response_input_item_to_chat(item: &Value) -> Option<Value> {
    let object = item.as_object()?;
    let role = object.get("role")?.as_str()?;
    let content = object.get("content")?;
    if content.is_string() {
        return Some(json!({ "role": role, "content": content }));
    }
    let content_array = content.as_array()?;
    let mut parts = Vec::new();
    for part in content_array {
        let part_type = part.get("type").and_then(Value::as_str).unwrap_or_default();
        if matches!(part_type, "input_text" | "output_text" | "text") {
            if let Some(value) = part.get("text").and_then(Value::as_str) {
                parts.push(json!({ "type": "text", "text": value }));
            }
        } else if matches!(part_type, "input_image" | "image_url" | "image") {
            let url = part
                .get("image_url")
                .and_then(|value| {
                    value
                        .as_str()
                        .or_else(|| value.get("url").and_then(Value::as_str))
                })
                .or_else(|| part.get("url").and_then(Value::as_str))?;
            parts.push(json!({ "type": "image_url", "image_url": { "url": url } }));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(json!({ "role": role, "content": parts }))
    }
}

fn translate_chat_response(
    bytes: &Bytes,
    requested_model: &str,
    request_id: &str,
) -> Result<Value, ProxyRouteError> {
    let upstream: Value =
        serde_json::from_slice(bytes).map_err(|_| ProxyRouteError::UpstreamInvalidResponse)?;
    let choice = upstream
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let message = choice
        .get("message")
        .and_then(Value::as_object)
        .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let text = message_content_text(message.get("content"))
        .ok_or(ProxyRouteError::UpstreamInvalidResponse)?;
    let response_id = format!("resp_{request_id}");
    let message_id = format!("msg_{request_id}");
    let mut response = json!({
        "id": response_id,
        "object": "response",
        "created_at": Utc::now().timestamp(),
        "status": "completed",
        "model": requested_model,
        "output": [{
            "id": message_id,
            "type": "message",
            "status": "completed",
            "role": "assistant",
            "content": [{
                "type": "output_text",
                "text": text,
                "annotations": []
            }]
        }],
        "output_text": text,
    });
    if let Some(usage) = upstream.get("usage") {
        response["usage"] = usage.clone();
    }
    Ok(response)
}

fn message_content_text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.clone()),
        Value::Array(parts) => {
            let mut text = String::new();
            for part in parts {
                if let Some(value) = part.get("text").and_then(Value::as_str) {
                    text.push_str(value);
                }
            }
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn responses_sse_response(payload: &Value, request_id: &str) -> Response {
    let response_id = payload
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("response");
    let model = payload.get("model").and_then(Value::as_str).unwrap_or("");
    let item_id = payload["output"]
        .as_array()
        .and_then(|items| items.first())
        .and_then(|item| item.get("id"))
        .and_then(Value::as_str)
        .unwrap_or("message");
    let text = payload
        .get("output_text")
        .and_then(Value::as_str)
        .unwrap_or("");
    let created = json!({
        "type": "response.created",
        "response": {
            "id": response_id,
            "object": "response",
            "status": "in_progress",
            "model": model,
        }
    });
    let delta = json!({
        "type": "response.output_text.delta",
        "item_id": item_id,
        "output_index": 0,
        "content_index": 0,
        "delta": text,
    });
    let completed = json!({ "type": "response.completed", "response": payload });
    let mut body = String::new();
    for event in [created, delta, completed] {
        body.push_str("data: ");
        body.push_str(&event.to_string());
        body.push_str("\n\n");
    }
    body.push_str("data: [DONE]\n\n");
    let mut response = Response::new(Body::from(body));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("request id is validated"),
    );
    response
}

fn select_upstreams(
    state: &ProxyServerState,
    requested_model: &str,
    request_id: &str,
) -> Result<Vec<UpstreamSelection>, Response> {
    let config = state.config();
    let candidate_models = model_candidates(&config, requested_model).map_err(|_| {
        ProxyRouteError::InvalidRequest("model alias or fallback resolution exceeded its limit")
            .response(request_id)
    })?;

    let mut selections = Vec::new();
    for (candidate_index, resolved_model) in candidate_models.into_iter().enumerate() {
        let mut matching = config
            .upstreams
            .iter()
            .filter(|upstream| {
                upstream.enabled
                    && !state.is_rate_limited(&upstream.id)
                    && (upstream.models.is_empty()
                        || upstream
                            .models
                            .iter()
                            .any(|model| model.id == resolved_model))
            })
            .collect::<Vec<_>>();
        if matching.is_empty() {
            continue;
        }
        // Priority is meaningful for every strategy. In particular, the
        // management account-activation endpoint raises an account's
        // priority and must affect the default first-compatible strategy as
        // well as fill-first and round-robin. `sort_by_key` is stable, so
        // equal priorities retain their configured order.
        matching.sort_by_key(|upstream| upstream.priority);
        if candidate_index == 0 && config.routing_strategy == ProxyRoutingStrategy::RoundRobin {
            let offset = state.next_route_index(matching.len());
            matching.rotate_left(offset);
        }
        selections.extend(
            matching
                .into_iter()
                .map(|upstream| UpstreamSelection {
                    upstream_id: upstream.id.clone(),
                    account_id: upstream.account_id.clone(),
                    base_url: super::ProxyServerConfig::normalized_upstream_url(upstream),
                    api_key: upstream.api_key.clone(),
                    oauth_provider: canonical_oauth_provider(upstream.oauth_provider.as_deref()),
                    protocol: upstream.protocol,
                    resolved_model: resolved_model.clone(),
                })
                .collect::<Vec<_>>(),
        );
    }

    if selections.is_empty() {
        Err(ProxyRouteError::ModelUnavailable.response(request_id))
    } else {
        Ok(selections)
    }
}

fn canonical_oauth_provider(value: Option<&str>) -> Option<String> {
    value
        .and_then(|value| value.parse::<crate::oauth::OAuthProviderId>().ok())
        .map(|provider| provider.to_string())
}

fn uses_native_codex_responses(selections: &[UpstreamSelection]) -> bool {
    !selections.is_empty()
        && selections
            .iter()
            .all(|selection| selection.oauth_provider.as_deref() == Some("codex"))
}

fn management_base_endpoint(config: &super::ProxyServerConfig) -> String {
    format!("http://{}:{}/v0/management", config.bind_host, config.port)
}

fn data_plane_base_endpoint(config: &super::ProxyServerConfig) -> String {
    format!("http://{}:{}/v1", config.bind_host, config.port)
}

fn resolve_model_in_config(
    config: &super::ProxyServerConfig,
    requested: &str,
) -> Result<String, ()> {
    let mut current = requested.to_string();
    for _ in 0..8 {
        match config.model_aliases.get(&current) {
            Some(next) if next != &current => current = next.clone(),
            Some(_) => return Err(()),
            None => return Ok(current),
        }
    }
    Err(())
}

fn model_candidates(config: &super::ProxyServerConfig, requested: &str) -> Result<Vec<String>, ()> {
    let resolved = resolve_model_in_config(config, requested)?;
    let mut candidates = vec![resolved.clone()];
    for key in [requested, resolved.as_str()] {
        let Some(fallbacks) = config.model_fallbacks.get(key) else {
            continue;
        };
        for fallback in fallbacks {
            let fallback = resolve_model_in_config(config, fallback)?;
            if !candidates.contains(&fallback) {
                candidates.push(fallback);
            }
            if candidates.len() > 9 {
                return Err(());
            }
        }
    }
    Ok(candidates)
}

fn parse_json_request(body: &Bytes, request_id: &str) -> Result<Value, Response> {
    if body.len() > MAX_PROXY_BODY_BYTES {
        return Err(ProxyRouteError::PayloadTooLarge.response(request_id));
    }
    serde_json::from_slice(body).map_err(|_| {
        ProxyRouteError::InvalidRequest("request body must be valid JSON").response(request_id)
    })
}

fn required_model(payload: &Value, request_id: &str) -> Result<String, Response> {
    payload
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|model| !model.is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| ProxyRouteError::InvalidRequest("model is required").response(request_id))
}

fn request_id(headers: &HeaderMap, state: &ProxyServerState) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 128
                && value.bytes().all(|byte| !byte.is_ascii_control())
        })
        .map(ToString::to_string)
        .unwrap_or_else(|| state.next_request_id())
}

fn require_auth(
    state: &ProxyServerState,
    headers: &HeaderMap,
    request_id: &str,
) -> Option<Response> {
    if state.config().allow_unauthenticated {
        return None;
    }
    let supplied = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let (scheme, token) = value.split_once(' ')?;
            scheme
                .eq_ignore_ascii_case("bearer")
                .then_some(token.trim())
        })
        .or_else(|| {
            headers
                .get("x-api-key")
                .and_then(|value| value.to_str().ok())
                .map(str::trim)
        });
    let authorized = supplied.is_some_and(|candidate| {
        state
            .config()
            .api_keys
            .iter()
            .any(|configured| constant_time_equal(candidate.as_bytes(), configured.as_bytes()))
    });
    if authorized {
        None
    } else {
        Some(ProxyRouteError::Unauthorized.response(request_id))
    }
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= (left.get(index).copied().unwrap_or_default()
            ^ right.get(index).copied().unwrap_or_default()) as usize;
    }
    difference == 0
}

fn json_response(status: StatusCode, payload: &Value, request_id: &str) -> Response {
    let bytes = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).expect("request id is validated"),
    );
    response
}

#[derive(Debug, Clone, Copy)]
enum ProxyRouteError {
    Unauthorized,
    RestartRequired,
    ConfigurationPersistence,
    PayloadTooLarge,
    InvalidRequest(&'static str),
    ModelUnavailable,
    UpstreamUnavailable,
    UpstreamInvalidResponse,
    OAuthCredentialUnavailable,
    ProviderNotFound,
    AccountNotFound,
    UpstreamStatus(u16),
}

impl ProxyRouteError {
    fn retryable(self) -> bool {
        match self {
            Self::UpstreamUnavailable | Self::UpstreamInvalidResponse => true,
            Self::UpstreamStatus(status) => matches!(status, 401 | 403 | 429) || status >= 500,
            _ => false,
        }
    }

    fn response(self, request_id: &str) -> Response {
        let (status, error_type, code, message) = match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "authentication_error",
                "invalid_api_key",
                "OpenMesh API key is missing or invalid",
            ),
            Self::RestartRequired => (
                StatusCode::CONFLICT,
                "configuration_error",
                "restart_required",
                "bindHost, port, or requestTimeoutSecs changes require restarting the OpenMesh proxy",
            ),
            Self::ConfigurationPersistence => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "configuration_error",
                "persistence_failed",
                "OpenMesh could not persist the proxy configuration",
            ),
            Self::PayloadTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "invalid_request_error",
                "body_too_large",
                "request body exceeds the OpenMesh proxy limit",
            ),
            Self::InvalidRequest(message) => (
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
                "invalid_request",
                message,
            ),
            Self::ModelUnavailable => (
                StatusCode::NOT_FOUND,
                "invalid_request_error",
                "model_not_found",
                "requested model is not configured in OpenMesh",
            ),
            Self::UpstreamUnavailable => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                "upstream_unavailable",
                "configured upstream could not be reached",
            ),
            Self::UpstreamInvalidResponse => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                "invalid_upstream_response",
                "configured upstream returned an invalid response",
            ),
            Self::OAuthCredentialUnavailable => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                "oauth_credential_unavailable",
                "the configured OAuth account is not available",
            ),
            Self::ProviderNotFound => (
                StatusCode::NOT_FOUND,
                "invalid_request_error",
                "provider_not_found",
                "the requested OpenMesh provider is not configured",
            ),
            Self::AccountNotFound => (
                StatusCode::NOT_FOUND,
                "invalid_request_error",
                "account_not_found",
                "the requested OpenMesh account is not configured",
            ),
            Self::UpstreamStatus(status) => (
                StatusCode::BAD_GATEWAY,
                "upstream_error",
                "upstream_http_error",
                match status {
                    401 | 403 => "configured upstream rejected its credential",
                    429 => "configured upstream rate-limited the request",
                    _ => "configured upstream returned an error",
                },
            ),
        };
        let payload = json!({
            "error": {
                "message": message,
                "type": error_type,
                "code": code,
                "requestId": request_id,
            }
        });
        let mut response = json_response(status, &payload, request_id);
        if matches!(self, Self::Unauthorized) {
            response
                .headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy_server::{ProxyModelConfig, ProxyServerConfig, ProxyUpstreamConfig};
    use std::collections::BTreeMap;

    fn server() -> crate::proxy_server::ProxyServer {
        let config = ProxyServerConfig {
            bind_host: "127.0.0.1".to_string(),
            port: 8317,
            api_keys: vec!["client-key".to_string()],
            allow_unauthenticated: false,
            request_timeout_secs: 30,
            routing_strategy: Default::default(),
            max_retries: crate::proxy_server::DEFAULT_MAX_RETRIES,
            upstreams: vec![ProxyUpstreamConfig {
                id: "primary".to_string(),
                base_url: "http://127.0.0.1:9000/v1".to_string(),
                protocol: super::ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider-key".to_string()),
                enabled: true,
                priority: 0,
                account_id: None,
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "provider-model".to_string(),
                    owned_by: "test".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            }],
            model_aliases: BTreeMap::from([("friendly".to_string(), "provider-model".to_string())]),
            model_fallbacks: BTreeMap::new(),
        };
        crate::proxy_server::ProxyServer::new(config).expect("test config")
    }

    #[test]
    fn alias_resolution_is_deterministic() {
        let server = server();
        let config = server.config();
        assert_eq!(
            resolve_model_in_config(&config, "friendly"),
            Ok("provider-model".to_string())
        );
        assert_eq!(
            resolve_model_in_config(&config, "provider-model"),
            Ok("provider-model".to_string())
        );
    }

    #[test]
    fn invalid_request_has_stable_error_shape() {
        let response = ProxyRouteError::InvalidRequest("model is required").response("req-1");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn openai_multimodal_and_tools_translate_to_anthropic() {
        let payload = json!({
            "model": "claude-model",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe this"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,aGVsbG8="}}
                ]
            }],
            "tools": [{"type": "function", "function": {
                "name": "lookup",
                "description": "Look up a value",
                "parameters": {"type": "object", "properties": {"key": {"type": "string"}}}
            }}],
            "tool_choice": "required"
        });
        let translated =
            openai_to_anthropic_payload(&payload, "claude-model").expect("anthropic payload");
        assert_eq!(translated["messages"][0]["content"][1]["type"], "image");
        assert_eq!(
            translated["messages"][0]["content"][1]["source"]["type"],
            "base64"
        );
        assert_eq!(translated["tools"][0]["input_schema"]["type"], "object");
        assert_eq!(translated["tool_choice"], "any");
    }

    #[test]
    fn openai_multimodal_and_tools_translate_to_gemini() {
        let payload = json!({
            "model": "gemini-model",
            "messages": [{
                "role": "user",
                "content": [
                    {"type": "text", "text": "describe this"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,aGVsbG8="}}
                ]
            }],
            "tools": [{"type": "function", "function": {
                "name": "lookup",
                "parameters": {"type": "object", "properties": {"key": {"type": "string"}}}
            }}],
            "tool_choice": "required"
        });
        let translated =
            openai_to_gemini_payload(&payload, "gemini-model").expect("gemini payload");
        assert_eq!(
            translated["contents"][0]["parts"][1]["inlineData"]["mimeType"],
            "image/png"
        );
        assert_eq!(
            translated["tools"][0]["functionDeclarations"][0]["name"],
            "lookup"
        );
        assert_eq!(
            translated["toolConfig"]["functionCallingConfig"]["mode"],
            "ANY"
        );
    }

    #[test]
    fn provider_tool_calls_become_openai_tool_calls() {
        let anthropic = provider_to_openai_response(
            &Bytes::from_static(br#"{
                "content":[{"type":"tool_use","id":"toolu_1","name":"lookup","input":{"key":"value"}}],
                "usage":{"input_tokens":3,"output_tokens":4}
            }"#),
            ProxyProviderProtocol::Anthropic,
            "claude-model",
        )
        .expect("anthropic response");
        assert_eq!(anthropic["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(
            anthropic["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "lookup"
        );

        let gemini = provider_to_openai_response(
            &Bytes::from_static(br#"{
                "candidates":[{"content":{"parts":[{"functionCall":{"name":"lookup","args":{"key":"value"}}}]}}],
                "usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":4,"totalTokenCount":7}
            }"#),
            ProxyProviderProtocol::Gemini,
            "gemini-model",
        )
        .expect("gemini response");
        assert_eq!(gemini["choices"][0]["finish_reason"], "tool_calls");
        assert_eq!(
            gemini["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
            "lookup"
        );
    }

    #[test]
    fn oauth_provider_aliases_are_canonicalized_before_execution() {
        assert_eq!(
            canonical_oauth_provider(Some("anthropic")),
            Some("claude".to_string())
        );
        assert_eq!(
            canonical_oauth_provider(Some("x-ai")),
            Some("grok".to_string())
        );
        assert_eq!(
            canonical_oauth_provider(Some("kimi")),
            Some("kimi".to_string())
        );
        assert_eq!(canonical_oauth_provider(None), None);
    }

    #[test]
    fn first_compatible_uses_account_priority() {
        let mut config = server().config();
        config.upstreams[0].priority = 10;
        config.upstreams.push(ProxyUpstreamConfig {
            id: "preferred".to_string(),
            base_url: "http://127.0.0.1:9001/v1".to_string(),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: Some("preferred-key".to_string()),
            enabled: true,
            priority: 0,
            account_id: Some("preferred-account".to_string()),
            oauth_provider: None,
            models: vec![ProxyModelConfig {
                id: "provider-model".to_string(),
                owned_by: "test".to_string(),
                capabilities: vec!["chat".to_string()],
            }],
        });
        let server = crate::proxy_server::ProxyServer::new(config).expect("priority config");
        let selections = select_upstreams(&server.state, "provider-model", "request-1")
            .expect("compatible upstreams");
        assert_eq!(selections[0].upstream_id, "preferred");
    }

    #[test]
    fn mixed_responses_candidates_do_not_use_codex_native_executor() {
        let mut config = server().config();
        config.upstreams[0].oauth_provider = Some("codex".to_string());
        config.upstreams[0].account_id = Some("codex-account".to_string());
        config.upstreams.push(ProxyUpstreamConfig {
            id: "fallback".to_string(),
            base_url: "http://127.0.0.1:9001/v1".to_string(),
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: Some("fallback-key".to_string()),
            enabled: true,
            priority: 0,
            account_id: None,
            oauth_provider: None,
            models: vec![ProxyModelConfig {
                id: "fallback-model".to_string(),
                owned_by: "test".to_string(),
                capabilities: vec!["chat".to_string()],
            }],
        });
        config.model_fallbacks.insert(
            "provider-model".to_string(),
            vec!["fallback-model".to_string()],
        );
        let server = crate::proxy_server::ProxyServer::new(config).expect("fallback config");
        let selections = select_upstreams(&server.state, "provider-model", "request-1")
            .expect("fallback candidates");
        assert_eq!(selections.len(), 2);
        assert!(!uses_native_codex_responses(&selections));
    }
}
