//! Pluggable voice engines (P5–P7).
//! Cloud STT + native OS TTS today; local sherpa/cpal adapters scaffolded (P6).

pub mod audio_input;
pub mod cloud_stt;
pub mod local_stt;
pub mod model_manager;
pub mod native_tts;
pub mod traits;

pub use cloud_stt::CloudSttEngine;
pub use local_stt::LocalSttEngine;
pub use native_tts::NativeTtsEngine;
pub use traits::{SttEngine, SttRequest, TtsEngine};
