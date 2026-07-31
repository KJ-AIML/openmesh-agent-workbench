//! Dev Track 0.1.7.5 — Answer receipt storage (append-only).

use crate::domain::ProxyAuthorityLevel;
use crate::storage::get_project_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const ANSWER_RECEIPT_DIR: &str = "proxy/receipts";
const RECEIPT_TEMP_EXTENSION: &str = "receipt-tmp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnswerReceipt {
    pub receipt_id: String,
    pub question_id: String,
    pub question_text: String,
    pub resolved_authority: ProxyAuthorityLevel,
    pub authority_decision_reason: String,
    pub context_pack_id: String,
    pub draft_text: String,
    pub claims_json: String,
    pub freshness_summary: String,
    pub generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub correction_of: Option<String>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AnswerReceiptError {
    #[error("project not initialized")]
    ProjectNotInitialized,
    #[error("receipt not found")]
    NotFound,
    #[error("malformed receipt JSON")]
    MalformedJson,
    #[error("failed to read receipt")]
    ReadFailed,
    #[error("failed to write receipt")]
    WriteFailed,
    #[error("atomic receipt replacement failed")]
    AtomicReplaceFailed,
}

pub trait AnswerReceiptStore {
    fn write(&self, project_path: &str, receipt: &AnswerReceipt) -> Result<(), AnswerReceiptError>;
    fn read(&self, project_path: &str, receipt_id: &str) -> Result<AnswerReceipt, AnswerReceiptError>;
}

#[derive(Debug, Default)]
pub struct FileAnswerReceiptStore;

impl AnswerReceiptStore for FileAnswerReceiptStore {
    fn write(&self, project_path: &str, receipt: &AnswerReceipt) -> Result<(), AnswerReceiptError> {
        write_answer_receipt(project_path, receipt)
    }

    fn read(&self, project_path: &str, receipt_id: &str) -> Result<AnswerReceipt, AnswerReceiptError> {
        read_answer_receipt(project_path, receipt_id)
    }
}

pub fn receipts_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(ANSWER_RECEIPT_DIR)
}

pub fn receipt_path(project_path: &str, receipt_id: &str) -> PathBuf {
    receipts_dir(project_path).join(format!("{receipt_id}.json"))
}

pub fn write_answer_receipt(
    project_path: &str,
    receipt: &AnswerReceipt,
) -> Result<(), AnswerReceiptError> {
    let dir = receipts_dir(project_path);
    fs::create_dir_all(&dir).map_err(|_| AnswerReceiptError::WriteFailed)?;
    let path = receipt_path(project_path, &receipt.receipt_id);
    write_json_atomic(&path, receipt)
}

pub fn read_answer_receipt(
    project_path: &str,
    receipt_id: &str,
) -> Result<AnswerReceipt, AnswerReceiptError> {
    let path = receipt_path(project_path, receipt_id);
    if !path.exists() {
        return Err(AnswerReceiptError::NotFound);
    }
    let content = fs::read_to_string(&path).map_err(|_| AnswerReceiptError::ReadFailed)?;
    serde_json::from_str(&content).map_err(|_| AnswerReceiptError::MalformedJson)
}

pub fn append_correction(
    project_path: &str,
    original_id: &str,
    mut correction: AnswerReceipt,
) -> Result<AnswerReceipt, AnswerReceiptError> {
    let _original = read_answer_receipt(project_path, original_id)?;
    correction.correction_of = Some(original_id.to_string());
    write_answer_receipt(project_path, &correction)?;
    Ok(correction)
}

fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> Result<(), AnswerReceiptError> {
    let parent = path
        .parent()
        .ok_or(AnswerReceiptError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| AnswerReceiptError::WriteFailed)?;
    let temp = path.with_extension(RECEIPT_TEMP_EXTENSION);
    let json = serde_json::to_string_pretty(value).map_err(|_| AnswerReceiptError::WriteFailed)?;
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| AnswerReceiptError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| AnswerReceiptError::WriteFailed)?;
        file.sync_all().map_err(|_| AnswerReceiptError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| AnswerReceiptError::AtomicReplaceFailed)
}
