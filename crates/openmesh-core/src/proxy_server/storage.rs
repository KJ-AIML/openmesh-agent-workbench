use super::{ProxyConfigError, ProxyServerConfig};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PROXY_CONFIG_DIR: &str = "openmesh";
pub const PROXY_CONFIG_FILE: &str = "proxy-config.yaml";

#[derive(Debug, thiserror::Error)]
pub enum ProxyConfigStorageError {
    #[error("could not read proxy config")]
    Read(#[source] std::io::Error),
    #[error("proxy config is malformed")]
    Malformed(#[source] serde_yaml::Error),
    #[error("proxy config is invalid: {0}")]
    Invalid(#[from] ProxyConfigError),
    #[error("could not serialize proxy config")]
    Serialize(#[source] serde_yaml::Error),
    #[error("could not write proxy config")]
    Write(#[source] std::io::Error),
}

pub fn default_proxy_config_path() -> PathBuf {
    dirs::config_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(PROXY_CONFIG_DIR)
        .join(PROXY_CONFIG_FILE)
}

pub fn read_proxy_config(path: &Path) -> Result<ProxyServerConfig, ProxyConfigStorageError> {
    let raw = fs::read_to_string(path).map_err(ProxyConfigStorageError::Read)?;
    let config: ProxyServerConfig =
        serde_yaml::from_str(&raw).map_err(ProxyConfigStorageError::Malformed)?;
    config.validate()?;
    Ok(config)
}

pub fn write_proxy_config(
    path: &Path,
    config: &ProxyServerConfig,
) -> Result<(), ProxyConfigStorageError> {
    config.validate()?;
    let parent = path.parent().ok_or_else(|| {
        ProxyConfigStorageError::Write(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "proxy config path has no parent",
        ))
    })?;
    fs::create_dir_all(parent).map_err(ProxyConfigStorageError::Write)?;
    let mut yaml = serde_yaml::to_string(config).map_err(ProxyConfigStorageError::Serialize)?;
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    let temp_path = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp_path)
            .map_err(ProxyConfigStorageError::Write)?;
        file.write_all(yaml.as_bytes())
            .map_err(ProxyConfigStorageError::Write)?;
        file.sync_all().map_err(ProxyConfigStorageError::Write)?;
    }
    fs::rename(&temp_path, path)
        .map_err(ProxyConfigStorageError::Write)
        .and_then(|()| {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                    .map_err(ProxyConfigStorageError::Write)?;
            }
            Ok(())
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy_server::{ProxyModelConfig, ProxyUpstreamConfig};

    fn config() -> ProxyServerConfig {
        ProxyServerConfig {
            bind_host: "127.0.0.1".to_string(),
            port: 8317,
            api_keys: vec!["client".to_string()],
            allow_unauthenticated: false,
            request_timeout_secs: 30,
            routing_strategy: Default::default(),
            max_retries: crate::proxy_server::DEFAULT_MAX_RETRIES,
            upstreams: vec![ProxyUpstreamConfig {
                id: "openai".to_string(),
                base_url: "https://api.openai.com/v1".to_string(),
                protocol: crate::proxy_server::ProxyProviderProtocol::OpenAiCompatible,
                api_key: Some("provider".to_string()),
                enabled: true,
                priority: 0,
                account_id: None,
                oauth_provider: None,
                models: vec![ProxyModelConfig {
                    id: "gpt-test".to_string(),
                    owned_by: "openai".to_string(),
                    capabilities: vec!["chat".to_string()],
                }],
            }],
            model_aliases: Default::default(),
            model_fallbacks: Default::default(),
        }
    }

    #[test]
    fn yaml_round_trip_preserves_contract() {
        let directory = tempfile::tempdir().expect("temp directory");
        let path = directory.path().join("proxy.yaml");
        write_proxy_config(&path, &config()).expect("write config");
        let loaded = read_proxy_config(&path).expect("read config");
        assert_eq!(loaded.bind_host, "127.0.0.1");
        assert_eq!(loaded.upstreams[0].models[0].id, "gpt-test");
    }

    #[test]
    fn debug_redaction_does_not_change_yaml_persistence() {
        let debug = format!("{:?}", config());
        assert!(!debug.contains("api_key: Some(\"provider\")"));
    }
}
