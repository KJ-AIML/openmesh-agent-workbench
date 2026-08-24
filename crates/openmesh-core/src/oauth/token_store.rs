//! Token persistence boundaries.
//!
//! The default durable implementation stores the serialized token bundle in
//! the platform credential manager through `keyring`. A memory implementation
//! is provided for tests and ephemeral CLI sessions; no plaintext-file
//! fallback is hidden behind this trait.

use super::{OAuthProviderId, OAuthToken};
use keyring::Entry;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

/// Stable OS credential-manager namespace shared by the desktop and CLI hosts.
/// OAuth access and refresh tokens are never written to the proxy YAML file.
pub const OPENMESH_OAUTH_KEYRING_SERVICE: &str = "com.openmesh.openmesh.oauth";

#[derive(Debug, thiserror::Error)]
pub enum TokenStoreError {
    #[error("invalid token store key: {0}")]
    InvalidKey(String),
    #[error("token serialization failed")]
    Serialization(#[source] serde_json::Error),
    #[error("secure credential store failed")]
    Backend(#[source] keyring::Error),
}

/// Synchronous by design: platform credential APIs are synchronous and hosts
/// can move calls to a blocking executor when they are on an async request
/// path. Keeping this boundary sync also makes accidental async secret races
/// visible in tests.
pub trait OAuthTokenStore: Send + Sync {
    fn load(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<OAuthToken>, TokenStoreError>;
    fn save(&self, token: &OAuthToken) -> Result<(), TokenStoreError>;
    fn delete(&self, provider: OAuthProviderId, account_id: &str) -> Result<(), TokenStoreError>;
}

#[derive(Clone, Default)]
pub struct MemoryTokenStore {
    tokens: Arc<Mutex<BTreeMap<(OAuthProviderId, String), OAuthToken>>>,
}

impl MemoryTokenStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl OAuthTokenStore for MemoryTokenStore {
    fn load(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<OAuthToken>, TokenStoreError> {
        let account_id = normalize_account_id(account_id)?;
        let tokens = self
            .tokens
            .lock()
            .map_err(|_| TokenStoreError::InvalidKey("token store lock poisoned".to_owned()))?;
        Ok(tokens.get(&(provider, account_id)).cloned())
    }

    fn save(&self, token: &OAuthToken) -> Result<(), TokenStoreError> {
        let account_id = normalize_account_id(&token.account_id)?;
        let mut tokens = self
            .tokens
            .lock()
            .map_err(|_| TokenStoreError::InvalidKey("token store lock poisoned".to_owned()))?;
        tokens.insert((token.provider, account_id), token.clone());
        Ok(())
    }

    fn delete(&self, provider: OAuthProviderId, account_id: &str) -> Result<(), TokenStoreError> {
        let account_id = normalize_account_id(account_id)?;
        let mut tokens = self
            .tokens
            .lock()
            .map_err(|_| TokenStoreError::InvalidKey("token store lock poisoned".to_owned()))?;
        tokens.remove(&(provider, account_id));
        Ok(())
    }
}

/// Platform-backed token store using the OS credential manager.
#[derive(Clone, Debug)]
pub struct KeyringTokenStore {
    service: String,
}

impl KeyringTokenStore {
    pub fn new(service: impl Into<String>) -> Result<Self, TokenStoreError> {
        let service = service.into();
        if service.trim().is_empty() {
            return Err(TokenStoreError::InvalidKey(
                "keyring service must not be empty".to_owned(),
            ));
        }
        Ok(Self { service })
    }

    pub fn service(&self) -> &str {
        &self.service
    }

    fn entry(&self, provider: OAuthProviderId, account_id: &str) -> Result<Entry, TokenStoreError> {
        let account_id = normalize_account_id(account_id)?;
        let username = format!("{}:{account_id}", provider.as_str());
        Entry::new(&self.service, &username).map_err(TokenStoreError::Backend)
    }
}

impl OAuthTokenStore for KeyringTokenStore {
    fn load(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<OAuthToken>, TokenStoreError> {
        let entry = self.entry(provider, account_id)?;
        let value = match entry.get_password() {
            Ok(value) => value,
            Err(keyring::Error::NoEntry) => return Ok(None),
            Err(error) => return Err(TokenStoreError::Backend(error)),
        };
        serde_json::from_str(&value)
            .map(Some)
            .map_err(TokenStoreError::Serialization)
    }

    fn save(&self, token: &OAuthToken) -> Result<(), TokenStoreError> {
        let entry = self.entry(token.provider, &token.account_id)?;
        let value = serde_json::to_string(token).map_err(TokenStoreError::Serialization)?;
        entry.set_password(&value).map_err(TokenStoreError::Backend)
    }

    fn delete(&self, provider: OAuthProviderId, account_id: &str) -> Result<(), TokenStoreError> {
        let entry = self.entry(provider, account_id)?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(TokenStoreError::Backend(error)),
        }
    }
}

fn normalize_account_id(account_id: &str) -> Result<String, TokenStoreError> {
    let account_id = account_id.trim();
    if account_id.is_empty() || account_id.len() > 256 || account_id.contains(['\n', '\r', '\0']) {
        return Err(TokenStoreError::InvalidKey(
            "account ID must be 1-256 characters without control separators".to_owned(),
        ));
    }
    Ok(account_id.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn token() -> OAuthToken {
        OAuthToken {
            provider: OAuthProviderId::Claude,
            account_id: "account@example.test".to_owned(),
            access_token: "access-secret".to_owned(),
            refresh_token: Some("refresh-secret".to_owned()),
            token_type: "Bearer".to_owned(),
            expires_at: Some(Utc::now()),
            scopes: vec!["user:profile".to_owned()],
            metadata: serde_json::Map::new(),
        }
    }

    #[test]
    fn memory_store_round_trips_and_deletes_tokens() {
        let store = MemoryTokenStore::new();
        let token = token();
        store.save(&token).unwrap();
        assert_eq!(
            store.load(token.provider, &token.account_id).unwrap(),
            Some(token.clone())
        );
        store.delete(token.provider, &token.account_id).unwrap();
        assert_eq!(store.load(token.provider, &token.account_id).unwrap(), None);
    }

    #[test]
    fn memory_store_rejects_unsafe_account_ids() {
        let store = MemoryTokenStore::new();
        assert!(matches!(
            store.load(OAuthProviderId::Codex, "bad\nkey"),
            Err(TokenStoreError::InvalidKey(_))
        ));
    }

    #[test]
    fn keyring_service_is_validated_without_touching_the_backend() {
        assert!(KeyringTokenStore::new("").is_err());
        assert_eq!(
            KeyringTokenStore::new("openmesh-test").unwrap().service(),
            "openmesh-test"
        );
    }
}
