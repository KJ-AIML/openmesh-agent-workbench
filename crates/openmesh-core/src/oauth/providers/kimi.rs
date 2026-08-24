//! Moonshot Kimi OAuth device-flow adapter.
//!
//! This follows the current CLIProxyAPI v7 Kimi Code contract: the device
//! authorization and token endpoints are on `auth.kimi.com`, device identity
//! headers accompany every OAuth request, and the token endpoint returns both
//! pending and successful states with HTTP 200.

use super::super::{
    required_nonempty, OAuthAuthorization, OAuthCallback, OAuthDeviceCode, OAuthError,
    OAuthProviderAdapter, OAuthProviderId, OAuthToken, PkceCodes,
};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use std::collections::BTreeMap;
use uuid::Uuid;

pub const KIMI_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
pub const KIMI_DEVICE_CODE_URL: &str = "https://auth.kimi.com/api/oauth/device_authorization";
pub const KIMI_TOKEN_URL: &str = "https://auth.kimi.com/api/oauth/token";
pub const KIMI_API_BASE_URL: &str = "https://api.kimi.com/coding";
pub const KIMI_DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
pub const KIMI_REFRESH_LEAD_SECS: i64 = 300;

#[derive(Clone, Debug)]
pub struct KimiOAuthEndpoints {
    pub device_code_url: String,
    pub token_url: String,
}

impl Default for KimiOAuthEndpoints {
    fn default() -> Self {
        Self {
            device_code_url: KIMI_DEVICE_CODE_URL.to_owned(),
            token_url: KIMI_TOKEN_URL.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct KimiOAuth {
    client: Client,
    endpoints: KimiOAuthEndpoints,
    device_id: String,
}

impl KimiOAuth {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            endpoints: KimiOAuthEndpoints::default(),
            device_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_endpoints(client: Client, endpoints: KimiOAuthEndpoints) -> Self {
        Self {
            client,
            endpoints,
            device_id: Uuid::new_v4().to_string(),
        }
    }

    pub fn with_device_id(mut self, device_id: impl Into<String>) -> Self {
        self.device_id = device_id.into();
        self
    }

    pub fn endpoints(&self) -> &KimiOAuthEndpoints {
        &self.endpoints
    }

    fn common_headers(&self) -> [(&'static str, String); 4] {
        [
            ("x-msh-platform", "OpenMesh".to_owned()),
            ("x-msh-version", env!("CARGO_PKG_VERSION").to_owned()),
            (
                "x-msh-device-name",
                std::env::var("HOSTNAME").unwrap_or_else(|_| "openmesh".to_owned()),
            ),
            ("x-msh-device-id", self.device_id.clone()),
        ]
    }
}

#[derive(Debug, Deserialize)]
struct KimiDeviceResponse {
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
struct KimiTokenResponse {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
    #[serde(default)]
    access_token: Option<String>,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<f64>,
    #[serde(default)]
    scope: Option<String>,
}

#[async_trait::async_trait]
impl OAuthProviderAdapter for KimiOAuth {
    fn provider(&self) -> OAuthProviderId {
        OAuthProviderId::Kimi
    }

    fn default_redirect_uri(&self) -> &str {
        "https://auth.kimi.com/oauth/callback"
    }

    fn refresh_lead(&self) -> Duration {
        Duration::seconds(KIMI_REFRESH_LEAD_SECS)
    }

    fn authorization_url(
        &self,
        _redirect_uri: &str,
        _state: &str,
        _pkce: &PkceCodes,
    ) -> Result<OAuthAuthorization, OAuthError> {
        Err(OAuthError::UnsupportedOperation(OAuthProviderId::Kimi))
    }

    async fn exchange_code(
        &self,
        _callback: &OAuthCallback,
        _redirect_uri: &str,
        _pkce_verifier: &str,
    ) -> Result<OAuthToken, OAuthError> {
        Err(OAuthError::UnsupportedOperation(OAuthProviderId::Kimi))
    }

    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError> {
        if token.provider != OAuthProviderId::Kimi {
            return Err(OAuthError::Request(
                "token provider does not match Kimi adapter".to_owned(),
            ));
        }
        let refresh_token = token
            .refresh_token
            .as_deref()
            .ok_or_else(|| OAuthError::Request("Kimi token has no refresh token".to_owned()))?;
        let mut form = BTreeMap::new();
        form.insert("client_id", KIMI_CLIENT_ID);
        form.insert("grant_type", "refresh_token");
        form.insert("refresh_token", refresh_token);
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .headers(self.common_headers().into_iter().fold(
                reqwest::header::HeaderMap::new(),
                |mut headers, (name, value)| {
                    headers.insert(
                        reqwest::header::HeaderName::from_static(name),
                        value.parse().expect("Kimi header value is valid"),
                    );
                    headers
                },
            ))
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Kimi,
                source,
            })?;
        self.parse_token_response(response, Some(token)).await
    }

    async fn start_device_flow(&self) -> Result<OAuthDeviceCode, OAuthError> {
        let mut form = BTreeMap::new();
        form.insert("client_id", KIMI_CLIENT_ID);
        let response = self
            .client
            .post(&self.endpoints.device_code_url)
            .headers(self.common_headers().into_iter().fold(
                reqwest::header::HeaderMap::new(),
                |mut headers, (name, value)| {
                    headers.insert(
                        reqwest::header::HeaderName::from_static(name),
                        value.parse().expect("Kimi header value is valid"),
                    );
                    headers
                },
            ))
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Kimi,
                source,
            })?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Kimi,
                status: response.status().as_u16(),
            });
        }
        let payload = response.json::<KimiDeviceResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Kimi,
                message: "device authorization response was not valid JSON".to_owned(),
            }
        })?;
        let mut metadata = serde_json::Map::new();
        metadata.insert("device_id".to_owned(), self.device_id.clone().into());
        let verification_uri = payload
            .verification_uri
            .or(payload.verification_uri_complete.clone())
            .ok_or_else(|| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Kimi,
                message: "device authorization response omitted verification URI".to_owned(),
            })?;
        Ok(OAuthDeviceCode {
            provider: OAuthProviderId::Kimi,
            device_code: required_nonempty(&payload.device_code, "device_code")?,
            user_code: required_nonempty(&payload.user_code, "user_code")?,
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
        if device.provider != OAuthProviderId::Kimi || device.expires_at <= Utc::now() {
            return Err(OAuthError::Request(
                "Kimi device authorization is expired or belongs to another provider".to_owned(),
            ));
        }
        let mut form = BTreeMap::new();
        form.insert("client_id", KIMI_CLIENT_ID);
        form.insert("device_code", device.device_code.as_str());
        form.insert("grant_type", KIMI_DEVICE_GRANT_TYPE);
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .headers(self.common_headers().into_iter().fold(
                reqwest::header::HeaderMap::new(),
                |mut headers, (name, value)| {
                    headers.insert(
                        reqwest::header::HeaderName::from_static(name),
                        value.parse().expect("Kimi header value is valid"),
                    );
                    headers
                },
            ))
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Kimi,
                source,
            })?;
        let payload = response.json::<KimiTokenResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Kimi,
                message: "device token response was not valid JSON".to_owned(),
            }
        })?;
        if let Some(error) = payload.error.as_deref() {
            match error {
                "authorization_pending" | "slow_down" => return Ok(None),
                "expired_token" => {
                    return Err(OAuthError::Request(
                        "Kimi device authorization expired".to_owned(),
                    ))
                }
                "access_denied" => {
                    return Err(OAuthError::Request(
                        "Kimi device authorization denied".to_owned(),
                    ))
                }
                _ => {
                    return Err(OAuthError::InvalidResponse {
                        provider: OAuthProviderId::Kimi,
                        message: payload
                            .error_description
                            .unwrap_or_else(|| error.to_owned()),
                    })
                }
            }
        }
        self.parse_token_payload(payload, None).map(Some)
    }
}

impl KimiOAuth {
    async fn parse_token_response(
        &self,
        response: reqwest::Response,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        let status = response.status();
        let payload = response.json::<KimiTokenResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Kimi,
                message: "token response was not valid JSON".to_owned(),
            }
        })?;
        if payload.error.is_some() || !status.is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Kimi,
                status: status.as_u16(),
            });
        }
        self.parse_token_payload(payload, previous)
    }

    fn parse_token_payload(
        &self,
        payload: KimiTokenResponse,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        let access_token = required_nonempty(
            payload.access_token.as_deref().unwrap_or_default(),
            "access_token",
        )?;
        let account_id = previous
            .map(|token| token.account_id.clone())
            .unwrap_or_else(|| format!("kimi-{}", self.device_id));
        let mut metadata = previous
            .map(|token| token.metadata.clone())
            .unwrap_or_default();
        metadata.insert("device_id".to_owned(), self.device_id.clone().into());
        metadata.insert("base_url".to_owned(), KIMI_API_BASE_URL.into());
        let refresh_token = payload
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| previous.and_then(|token| token.refresh_token.clone()));
        let expires_at = payload
            .expires_in
            .filter(|value| *value > 0.0)
            .map(|value| Utc::now() + Duration::seconds(value as i64));
        Ok(OAuthToken {
            provider: OAuthProviderId::Kimi,
            account_id,
            access_token,
            refresh_token,
            token_type: payload
                .token_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Bearer".to_owned()),
            expires_at,
            scopes: payload
                .scope
                .unwrap_or_default()
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            metadata,
        })
    }
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
            r#"{"device_code":"device","user_code":"ABCD","verification_uri":"https://kimi.com/device","expires_in":900,"interval":1}"#,
        )
    }

    async fn token_handler(State(state): State<MockState>) -> impl IntoResponse {
        let mut polls = state.polls.lock().unwrap();
        *polls += 1;
        if *polls == 1 {
            return (
                StatusCode::OK,
                [("content-type", "application/json")],
                r#"{"error":"authorization_pending"}"#,
            );
        }
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"access_token":"kimi-access","refresh_token":"kimi-refresh","token_type":"Bearer","expires_in":3600,"scope":"coding"}"#,
        )
    }

    #[tokio::test]
    async fn device_flow_handles_kimi_pending_response() {
        let state = MockState::default();
        let router = Router::new()
            .route("/device", post(device_handler))
            .route("/token", post(token_handler))
            .with_state(state);
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let adapter = KimiOAuth::with_endpoints(
            Client::new(),
            KimiOAuthEndpoints {
                device_code_url: format!("http://{address}/device"),
                token_url: format!("http://{address}/token"),
            },
        )
        .with_device_id("device-id");
        let device = adapter.start_device_flow().await.unwrap();
        assert_eq!(adapter.poll_device_flow(&device).await.unwrap(), None);
        let token = adapter
            .poll_device_flow(&device)
            .await
            .unwrap()
            .expect("authorized token");
        assert_eq!(token.account_id, "kimi-device-id");
        assert_eq!(token.metadata["device_id"], "device-id");
    }
}
