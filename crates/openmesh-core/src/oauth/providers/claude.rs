//! Anthropic Claude Code OAuth adapter.
//!
//! Request shapes are pinned to the current CLIProxyAPI v7 native adapter
//! source inspected for this implementation. Upstream OAuth endpoints are
//! intentionally injectable for contract tests.

use super::super::{
    callback_code, required_nonempty, OAuthAuthorization, OAuthCallback, OAuthError,
    OAuthProviderAdapter, OAuthProviderId, OAuthToken, PkceCodes,
};
use chrono::{Duration, Utc};
use reqwest::{Client, Response};
use serde::{Deserialize, Serialize};
use url::Url;

pub const CLAUDE_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
pub const CLAUDE_AUTHORIZATION_URL: &str = "https://claude.ai/oauth/authorize";
pub const CLAUDE_TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const CLAUDE_PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
pub const CLAUDE_ROLES_URL: &str = "https://api.anthropic.com/api/oauth/claude_cli/roles";
pub const CLAUDE_REDIRECT_URI: &str = "http://localhost:54545/callback";
pub const CLAUDE_OAUTH_SCOPE: &str =
    "user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";

#[derive(Clone, Debug)]
pub struct ClaudeOAuthEndpoints {
    pub authorization_url: String,
    pub token_url: String,
    pub profile_url: String,
    pub roles_url: String,
}

impl Default for ClaudeOAuthEndpoints {
    fn default() -> Self {
        Self {
            authorization_url: CLAUDE_AUTHORIZATION_URL.to_owned(),
            token_url: CLAUDE_TOKEN_URL.to_owned(),
            profile_url: CLAUDE_PROFILE_URL.to_owned(),
            roles_url: CLAUDE_ROLES_URL.to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct ClaudeOAuth {
    client: Client,
    endpoints: ClaudeOAuthEndpoints,
}

impl ClaudeOAuth {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            endpoints: ClaudeOAuthEndpoints::default(),
        }
    }

    pub fn with_endpoints(client: Client, endpoints: ClaudeOAuthEndpoints) -> Self {
        Self { client, endpoints }
    }

    pub fn endpoints(&self) -> &ClaudeOAuthEndpoints {
        &self.endpoints
    }

    /// Identity lookup is advisory, matching native Claude Code behavior: a
    /// successful token exchange must not fail only because profile/roles
    /// inspection is unavailable.
    pub async fn fetch_profile(&self, access_token: &str) -> Result<ClaudeProfile, OAuthError> {
        let access_token = required_nonempty(access_token, "access_token")?;
        let response = self
            .client
            .get(&self.endpoints.profile_url)
            .header("accept", "application/json, text/plain, */*")
            .header("authorization", format!("Bearer {access_token}"))
            .header("user-agent", "axios/1.15.2")
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Claude,
                source,
            })?;
        parse_json_response(response, OAuthProviderId::Claude, "profile").await
    }

    pub async fn fetch_roles(&self, access_token: &str) -> Result<serde_json::Value, OAuthError> {
        let access_token = required_nonempty(access_token, "access_token")?;
        let response = self
            .client
            .get(&self.endpoints.roles_url)
            .header("accept", "application/json, text/plain, */*")
            .header("authorization", format!("Bearer {access_token}"))
            .header("user-agent", "axios/1.15.2")
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Claude,
                source,
            })?;
        parse_json_response(response, OAuthProviderId::Claude, "roles").await
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClaudeProfile {
    #[serde(default)]
    pub account: ClaudeProfileAccount,
    #[serde(default)]
    pub organization: ClaudeProfileOrganization,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClaudeProfileAccount {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub email: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ClaudeProfileOrganization {
    #[serde(default)]
    pub uuid: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Serialize)]
struct ClaudeCodeExchangeRequest<'a> {
    grant_type: &'static str,
    code: &'a str,
    redirect_uri: &'a str,
    client_id: &'static str,
    code_verifier: &'a str,
    state: &'a str,
}

#[derive(Debug, Serialize)]
struct ClaudeRefreshRequest<'a> {
    client_id: &'static str,
    grant_type: &'static str,
    refresh_token: &'a str,
    scope: &'static str,
}

#[derive(Debug, Default, Deserialize)]
struct ClaudeTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    token_type: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    organization: ClaudeTokenOrganization,
    #[serde(default)]
    account: ClaudeTokenAccount,
}

#[derive(Debug, Default, Deserialize)]
struct ClaudeTokenAccount {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    email_address: String,
}

#[derive(Debug, Default, Deserialize)]
struct ClaudeTokenOrganization {
    #[serde(default)]
    uuid: String,
    #[serde(default)]
    name: String,
}

#[async_trait::async_trait]
impl OAuthProviderAdapter for ClaudeOAuth {
    fn provider(&self) -> OAuthProviderId {
        OAuthProviderId::Claude
    }

    fn default_redirect_uri(&self) -> &str {
        CLAUDE_REDIRECT_URI
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
            OAuthError::InvalidConfiguration("Claude authorization URL is invalid".to_owned())
        })?;
        url.query_pairs_mut()
            .append_pair("code", "true")
            .append_pair("client_id", CLAUDE_CLIENT_ID)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("scope", CLAUDE_OAUTH_SCOPE)
            .append_pair("code_challenge", &pkce.challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state);
        Ok(OAuthAuthorization {
            provider: OAuthProviderId::Claude,
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
        let raw_code = callback_code(callback)?;
        let (code, embedded_state) = raw_code
            .split_once('#')
            .map_or((raw_code.as_str(), ""), |(code, state)| (code, state));
        let code = required_nonempty(code, "code")?;
        let redirect_uri = required_nonempty(redirect_uri, "redirect_uri")?;
        let pkce_verifier = PkceCodes::from_verifier(pkce_verifier.to_owned())?.verifier;
        let state = callback.state.as_deref().unwrap_or(embedded_state);
        let body = ClaudeCodeExchangeRequest {
            grant_type: "authorization_code",
            code: &code,
            redirect_uri: &redirect_uri,
            client_id: CLAUDE_CLIENT_ID,
            code_verifier: &pkce_verifier,
            state,
        };
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/json")
            .header("user-agent", "axios/1.15.2")
            .json(&body)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Claude,
                source,
            })?;
        let mut token = self.parse_token_response(response, None).await?;
        if let Ok(profile) = self.fetch_profile(&token.access_token).await {
            merge_profile(&mut token, &profile);
        }
        let _ = self.fetch_roles(&token.access_token).await;
        Ok(token)
    }

    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError> {
        if token.provider != OAuthProviderId::Claude {
            return Err(OAuthError::Request(
                "token provider does not match Claude adapter".to_owned(),
            ));
        }
        let refresh_token = token
            .refresh_token
            .as_deref()
            .ok_or_else(|| OAuthError::Request("Claude token has no refresh token".to_owned()))?;
        let body = ClaudeRefreshRequest {
            client_id: CLAUDE_CLIENT_ID,
            grant_type: "refresh_token",
            refresh_token,
            scope: CLAUDE_OAUTH_SCOPE,
        };
        let response = self
            .client
            .post(&self.endpoints.token_url)
            .header("accept", "application/json, text/plain, */*")
            .header("content-type", "application/json")
            .header("user-agent", "axios/1.15.2")
            .json(&body)
            .send()
            .await
            .map_err(|source| OAuthError::Http {
                provider: OAuthProviderId::Claude,
                source,
            })?;
        let mut refreshed = self.parse_token_response(response, Some(token)).await?;
        if let Ok(profile) = self.fetch_profile(&refreshed.access_token).await {
            merge_profile(&mut refreshed, &profile);
        }
        Ok(refreshed)
    }
}

impl ClaudeOAuth {
    async fn parse_token_response(
        &self,
        response: Response,
        previous: Option<&OAuthToken>,
    ) -> Result<OAuthToken, OAuthError> {
        let payload =
            parse_json_response::<ClaudeTokenResponse>(response, OAuthProviderId::Claude, "token")
                .await?;
        let access_token = required_nonempty(&payload.access_token, "access_token")?;
        let account_id = if !payload.account.uuid.trim().is_empty() {
            payload.account.uuid.clone()
        } else if !payload.account.email_address.trim().is_empty() {
            payload.account.email_address.clone()
        } else if let Some(previous) = previous {
            previous.account_id.clone()
        } else {
            "claude-default".to_owned()
        };
        let mut metadata = previous
            .map(|token| token.metadata.clone())
            .unwrap_or_default();
        insert_nonempty(&mut metadata, "email", &payload.account.email_address);
        insert_nonempty(&mut metadata, "account_uuid", &payload.account.uuid);
        insert_nonempty(
            &mut metadata,
            "organization_uuid",
            &payload.organization.uuid,
        );
        insert_nonempty(
            &mut metadata,
            "organization_name",
            &payload.organization.name,
        );
        let refresh_token = payload
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| previous.and_then(|value| value.refresh_token.clone()));
        let expires_at = payload
            .expires_in
            .filter(|seconds| *seconds > 0)
            .map(|seconds| Utc::now() + Duration::seconds(seconds));
        Ok(OAuthToken {
            provider: OAuthProviderId::Claude,
            account_id,
            access_token,
            refresh_token,
            token_type: payload
                .token_type
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "Bearer".to_owned()),
            expires_at,
            scopes: CLAUDE_OAUTH_SCOPE
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect(),
            metadata,
        })
    }
}

async fn parse_json_response<T: for<'de> Deserialize<'de>>(
    response: Response,
    provider: OAuthProviderId,
    operation: &str,
) -> Result<T, OAuthError> {
    if !response.status().is_success() {
        return Err(OAuthError::Provider {
            provider,
            status: response.status().as_u16(),
        });
    }
    response
        .json::<T>()
        .await
        .map_err(|_| OAuthError::InvalidResponse {
            provider,
            message: format!("{operation} response was not valid JSON"),
        })
}

fn insert_nonempty(
    metadata: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: &str,
) {
    if !value.trim().is_empty() {
        metadata.insert(key.to_owned(), serde_json::Value::String(value.to_owned()));
    }
}

fn merge_profile(token: &mut OAuthToken, profile: &ClaudeProfile) {
    if !profile.account.uuid.trim().is_empty() {
        token.account_id = profile.account.uuid.clone();
    } else if token.account_id == "claude-default" && !profile.account.email.trim().is_empty() {
        token.account_id = profile.account.email.clone();
    }
    insert_nonempty(&mut token.metadata, "email", &profile.account.email);
    insert_nonempty(&mut token.metadata, "account_uuid", &profile.account.uuid);
    insert_nonempty(
        &mut token.metadata,
        "organization_uuid",
        &profile.organization.uuid,
    );
    insert_nonempty(
        &mut token.metadata,
        "organization_name",
        &profile.organization.name,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::State,
        http::HeaderMap,
        response::IntoResponse,
        routing::{get, post},
        Router,
    };
    use std::sync::{Arc, Mutex};
    use tokio::net::TcpListener;

    #[derive(Clone, Default)]
    struct MockState {
        body: Arc<Mutex<String>>,
        profile: Arc<Mutex<String>>,
    }

    async fn token_handler(
        State(state): State<MockState>,
        headers: HeaderMap,
    ) -> impl IntoResponse {
        assert_eq!(
            headers
                .get("content-type")
                .and_then(|value| value.to_str().ok()),
            Some("application/json")
        );
        (
            axum::http::StatusCode::OK,
            [("content-type", "application/json")],
            state.body.lock().unwrap().clone(),
        )
    }

    async fn profile_handler(State(state): State<MockState>) -> impl IntoResponse {
        (
            axum::http::StatusCode::OK,
            [("content-type", "application/json")],
            state.profile.lock().unwrap().clone(),
        )
    }

    async fn mock_server(body: &str, profile: &str) -> (String, String) {
        let state = MockState {
            body: Arc::new(Mutex::new(body.to_owned())),
            profile: Arc::new(Mutex::new(profile.to_owned())),
        };
        let router = Router::new()
            .route("/token", post(token_handler))
            .route("/profile", get(profile_handler))
            .route(
                "/roles",
                get(|| async {
                    (
                        axum::http::StatusCode::OK,
                        [("content-type", "application/json")],
                        "{}",
                    )
                }),
            )
            .with_state(state);
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        (
            format!("http://{address}/token"),
            format!("http://{address}/profile"),
        )
    }

    #[test]
    fn authorization_url_has_claude_code_parameters() {
        let adapter = ClaudeOAuth::new(Client::new());
        let pkce = PkceCodes::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let auth = adapter
            .authorization_url("http://127.0.0.1:54545/callback", "state", &pkce)
            .unwrap();
        let query: std::collections::BTreeMap<_, _> =
            auth.authorization_url.query_pairs().into_owned().collect();
        assert_eq!(
            query.get("client_id").map(String::as_str),
            Some(CLAUDE_CLIENT_ID)
        );
        assert_eq!(query.get("code").map(String::as_str), Some("true"));
        assert_eq!(
            query.get("scope").map(String::as_str),
            Some(CLAUDE_OAUTH_SCOPE)
        );
    }

    #[tokio::test]
    async fn exchange_and_refresh_preserve_rotating_refresh_token() {
        let (token_url, profile_url) = mock_server(
            r#"{"access_token":"new-access","expires_in":3600,"account":{"uuid":"account-uuid","email_address":"a@example.test"},"organization":{"uuid":"org","name":"Org"}}"#,
            r#"{"account":{"uuid":"profile-uuid","email":"profile@example.test"},"organization":{"uuid":"profile-org","name":"Profile Org"}}"#,
        ).await;
        let adapter = ClaudeOAuth::with_endpoints(
            Client::new(),
            ClaudeOAuthEndpoints {
                token_url,
                profile_url,
                roles_url: "http://127.0.0.1:9/roles".to_owned(),
                ..ClaudeOAuthEndpoints::default()
            },
        );
        let pkce = PkceCodes::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk").unwrap();
        let token = adapter
            .exchange_code(
                &OAuthCallback {
                    code: Some("code#embedded-state".to_owned()),
                    state: None,
                    ..Default::default()
                },
                "http://localhost/callback",
                &pkce.verifier,
            )
            .await
            .unwrap();
        assert_eq!(token.account_id, "profile-uuid");
        assert_eq!(
            token.metadata.get("email").and_then(|value| value.as_str()),
            Some("profile@example.test")
        );

        let refresh = OAuthToken {
            refresh_token: Some("old-refresh".to_owned()),
            ..token
        };
        let refreshed = adapter.refresh(&refresh).await.unwrap();
        assert_eq!(refreshed.refresh_token.as_deref(), Some("old-refresh"));
    }
}
