//! Local STT adapter stub (P6). Real sherpa-onnx/cpal wiring lands when models ship.

use super::model_manager;
use super::traits::{SttEngine, SttRequest};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct LocalSttEngine {
    pub app_data: PathBuf,
}

impl SttEngine for LocalSttEngine {
    fn transcribe(&self, _request: &SttRequest<'_>) -> Result<String, String> {
        let state = model_manager::load_state(&self.app_data);
        if state.active_id.is_none() {
            return Err(model_manager::status_message(&self.app_data));
        }
        Err(
            "Local STT runtime (sherpa-onnx) is not linked in this build. Deactivate local model or use cloud STT."
                .into(),
        )
    }
}
