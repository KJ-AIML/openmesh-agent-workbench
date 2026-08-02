//! Storage under `.openmesh/online-proxy/`.

use crate::online_proxy::contract::{
    validate_online_proxy_answer, validate_online_proxy_config, OnlineProxyAnswer, OnlineProxyConfig,
};
use crate::storage::{get_project_dir, read_project, Project};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const ONLINE_PROXY_DIR: &str = "online-proxy";
const CONFIG_FILE: &str = "config.json";
const ANSWERS_DIR: &str = "answers";
const TEMP: &str = "online-proxy-tmp";

#[derive(Debug, thiserror::Error)]
pub enum OnlineProxyStorageError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("config missing")]
    ConfigMissing,
    #[error("answer missing")]
    AnswerMissing,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("io failed")]
    Io,
    #[error("malformed JSON")]
    MalformedJson,
}

pub fn online_proxy_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(ONLINE_PROXY_DIR)
}

pub fn config_path(project_path: &str) -> PathBuf {
    online_proxy_dir(project_path).join(CONFIG_FILE)
}

pub fn answer_path(project_path: &str, answer_id: &str) -> PathBuf {
    online_proxy_dir(project_path)
        .join(ANSWERS_DIR)
        .join(format!("{answer_id}.json"))
}

pub fn write_config(
    project_path: &str,
    cfg: &OnlineProxyConfig,
) -> Result<(), OnlineProxyStorageError> {
    let _ = load_project(project_path)?;
    validate_online_proxy_config(cfg)
        .map_err(|e| OnlineProxyStorageError::Validation(e.to_string()))?;
    fs::create_dir_all(online_proxy_dir(project_path)).map_err(|_| OnlineProxyStorageError::Io)?;
    write_json_atomic(&config_path(project_path), cfg)
}

pub fn read_config(project_path: &str) -> Result<OnlineProxyConfig, OnlineProxyStorageError> {
    let _ = load_project(project_path)?;
    let path = config_path(project_path);
    if !path.exists() {
        return Err(OnlineProxyStorageError::ConfigMissing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| OnlineProxyStorageError::Io)?;
    let cfg: OnlineProxyConfig =
        serde_json::from_str(&raw).map_err(|_| OnlineProxyStorageError::MalformedJson)?;
    validate_online_proxy_config(&cfg)
        .map_err(|e| OnlineProxyStorageError::Validation(e.to_string()))?;
    Ok(cfg)
}

pub fn write_answer(
    project_path: &str,
    answer: &OnlineProxyAnswer,
) -> Result<(), OnlineProxyStorageError> {
    let _ = load_project(project_path)?;
    validate_online_proxy_answer(answer)
        .map_err(|e| OnlineProxyStorageError::Validation(e.to_string()))?;
    let dir = online_proxy_dir(project_path).join(ANSWERS_DIR);
    fs::create_dir_all(&dir).map_err(|_| OnlineProxyStorageError::Io)?;
    write_json_atomic(&answer_path(project_path, &answer.answer_id), answer)
}

pub fn read_answer(
    project_path: &str,
    answer_id: &str,
) -> Result<OnlineProxyAnswer, OnlineProxyStorageError> {
    let _ = load_project(project_path)?;
    let path = answer_path(project_path, answer_id);
    if !path.exists() {
        return Err(OnlineProxyStorageError::AnswerMissing);
    }
    let raw = fs::read_to_string(&path).map_err(|_| OnlineProxyStorageError::Io)?;
    let answer: OnlineProxyAnswer =
        serde_json::from_str(&raw).map_err(|_| OnlineProxyStorageError::MalformedJson)?;
    validate_online_proxy_answer(&answer)
        .map_err(|e| OnlineProxyStorageError::Validation(e.to_string()))?;
    Ok(answer)
}

fn load_project(project_path: &str) -> Result<Project, OnlineProxyStorageError> {
    read_project::<Project>(project_path, "project.json")
        .ok_or(OnlineProxyStorageError::ProjectNotInitialized)
}

fn write_json_atomic<T: serde::Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), OnlineProxyStorageError> {
    let parent = path.parent().ok_or(OnlineProxyStorageError::Io)?;
    fs::create_dir_all(parent).map_err(|_| OnlineProxyStorageError::Io)?;
    let temp = path.with_extension(TEMP);
    let mut json =
        serde_json::to_string_pretty(value).map_err(|_| OnlineProxyStorageError::Io)?;
    json.push('\n');
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| OnlineProxyStorageError::Io)?;
        file.write_all(json.as_bytes())
            .map_err(|_| OnlineProxyStorageError::Io)?;
        file.sync_all().map_err(|_| OnlineProxyStorageError::Io)?;
    }
    fs::rename(&temp, path).map_err(|_| OnlineProxyStorageError::Io)
}
