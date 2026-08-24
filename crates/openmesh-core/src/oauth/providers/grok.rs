//! xAI Grok OAuth device-flow adapter.
//!
//! The wire contract is pinned to the current CLIProxyAPI v7 xAI adapter:
//! OIDC discovery resolves the device and token endpoints, the device grant
//! uses the public Grok CLI client ID, and credentials are refreshed through
//! the discovered token endpoint. Endpoint injection keeps the contract fully
//! testable without live provider credentials.

use super::super::{
    required_nonempty, OAuthAuthorization, OAuthCallback, OAuthDeviceCode, OAuthError,
    OAuthProviderAdapter, OAuthProviderId, OAuthToken, PkceCodes,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;

pub const GROK_DISCOVERY_URL: &str = "https://auth.x.ai/.well-known/openid-configuration";
pub const GROK_CLIENT_ID: &str = "b1a00492-073a-47ea-816f-4c329264a828";
pub const GROK_SCOPE: &str = "openid profile email offline_access grok-cli:access api:access";
pub const GROK_DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const GROK_REDIRECT_URI: &str = "https://auth.x.ai/oauth/callback";
pub const GROK_DEFAULT_API_BASE_URL: &str = "https://cli-chat-proxy.grok.com/v1";

#[derive(Clone, Debug)]
pub struct GrokOAuthEndpoints {
    pub discovery_url: String,
    pub device_authorization_url: Option<String>,
    pub token_url: Option<String>,
}

impl Default for GrokOAuthEndpoints {
    fn default() -> Self {
        Self {
            discovery_url: GROK_DISCOVERY_URL.to_owned(),
            device_authorization_url: None,
            token_url: None,
        }
    }
}

#[derive(Clone)]
pub struct GrokOAuth {
    client: Client,
    endpoints: GrokOAuthEndpoints,
}

impl GrokOAuth {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            endpoints: GrokOAuthEndpoints::default(),
        }
    }

    pub fn with_endpoints(client: Client, endpoints: GrokOAuthEndpoints) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &GrokOAuthEndpoints {
        &self.endpoints
    }

    async fn resolve_endpoints(&self) -> Result<(String, String), OAuthError> {
        if let (Some(device), Some(token)) = (
            self.endpoints.device_authorization_url.clone(),
            self.endpoints.token_url.clone(),
        ) {
            return Ok((device, token));
        }
        let response = self
            .client
            .get(&self.endpoints.discovery_url)
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Grok,
                source,
            })?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Grok,
                status: response.status().as_u16(),
            });
        }
        let payload = response
            .json::<GrokDiscoveryResponse>()
            .await
            .map_err(|_| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Grok,
                message: "OIDC discovery response was not valid JSON".to_owned(),
            })?;
        let device = validate_xai_endpoint(
            &payload.device_authorization_endpoint,
            "device_authorization_endpoint",
        )?;
        let token = validate_xai_endpoint(&payload.token_endpoint, "token_endpoint")?;
        Ok((device, token))
    }
}

#[derive(Debug, Deserialize)]
struct GrokDiscoveryResponse {
    device_authorization_endpoint: String,
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct GrokDeviceResponse {
    device_code: String,
    user_code: String,
    #[serde(default)]
    verification_uri: Option<String>,
    #[serde(default)]
    verification_uri_complete: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GrokTokenResponse {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    id_token: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
}

#[async_trait::async_trait]
impl OAuthProviderAdapter for GrokOAuth {
    fn provider(&self) -> OAuthProviderId {
        OAuthProviderId::Grok
    }

    fn default_redirect_uri(&self) -> &str {
        GROK_REDIRECT_URI
    }

    fn authorization_url(
        &self,
        _redirect_uri: &str,
        _state: &str,
        _pkce: &PkceCodes,
    ) -> Result<OAuthAuthorization, OAuthError> {
        Err(OAuthError::UnsupportedOperation(OAuthProviderId::Grok))
    }

    async fn exchange_code(
        &self,
        _callback: &OAuthCallback,
        _redirect_uri: &str,
        _pkce_verifier: &str,
    ) -> Result<OAuthToken, OAuthError> {
        Err(OAuthError::UnsupportedOperation(OAuthProviderId::Grok))
    }

    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError> {
        if token.provider != OAuthProviderId::Grok {
            return Err(OAuthError::Request(
                "token provider does not match Grok adapter".to_owned(),
            ));
        }
        let (_, token_url) = self.resolve_endpoints().await?;
        let refresh_token = token
            .refresh_token
            .as_deref()
            .ok_or_else(|| OAuthError::Request("Grok token has no refresh token".to_owned()))?;
        let mut form = BTreeMap::new();
        form.insert("grant_type", "refresh_token");
        form.insert("client_id", GROK_CLIENT_ID);
        form.insert("refresh_token", refresh_token);
        let response = self
            .client
            .post(token_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Grok,
                source,
            })?;
        self.parse_token_response(response, Some(token)).await
    }

    async fn start_device_flow(&self) -> Result<OAuthDeviceCode, OAuthError> {
        let (device_url, token_url) = self.resolve_endpoints().await?;
        let mut form = BTreeMap::new();
        form.insert("client_id", GROK_CLIENT_ID);
        form.insert("scope", GROK_SCOPE);
        let response = self
            .client
            .post(device_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Grok,
                source,
            })?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Grok,
                status: response.status().as_u16(),
            });
        }
        let payload = response.json::<GrokDeviceResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Grok,
                message: "device authorization response was not valid JSON".to_owned(),
            }
        })?;
        let device_code = required_nonempty(&payload.device_code, "device_code")?;
        let user_code = required_nonempty(&payload.user_code, "user_code")?;
        let verification_uri = payload
            .verification_uri
            .or(payload.verification_uri_complete.clone())
            .ok_or_else(|| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Grok,
                message: "device authorization response omitted verification URI".to_owned(),
            })?;
        let mut metadata = serde_json::Map::new();
        metadata.insert("token_endpoint".to_owned(), token_url.into());
        Ok(OAuthDeviceCode {
            provider: OAuthProviderId::Grok,
            device_code,
            user_code,
            verification_uri: required_nonempty(&verification_uri, "verification_uri")?,
            verification_uri_complete: payload.verification_uri_complete,
            expires_at: Utc::now()
                + Duration::seconds(payload.expires_in.filter(|value| *value > 0).unwrap_or(900)),
            interval_seconds: payload.interval.filter(|value| *value > 0).unwrap_or(5),
            metadata,
        })
    }

    async fn poll_device_flow(
        &self,
        device: &OAuthDeviceCode,
    ) -> Result<Option<OAuthToken>, OAuthError> {
        if device.provider != OAuthProviderId::Grok || device.expires_at <= Utc::now() {
            return Err(OAuthError::Request(
                "Grok device authorization is expired or belongs to another provider".to_owned(),
            ));
        }
        let token_url = device
            .metadata
            .get("token_endpoint")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
            .ok_or_else(|| OAuthError::Request("Grok token endpoint is missing".to_owned()))?;
        let mut form = BTreeMap::new();
        form.insert("grant_type", GROK_DEVICE_GRANT_TYPE);
        form.insert("device_code", device.device_code.as_str());
        form.insert("client_id", GROK_CLIENT_ID);
        let response = self
            .client
            .post(token_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Grok,
                source,
            })?;
        let status = response.status();
        let payload = response.json::<GrokTokenResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Grok,
                message: "device token response was not valid JSON".to_owned(),
            }
        })?;
        if let Some(error) = payload.error.as_deref() {
            match error {
                "authorization_pending" | "slow_down" => return Ok(None),
                "expired_token" => {
                    return Err(OAuthError::Request(
                        "Grok device authorization expired".to_owned(),
                    ))
                }
                "access_denied" => {
                    return Err(OAuthError::Request(
                        "Grok device authorization denied".to_owned(),
                    ))
                }
                _ => {
                    return Err(OAuthError::InvalidResponse {
                        provider: OAuthProviderId::Grok,
                        message: payload
                            .error_description
                            .unwrap_or_else(|| error.to_owned()),
                    })
                }
            }
        }
        if !status.is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Grok,
                status: status.as_u16(),
            });
        }
        self.parse_token_response_from_payload(payload, None)
            .map(Some)
    }
}

impl GrokOAuth {
    async fn parse_token_response(
        &self,
        response: reqwest::Response,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        let status = response.status();
        let payload = response.json::<GrokTokenResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Grok,
                message: "token response was not valid JSON".to_owned(),
            }
        })?;
        if payload.error.is_some() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Grok,
                status: status.as_u16(),
            });
        }
        if !status.is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Grok,
                status: status.as_u16(),
            });
        }
        self.parse_token_response_from_payload(payload, previous)
    }

    fn parse_token_response_from_payload(
        &self,
        payload: GrokTokenResponse,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        let access_token = required_nonempty(
            payload.access_token.as_deref().unwrap_or_default(),
            "access_token",
        )?;
        let (email, subject) = payload
            .id_token
            .as_deref()
            .and_then(parse_id_token_identity)
            .unwrap_or_default();
        let account_id = if !email.is_empty() {
            email.clone()
        } else if !subject.is_empty() {
            subject.clone()
        } else if let Some(previous) = previous {
            previous.account_id.clone()
        } else {
            "grok-default".to_owned()
        };
        let mut metadata = previous
            .map(|token| token.metadata.clone())
            .unwrap_or_default();
        insert_nonempty(&mut metadata, "email", &email);
        insert_nonempty(&mut metadata, "sub", &subject);
        insert_nonempty(&mut metadata, "base_url", GROK_DEFAULT_API_BASE_URL);
        let refresh_token = payload
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| previous.and_then(|token| token.refresh_token.clone()));
        let expires_at = payload
            .expires_in
            .filter(|value| *value > 0)
            .map(|value| Utc::now() + Duration::seconds(value));
        Ok(OAuthToken {
            provider: OAuthProviderId::Grok,
            account_id,
            access_token,
            refresh_token,
            token_type: payload
                .token_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Bearer".to_owned()),
            expires_at,
            scopes: GROK_SCOPE
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            metadata,
        })
    }
}

fn validate_xai_endpoint(raw: &str, field: &str) -> Result<String, OAuthError> {
    let url = reqwest::Url::parse(raw).map_err(|_| {
        OAuthError::InvalidConfiguration(format!("Grok discovery {field} is invalid"))
    })?;
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    if url.scheme() != "https" || !(host == "x.ai" || host.ends_with(".x.ai")) {
        return Err(OAuthError::InvalidConfiguration(format!(
            "Grok discovery {field} must point to an x.ai HTTPS endpoint"
        )));
    }
    Ok(raw.to_owned())
}

fn insert_nonempty(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: &str,
) {
    if !value.trim().is_empty() {
        metadata.insert(key.to_owned(), value.to_owned().into());
    }
}

fn parse_id_token_identity(token: &str) -> Option<(String, String)> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    Some((
        claims
            .get("email")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        claims
            .get("sub")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;

    #[derive(Clone, Default)]
    struct MockState {
        polls: Arc<Mutex<u32>>,
    }

    async fn device_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"device_code":"device","user_code":"ABCD","verification_uri":"https://x.ai/device","expires_in":900,"interval":1}"#,
        )
    }

    async fn token_handler(State(state): State<MockState>) -> impl IntoResponse {
        let mut polls = state.polls.lock().unwrap();
        *polls += 1;
        if *polls == 1 {
            return (
                StatusCode::BAD_REQUEST,
                [("content-type", "application/json")],
                r#"{"error":"authorization_pending"}"#,
            );
        }
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"access_token":"grok-access","refresh_token":"grok-refresh","expires_in":3600,"token_type":"Bearer"}"#,
        )
    }

    #[tokio::test]
    async fn device_flow_keeps_pending_distinct_from_success() {
        let state = MockState::default();
        let router = Router::new()
            .route("/device", post(device_handler))
            .route("/token", post(token_handler))
            .with_state(state);
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let adapter = GrokOAuth::with_endpoints(
            Client::new(),
            GrokOAuthEndpoints {
                discovery_url: String::new(),
                device_authorization_url: Some(format!("http://{address}/device")),
                token_url: Some(format!("http://{address}/token")),
            },
        );
        let device = adapter.start_device_flow().await.unwrap();
        assert_eq!(adapter.poll_device_flow(&device).await.unwrap(), None);
        let token = adapter
            .poll_device_flow(&device)
            .await
            .unwrap()
            .expect("authorized token");
        assert_eq!(token.provider, OAuthProviderId::Grok);
        assert_eq!(token.account_id, "grok-default");
    }
}
