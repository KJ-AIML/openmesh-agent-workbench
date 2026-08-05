//! Local STT model catalog / install stubs (P6).
//! No large models are downloaded by default — user must opt in later.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelManifestEntry {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub license: String,
    pub sha256: String,
    pub runtime: String,
    pub languages: Vec<String>,
    pub download_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstalledModel {
    pub id: String,
    pub path: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ModelManagerState {
    pub installed: Vec<InstalledModel>,
    pub active_id: Option<String>,
}

pub fn catalog() -> Vec<ModelManifestEntry> {
    vec![
        ModelManifestEntry {
            id: "sherpa-en-small".into(),
            name: "Sherpa English small (placeholder)".into(),
            size_bytes: 75_000_000,
            license: "Apache-2.0 (verify per pack)".into(),
            sha256: "pending".into(),
            runtime: "sherpa-onnx>=1.10".into(),
            languages: vec!["en".into()],
            download_url: None,
        },
        ModelManifestEntry {
            id: "sherpa-th-small".into(),
            name: "Sherpa Thai small (placeholder)".into(),
            size_bytes: 90_000_000,
            license: "Review before install".into(),
            sha256: "pending".into(),
            runtime: "sherpa-onnx>=1.10".into(),
            languages: vec!["th".into()],
            download_url: None,
        },
    ]
}

pub fn models_dir(app_data: &Path) -> PathBuf {
    app_data.join("openmesh").join("voice-models")
}

pub fn load_state(app_data: &Path) -> ModelManagerState {
    let path = models_dir(app_data).join("state.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_state(app_data: &Path, state: &ModelManagerState) -> Result<(), String> {
    let dir = models_dir(app_data);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("state.json");
    let raw = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, raw).map_err(|e| e.to_string())
}

/// Install is intentionally blocked until a signed URL + checksum ship.
pub fn install_model(_app_data: &Path, model_id: &str) -> Result<(), String> {
    Err(format!(
        "Model '{model_id}' is catalogued but download is not enabled yet. Use cloud STT, or ship a signed pack in a later release."
    ))
}

pub fn status_message(app_data: &Path) -> String {
    let state = load_state(app_data);
    if let Some(id) = state.active_id {
        format!("Local STT model active: {id}")
    } else {
        "No local STT model installed. Cloud STT is used. Open Voice settings to review the catalog (downloads opt-in)."
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_nonempty() {
        assert!(!catalog().is_empty());
    }
}
