//! Agent API key storage — never in project `.openmesh/` JSON.

use super::types::AgentEngineError;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub trait AgentSecretStore: Send + Sync {
    fn get_api_key(&self) -> Result<Option<String>, AgentEngineError>;
    fn set_api_key(&self, key: &str) -> Result<(), AgentEngineError>;
    fn clear_api_key(&self) -> Result<(), AgentEngineError>;
    fn is_configured(&self) -> Result<bool, AgentEngineError> {
        Ok(self.get_api_key()?.map(|k| !k.trim().is_empty()).unwrap_or(false))
    }
}

#[derive(Debug, Default)]
pub struct MemorySecretStore {
    key: std::sync::Mutex<Option<String>>,
}

impl MemorySecretStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_key(key: impl Into<String>) -> Self {
        Self {
            key: std::sync::Mutex::new(Some(key.into())),
        }
    }
}

impl AgentSecretStore for MemorySecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AgentEngineError> {
        Ok(self.key.lock().map_err(|e| AgentEngineError::Io(e.to_string()))?.clone())
    }

    fn set_api_key(&self, key: &str) -> Result<(), AgentEngineError> {
        if key.trim().is_empty() {
            return Err(AgentEngineError::MissingApiKey);
        }
        *self.key.lock().map_err(|e| AgentEngineError::Io(e.to_string()))? = Some(key.to_string());
        Ok(())
    }

    fn clear_api_key(&self) -> Result<(), AgentEngineError> {
        *self.key.lock().map_err(|e| AgentEngineError::Io(e.to_string()))? = None;
        Ok(())
    }
}

/// Env-backed store: `OPENMESH_AGENT_API_KEY` then `OPENAI_API_KEY`.
#[derive(Debug, Default, Clone, Copy)]
pub struct EnvSecretStore;

impl AgentSecretStore for EnvSecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AgentEngineError> {
        for var in ["OPENMESH_AGENT_API_KEY", "OPENAI_API_KEY", "DEEPSEEK_API_KEY"] {
            if let Ok(v) = std::env::var(var) {
                if !v.trim().is_empty() {
                    return Ok(Some(v));
                }
            }
        }
        Ok(None)
    }

    fn set_api_key(&self, _key: &str) -> Result<(), AgentEngineError> {
        Err(AgentEngineError::Io(
            "EnvSecretStore is read-only; set OPENMESH_AGENT_API_KEY".into(),
        ))
    }

    fn clear_api_key(&self) -> Result<(), AgentEngineError> {
        Err(AgentEngineError::Io(
            "EnvSecretStore is read-only".into(),
        ))
    }
}

/// User-level file store (not under project `.openmesh/`).
#[derive(Debug, Clone)]
pub struct FileSecretStore {
    path: PathBuf,
}

impl FileSecretStore {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openmesh")
            .join("agent-api-key")
    }

    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn user_default() -> Self {
        Self::new(Self::default_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl AgentSecretStore for FileSecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AgentEngineError> {
        if !self.path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&self.path).map_err(|e| AgentEngineError::Io(e.to_string()))?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    fn set_api_key(&self, key: &str) -> Result<(), AgentEngineError> {
        if key.trim().is_empty() {
            return Err(AgentEngineError::MissingApiKey);
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| AgentEngineError::Io(e.to_string()))?;
        }
        let mut f = fs::File::create(&self.path).map_err(|e| AgentEngineError::Io(e.to_string()))?;
        f.write_all(key.trim().as_bytes())
            .map_err(|e| AgentEngineError::Io(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600));
        }
        Ok(())
    }

    fn clear_api_key(&self) -> Result<(), AgentEngineError> {
        if self.path.exists() {
            fs::remove_file(&self.path).map_err(|e| AgentEngineError::Io(e.to_string()))?;
        }
        Ok(())
    }
}

/// Prefer file store, fall back to env.
#[derive(Debug, Clone)]
pub struct CascadingSecretStore {
    pub file: FileSecretStore,
}

impl Default for CascadingSecretStore {
    fn default() -> Self {
        Self {
            file: FileSecretStore::user_default(),
        }
    }
}

impl AgentSecretStore for CascadingSecretStore {
    fn get_api_key(&self) -> Result<Option<String>, AgentEngineError> {
        if let Some(k) = self.file.get_api_key()? {
            return Ok(Some(k));
        }
        EnvSecretStore.get_api_key()
    }

    fn set_api_key(&self, key: &str) -> Result<(), AgentEngineError> {
        self.file.set_api_key(key)
    }

    fn clear_api_key(&self) -> Result<(), AgentEngineError> {
        self.file.clear_api_key()
    }
}
