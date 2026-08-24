//! Built-in OAuth provider adapters.

pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod grok;
pub mod kimi;

use super::{OAuthError, OAuthProviderAdapter, OAuthProviderId};
use std::sync::Arc;

pub use antigravity::{
    AntigravityOAuth, AntigravityOAuthCredentials, AntigravityOAuthEndpoints,
    ANTIGRAVITY_CLIENT_ID_ENV, ANTIGRAVITY_CLIENT_SECRET_ENV,
};
pub use claude::{ClaudeOAuth, ClaudeOAuthEndpoints};
pub use codex::{CodexDevicePoll, CodexOAuth, CodexOAuthEndpoints};
pub use grok::{GrokOAuth, GrokOAuthEndpoints};
pub use kimi::{KimiOAuth, KimiOAuthEndpoints};

/// Construct the adapters whose native wire contracts are implemented in this
/// slice. The remaining provider IDs stay visible to management/UI code but
/// return `AdapterUnavailable` until their contracts are pinned and tested.
pub fn built_in_adapters(client: reqwest::Client) -> Vec<Arc<dyn OAuthProviderAdapter>> {
    vec![
        Arc::new(CodexOAuth::new(client.clone())),
        Arc::new(ClaudeOAuth::new(client.clone())),
        Arc::new(AntigravityOAuth::from_environment(client.clone())),
        Arc::new(GrokOAuth::new(client.clone())),
        Arc::new(KimiOAuth::new(client)),
    ]
}

pub fn adapter_for(
    provider: OAuthProviderId,
    client: reqwest::Client,
) -> Result<Arc<dyn OAuthProviderAdapter>, OAuthError> {
    match provider {
        OAuthProviderId::Codex => Ok(Arc::new(CodexOAuth::new(client))),
        OAuthProviderId::Claude => Ok(Arc::new(ClaudeOAuth::new(client))),
        OAuthProviderId::Antigravity => Ok(Arc::new(AntigravityOAuth::from_environment(client))),
        OAuthProviderId::Grok => Ok(Arc::new(GrokOAuth::new(client))),
        OAuthProviderId::Kimi => Ok(Arc::new(KimiOAuth::new(client))),
        _ => Err(OAuthError::AdapterUnavailable(provider)),
    }
}
