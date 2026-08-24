//! Shared OAuth contracts for OpenMesh hosts and provider adapters.
//!
//! The module deliberately owns the protocol boundary, while browser opening,
//! desktop notifications, and account routing remain host concerns. Provider
//! adapters are constructed with an injected HTTP client so their wire
//! contracts can be tested without live credentials.

pub mod callback;
pub mod pkce;
pub mod providers;
pub mod token_store;

pub use callback::{
    parse_callback_url, OAuthCallbackHandle, OAuthCallbackServer, OAuthCallbackServerError,
};
pub use pkce::{generate_state, PkceCodes, PkceError};
pub use providers::adapter_for;
pub use token_store::{
    KeyringTokenStore, MemoryTokenStore, OAuthTokenStore, TokenStoreError,
    OPENMESH_OAUTH_KEYRING_SERVICE,
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use url::Url;

/// Provider identifiers used by OAuth storage, management APIs, and routing.
///
/// These IDs are intentionally stable and lower-case. Aliases accepted by
/// `FromStr` are migration conveniences and are not emitted on the wire.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OAuthProviderId {
    Codex,
    Claude,
    Gemini,
    Grok,
    Qwen,
    #[serde(rename = "iflow")]
    IFlow,
    Antigravity,
    Kimi,
}

impl OAuthProviderId {
    pub const ALL: [Self; 8] = [
        Self::Codex,
        Self::Claude,
        Self::Gemini,
        Self::Grok,
        Self::Qwen,
        Self::IFlow,
        Self::Antigravity,
        Self::Kimi,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
            Self::Grok => "grok",
            Self::Qwen => "qwen",
            Self::IFlow => "iflow",
            Self::Antigravity => "antigravity",
            Self::Kimi => "kimi",
        }
    }
}

impl fmt::Display for OAuthProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for OAuthProviderId {
    type Err = OAuthError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "codex" | "openai" => Ok(Self::Codex),
            "claude" | "anthropic" => Ok(Self::Claude),
            "gemini" | "google" | "gemini-cli" => Ok(Self::Gemini),
            "grok" | "xai" | "x-ai" => Ok(Self::Grok),
            "qwen" | "qwen-code" => Ok(Self::Qwen),
            "iflow" | "i-flow" => Ok(Self::IFlow),
            "antigravity" => Ok(Self::Antigravity),
            "kimi" => Ok(Self::Kimi),
            other => Err(OAuthError::UnsupportedProvider(other.to_owned())),
        }
    }
}

/// A persisted provider token. Secret fields are redacted from `Debug`.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct OAuthToken {
    pub provider: OAuthProviderId,
    pub account_id: String,
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

fn default_token_type() -> String {
    "Bearer".to_owned()
}

impl fmt::Debug for OAuthToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthToken")
            .field("provider", &self.provider)
            .field("account_id", &self.account_id)
            .field("access_token", &"<redacted>")
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("token_type", &self.token_type)
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl OAuthToken {
    pub fn needs_refresh(&self, now: DateTime<Utc>, lead: Duration) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= now + lead)
    }

    pub fn authorization_header(&self) -> Option<String> {
        let access_token = self.access_token.trim();
        if access_token.is_empty() {
            return None;
        }
        let token_type = if self.token_type.trim().is_empty() {
            "Bearer"
        } else {
            self.token_type.trim()
        };
        Some(format!("{token_type} {access_token}"))
    }
}

/// The result of preparing an interactive authorization request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OAuthAuthorization {
    pub provider: OAuthProviderId,
    pub authorization_url: Url,
    pub redirect_uri: String,
    pub state: String,
}

/// Parsed result delivered by a local OAuth callback server.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OAuthCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// Generic device authorization state. Provider-specific values can be kept in
/// `metadata` without making the shared contract guess at provider semantics.
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct OAuthDeviceCode {
    pub provider: OAuthProviderId,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub interval_seconds: u64,
    #[serde(default)]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl fmt::Debug for OAuthDeviceCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OAuthDeviceCode")
            .field("provider", &self.provider)
            .field("device_code", &"<redacted>")
            .field("user_code", &self.user_code)
            .field("verification_uri", &self.verification_uri)
            .field("verification_uri_complete", &self.verification_uri_complete)
            .field("expires_at", &self.expires_at)
            .field("interval_seconds", &self.interval_seconds)
            .field("metadata", &self.metadata)
            .finish()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    #[error("unsupported OAuth provider: {0}")]
    UnsupportedProvider(String),
    #[error("OAuth adapter is unavailable for {0}")]
    AdapterUnavailable(OAuthProviderId),
    #[error("invalid OAuth configuration: {0}")]
    InvalidConfiguration(String),
    #[error("OAuth state did not match the pending authorization")]
    StateMismatch,
    #[error("OAuth callback did not contain an authorization code")]
    MissingAuthorizationCode,
    #[error("{provider} OAuth provider returned HTTP status {status}")]
    Provider {
        provider: OAuthProviderId,
        status: u16,
    },
    #[error("{provider} OAuth request failed")]
    Http {
        provider: OAuthProviderId,
        #[source]
        source: reqwest::Error,
    },
    #[error("invalid {provider} OAuth response: {message}")]
    InvalidResponse {
        provider: OAuthProviderId,
        message: String,
    },
    #[error("unsupported OAuth operation for {0}")]
    UnsupportedOperation(OAuthProviderId),
    #[error("OAuth storage error: {0}")]
    Storage(String),
    #[error("PKCE error: {0}")]
    Pkce(#[from] PkceError),
    #[error("OAuth request is invalid: {0}")]
    Request(String),
    #[error("OAuth callback server error: {0}")]
    Callback(#[from] OAuthCallbackServerError),
}

/// Common provider adapter boundary. Hosts own the pending session and token
/// store; adapters own provider-specific URL and token-exchange wire formats.
#[async_trait]
pub trait OAuthProviderAdapter: Send + Sync {
    fn provider(&self) -> OAuthProviderId;
    fn default_redirect_uri(&self) -> &str;
    fn refresh_lead(&self) -> Duration {
        Duration::minutes(5)
    }
    fn authorization_url(
        &self,
        redirect_uri: &str,
        state: &str,
        pkce: &PkceCodes,
    ) -> Result<OAuthAuthorization, OAuthError>;
    async fn exchange_code(
        &self,
        callback: &OAuthCallback,
        redirect_uri: &str,
        pkce_verifier: &str,
    ) -> Result<OAuthToken, OAuthError>;
    async fn refresh(&self, token: &OAuthToken) -> Result<OAuthToken, OAuthError>;

    /// Start a provider-specific device authorization flow when supported.
    /// Browser/loopback OAuth remains the default operation for adapters that
    /// do not override this method.
    async fn start_device_flow(&self) -> Result<OAuthDeviceCode, OAuthError> {
        Err(OAuthError::UnsupportedOperation(self.provider()))
    }

    /// Poll a device authorization flow once. `Ok(None)` means the user has
    /// not finished authorizing yet; the host controls the polling cadence.
    async fn poll_device_flow(
        &self,
        _device: &OAuthDeviceCode,
    ) -> Result<Option<OAuthToken>, OAuthError> {
        Err(OAuthError::UnsupportedOperation(self.provider()))
    }
}

pub fn required_nonempty(value: &str, field: &str) -> Result<String, OAuthError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(OAuthError::InvalidConfiguration(format!(
            "{field} must not be empty"
        )));
    }
    Ok(value.to_owned())
}

pub fn callback_code(callback: &OAuthCallback) -> Result<String, OAuthError> {
    if let Some(error) = callback.error.as_deref() {
        let description = callback
            .error_description
            .as_deref()
            .unwrap_or("provider rejected authorization");
        return Err(OAuthError::Request(format!("{error}: {description}")));
    }
    callback
        .code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or(OAuthError::MissingAuthorizationCode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_aliases_are_stable() {
        assert_eq!(
            "openai".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::Codex
        );
        assert_eq!(
            "anthropic".parse::<OAuthProviderId>().unwrap(),
            OAuthProviderId::Claude
        );
        assert_eq!(OAuthProviderId::IFlow.to_string(), "iflow");
        assert_eq!(OAuthProviderId::ALL.len(), 8);
    }

    #[test]
    fn token_debug_and_refresh_policy_are_safe() {
        let token = OAuthToken {
            provider: OAuthProviderId::Codex,
            account_id: "account".to_owned(),
            access_token: "access-secret".to_owned(),
            refresh_token: Some("refresh-secret".to_owned()),
            token_type: "Bearer".to_owned(),
            expires_at: Some(Utc::now() + Duration::minutes(1)),
            scopes: vec!["openid".to_owned()],
            metadata: serde_json::Map::new(),
        };
        let debug = format!("{token:?}");
        assert!(!debug.contains("access-secret"));
        assert!(!debug.contains("refresh-secret"));
        assert!(token.needs_refresh(Utc::now(), Duration::minutes(5)));
        assert_eq!(
            token.authorization_header().as_deref(),
            Some("Bearer access-secret")
        );
    }

    #[test]
    fn callback_errors_take_precedence_over_missing_code() {
        let callback = OAuthCallback {
            error: Some("access_denied".to_owned()),
            error_description: Some("cancelled".to_owned()),
            ..OAuthCallback::default()
        };
        assert!(matches!(
            callback_code(&callback),
            Err(OAuthError::Request(_))
        ));
    }
}
