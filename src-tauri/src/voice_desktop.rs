//! OpenMesh Voice — Tauri command surface.
//! Blocking STT/TTS work runs on spawn_blocking so the UI thread doesn't beachball.

use crate::voice::cloud_stt::{DEFAULT_OPENAI_STT, DEFAULT_OPENROUTER_STT};
use crate::voice::model_manager::{self, ModelManifestEntry};
use crate::voice::{
    CloudSttEngine, LocalSttEngine, NativeTtsEngine, SttEngine, SttRequest, TtsEngine,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use openmesh_core::agent_engine::{AgentSecretStore, CascadingSecretStore};
use openmesh_core::storage::{default_settings, read_global, Settings};
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;

fn secrets() -> CascadingSecretStore {
    CascadingSecretStore::default()
}

fn tts() -> NativeTtsEngine {
    NativeTtsEngine
}

fn app_data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Speak `text` via macOS `say` / Windows SpeechSynthesizer and wait until audio ends
/// (so the Voice HUD stays in "speaking" and the mic doesn't reopen mid-reply).
#[tauri::command]
pub async fn voice_speak(text: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let engine = tts();
        engine.speak(&text)?;
        // Cap wait so a stuck synthesizer can't hang forever.
        engine.wait_until_done(Duration::from_secs(45))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn voice_speak_stop() -> Result<(), String> {
    tokio::task::spawn_blocking(|| tts().stop())
        .await
        .map_err(|e| e.to_string())?
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceTranscribeRequest {
    pub audio_base64: String,
    #[serde(default)]
    pub mime: String,
    #[serde(default)]
    pub provider_name: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub prefer_local: Option<bool>,
    #[serde(default)]
    pub stt_model: Option<String>,
    #[serde(default)]
    pub stt_language: Option<String>,
}

#[tauri::command]
pub async fn voice_transcribe(request: VoiceTranscribeRequest) -> Result<String, String> {
    tokio::task::spawn_blocking(move || transcribe_blocking(request))
        .await
        .map_err(|e| e.to_string())?
}

fn transcribe_blocking(request: VoiceTranscribeRequest) -> Result<String, String> {
    let settings = read_global::<Settings>("settings.json").unwrap_or_else(default_settings);
    let bytes = B64
        .decode(request.audio_base64.trim())
        .map_err(|e| format!("invalid audio base64: {e}"))?;

    if request.prefer_local.unwrap_or(false) {
        let local = LocalSttEngine {
            app_data: app_data_dir(),
        };
        let local_req = SttRequest {
            audio: &bytes,
            mime: &request.mime,
            api_key: "",
            provider_name: "local",
            base_url: "",
            model: "",
            language: None,
        };
        if let Ok(text) = local.transcribe(&local_req) {
            return Ok(text);
        }
    }

    let api_key = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
        .filter(|k| !k.trim().is_empty())
        .or_else(|| secrets().get_api_key().ok().flatten())
        .filter(|k| !k.trim().is_empty())
        .ok_or_else(|| {
            "No API key for STT. Save your OpenRouter/OpenAI key in Settings → Provider.".to_string()
        })?;

    let provider = request
        .provider_name
        .as_deref()
        .or(settings.provider.name.as_deref())
        .unwrap_or("");
    let base_hint = request
        .base_url
        .as_deref()
        .or(settings.provider.api_base_url.as_deref())
        .unwrap_or("");

    let model_owned = request
        .stt_model
        .as_deref()
        .or(settings.voice.stt_model.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string();

    let lang_owned = request
        .stt_language
        .as_deref()
        .or(settings.voice.stt_language.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let use_or = looks_openrouter(&api_key, provider, base_hint);
    let model = if model_owned.is_empty() {
        if use_or {
            DEFAULT_OPENROUTER_STT
        } else {
            DEFAULT_OPENAI_STT
        }
    } else {
        model_owned.as_str()
    };

    CloudSttEngine.transcribe(&SttRequest {
        audio: &bytes,
        mime: &request.mime,
        api_key: &api_key,
        provider_name: provider,
        base_url: base_hint,
        model,
        language: lang_owned.as_deref(),
    })
}

fn looks_openrouter(api_key: &str, provider: &str, base_url: &str) -> bool {
    let key = api_key.trim();
    let provider = provider.trim().to_ascii_lowercase();
    let base = base_url.trim().to_ascii_lowercase();
    key.starts_with("sk-or-")
        || provider.contains("openrouter")
        || base.contains("openrouter.ai")
}

#[tauri::command]
pub fn voice_model_catalog() -> Vec<ModelManifestEntry> {
    model_manager::catalog()
}

#[tauri::command]
pub fn voice_model_status() -> String {
    model_manager::status_message(&app_data_dir())
}
