// ============================================================================
// Production proxy draft runtime factory — Dev Track 0.1.6 Checkpoint E
// ============================================================================

use openmesh_core::proxy_runtime::{ProxyDraftRuntime, UnconfiguredProxyDraftRuntime};
use openmesh_core::proxy_runtime_axga::{
    AxgaAiProviderKind, AxgaAiProxyDraftRuntime, AxgaAiProxyDraftRuntimeConfig,
    AxgaAiRuntimeConfigError, PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC,
    PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK, PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI,
};
use std::collections::BTreeMap;

pub const ENV_OPENMESH_PROXY_PROVIDER: &str = "OPENMESH_PROXY_PROVIDER";
pub const ENV_OPENMESH_PROXY_MODEL: &str = "OPENMESH_PROXY_MODEL";
pub const ENV_OPENAI_API_KEY: &str = "OPENAI_API_KEY";
pub const ENV_ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
pub const ENV_DEEPSEEK_API_KEY: &str = "DEEPSEEK_API_KEY";
pub const ENV_OPENMESH_PROXY_OPENAI_BASE_URL: &str = "OPENMESH_PROXY_OPENAI_BASE_URL";
pub const ENV_OPENMESH_PROXY_DEEPSEEK_BASE_URL: &str = "OPENMESH_PROXY_DEEPSEEK_BASE_URL";

const OPENMESH_SELECTOR_VARS: [&str; 4] = [
    ENV_OPENMESH_PROXY_PROVIDER,
    ENV_OPENMESH_PROXY_MODEL,
    ENV_OPENMESH_PROXY_OPENAI_BASE_URL,
    ENV_OPENMESH_PROXY_DEEPSEEK_BASE_URL,
];

/// Reads process environment variables for production runtime resolution.
pub trait ProxyRuntimeEnvironment: Send + Sync {
    fn get(&self, key: &str) -> Option<String>;

    /// Returns `true` when the key is present but its value is not valid Unicode.
    fn has_non_unicode_value(&self, key: &str) -> bool {
        let _ = key;
        false
    }
}

/// Production environment reader.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessProxyRuntimeEnvironment;

impl ProxyRuntimeEnvironment for ProcessProxyRuntimeEnvironment {
    fn get(&self, key: &str) -> Option<String> {
        match std::env::var(key) {
            Ok(value) => Some(value),
            Err(std::env::VarError::NotPresent) => None,
            Err(std::env::VarError::NotUnicode(_)) => None,
        }
    }

    fn has_non_unicode_value(&self, key: &str) -> bool {
        matches!(std::env::var(key), Err(std::env::VarError::NotUnicode(_)))
    }
}

/// In-memory environment map for deterministic tests (compiled into integration tests via `#[path]`).
#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct MapProxyRuntimeEnvironment {
    values: BTreeMap<String, String>,
}

#[allow(dead_code)]
impl MapProxyRuntimeEnvironment {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.values.insert(key.into(), value.into());
        self
    }

    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }
}

impl ProxyRuntimeEnvironment for MapProxyRuntimeEnvironment {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }
}

/// Typed, secret-safe production runtime factory errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProxyRuntimeFactoryError {
    PartialConfiguration,
    UnsupportedProvider,
    MissingModel,
    MissingCredential,
    InvalidBaseUrl,
    NonUnicodeConfiguration,
    AdapterConfigurationFailed,
}

impl std::fmt::Display for ProxyRuntimeFactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::PartialConfiguration => "proxy runtime configuration is incomplete",
            Self::UnsupportedProvider => "proxy runtime provider is unsupported",
            Self::MissingModel => "proxy runtime model is missing",
            Self::MissingCredential => "proxy runtime provider credential is missing",
            Self::InvalidBaseUrl => "proxy runtime base url is invalid",
            Self::NonUnicodeConfiguration => "proxy runtime environment value is not valid unicode",
            Self::AdapterConfigurationFailed => "proxy runtime adapter configuration failed",
        };
        write!(f, "{message}")
    }
}

/// Resolve the production Ask My Proxy runtime from process environment.
pub fn resolve_production_proxy_draft_runtime(
) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError> {
    resolve_production_proxy_draft_runtime_with_env(&ProcessProxyRuntimeEnvironment)
}

/// Resolve the production runtime using an injectable environment reader.
pub fn resolve_production_proxy_draft_runtime_with_env(
    env: &dyn ProxyRuntimeEnvironment,
) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError> {
    let selector_present = OPENMESH_SELECTOR_VARS
        .iter()
        .any(|key| env_value_present(env, key));

    if !selector_present {
        return Ok(Box::new(UnconfiguredProxyDraftRuntime::new()));
    }

    let provider_raw = required_env(env, ENV_OPENMESH_PROXY_PROVIDER)?;
    let model_raw = required_env(env, ENV_OPENMESH_PROXY_MODEL)?;
    let openai_base_url = optional_env(env, ENV_OPENMESH_PROXY_OPENAI_BASE_URL)?;
    let deepseek_base_url = optional_env(env, ENV_OPENMESH_PROXY_DEEPSEEK_BASE_URL)?;

    let provider = normalize_provider(&provider_raw)?;
    let model_id = normalize_required_value(&model_raw, MissingValueKind::Model)?;

    if matches!(provider, AxgaAiProviderKind::Anthropic)
        && (openai_base_url.is_some() || deepseek_base_url.is_some())
    {
        return Err(ProxyRuntimeFactoryError::InvalidBaseUrl);
    }

    let (api_key, base_url) =
        credential_and_base_url_for_provider(env, provider, openai_base_url, deepseek_base_url)?;

    let config = AxgaAiProxyDraftRuntimeConfig::new(provider, model_id, api_key, base_url)
        .map_err(map_adapter_config_error)?;

    let runtime = AxgaAiProxyDraftRuntime::new(config)
        .map_err(|_| ProxyRuntimeFactoryError::AdapterConfigurationFailed)?;

    Ok(Box::new(runtime))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MissingValueKind {
    Model,
    Credential,
}

fn env_value_present(env: &dyn ProxyRuntimeEnvironment, key: &str) -> bool {
    env.get(key).is_some_and(|value| !value.is_empty())
}

fn required_env(
    env: &dyn ProxyRuntimeEnvironment,
    key: &str,
) -> Result<String, ProxyRuntimeFactoryError> {
    if env.has_non_unicode_value(key) {
        return Err(ProxyRuntimeFactoryError::NonUnicodeConfiguration);
    }
    match env.get(key) {
        Some(value) if !value.is_empty() => Ok(value),
        Some(_) => Err(ProxyRuntimeFactoryError::PartialConfiguration),
        None => Err(ProxyRuntimeFactoryError::PartialConfiguration),
    }
}

fn optional_env(
    env: &dyn ProxyRuntimeEnvironment,
    key: &str,
) -> Result<Option<String>, ProxyRuntimeFactoryError> {
    if env.has_non_unicode_value(key) {
        return Err(ProxyRuntimeFactoryError::NonUnicodeConfiguration);
    }
    match env.get(key) {
        None => Ok(None),
        Some(value) if value.trim().is_empty() => Ok(None),
        Some(value) => Ok(Some(value)),
    }
}

fn normalize_provider(raw: &str) -> Result<AxgaAiProviderKind, ProxyRuntimeFactoryError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "openai" => Ok(AxgaAiProviderKind::OpenAi),
        "anthropic" => Ok(AxgaAiProviderKind::Anthropic),
        "deepseek" => Ok(AxgaAiProviderKind::DeepSeek),
        _ => Err(ProxyRuntimeFactoryError::UnsupportedProvider),
    }
}

fn normalize_required_value(
    raw: &str,
    kind: MissingValueKind,
) -> Result<String, ProxyRuntimeFactoryError> {
    if raw.trim().is_empty() {
        return Err(match kind {
            MissingValueKind::Model => ProxyRuntimeFactoryError::MissingModel,
            MissingValueKind::Credential => ProxyRuntimeFactoryError::MissingCredential,
        });
    }
    Ok(raw.trim().to_string())
}

fn credential_and_base_url_for_provider(
    env: &dyn ProxyRuntimeEnvironment,
    provider: AxgaAiProviderKind,
    openai_base_url: Option<String>,
    deepseek_base_url: Option<String>,
) -> Result<(String, Option<String>), ProxyRuntimeFactoryError> {
    match provider {
        AxgaAiProviderKind::OpenAi => {
            let api_key = required_credential(env, ENV_OPENAI_API_KEY)?;
            validate_base_url_option(openai_base_url.as_deref())?;
            Ok((api_key, openai_base_url))
        }
        AxgaAiProviderKind::Anthropic => {
            let api_key = required_credential(env, ENV_ANTHROPIC_API_KEY)?;
            Ok((api_key, None))
        }
        AxgaAiProviderKind::DeepSeek => {
            let api_key = required_credential(env, ENV_DEEPSEEK_API_KEY)?;
            validate_base_url_option(deepseek_base_url.as_deref())?;
            Ok((api_key, deepseek_base_url))
        }
    }
}

fn required_credential(
    env: &dyn ProxyRuntimeEnvironment,
    key: &str,
) -> Result<String, ProxyRuntimeFactoryError> {
    let value = required_env(env, key)?;
    normalize_required_value(&value, MissingValueKind::Credential)
}

fn validate_base_url_option(base_url: Option<&str>) -> Result<(), ProxyRuntimeFactoryError> {
    let Some(base_url) = base_url else {
        return Ok(());
    };
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://"))
        || trimmed.contains(char::is_whitespace)
    {
        return Err(ProxyRuntimeFactoryError::InvalidBaseUrl);
    }
    if trimmed.contains('@') {
        return Err(ProxyRuntimeFactoryError::InvalidBaseUrl);
    }
    Ok(())
}

fn map_adapter_config_error(err: AxgaAiRuntimeConfigError) -> ProxyRuntimeFactoryError {
    match err {
        AxgaAiRuntimeConfigError::EmptyApiKey => ProxyRuntimeFactoryError::MissingCredential,
        AxgaAiRuntimeConfigError::EmptyModelId => ProxyRuntimeFactoryError::MissingModel,
        AxgaAiRuntimeConfigError::InvalidBaseUrl
        | AxgaAiRuntimeConfigError::UnsupportedBaseUrlForProvider => {
            ProxyRuntimeFactoryError::InvalidBaseUrl
        }
        AxgaAiRuntimeConfigError::ProviderConstructionFailed => {
            ProxyRuntimeFactoryError::AdapterConfigurationFailed
        }
    }
}

/// Expected runtime kind string for a normalized provider value.
/// Maps a normalized provider label to the frozen AXGA runtime kind string (integration tests).
#[allow(dead_code)]
pub fn expected_runtime_kind_for_provider(provider: &str) -> Option<&'static str> {
    match provider.trim().to_ascii_lowercase().as_str() {
        "openai" => Some(PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI),
        "anthropic" => Some(PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC),
        "deepseek" => Some(PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK),
        _ => None,
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use openmesh_core::proxy_runtime::{
        DeterministicStubProxyDraftRuntime, PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED,
    };

    #[test]
    fn absent_selector_returns_unconfigured() {
        let env = MapProxyRuntimeEnvironment::new();
        let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
        assert_eq!(
            runtime.runtime_kind(),
            PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
        );
    }

    #[test]
    fn generic_api_key_does_not_configure() {
        let env = MapProxyRuntimeEnvironment::new().set(ENV_OPENAI_API_KEY, "sk-test");
        let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
        assert_eq!(
            runtime.runtime_kind(),
            PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
        );
    }

    #[test]
    fn factory_never_returns_stub() {
        let env = MapProxyRuntimeEnvironment::new();
        let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
        let _stub: Box<dyn ProxyDraftRuntime> =
            Box::new(DeterministicStubProxyDraftRuntime::new_for_tests());
        assert_ne!(
            runtime.runtime_kind(),
            DeterministicStubProxyDraftRuntime::new_for_tests().runtime_kind()
        );
    }
}
