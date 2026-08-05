//! Cloud STT: OpenRouter Whisper JSON or OpenAI-compatible multipart.

use super::traits::{SttEngine, SttRequest};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use serde_json::json;

pub const DEFAULT_OPENROUTER_STT: &str = "openai/whisper-large-v3";
pub const DEFAULT_OPENAI_STT: &str = "whisper-1";

#[derive(Debug, Default)]
pub struct CloudSttEngine;

#[derive(Debug, Deserialize)]
struct WhisperResponse {
    text: Option<String>,
    error: Option<WhisperErrorBody>,
}

#[derive(Debug, Deserialize)]
struct WhisperErrorBody {
    message: Option<String>,
}

impl SttEngine for CloudSttEngine {
    fn transcribe(&self, request: &SttRequest<'_>) -> Result<String, String> {
        if request.audio.is_empty() {
            return Err("empty audio".into());
        }
        if request.audio.len() > 25 * 1024 * 1024 {
            return Err("audio too large".into());
        }

        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(90))
            .build()
            .map_err(|e| e.to_string())?;

        if looks_like_openrouter(request.api_key, request.provider_name, request.base_url) {
            let model = if request.model.trim().is_empty() {
                DEFAULT_OPENROUTER_STT
            } else {
                request.model.trim()
            };
            transcribe_openrouter(
                &client,
                request.api_key,
                request.audio,
                request.mime,
                model,
                request.language,
            )
        } else {
            let model = openai_model_id(request.model);
            transcribe_openai_multipart(
                &client,
                request.api_key,
                request.base_url,
                request.audio.to_vec(),
                request.mime,
                model,
                request.language,
            )
        }
    }
}

fn openai_model_id(raw: &str) -> &str {
    let t = raw.trim();
    if t.is_empty() {
        return DEFAULT_OPENAI_STT;
    }
    t.strip_prefix("openai/").unwrap_or(t)
}

fn looks_like_openrouter(api_key: &str, provider: &str, base_url: &str) -> bool {
    let key = api_key.trim();
    let provider = provider.trim().to_ascii_lowercase();
    let base = base_url.trim().to_ascii_lowercase();
    key.starts_with("sk-or-")
        || provider.contains("openrouter")
        || base.contains("openrouter.ai")
}

fn transcribe_openrouter(
    client: &reqwest::blocking::Client,
    api_key: &str,
    bytes: &[u8],
    mime: &str,
    model: &str,
    language: Option<&str>,
) -> Result<String, String> {
    let format = mime_to_ext(mime);
    let b64 = B64.encode(bytes);
    let mut body = json!({
        "model": model,
        "input_audio": {
            "data": b64,
            "format": format,
        }
    });
    if let Some(lang) = language.map(str::trim).filter(|s| !s.is_empty()) {
        body["language"] = json!(lang);
    }

    let resp = client
        .post("https://openrouter.ai/api/v1/audio/transcriptions")
        .bearer_auth(api_key.trim())
        .header("HTTP-Referer", "https://openmesh.app")
        .header("X-Title", "OpenMesh Voice")
        .json(&body)
        .send()
        .map_err(|e| format!("OpenRouter STT request failed: {e}"))?;

    let status = resp.status();
    let text_body = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "OpenRouter STT HTTP {status}: {}",
            text_body.chars().take(280).collect::<String>()
        ));
    }
    parse_whisper_text(&text_body)
}

fn transcribe_openai_multipart(
    client: &reqwest::blocking::Client,
    api_key: &str,
    base_hint: &str,
    bytes: Vec<u8>,
    mime: &str,
    model: &str,
    language: Option<&str>,
) -> Result<String, String> {
    let ext = mime_to_ext(mime);
    let filename = format!("utterance.{ext}");
    let file_part = reqwest::blocking::multipart::Part::bytes(bytes)
        .file_name(filename)
        .mime_str(if mime.trim().is_empty() {
            "application/octet-stream"
        } else {
            mime.trim()
        })
        .map_err(|e| e.to_string())?;

    let mut form = reqwest::blocking::multipart::Form::new()
        .text("model", model.to_string())
        .part("file", file_part);
    if let Some(lang) = language.map(str::trim).filter(|s| !s.is_empty()) {
        form = form.text("language", lang.to_string());
    }

    let base = if !base_hint.trim().is_empty() {
        base_hint.trim().to_string()
    } else {
        std::env::var("OPENAI_BASE_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "https://api.openai.com/v1".into())
    };
    let url = format!("{}/audio/transcriptions", base.trim_end_matches('/'));

    let resp = client
        .post(&url)
        .bearer_auth(api_key.trim())
        .multipart(form)
        .send()
        .map_err(|e| format!("Whisper request failed: {e}"))?;

    let status = resp.status();
    let text_body = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!(
            "Whisper HTTP {status}: {}",
            text_body.chars().take(280).collect::<String>()
        ));
    }
    parse_whisper_text(&text_body)
}

fn parse_whisper_text(body: &str) -> Result<String, String> {
    let parsed: WhisperResponse =
        serde_json::from_str(body).map_err(|e| format!("STT parse: {e}; body={body}"))?;
    if let Some(err) = parsed.error {
        return Err(err.message.unwrap_or_else(|| "STT error".into()));
    }
    let text = parsed.text.unwrap_or_default().trim().to_string();
    if text.is_empty() {
        return Err("STT returned empty text".into());
    }
    Ok(text)
}

fn mime_to_ext(mime: &str) -> &'static str {
    let m = mime.to_ascii_lowercase();
    if m.contains("mp4") || m.contains("m4a") {
        "mp4"
    } else if m.contains("ogg") {
        "ogg"
    } else if m.contains("wav") {
        "wav"
    } else if m.contains("mpeg") || m.contains("mp3") {
        "mp3"
    } else {
        "webm"
    }
}
