//! Desktop IPC for OpenMesh skills / hooks / plugins (local MVP).

use openmesh_core::agent_engine::{
    install_from_path, load_inventory, local_catalog, CatalogEntry, ExtensionsInventory,
    ExtensionsSettings,
};
use openmesh_core::storage::{default_settings, read_global, write_global, Settings};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn load_settings() -> Settings {
    read_global::<Settings>("settings.json").unwrap_or_else(default_settings)
}

fn save_settings(settings: &Settings) -> Result<(), String> {
    write_global("settings.json", settings)
}

#[tauri::command]
pub fn extensions_list(project_path: Option<String>) -> Result<ExtensionsInventory, String> {
    let settings = load_settings();
    Ok(load_inventory(
        project_path.as_deref().filter(|p| !p.trim().is_empty()),
        &settings.extensions,
    ))
}

#[tauri::command]
pub fn extensions_catalog(project_path: Option<String>) -> Result<Vec<CatalogEntry>, String> {
    let settings = load_settings();
    let inv = load_inventory(
        project_path.as_deref().filter(|p| !p.trim().is_empty()),
        &settings.extensions,
    );
    Ok(local_catalog(&inv))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsSetEnabledRequest {
    pub kind: String,
    pub id: String,
    pub enabled: bool,
}

#[tauri::command]
pub fn extensions_set_enabled(
    request: ExtensionsSetEnabledRequest,
) -> Result<ExtensionsSettings, String> {
    let mut settings = load_settings();
    match request.kind.as_str() {
        "skill" | "skills" => settings.extensions.set_skill(&request.id, request.enabled),
        "hook" | "hooks" => settings.extensions.set_hook(&request.id, request.enabled),
        "plugin" | "plugins" => settings.extensions.set_plugin(&request.id, request.enabled),
        other => return Err(format!("unknown extension kind: {other}")),
    }
    save_settings(&settings)?;
    Ok(settings.extensions)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionsInstallResult {
    pub installed: String,
    pub path: String,
}

#[tauri::command]
pub fn extensions_install(source_path: String) -> Result<ExtensionsInstallResult, String> {
    let path = PathBuf::from(source_path.trim());
    let installed = install_from_path(&path).map_err(|e| e.to_string())?;
    Ok(ExtensionsInstallResult {
        installed,
        path: path.display().to_string(),
    })
}
