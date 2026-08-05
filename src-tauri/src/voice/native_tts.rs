//! Native OS TTS (`say` / PowerShell SpeechSynthesizer).
//! Stop must not beachball the UI; speak must not race with killall.

use super::traits::TtsEngine;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

static SPEAK_CHILD: Mutex<Option<Child>> = Mutex::new(None);

#[derive(Debug, Default)]
pub struct NativeTtsEngine;

impl TtsEngine for NativeTtsEngine {
    fn speak(&self, text: &str) -> Result<(), String> {
        // Stop previous utterance without racing a new `say` (see stop_inner).
        self.stop_inner(false)?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let clipped: String = trimmed.chars().take(240).collect();

        #[cfg(target_os = "macos")]
        {
            let child = Command::new("say")
                .arg(&clipped)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("voice_speak failed: {e}"))?;
            *SPEAK_CHILD.lock().map_err(|e| e.to_string())? = Some(child);
            return Ok(());
        }

        #[cfg(target_os = "windows")]
        {
            let escaped = clipped.replace('\'', "''");
            let child = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    &format!("Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.Speak('{escaped}')"),
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|e| format!("voice_speak failed: {e}"))?;
            *SPEAK_CHILD.lock().map_err(|e| e.to_string())? = Some(child);
            return Ok(());
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        {
            let _ = clipped;
            Err("voice_speak is not supported on this OS yet".into())
        }
    }

    fn stop(&self) -> Result<(), String> {
        // User cancel / barge-in — kill any leftover `say` processes too.
        self.stop_inner(true)
    }
}

impl NativeTtsEngine {
    fn stop_inner(&self, killall_fallback: bool) -> Result<(), String> {
        if let Ok(mut guard) = SPEAK_CHILD.lock() {
            if let Some(mut child) = guard.take() {
                let _ = child.kill();
                thread::spawn(move || {
                    let _ = child.wait();
                });
            }
        }
        if killall_fallback {
            #[cfg(target_os = "macos")]
            {
                // Wait for killall to finish so a following speak() isn't murdered.
                let _ = Command::new("killall")
                    .args(["-9", "say"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .output();
                thread::sleep(Duration::from_millis(40));
            }
        }
        Ok(())
    }

    /// Block until the current `say` / synthesizer child exits (or timeout).
    pub fn wait_until_done(&self, timeout: Duration) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            let mut guard = SPEAK_CHILD.lock().map_err(|e| e.to_string())?;
            let Some(child) = guard.as_mut() else {
                return Ok(());
            };
            match child.try_wait() {
                Ok(Some(_status)) => {
                    let _ = guard.take();
                    return Ok(());
                }
                Ok(None) => {
                    if started.elapsed() >= timeout {
                        // Don't leave a runaway process; stop tracked child only.
                        drop(guard);
                        let _ = self.stop_inner(false);
                        return Ok(());
                    }
                    drop(guard);
                    thread::sleep(Duration::from_millis(80));
                }
                Err(e) => {
                    let _ = guard.take();
                    return Err(e.to_string());
                }
            }
        }
    }
}
