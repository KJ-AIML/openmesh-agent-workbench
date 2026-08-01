//! Dev Track 0.1.7.2 — Pending proxy questions created by Must Ask / deny paths.

use crate::authority_policy::{AuthorityPolicyDecision, QuestionRiskCategory};
use crate::domain::ProxyAuthorityLevel;
use crate::storage::get_project_dir;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const PENDING_PROXY_QUESTION_DIR: &str = "proxy/pending";
const PENDING_TEMP_EXTENSION: &str = "pending-tmp";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingProxyQuestion {
    pub pending_id: String,
    pub question_text: String,
    pub risk: String,
    pub resolved_authority: String,
    pub reason: String,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PendingProxyQuestionError {
    #[error("failed to write pending proxy question")]
    WriteFailed,
    #[error("atomic pending write failed")]
    AtomicReplaceFailed,
}

pub fn pending_questions_dir(project_path: &str) -> PathBuf {
    get_project_dir(project_path).join(PENDING_PROXY_QUESTION_DIR)
}

pub fn pending_question_path(project_path: &str, pending_id: &str) -> PathBuf {
    pending_questions_dir(project_path).join(format!("{pending_id}.json"))
}

/// Persist a must-ask / denied question for later human attention (append-only file).
pub fn write_pending_proxy_question(
    project_path: &str,
    question_text: &str,
    risk: QuestionRiskCategory,
    decision: &AuthorityPolicyDecision,
    created_at: &str,
) -> Result<PendingProxyQuestion, PendingProxyQuestionError> {
    let pending_id = format!(
        "pending-{}",
        simple_hash(&format!("{question_text}|{created_at}"))
    );
    let record = PendingProxyQuestion {
        pending_id: pending_id.clone(),
        question_text: question_text.to_string(),
        risk: risk_wire(risk).to_string(),
        resolved_authority: authority_wire(decision.resolved_authority).to_string(),
        reason: decision
            .deny_reason
            .clone()
            .unwrap_or_else(|| decision.decision_reason.clone()),
        created_at: created_at.to_string(),
        status: "open".to_string(),
    };
    let dir = pending_questions_dir(project_path);
    fs::create_dir_all(&dir).map_err(|_| PendingProxyQuestionError::WriteFailed)?;
    let path = pending_question_path(project_path, &pending_id);
    write_json_atomic(&path, &record)?;
    Ok(record)
}

fn risk_wire(risk: QuestionRiskCategory) -> &'static str {
    match risk {
        QuestionRiskCategory::Progress => "progress",
        QuestionRiskCategory::Status => "status",
        QuestionRiskCategory::Decision => "decision",
        QuestionRiskCategory::Commitment => "commitment",
        QuestionRiskCategory::Secret => "secret",
        QuestionRiskCategory::Personal => "personal",
        QuestionRiskCategory::Unknown => "unknown",
    }
}

fn authority_wire(level: ProxyAuthorityLevel) -> &'static str {
    match level {
        ProxyAuthorityLevel::CanAnswer => "can-answer",
        ProxyAuthorityLevel::CanSuggest => "can-suggest",
        ProxyAuthorityLevel::CanDraft => "can-draft",
        ProxyAuthorityLevel::MustAskHuman => "must-ask-human",
        ProxyAuthorityLevel::CannotAnswer => "cannot-answer",
    }
}

fn simple_hash(input: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn write_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), PendingProxyQuestionError> {
    let parent = path
        .parent()
        .ok_or(PendingProxyQuestionError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| PendingProxyQuestionError::WriteFailed)?;
    let temp = path.with_extension(PENDING_TEMP_EXTENSION);
    let json =
        serde_json::to_string_pretty(value).map_err(|_| PendingProxyQuestionError::WriteFailed)?;
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|_| PendingProxyQuestionError::WriteFailed)?;
        file.write_all(json.as_bytes())
            .map_err(|_| PendingProxyQuestionError::WriteFailed)?;
        file.sync_all()
            .map_err(|_| PendingProxyQuestionError::WriteFailed)?;
    }
    fs::rename(&temp, path).map_err(|_| PendingProxyQuestionError::AtomicReplaceFailed)
}
