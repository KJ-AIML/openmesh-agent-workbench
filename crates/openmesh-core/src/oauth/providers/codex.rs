//! OpenAI Codex OAuth and device authorization adapter.
//!
//! The constants and request shapes are pinned to the current CLIProxyAPI v7
//! source inspected for this implementation. They are adapter contracts, not
//! a claim that an upstream provider will keep them unchanged forever.

use super::super::{
    callback_code, required_nonempty, OAuthAuthorization, OAuthCallback, OAuthDeviceCode,
    OAuthError, OAuthProviderAdapter, OAuthProviderId, OAuthToken, PkceCodes,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::collections::BTreeMap;
use url::Url;

pub const CODEX_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
pub const CODEX_AUTHORIZATION_URL: &str = "https://auth.openai.com/oauth/authorize";
pub const CODEX_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
pub const CODEX_REDIRECT_URI: &str = "http://localhost:1455/auth/callback";
pub const CODEX_DEVICE_USER_CODE_URL: &str =
    "https://auth.openai.com/api/accounts/deviceauth/usercode";
pub const CODEX_DEVICE_TOKEN_URL: &str = "https://auth.openai.com/api/accounts/deviceauth/token";
pub const CODEX_DEVICE_VERIFICATION_URL: &str = "https://auth.openai.com/codex/device";
pub const CODEX_DEVICE_TOKEN_REDIRECT_URI: &str = "https://auth.openai.com/deviceauth/callback";

#[derive(Clone, Debug)]
pub struct CodexOAuthEndpoints {
    pub authorization_url: String,
    pub token_url: String,
    pub device_user_code_url: String,
    pub device_token_url: String,
    pub device_verification_url: String,
    pub device_token_redirect_uri: String,
}

impl Default for CodexOAuthEndpoints {
    fn default() -> Self {
        Self {
            authorization_url: CODEX_AUTHORIZATION_URL.to_owned(),
            token_url: CODEX_TOKEN_URL.to_owned(),
            device_user_code_url: CODEX_DEVICE_USER_CODE_URL.to_owned(),
            device_token_url: CODEX_DEVICE_TOKEN_URL.to_owned(),
            device_verification_url: CODEX_DEVICE_VERIFICATION_URL.to_owned(),
            device_token_redirect_uri: CODEX_DEVICE_TOKEN_REDIRECT_URI.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct CodexOAuth {
    client: Client,
    endpoints: CodexOAuthEndpoints,
}

impl CodexOAuth {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            endpoints: CodexOAuthEndpoints::default(),
        }
    }

    pub fn with_endpoints(client: Client, endpoints: CodexOAuthEndpoints) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &CodexOAuthEndpoints {
        &self.endpoints
    }

    pub async fn start_device_flow(&self) -> Result<OAuthDeviceCode, OAuthError> {
        let response = self
            .client
            .post(&self.endpoints.device_user_code_url)
            .header("accept", "application/json")
            .json(&serde_json::json!({ "client_id": CODEX_CLIENT_ID }))
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Codex,
                source,
            })?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Codex,
                status: response.status().as_u16(),
            });
        }
        let payload = response
            .json::<CodexDeviceStartResponse>()
            .await
            .map_err(|_| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Codex,
                message: "device authorization response was not valid JSON".to_owned(),
            })?;
        let device_auth_id = required_nonempty(&payload.device_auth_id, "device_auth_id")?;
        let user_code =
            payload
                .user_code
                .or(payload.usercode)
                .ok_or_else(|| OAuthError::InvalidResponse {
                    provider: OAuthProviderId::Codex,
                    message: "device authorization response omitted user code".to_owned(),
                })?;
        let user_code = required_nonempty(&user_code, "user_code")?;
        let interval_seconds = payload
            .interval
            .as_ref()
            .and_then(parse_interval)
            .filter(|value| *value > 0)
            .unwrap_or(5);
        Ok(OAuthDeviceCode {
            provider: OAuthProviderId::Codex,
            device_code: device_auth_id,
            user_code,
            verification_uri: self.endpoints.device_verification_url.clone(),
            verification_uri_complete: None,
            expires_at: Utc::now() + Duration::minutes(15),
            interval_seconds,
            metadata: serde_json::Map::new(),
        })
    }

    /// Poll the Codex device endpoint once. `Ok(None)` means the user has not
    /// completed the browser step yet; the host controls the polling cadence.
    pub async fn poll_device_flow_once(
        &self,
        device: &OAuthDeviceCode,
    ) -> Result<Option<CodexDevicePoll>, OAuthError> {
        if device.provider != OAuthProviderId::Codex || device.expires_at <= Utc::now() {
            return Err(OAuthError::Request(
                "Codex device authorization is expired or belongs to another provider".to_owned(),
            ));
        }
        let response = self
            .client
            .post(&self.endpoints.device_token_url)
            .header("accept", "application/json")
            .json(&serde_json::json!({
                "device_auth_id": device.device_code,
                "user_code": device.user_code,
            }))
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Codex,
                source,
            })?;
        if response.status() == StatusCode::FORBIDDEN || response.status() == StatusCode::NOT_FOUND
        {
            return Ok(None);
        }
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Codex,
                status: response.status().as_u16(),
            });
        }
        let payload = response
            .json::<CodexDeviceTokenResponse>()
            .await
            .map_err(|_| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Codex,
                message: "device token response was not valid JSON".to_owned(),
            })?;
        Ok(Some(CodexDevicePoll {
            authorization_code: required_nonempty(
                &payload.authorization_code,
                "authorization_code",
            )?,
            code_verifier: required_nonempty(&payload.code_verifier, "code_verifier")?,
            code_challenge: required_nonempty(&payload.code_challenge, "code_challenge")?,
        }))
    }

    pub async fn exchange_device_poll(
        &self,
        poll: &CodexDevicePoll,
    ) -> Result<OAuthToken, OAuthError> {
        let callback = OAuthCallback {
            code: Some(poll.authorization_code.clone()),
            ..OAuthCallback::default()
        };
        self.exchange_code(
            &callback,
            &self.endpoints.device_token_redirect_uri,
            &poll.code_verifier,
        )
        .await
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodexDevicePoll {
    pub authorization_code: String,
    pub code_verifier: String,
    pub code_challenge: String,
}

#[derive(Debug, Deserialize)]
struct CodexTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    id_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CodexDeviceStartResponse {
    device_auth_id: String,
    #[serde(default)]
    user_code: Option<String>,
    #[serde(default)]
    usercode: Option<String>,
    #[serde(default)]
    interval: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct CodexDeviceTokenResponse {
    authorization_code: String,
    code_verifier: String,
    code_challenge: String,
}

#[async_trait::async_trait]
impl OAuthProviderAdapter for CodexOAuth {
    fn provider(&self) -> OAuthProviderId {
        OAuthProviderId::Codex
    }

    fn default_redirect_uri(&self) -> &str {
        CODEX_REDIRECT_URI
    }

    fn refresh_lead(&self) -> Duration {
        Duration::minutes(5)
    }

    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        pkce: &PkceCodes,
    ) -> Result<OAuthAuthorization, OAuthError> {
        let redirect_uri = required_nonempty(redirect_uri, "redirect_uri")?;
        let state = required_nonempty(state, "state")?;
        let mut url = Url::parse(&self.endpoints.authorization_url).map_err(|_| {
            OAuthError::InvalidConfiguration("Codex authorization URL is invalid".to_owned())
        })?;
        url.query_pairs_mut()
            .append_pair("client_id", CODEX_CLIENT_ID)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", "openid email profile offline_access")
            .append_pair("state", &state)
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("prompt", "login")
            .append_pair("id_token_add_organizations", "true")
            .append_pair("codex_cli_simplified_flow", "true");
        Ok(OAuthAuthorization {
            provider: OAuthProviderId::Codex,
            authorization_url: url,
            redirect_uri,
            state,
        })
    }

    async fn exchange_code(
        &self,
        callback: &OAuthCallback,
        redirect_uri: &str,
        pkce_verifier: &str,
    ) -> Result<OAuthToken, OAuthError> {
        let code = callback_code(callback)?;
        let redirect_uri = required_nonempty(redirect_uri, "redirect_uri")?;
        let pkce_verifier = PkceCodes::from_verifier(pkce_verifier.to_owned())?.verifier;
        let mut form = BTreeMap::new();
        form.insert("grant_type", "authorization_code");
        form.insert("client_id", CODEX_CLIENT_ID);
        form.insert("code", code.as_str());
        form.insert("redirect_uri", redirect_uri.as_str());
        form.insert("code_verifier", pkce_verifier.as_str());
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Codex,
                source,
            })?;
        self.parse_token_response(response, None).await
    }

    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError> {
        if token.provider != OAuthProviderId::Codex {
            return Err(OAuthError::Request(
                "token provider does not match Codex adapter".to_owned(),
            ));
        }
        let refresh_token = token
            .refresh_token
            .as_deref()
            .ok_or_else(|| OAuthError::Request("Codex token has no refresh token".to_owned()))?;
        let mut form = BTreeMap::new();
        form.insert("client_id", CODEX_CLIENT_ID);
        form.insert("grant_type", "refresh_token");
        form.insert("refresh_token", refresh_token);
        form.insert("scope", "openid profile email");
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Codex,
                source,
            })?;
        self.parse_token_response(response, Some(token)).await
    }

    async fn start_device_flow(&self) -> Result<OAuthDeviceCode, OAuthError> {
        CodexOAuth::start_device_flow(self).await
    }

    async fn poll_device_flow(
        &self,
        device: &OAuthDeviceCode,
    ) -> Result<Option<OAuthToken>, OAuthError> {
        let Some(poll) = self.poll_device_flow_once(device).await? else {
            return Ok(None);
        };
        self.exchange_device_poll(&poll).await.map(Some)
    }
}

impl CodexOAuth {
    async fn parse_token_response(
        &self,
        response: reqwest::Response,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Codex,
                status: response.status().as_u16(),
            });
        }
        let payload = response.json::<CodexTokenResponse>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Codex,
                message: "token response was not valid JSON".to_owned(),
            }
        })?;
        let access_token = required_nonempty(&payload.access_token, "access_token")?;
        let mut metadata = serde_json::Map::new();
        let (account_id, email) = payload
            .id_token
            .as_deref()
            .and_then(parse_id_token_identity)
            .unwrap_or_default();
        if !email.is_empty() {
            metadata.insert("email".to_owned(), serde_json::Value::String(email.clone()));
        }
        if !account_id.is_empty() {
            metadata.insert(
                "account_id".to_owned(),
                serde_json::Value::String(account_id.clone()),
            );
        }
        let account_id = if !account_id.is_empty() {
            account_id
        } else if !email.is_empty() {
            email
        } else if let Some(previous) = previous {
            previous.account_id.clone()
        } else {
            "codex-default".to_owned()
        };
        let refresh_token = payload
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| previous.and_then(|value| value.refresh_token.clone()));
        let expires_at = payload
            .expires_in
            .filter(|seconds| *seconds > 0)
            .map(|seconds| Utc::now() + Duration::seconds(seconds));
        Ok(OAuthToken {
            provider: OAuthProviderId::Codex,
            account_id,
            access_token,
            refresh_token,
            token_type: payload
                .token_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Bearer".to_owned()),
            expires_at,
            scopes: vec![
                "openid".to_owned(),
                "profile".to_owned(),
                "email".to_owned(),
            ],
            metadata,
        })
    }
}

fn parse_interval(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn parse_id_token_identity(token: &str) -> Option<(String, String)> {
    let payload = token.split('.').nth(1)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let claims = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
    let email = claims
        .get("email")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    let account_id = claims
        .get("https://api.openai.com/auth")
        .and_then(|value| value.get("chatgpt_account_id"))
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_owned();
    Some((account_id, email))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::HeaderMap, response::IntoResponse, routing::post, Router};
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;

    #[derive(Clone, Default)]
    struct MockState {
        body: Arc<Mutex<String>>,
    }

    async fn token_handler(
        State(state): State<MockState>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        let _ = headers;
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            state.body.lock().unwrap().clone(),
        )
    }

    async fn mock_server(body: &str) -> (String, MockState) {
        let state = MockState {
            body: Arc::new(Mutex::new(body.to_owned())),
        };
        let router = Router::new()
            .route("/token", post(token_handler))
            .with_state(state.clone());
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (format!("http://{address}/token"), state)
    }

    #[test]
    fn authorization_url_contains_pinned_codex_parameters() {
        let client = Client::new();
        let adapter = CodexOAuth::new(client);
        let pkce = PkceCodes::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let auth = adapter
            .authorization_url("http://127.0.0.1:1455/callback", "state", &pkce)
            .unwrap();
        let query: BTreeMap<_, _> = auth.authorization_url.query_pairs().into_owned().collect();
        assert_eq!(
            query.get("client_id").map(String::as_str),
            Some(CODEX_CLIENT_ID)
        );
        assert_eq!(
            query.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(query.get("prompt").map(String::as_str), Some("login"));
    }

    #[tokio::test]
    async fn exchange_uses_form_contract_and_redacts_failures() {
        let (token_url, _) = mock_server(r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600,"token_type":"Bearer"}"#).await;
        let adapter = CodexOAuth::with_endpoints(
            Client::new(),
            CodexOAuthEndpoints {
                token_url,
                ..CodexOAuthEndpoints::default()
            },
        );
        let pkce = PkceCodes::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let token = adapter
            .exchange_code(
                &OAuthCallback {
                    code: Some("code".to_owned()),
                    ..Default::default()
                },
                "http://localhost/callback",
                &pkce.verifier,
            )
            .await
            .unwrap();
        assert_eq!(token.access_token, "access");
        assert_eq!(token.refresh_token.as_deref(), Some("refresh"));
        assert_eq!(token.provider, OAuthProviderId::Codex);
    }

    #[tokio::test]
    async fn device_pending_status_is_nonterminal() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new().route(
            "/device",
            post(|| async { (StatusCode::FORBIDDEN, "pending") }),
        );
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let adapter = CodexOAuth::with_endpoints(
            Client::new(),
            CodexOAuthEndpoints {
                device_token_url: format!("http://{address}/device"),
                ..CodexOAuthEndpoints::default()
            },
        );
        let device = OAuthDeviceCode {
            provider: OAuthProviderId::Codex,
            device_code: "id".to_owned(),
            user_code: "code".to_owned(),
            verification_uri: "http://device".to_owned(),
            verification_uri_complete: None,
            expires_at: Utc::now() + Duration::minutes(1),
            interval_seconds: 1,
            metadata: serde_json::Map::new(),
        };
        assert!(adapter
            .poll_device_flow_once(&device)
            .await
            .unwrap()
            .is_none());
    }
}
