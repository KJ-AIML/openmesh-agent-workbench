//! CPAL capture seam (P6). WebView MediaRecorder remains the default capture path
//! until a signed local model + cpal pipeline is enabled.

#![allow(dead_code)]

/// Placeholder config for a future Rust-owned mic stream.
#[derive(Debug, Clone)]
pub struct AudioInputConfig {
    pub sample_rate: u32,
    pub channels: u16,
}

impl Default for AudioInputConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16_000,
            channels: 1,
        }
    }
}

/// Describes why native capture is not started yet.
pub fn native_capture_status() -> &'static str {
    "Native cpal capture is scaffolded but disabled. Desktop uses MediaRecorder → Cloud/Local SttEngine."
}
