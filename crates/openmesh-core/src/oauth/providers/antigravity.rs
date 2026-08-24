//! Google Antigravity OAuth adapter.
//!
//! Antigravity is not a plain Gemini bearer-token flow: the current native
//! contract exchanges a Google authorization code, resolves the account email,
//! and discovers a Cloud Code project through `loadCodeAssist` (with an
//! `onboardUser` fallback). The adapter owns that control-plane sequence; the
//! provider-specific generation transport remains a separate data-plane slice.

use super::super::{
    callback_code, required_nonempty, OAuthAuthorization, OAuthCallback, OAuthError,
    OAuthProviderAdapter, OAuthProviderId, OAuthToken, PkceCodes,
};
use chrono::{Duration, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
use url::Url;

/// Runtime configuration names for the Google OAuth client used by
/// Antigravity. Credentials are intentionally not shipped in the binary or
/// source tree; hosts must provide them through their environment.
pub const ANTIGRAVITY_CLIENT_ID_ENV: &str = "OPENMESH_ANTIGRAVITY_CLIENT_ID";
pub const ANTIGRAVITY_CLIENT_SECRET_ENV: &str = "OPENMESH_ANTIGRAVITY_CLIENT_SECRET";
pub const ANTIGRAVITY_AUTHORIZATION_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const ANTIGRAVITY_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const ANTIGRAVITY_USER_INFO_URL: &str =
    "https://www.googleapis.com/oauth2/v2/userinfo?alt=json";
pub const ANTIGRAVITY_LOAD_CODE_ASSIST_URL: &str =
    "https://cloudcode-pa.googleapis.com/v1internal:loadCodeAssist";
pub const ANTIGRAVITY_ONBOARD_USER_URL: &str =
    "https://daily-cloudcode-pa.googleapis.com/v1internal:onboardUser";
pub const ANTIGRAVITY_REDIRECT_URI: &str = "http://localhost:51121/oauth-callback";
pub const ANTIGRAVITY_SCOPE: &str = "https://www.googleapis.com/auth/cloud-platform https://www.googleapis.com/auth/userinfo.email https://www.googleapis.com/auth/userinfo.profile https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/experimentsandconfigs";
pub const ANTIGRAVITY_USER_AGENT: &str = "antigravity/hub/2.9.1 darwin/arm64";
pub const ANTIGRAVITY_NODE_USER_AGENT: &str =
    "antigravity/hub/2.9.1 darwin/arm64 google-api-nodejs-client/10.3.0";
pub const ANTIGRAVITY_GOOG_API_CLIENT: &str = "gl-node/22.21.1";
pub const ANTIGRAVITY_REFRESH_LEAD_SECS: i64 = 300;

#[derive(Clone, Debug)]
pub struct AntigravityOAuthEndpoints {
    pub authorization_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub load_code_assist_url: String,
    pub onboard_user_url: String,
}

#[derive(Clone)]
pub struct AntigravityOAuthCredentials {
    client_id: String,
    client_secret: String,
}

impl AntigravityOAuthCredentials {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }

    pub fn from_environment() -> Self {
        Self::new(
            std::env::var(ANTIGRAVITY_CLIENT_ID_ENV).unwrap_or_default(),
            std::env::var(ANTIGRAVITY_CLIENT_SECRET_ENV).unwrap_or_default(),
        )
    }

    fn required(&self) -> Result<(&str, &str), OAuthError> {
        let client_id = self.client_id.trim();
        if client_id.is_empty() {
            return Err(OAuthError::InvalidConfiguration(
                "Antigravity OAuth client ID is not configured".to_owned(),
            ));
        }
        let client_secret = self.client_secret.trim();
        if client_secret.is_empty() {
            return Err(OAuthError::InvalidConfiguration(
                "Antigravity OAuth client secret is not configured".to_owned(),
            ));
        }
        Ok((client_id, client_secret))
    }
}

impl Default for AntigravityOAuthEndpoints {
    fn default() -> Self {
        Self {
            authorization_url: ANTIGRAVITY_AUTHORIZATION_URL.to_owned(),
            token_url: ANTIGRAVITY_TOKEN_URL.to_owned(),
            user_info_url: ANTIGRAVITY_USER_INFO_URL.to_owned(),
            load_code_assist_url: ANTIGRAVITY_LOAD_CODE_ASSIST_URL.to_owned(),
            onboard_user_url: ANTIGRAVITY_ONBOARD_USER_URL.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct AntigravityOAuth {
    client: Client,
    endpoints: AntigravityOAuthEndpoints,
    credentials: AntigravityOAuthCredentials,
}

impl AntigravityOAuth {
    pub fn new(client: Client) -> Self {
        Self::with_endpoints_and_credentials(
            client,
            AntigravityOAuthEndpoints::default(),
            AntigravityOAuthCredentials::new("", ""),
        )
    }

    pub fn from_environment(client: Client) -> Self {
        Self::with_endpoints_and_credentials(
            client,
            AntigravityOAuthEndpoints::default(),
            AntigravityOAuthCredentials::from_environment(),
        )
    }

    pub fn with_credentials(client: Client, credentials: AntigravityOAuthCredentials) -> Self {
        Self::with_endpoints_and_credentials(
            client,
            AntigravityOAuthEndpoints::default(),
            credentials,
        )
    }

    pub fn with_client_credentials(
        client: Client,
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self::with_credentials(
            client,
            AntigravityOAuthCredentials::new(client_id, client_secret),
        )
    }

    pub fn with_endpoints(client: Client, endpoints: AntigravityOAuthEndpoints) -> Self {
        Self::with_endpoints_and_credentials(
            client,
            endpoints,
            AntigravityOAuthCredentials::new("", ""),
        )
    }

    pub fn with_endpoints_and_credentials(
        client: Client,
        endpoints: AntigravityOAuthEndpoints,
        credentials: AntigravityOAuthCredentials,
    ) -> Self {
        Self {
            client,
            endpoints,
            credentials,
        }
    }

    pub fn endpoints(&self) -> &AntigravityOAuthEndpoints {
        &self.endpoints
    }
}

#[derive(Debug, Deserialize)]
struct AntigravityTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AntigravityUserInfo {
    #[serde(default)]
    email: String,
}

#[async_trait::async_trait]
impl OAuthProviderAdapter for AntigravityOAuth {
    fn provider(&self) -> OAuthProviderId {
        OAuthProviderId::Antigravity
    }

    fn default_redirect_uri(&self) -> &str {
        ANTIGRAVITY_REDIRECT_URI
    }

    fn refresh_lead(&self) -> Duration {
        Duration::seconds(ANTIGRAVITY_REFRESH_LEAD_SECS)
    }

    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        _pkce: &PkceCodes,
    ) -> Result<OAuthAuthorization, OAuthError> {
        let redirect_uri = required_nonempty(redirect_uri, "redirect_uri")?;
        let state = required_nonempty(state, "state")?;
        let (client_id, _) = self.credentials.required()?;
        let mut url = Url::parse(&self.endpoints.authorization_url).map_err(|_| {
            OAuthError::InvalidConfiguration("Antigravity authorization URL is invalid".to_owned())
        })?;
        url.query_pairs_mut()
            .append_pair("access_type", "offline")
            .append_pair("client_id", client_id)
            .append_pair("prompt", "consent")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", ANTIGRAVITY_SCOPE)
            .append_pair("state", &state);
        Ok(OAuthAuthorization {
            provider: OAuthProviderId::Antigravity,
            authorization_url: url,
            redirect_uri,
            state,
        })
    }

    async fn exchange_code(
        &self,
        callback: &OAuthCallback,
        redirect_uri: &str,
        _pkce_verifier: &str,
    ) -> Result<OAuthToken, OAuthError> {
        let code = callback_code(callback)?;
        let redirect_uri = required_nonempty(redirect_uri, "redirect_uri")?;
        let (client_id, client_secret) = self.credentials.required()?;
        let mut form = BTreeMap::new();
        form.insert("code", code.as_str());
        form.insert("client_id", client_id);
        form.insert("client_secret", client_secret);
        form.insert("redirect_uri", redirect_uri.as_str());
        form.insert("grant_type", "authorization_code");
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Antigravity,
                source,
            })?;
        let mut token = self.parse_token_response(response, None).await?;
        self.enrich_identity(&mut token).await?;
        Ok(token)
    }

    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError> {
        if token.provider != OAuthProviderId::Antigravity {
            return Err(OAuthError::Request(
                "token provider does not match Antigravity adapter".to_owned(),
            ));
        }
        let refresh_token = token.refresh_token.as_deref().ok_or_else(|| {
            OAuthError::Request("Antigravity token has no refresh token".to_owned())
        })?;
        let (client_id, client_secret) = self.credentials.required()?;
        let mut form = BTreeMap::new();
        form.insert("client_id", client_id);
        form.insert("client_secret", client_secret);
        form.insert("grant_type", "refresh_token");
        form.insert("refresh_token", refresh_token);
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("content-type", "application/x-www-form-urlencoded")
            .form(&form)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Antigravity,
                source,
            })?;
        self.parse_token_response(response, Some(token)).await
    }
}

impl AntigravityOAuth {
    async fn parse_token_response(
        &self,
        response: reqwest::Response,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Antigravity,
                status: response.status().as_u16(),
            });
        }
        let payload = response
            .json::<AntigravityTokenResponse>()
            .await
            .map_err(|_| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Antigravity,
                message: "token response was not valid JSON".to_owned(),
            })?;
        let access_token = required_nonempty(&payload.access_token, "access_token")?;
        let mut metadata = previous
            .map(|token| token.metadata.clone())
            .unwrap_or_default();
        metadata.insert(
            "base_url".to_owned(),
            ANTIGRAVITY_LOAD_CODE_ASSIST_URL.into(),
        );
        let refresh_token = payload
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| previous.and_then(|token| token.refresh_token.clone()));
        let expires_at = payload
            .expires_in
            .filter(|value| *value > 0)
            .map(|value| Utc::now() + Duration::seconds(value));
        Ok(OAuthToken {
            provider: OAuthProviderId::Antigravity,
            account_id: previous
                .map(|token| token.account_id.clone())
                .unwrap_or_else(|| "antigravity-default".to_owned()),
            access_token,
            refresh_token,
            token_type: payload
                .token_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Bearer".to_owned()),
            expires_at,
            scopes: ANTIGRAVITY_SCOPE
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            metadata,
        })
    }

    async fn enrich_identity(&self, token: &mut OAuthToken) -> Result<(), OAuthError> {
        let email_response = self
            .client
            .get(&self.endpoints.user_info_url)
            .header(
                "authorization",
                token
                    .authorization_header()
                    .ok_or_else(|| OAuthError::InvalidResponse {
                        provider: OAuthProviderId::Antigravity,
                        message: "token response omitted access token".to_owned(),
                    })?,
            )
            .header("user-agent", ANTIGRAVITY_USER_AGENT)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Antigravity,
                source,
            })?;
        if !email_response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Antigravity,
                status: email_response.status().as_u16(),
            });
        }
        let user = email_response
            .json::<AntigravityUserInfo>()
            .await
            .map_err(|_| OAuthError::InvalidResponse {
                provider: OAuthProviderId::Antigravity,
                message: "user info response was not valid JSON".to_owned(),
            })?;
        let email = required_nonempty(&user.email, "email")?;
        token.account_id = email.clone();
        token.metadata.insert("email".to_owned(), email.into());

        let project_id = self.fetch_project_id(&token.access_token).await?;
        token
            .metadata
            .insert("project_id".to_owned(), project_id.into());
        Ok(())
    }

    async fn fetch_project_id(&self, access_token: &str) -> Result<String, OAuthError> {
        let response = self
            .client
            .post(&self.endpoints.load_code_assist_url)
            .header("authorization", format!("Bearer {access_token}"))
            .header("accept", "*/*")
            .header("content-type", "application/json")
            .header("user-agent", ANTIGRAVITY_USER_AGENT)
            .json(&json!({"metadata": {"ideType": "ANTIGRAVITY"}}))
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Antigravity,
                source,
            })?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider {
                provider: OAuthProviderId::Antigravity,
                status: response.status().as_u16(),
            });
        }
        let payload = response.json::<serde_json::Value>().await.map_err(|_| {
            OAuthError::InvalidResponse {
                provider: OAuthProviderId::Antigravity,
                message: "loadCodeAssist response was not valid JSON".to_owned(),
            }
        })?;
        if let Some(project_id) = extract_project_id(&payload) {
            return Ok(project_id);
        }
        self.onboard_user(access_token, &payload).await
    }

    async fn onboard_user(
        &self,
        access_token: &str,
        load_response: &serde_json::Value,
    ) -> Result<String, OAuthError> {
        let tier_id = default_tier_id(load_response);
        let body = json!({
            "tier_id": tier_id,
            "metadata": {
                "ide_type": "ANTIGRAVITY",
                "ide_version": "2.9.1",
                "ide_name": "antigravity"
            }
        });
        for attempt in 0..5 {
            let response = self
                .client
                .post(&self.endpoints.onboard_user_url)
                .header("authorization", format!("Bearer {access_token}"))
                .header("accept", "*/*")
                .header("content-type", "application/json")
                .header("user-agent", ANTIGRAVITY_NODE_USER_AGENT)
                .header("x-goog-api-client", ANTIGRAVITY_GOOG_API_CLIENT)
                .json(&body)
                .send()
                .await
                .map_err(|source| OAuthError::Http {
                    provider: OAuthProviderId::Antigravity,
                    source,
                })?;
            if !response.status().is_success() {
                return Err(OAuthError::Provider {
                    provider: OAuthProviderId::Antigravity,
                    status: response.status().as_u16(),
                });
            }
            let payload = response.json::<serde_json::Value>().await.map_err(|_| {
                OAuthError::InvalidResponse {
                    provider: OAuthProviderId::Antigravity,
                    message: "onboardUser response was not valid JSON".to_owned(),
                }
            })?;
            if payload.get("done").and_then(serde_json::Value::as_bool) == Some(true) {
                if let Some(project_id) = payload.get("response").and_then(extract_project_id) {
                    return Ok(project_id);
                }
                return Err(OAuthError::InvalidResponse {
                    provider: OAuthProviderId::Antigravity,
                    message: "onboardUser response omitted project ID".to_owned(),
                });
            }
            if attempt < 4 {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
        }
        Err(OAuthError::Request(
            "Antigravity project onboarding did not complete".to_owned(),
        ))
    }
}

fn extract_project_id(value: &serde_json::Value) -> Option<String> {
    let object = value.as_object()?;
    for key in ["cloudaicompanionProject", "projectId", "project"] {
        match object.get(key) {
            Some(serde_json::Value::String(project)) if !project.trim().is_empty() => {
                return Some(project.trim().to_owned())
            }
            Some(serde_json::Value::Object(project)) => {
                if let Some(id) = project.get("id").and_then(serde_json::Value::as_str) {
                    if !id.trim().is_empty() {
                        return Some(id.trim().to_owned());
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn default_tier_id(value: &serde_json::Value) -> String {
    value
        .get("allowedTiers")
        .and_then(serde_json::Value::as_array)
        .and_then(|tiers| {
            tiers.iter().find_map(|tier| {
                (tier.get("isDefault").and_then(serde_json::Value::as_bool) == Some(true))
                    .then(|| tier.get("id").and_then(serde_json::Value::as_str))
                    .flatten()
                    .filter(|id| !id.trim().is_empty())
                    .map(ToOwned::to_owned)
            })
        })
        .or_else(|| {
            value
                .get("currentTier")
                .and_then(|tier| tier.get("id"))
                .and_then(serde_json::Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| "free-tier".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        http::StatusCode,
        response::IntoResponse,
        routing::{get, post},
        Router,
    };
    use tokio::net::TcpListener;

    async fn token_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"access_token":"access","refresh_token":"refresh","expires_in":3600,"token_type":"Bearer"}"#,
        )
    }

    async fn user_info_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"email":"user@example.test"}"#,
        )
    }

    async fn load_handler() -> impl IntoResponse {
        (
            StatusCode::OK,
            [("content-type", "application/json")],
            r#"{"cloudaicompanionProject":"project-1"}"#,
        )
    }

    #[test]
    fn authorization_url_has_google_antigravity_contract() {
        let adapter = AntigravityOAuth::with_client_credentials(
            Client::new(),
            "test-client-id",
            "test-client-secret",
        );
        let pkce = PkceCodes::generate().unwrap();
        let auth = adapter
            .authorization_url(ANTIGRAVITY_REDIRECT_URI, "state", &pkce)
            .unwrap();
        let query: BTreeMap<_, _> = auth.authorization_url.query_pairs().into_owned().collect();
        assert_eq!(
            query.get("client_id").map(String::as_str),
            Some("test-client-id")
        );
        assert_eq!(
            query.get("access_type").map(String::as_str),
            Some("offline")
        );
        assert_eq!(query.get("prompt").map(String::as_str), Some("consent"));
        assert_eq!(query.get("state").map(String::as_str), Some("state"));
    }

    #[tokio::test]
    async fn exchange_enriches_email_and_project_metadata() {
        let router = Router::new()
            .route("/token", post(token_handler))
            .route("/userinfo", get(user_info_handler))
            .route("/load", post(load_handler));
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let adapter = AntigravityOAuth::with_endpoints_and_credentials(
            Client::new(),
            AntigravityOAuthEndpoints {
                token_url: format!("http://{address}/token"),
                user_info_url: format!("http://{address}/userinfo"),
                load_code_assist_url: format!("http://{address}/load"),
                ..AntigravityOAuthEndpoints::default()
            },
            AntigravityOAuthCredentials::new("test-client-id", "test-client-secret"),
        );
        let token = adapter
            .exchange_code(
                &OAuthCallback {
                    code: Some("code".to_owned()),
                    ..Default::default()
                },
                ANTIGRAVITY_REDIRECT_URI,
                "unused",
            )
            .await
            .unwrap();
        assert_eq!(token.account_id, "user@example.test");
        assert_eq!(token.metadata["project_id"], "project-1");
        assert_eq!(token.metadata["email"], "user@example.test");
    }

    #[test]
    fn missing_client_credentials_fail_closed() {
        let adapter = AntigravityOAuth::new(Client::new());
        let pkce = PkceCodes::generate().unwrap();
        let error = adapter
            .authorization_url(ANTIGRAVITY_REDIRECT_URI, "state", &pkce)
            .unwrap_err();
        assert!(
            matches!(error, OAuthError::InvalidConfiguration(message) if message.contains("client ID"))
        );
    }
}
