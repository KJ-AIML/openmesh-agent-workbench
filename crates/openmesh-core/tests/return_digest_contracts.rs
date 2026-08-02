//! Dev Track 0.1.9 — return digest / pending questions pure contract tests.

use openmesh_core::domain::{CatchUpSections, CatchUpWindow, EvidenceRef};
use openmesh_core::return_digest::{
    validate_pending_question_item, validate_pending_questions_view, validate_return_digest,
    HandoffDigestRef, PendingQuestionItem, PendingQuestionSourceCounts, PendingQuestionSourceKind,
    PendingQuestionsView, ReturnDigest, PENDING_QUESTIONS_PROTOCOL_VERSION,
    RETURN_DIGEST_PROTOCOL_VERSION,
};

fn valid_item() -> PendingQuestionItem {
    PendingQuestionItem {
        id: "pq-proxy-pending-1".into(),
        summary: "Should we ship the digest CLI?".into(),
        source: PendingQuestionSourceKind::ProxyPending,
        source_id: "pending-abc".into(),
        status: "open".into(),
        severity: "high".into(),
        created_at: "2026-08-01T12:00:00Z".into(),
        reason: "must-ask-human".into(),
        risk: Some("decision".into()),
        resolved_authority: Some("must-ask-human".into()),
        evidence_refs: vec![EvidenceRef::FilePath("docs/plan.md".into())],
    }
}

#[test]
fn pending_question_item_validates() {
    validate_pending_question_item(&valid_item()).expect("valid");
}

#[test]
fn pending_question_item_rejects_empty_summary() {
    let mut item = valid_item();
    item.summary = "  ".into();
    assert!(validate_pending_question_item(&item).is_err());
}

#[test]
fn pending_questions_view_open_count_and_source_counts_must_match() {
    let item = valid_item();
    let view = PendingQuestionsView {
        workspace_id: "ws-1".into(),
        generated_at: "2026-08-01T12:00:00Z".into(),
        protocol_version: PENDING_QUESTIONS_PROTOCOL_VERSION.into(),
        items: vec![item],
        open_count: 1,
        source_counts: PendingQuestionSourceCounts {
            proxy_pending: 1,
            continuity_attention: 0,
            unresolved_signal: 0,
        },
        limitations: vec![],
    };
    validate_pending_questions_view(&view).expect("valid view");

    let mut bad = view.clone();
    bad.open_count = 0;
    assert!(validate_pending_questions_view(&bad).is_err());
}

#[test]
fn return_digest_validates_roundtrip_json() {
    let digest = ReturnDigest {
        workspace_id: "ws-1".into(),
        generated_at: "2026-08-02T12:00:00Z".into(),
        protocol_version: RETURN_DIGEST_PROTOCOL_VERSION.into(),
        window: CatchUpWindow {
            since: "2026-08-01T12:00:00Z".into(),
            until: "2026-08-02T12:00:00Z".into(),
        },
        summary: "Return digest: 1 item(s) need you; 0 continuity item(s) in the absence window; 0 handoff note(s).".into(),
        needs_me: vec![valid_item()],
        what_i_missed: CatchUpSections {
            completed: vec![],
            changed: vec![],
            blocked: vec![],
            decided: vec![],
            needs_attention: vec![],
            still_open: vec![],
        },
        catch_up_summary: "no activity".into(),
        handoffs: vec![HandoffDigestRef {
            handoff_id: "handoff-1".into(),
            status: "draft".into(),
            recipient_label: "future-me".into(),
            created_at: "2026-08-01T10:00:00Z".into(),
            updated_at: "2026-08-01T10:00:00Z".into(),
            window_since: Some("2026-07-25T10:00:00Z".into()),
            window_until: Some("2026-08-01T10:00:00Z".into()),
        }],
        evidence_refs: vec![],
        limitations: vec![],
    };
    validate_return_digest(&digest).expect("valid digest");
    let json = serde_json::to_string(&digest).expect("serialize");
    let restored: ReturnDigest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(restored, digest);
}

#[test]
fn return_digest_rejects_inverted_window() {
    let mut digest = ReturnDigest {
        workspace_id: "ws-1".into(),
        generated_at: "2026-08-02T12:00:00Z".into(),
        protocol_version: RETURN_DIGEST_PROTOCOL_VERSION.into(),
        window: CatchUpWindow {
            since: "2026-08-02T12:00:00Z".into(),
            until: "2026-08-01T12:00:00Z".into(),
        },
        summary: "x".into(),
        needs_me: vec![],
        what_i_missed: CatchUpSections {
            completed: vec![],
            changed: vec![],
            blocked: vec![],
            decided: vec![],
            needs_attention: vec![],
            still_open: vec![],
        },
        catch_up_summary: "x".into(),
        handoffs: vec![],
        evidence_refs: vec![],
        limitations: vec![],
    };
    assert!(validate_return_digest(&digest).is_err());
    digest.window.since = "2026-08-01T12:00:00Z".into();
    digest.window.until = "2026-08-02T12:00:00Z".into();
    validate_return_digest(&digest).expect("fixed");
}
