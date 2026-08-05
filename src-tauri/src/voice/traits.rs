//! Voice engine contracts — STT / TTS backends are swappable.

/// Input for a one-shot transcription.
pub struct SttRequest<'a> {
    pub audio: &'a [u8],
    pub mime: &'a str,
    pub api_key: &'a str,
    pub provider_name: &'a str,
    pub base_url: &'a str,
    /// OpenRouter slug or OpenAI model id (e.g. openai/whisper-large-v3).
    pub model: &'a str,
    /// Optional ISO-639-1 language hint.
    pub language: Option<&'a str>,
}

pub trait SttEngine: Send + Sync {
    fn transcribe(&self, request: &SttRequest<'_>) -> Result<String, String>;
}

pub trait TtsEngine: Send + Sync {
    fn speak(&self, text: &str) -> Result<(), String>;
    fn stop(&self) -> Result<(), String>;
}
