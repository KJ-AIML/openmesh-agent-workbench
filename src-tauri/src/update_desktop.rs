//! Download a GitHub release installer and open it with the OS shell.
//! Honest v1: does not replace the running app in-place.

use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

pub const DOWNLOAD_PROGRESS_EVENT: &str = "update-download-progress";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgressPayload {
    received_bytes: u64,
    total_bytes: Option<u64>,
    percent: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallUpdateResult {
    pub path: String,
    pub opened: bool,
    pub next_steps: String,
    pub platform_os: String,
}

/// Host CPU architecture (`aarch64`, `x86_64`, …).
#[tauri::command]
pub fn get_host_arch() -> String {
    std::env::consts::ARCH.to_string()
}

fn sanitize_filename(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("Missing installer filename.".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains("..") {
        return Err("Invalid installer filename.".to_string());
    }
    let ok = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' '));
    if !ok {
        return Err("Invalid installer filename.".to_string());
    }
    Ok(trimmed.to_string())
}

fn allowed_download_url(url: &str) -> bool {
    let lower = url.trim().to_ascii_lowercase();
    if !lower.starts_with("https://") {
        return false;
    }
    (lower.starts_with("https://github.com/") && lower.contains("/releases/download/"))
        || lower.starts_with("https://objects.githubusercontent.com/")
}

fn download_dir() -> Result<PathBuf, String> {
    let base = dirs::download_dir()
        .or_else(dirs::cache_dir)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("OpenMeshUpdates");
    std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create download dir: {e}"))?;
    Ok(dir)
}

fn next_steps_for(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if cfg!(target_os = "macos") || name.ends_with(".dmg") {
        return "Drag OpenMesh into Applications, then clear Gatekeeper quarantine \
(preview builds are unsigned):\n\
xattr -cr /Applications/OpenMesh.app\n\
Or right-click the app → Open. If macOS says the app is damaged, that is usually Gatekeeper — not a corrupt download."
            .to_string();
    }
    if cfg!(target_os = "windows") || name.ends_with(".exe") || name.ends_with(".msi") {
        return "Complete the installer wizard. Windows SmartScreen may warn on unsigned preview builds — choose More info → Run anyway if you trust this GitHub release.".to_string();
    }
    "Open or install the package from your file manager. Preview builds are unsigned.".to_string()
}

fn emit_progress(app: &AppHandle, received: u64, total: Option<u64>) {
    let percent = total.and_then(|t| {
        if t == 0 {
            None
        } else {
            Some(((received.min(t) as f64 / t as f64) * 100.0).round() as u32)
        }
    });
    let _ = app.emit(
        DOWNLOAD_PROGRESS_EVENT,
        DownloadProgressPayload {
            received_bytes: received,
            total_bytes: total,
            percent,
        },
    );
}

fn download_to_file(app: &AppHandle, url: &str, dest: &Path) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|e| format!("Could not start download client: {e}"))?;

    let mut response = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "OpenMesh-Desktop-Update/1.0")
        .send()
        .map_err(|e| format!("Download failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed: HTTP {}.",
            response.status().as_u16()
        ));
    }

    let total = response.content_length();
    let mut file =
        File::create(dest).map_err(|e| format!("Could not write installer file: {e}"))?;
    let mut buf = [0u8; 64 * 1024];
    let mut received: u64 = 0;
    let mut last_emit = 0u64;

    emit_progress(app, 0, total);

    loop {
        let n = response
            .read(&mut buf)
            .map_err(|e| format!("Download interrupted: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Could not write installer file: {e}"))?;
        received += n as u64;
        if received - last_emit >= 256 * 1024 || total == Some(received) {
            emit_progress(app, received, total);
            last_emit = received;
        }
    }

    file.flush()
        .map_err(|e| format!("Could not finalize installer file: {e}"))?;
    emit_progress(app, received, total.or(Some(received)));
    Ok(())
}

/// Download a release asset to the downloads folder and open it with the OS.
#[tauri::command]
pub fn download_and_open_update(
    app: AppHandle,
    url: String,
    filename: String,
) -> Result<InstallUpdateResult, String> {
    let url = url.trim().to_string();
    if url.is_empty() {
        return Err("Missing download URL.".to_string());
    }
    if !allowed_download_url(&url) {
        return Err("Download URL is not an allowed GitHub release asset.".to_string());
    }
    let filename = sanitize_filename(&filename)?;
    let dest = download_dir()?.join(&filename);

    let _ = std::fs::remove_file(&dest);

    download_to_file(&app, &url, &dest)?;

    let next_steps = next_steps_for(&dest);
    match open::that(&dest) {
        Ok(()) => Ok(InstallUpdateResult {
            path: dest.to_string_lossy().to_string(),
            opened: true,
            next_steps,
            platform_os: std::env::consts::OS.to_string(),
        }),
        Err(e) => Ok(InstallUpdateResult {
            path: dest.to_string_lossy().to_string(),
            opened: false,
            next_steps: format!(
                "Downloaded to {}, but could not open automatically ({e}). Open the file manually.\n\n{next_steps}",
                dest.display()
            ),
            platform_os: std::env::consts::OS.to_string(),
        }),
    }
}
