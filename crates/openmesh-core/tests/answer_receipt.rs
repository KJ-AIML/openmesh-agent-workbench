use openmesh_core::answer_receipt::{append_correction, read_answer_receipt, write_answer_receipt, AnswerReceipt};
use openmesh_core::domain::ProxyAuthorityLevel;
use openmesh_core::storage::init_project;
use std::fs;

fn temp_project(label: &str) -> String {
    let dir = std::env::temp_dir().join(format!(
        "openmesh-answer-receipt-{label}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).unwrap();
    dir.to_string_lossy().to_string()
}

fn sample_receipt(id: &str) -> AnswerReceipt {
    AnswerReceipt {
        receipt_id: id.into(),
        question_id: "q-1".into(),
        question_text: "status?".into(),
        resolved_authority: ProxyAuthorityLevel::CanDraft,
        authority_decision_reason: "ok".into(),
        context_pack_id: "pack-1".into(),
        draft_text: "draft".into(),
        claims_json: "[]".into(),
        freshness_summary: "{}".into(),
        generated_at: "2026-07-24T10:00:00Z".into(),
        correction_of: None,
    }
}

#[test]
fn write_and_read_receipt_roundtrip() {
    let project = temp_project("roundtrip");
    let receipt = sample_receipt("r-1");
    write_answer_receipt(&project, &receipt).expect("write");
    let loaded = read_answer_receipt(&project, "r-1").expect("read");
    assert_eq!(loaded.question_text, "status?");
}

#[test]
fn correction_links_original() {
    let project = temp_project("correction");
    write_answer_receipt(&project, &sample_receipt("r-orig")).expect("write");
    let corrected = append_correction(
        &project,
        "r-orig",
        sample_receipt("r-corr"),
    )
    .expect("correction");
    assert_eq!(corrected.correction_of.as_deref(), Some("r-orig"));
}
