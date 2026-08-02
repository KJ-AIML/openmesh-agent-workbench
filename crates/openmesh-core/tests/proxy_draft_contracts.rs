//! Dev Track 0.1.6 Checkpoint A — Ask My Proxy domain and wire contracts (pure).

use openmesh_core::domain::{
    is_supported_proxy_draft_protocol, is_supported_proxy_draft_trace_metadata_protocol,
    is_supported_proxy_prompt_bundle_protocol, is_supported_proxy_question_protocol,
    normalize_proxy_question_text, validate_proxy_draft, validate_proxy_draft_evidence_summary,
    validate_proxy_draft_runtime_metadata, validate_proxy_draft_trace_metadata,
    validate_proxy_prompt_bundle, validate_proxy_question, validate_proxy_question_id,
    validate_proxy_runtime_output, validate_proxy_runtime_request, ProxyDraft,
    ProxyDraftEvidenceSummary, ProxyDraftRuntimeMetadata, ProxyDraftTraceMetadata,
    ProxyDraftValidationError, ProxyPromptBundle, ProxyPromptBundleValidationError, ProxyQuestion,
    ProxyQuestionValidationError, ProxyRuntimeOutput, ProxyRuntimeOutputValidationError,
    ProxyRuntimeRequest, ProxyRuntimeRequestValidationError, MAX_PROXY_DRAFT_LIMITATIONS,
    MAX_PROXY_DRAFT_TEXT_BYTES, MAX_PROXY_QUESTION_TEXT_BYTES, PROXY_DRAFT_AUTHORITY_NOTICE,
    PROXY_DRAFT_CLASSIFICATION, PROXY_DRAFT_EXECUTION_BOUNDARY, PROXY_DRAFT_PROTOCOL_VERSION,
    PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION, PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION,
    PROXY_QUESTION_PROTOCOL_VERSION,
};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

fn sample_evidence_summary() -> ProxyDraftEvidenceSummary {
    let mut source_counts = BTreeMap::new();
    source_counts.insert("continuityItem".into(), 1);
    source_counts.insert("pendingAttention".into(), 1);
    ProxyDraftEvidenceSummary {
        evidence_index_count: 2,
        source_counts,
        secret_items_omitted: 1,
    }
}

fn sample_trace() -> ProxyDraftTraceMetadata {
    ProxyDraftTraceMetadata {
        protocol_version: PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION.to_string(),
        workspace_id: "ws-fixture-0.1.6".into(),
        profile_id: "profile-ws-fixture-0.1.6".into(),
        profile_version: "1.0".into(),
        context_pack_id: "context-pack-fnv1a-6dd176ff3e7276a3".into(),
        build_inputs_hash: "fnv1a-6dd176ff3e7276a3".into(),
        evidence_summary: sample_evidence_summary(),
    }
}

fn sample_runtime() -> ProxyDraftRuntimeMetadata {
    ProxyDraftRuntimeMetadata {
        runtime_kind: "local-stub".into(),
        provider_id: "deterministic-stub".into(),
        model_id: "fixture-model".into(),
        network_used: false,
        duration_ms: 42,
    }
}

fn sample_draft() -> ProxyDraft {
    ProxyDraft {
        protocol_version: PROXY_DRAFT_PROTOCOL_VERSION.to_string(),
        question_id: "proxy-q-1a2b3c4d5e6f7890-1a2b-3".into(),
        generated_at: "2026-07-18T10:00:00Z".into(),
        classification: PROXY_DRAFT_CLASSIFICATION.to_string(),
        draft_text:
            "Based on available context, the fixture task is in progress. This is a draft only."
                .into(),
        authority_notice: PROXY_DRAFT_AUTHORITY_NOTICE.to_string(),
        execution_boundary: PROXY_DRAFT_EXECUTION_BOUNDARY.to_string(),
        trace: sample_trace(),
        runtime: sample_runtime(),
        limitations: vec![
            "draft-only response; no authority execution".into(),
            "aggregate evidence metadata only".into(),
        ],
    }
}

fn sample_question() -> ProxyQuestion {
    ProxyQuestion {
        protocol_version: PROXY_QUESTION_PROTOCOL_VERSION.to_string(),
        question_id: "proxy-q-1a2b3c4d5e6f7890-1a2b-3".into(),
        text: "What is the current status?".into(),
    }
}

fn sample_prompt_bundle() -> ProxyPromptBundle {
    ProxyPromptBundle {
        protocol_version: PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION.to_string(),
        system_message: "You are a local proxy draft assistant.".into(),
        context_json: r#"{"workspaceId":"ws-fixture-0.1.6","limitations":["metadata only"]}"#
            .into(),
        user_message: "What is the current status?".into(),
    }
}

fn sample_runtime_request() -> ProxyRuntimeRequest {
    ProxyRuntimeRequest {
        prompt: sample_prompt_bundle(),
        timeout_ms: 30_000,
        max_output_bytes: MAX_PROXY_DRAFT_TEXT_BYTES as u32,
    }
}

fn sample_runtime_output() -> ProxyRuntimeOutput {
    ProxyRuntimeOutput {
        draft_text: "Draft response text.".into(),
        provider_id: "deterministic-stub".into(),
        model_id: "fixture-model".into(),
        network_used: false,
        duration_ms: 42,
    }
}

#[test]
fn valid_proxy_draft_fixture_round_trips() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let raw = fs::read_to_string(root.join("tests/fixtures/proxy/proxy-draft-valid.json"))
        .expect("read fixture");
    let draft: ProxyDraft = serde_json::from_str(&raw).expect("deserialize fixture");
    validate_proxy_draft(&draft).expect("fixture must validate");
    let json = serde_json::to_string(&draft).expect("serialize");
    let restored: ProxyDraft = serde_json::from_str(&json).expect("round-trip deserialize");
    validate_proxy_draft(&restored).expect("round-trip must validate");
    assert_eq!(restored, draft);
}

#[test]
fn proxy_question_wire_shape_is_exact() {
    let question = sample_question();
    let json = serde_json::to_string(&question).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let keys: Vec<_> = obj.keys().cloned().collect();
    assert_eq!(keys, vec!["protocolVersion", "questionId", "text"]);
}

#[test]
fn proxy_question_rejects_unknown_fields() {
    let json = r#"{
        "protocolVersion": "1.0",
        "questionId": "proxy-q-1a2b3c4d5e6f7890-1a2b-3",
        "text": "hello",
        "byteLength": 5
    }"#;
    let result: Result<ProxyQuestion, _> = serde_json::from_str(json);
    assert!(result.is_err(), "unknown byteLength must fail deserialize");
}

#[test]
fn proxy_question_rejects_wrong_protocol() {
    let mut question = sample_question();
    question.protocol_version = "99.0".into();
    assert!(matches!(
        validate_proxy_question(&question),
        Err(ProxyQuestionValidationError::UnsupportedProtocolVersion { .. })
    ));
}

#[test]
fn proxy_question_rejects_empty_text() {
    let mut question = sample_question();
    question.text = "".into();
    assert_eq!(
        validate_proxy_question(&question),
        Err(ProxyQuestionValidationError::EmptyText)
    );
}

#[test]
fn proxy_question_rejects_whitespace_only_text() {
    let mut question = sample_question();
    question.text = "   \n\t  ".into();
    assert_eq!(
        validate_proxy_question(&question),
        Err(ProxyQuestionValidationError::EmptyText)
    );
}

#[test]
fn proxy_question_accepts_exact_utf8_byte_limit() {
    let mut question = sample_question();
    question.text = "x".repeat(MAX_PROXY_QUESTION_TEXT_BYTES);
    validate_proxy_question(&question).expect("exact byte limit must pass");
}

#[test]
fn proxy_question_rejects_over_utf8_byte_limit() {
    let mut question = sample_question();
    question.text = "x".repeat(MAX_PROXY_QUESTION_TEXT_BYTES + 1);
    assert!(matches!(
        validate_proxy_question(&question),
        Err(ProxyQuestionValidationError::TextTooLong { .. })
    ));
}

#[test]
fn proxy_question_accepts_thai_utf8() {
    let mut question = sample_question();
    question.text = "สถานะปัจจุบันคืออะไร".into();
    validate_proxy_question(&question).expect("Thai UTF-8 question must validate");
}

#[test]
fn proxy_question_id_accepts_frozen_format() {
    validate_proxy_question_id("proxy-q-1a2b3c4d5e6f7890-1a2b-3").expect("frozen format");
}

#[test]
fn proxy_question_id_rejects_invalid_shape() {
    assert!(validate_proxy_question_id("proxy-q-only-two-segments-abc").is_err());
    assert!(validate_proxy_question_id("not-proxy-q-abc-def-ghi").is_err());
    assert!(validate_proxy_question_id("proxy-q-ABCDEF-1234-5678").is_err());
    assert!(validate_proxy_question_id("proxy-q-ghijkl-mnop-zzzz").is_err());
}

#[test]
fn proxy_question_id_rejects_path_separators() {
    assert!(validate_proxy_question_id("proxy-q-abc/def-ghi-jkl").is_err());
    assert!(validate_proxy_question_id("proxy-q-abc\\def-ghi-jkl").is_err());
}

#[test]
fn proxy_question_id_does_not_require_question_fingerprint() {
    let question = sample_question();
    validate_proxy_question(&question).expect("question id is opaque; no fingerprint required");
}

#[test]
fn proxy_prompt_bundle_wire_shape_is_exact() {
    let bundle = sample_prompt_bundle();
    let json = serde_json::to_string(&bundle).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    let mut expected = vec![
        "protocolVersion",
        "systemMessage",
        "contextJson",
        "userMessage",
    ];
    expected.sort();
    assert_eq!(keys, expected);
}

#[test]
fn proxy_prompt_bundle_rejects_invalid_context_json() {
    let mut bundle = sample_prompt_bundle();
    bundle.context_json = "{not-json".into();
    assert!(matches!(
        validate_proxy_prompt_bundle(&bundle),
        Err(ProxyPromptBundleValidationError::InvalidContextJson(_))
    ));
}

#[test]
fn proxy_prompt_bundle_contains_no_prompt_hash_field() {
    let json = serde_json::to_string(&sample_prompt_bundle()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    assert!(!obj.contains_key("promptHash"));
    assert!(!obj.contains_key("promptSemanticHash"));
}

#[test]
fn proxy_prompt_bundle_contains_no_stable_internal_ids() {
    let json = serde_json::to_string(&sample_prompt_bundle()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "\"workspaceid\"",
        "\"profileid\"",
        "\"contextpackid\"",
        "\"buildinputshash\"",
        "\"evidenceref\"",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "prompt bundle must not contain {forbidden}"
        );
    }
}

#[test]
fn proxy_runtime_request_contains_no_trace_authority_or_tool_fields() {
    let json = serde_json::to_string(&sample_runtime_request()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    for forbidden in [
        "workspaceId",
        "profileId",
        "contextPackId",
        "buildInputsHash",
        "evidenceSummary",
        "authorityDecision",
        "approvalResult",
        "executionPermission",
        "tools",
        "toolCalls",
        "credentials",
        "projectPath",
    ] {
        assert!(
            !obj.contains_key(forbidden),
            "runtime request must not contain {forbidden}"
        );
    }
}

#[test]
fn proxy_runtime_request_rejects_zero_timeout() {
    let mut request = sample_runtime_request();
    request.timeout_ms = 0;
    assert!(matches!(
        validate_proxy_runtime_request(&request),
        Err(ProxyRuntimeRequestValidationError::ZeroTimeout)
    ));
}

#[test]
fn proxy_runtime_request_rejects_invalid_output_bound() {
    let mut request = sample_runtime_request();
    request.max_output_bytes = 0;
    assert!(matches!(
        validate_proxy_runtime_request(&request),
        Err(ProxyRuntimeRequestValidationError::ZeroMaxOutputBytes)
    ));

    request = sample_runtime_request();
    request.max_output_bytes = (MAX_PROXY_DRAFT_TEXT_BYTES + 1) as u32;
    assert!(matches!(
        validate_proxy_runtime_request(&request),
        Err(ProxyRuntimeRequestValidationError::MaxOutputBytesTooLarge { .. })
    ));
}

#[test]
fn proxy_runtime_output_wire_shape_is_runtime_owned_only() {
    let output = sample_runtime_output();
    let json = serde_json::to_string(&output).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    let mut expected = vec![
        "draftText",
        "providerId",
        "modelId",
        "networkUsed",
        "durationMs",
    ];
    expected.sort();
    assert_eq!(keys, expected);
}

#[test]
fn proxy_runtime_output_rejects_authority_field_injection() {
    let json = r#"{
        "draftText": "text",
        "providerId": "stub",
        "modelId": "model",
        "networkUsed": false,
        "durationMs": 1,
        "authorityNotice": "forged"
    }"#;
    let result: Result<ProxyRuntimeOutput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn proxy_runtime_output_rejects_trace_field_injection() {
    let json = r#"{
        "draftText": "text",
        "providerId": "stub",
        "modelId": "model",
        "networkUsed": false,
        "durationMs": 1,
        "trace": { "workspaceId": "ws-1" }
    }"#;
    let result: Result<ProxyRuntimeOutput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn proxy_runtime_output_rejects_evidence_summary_injection() {
    let json = r#"{
        "draftText": "text",
        "providerId": "stub",
        "modelId": "model",
        "networkUsed": false,
        "durationMs": 1,
        "evidenceSummary": { "evidenceIndexCount": 1, "sourceCounts": {}, "secretItemsOmitted": 0 }
    }"#;
    let result: Result<ProxyRuntimeOutput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn proxy_draft_evidence_summary_is_aggregate_only() {
    let summary = sample_evidence_summary();
    validate_proxy_draft_evidence_summary(&summary).expect("aggregate summary validates");
    let json = serde_json::to_string(&summary).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    let mut expected = vec!["evidenceIndexCount", "sourceCounts", "secretItemsOmitted"];
    expected.sort();
    assert_eq!(keys, expected);
}

#[test]
fn proxy_draft_evidence_summary_contains_no_evidence_refs() {
    let json = serde_json::to_string(&sample_evidence_summary()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "evidenceref",
        "evidenceid",
        "eventid",
        "signalid",
        "canonicalref",
        "filepath",
        "sourcepath",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "evidence summary must not contain {forbidden}"
        );
    }
}

#[test]
fn proxy_draft_evidence_summary_source_counts_are_deterministic() {
    let mut first = BTreeMap::new();
    first.insert("zebra".into(), 1);
    first.insert("alpha".into(), 2);
    let summary = ProxyDraftEvidenceSummary {
        evidence_index_count: 3,
        source_counts: first,
        secret_items_omitted: 0,
    };
    let json = serde_json::to_string(&summary).expect("serialize");
    assert!(
        json.find("\"alpha\"").expect("alpha") < json.find("\"zebra\"").expect("zebra"),
        "BTreeMap sourceCounts must serialize in deterministic key order"
    );
}

#[test]
fn proxy_draft_evidence_summary_rejects_path_like_source_key() {
    let mut summary = sample_evidence_summary();
    summary.source_counts.insert("docs/readme.md".into(), 1);
    assert!(validate_proxy_draft_evidence_summary(&summary).is_err());
}

#[test]
fn proxy_draft_trace_metadata_wire_shape_is_exact() {
    let trace = sample_trace();
    let json = serde_json::to_string(&trace).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    let mut expected = vec![
        "protocolVersion",
        "workspaceId",
        "profileId",
        "profileVersion",
        "contextPackId",
        "buildInputsHash",
        "evidenceSummary",
    ];
    expected.sort();
    assert_eq!(keys, expected);
}

#[test]
fn proxy_draft_trace_metadata_rejects_unknown_fields() {
    let json = r#"{
        "protocolVersion": "1.0",
        "workspaceId": "ws-1",
        "profileId": "profile-1",
        "profileVersion": "1.0",
        "contextPackId": "context-pack-abc",
        "buildInputsHash": "fnv1a-abc",
        "evidenceSummary": {
            "evidenceIndexCount": 0,
            "sourceCounts": {},
            "secretItemsOmitted": 0
        },
        "rawContextPack": {}
    }"#;
    let result: Result<ProxyDraftTraceMetadata, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn proxy_draft_trace_metadata_contains_no_paths() {
    let json = serde_json::to_string(&sample_trace()).expect("serialize");
    assert!(!json.contains("docs/"));
    assert!(!json.contains("\\\\"));
    assert!(!json.contains("openmesh://"));
}

#[test]
fn proxy_draft_runtime_metadata_contains_no_credentials() {
    let json = serde_json::to_string(&sample_runtime()).expect("serialize");
    let lowered = json.to_ascii_lowercase();
    for forbidden in [
        "api_key",
        "password",
        "bearer",
        "token",
        "credential",
        "prompt",
        "responsebody",
        "trace",
        "evidencesummary",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "runtime metadata must not contain {forbidden}"
        );
    }
}

#[test]
fn proxy_draft_wire_shape_is_exact() {
    let draft = sample_draft();
    let json = serde_json::to_string(&draft).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    let mut keys: Vec<_> = obj.keys().cloned().collect();
    keys.sort();
    let mut expected = vec![
        "protocolVersion",
        "questionId",
        "generatedAt",
        "classification",
        "draftText",
        "authorityNotice",
        "executionBoundary",
        "trace",
        "runtime",
        "limitations",
    ];
    expected.sort();
    assert_eq!(keys, expected);
}

#[test]
fn proxy_draft_identity_fields_exist_only_inside_trace() {
    let draft = sample_draft();
    let json = serde_json::to_string(&draft).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let top = value.as_object().expect("top object");
    assert!(!top.contains_key("workspaceId"));
    assert!(!top.contains_key("profileId"));
    assert!(!top.contains_key("profileVersion"));
    assert!(!top.contains_key("contextPackId"));
    assert!(!top.contains_key("buildInputsHash"));
    assert!(!top.contains_key("evidenceSummary"));
    let trace = top.get("trace").and_then(|v| v.as_object()).expect("trace");
    assert!(trace.contains_key("workspaceId"));
    assert!(trace.contains_key("evidenceSummary"));
}

#[test]
fn proxy_draft_rejects_duplicate_top_level_workspace_id() {
    let json = r#"{
        "protocolVersion": "1.0",
        "questionId": "proxy-q-1a2b3c4d5e6f7890-1a2b-3",
        "generatedAt": "2026-07-18T10:00:00Z",
        "classification": "local-proxy-draft",
        "draftText": "draft",
        "authorityNotice": "Policy metadata only — no authority decision was executed.",
        "executionBoundary": "draft-only; no authority execution in 0.1.6",
        "workspaceId": "ws-dup",
        "trace": {
            "protocolVersion": "1.0",
            "workspaceId": "ws-fixture-0.1.6",
            "profileId": "profile-ws-fixture-0.1.6",
            "profileVersion": "1.0",
            "contextPackId": "context-pack-fnv1a-6dd176ff3e7276a3",
            "buildInputsHash": "fnv1a-6dd176ff3e7276a3",
            "evidenceSummary": {
                "evidenceIndexCount": 1,
                "sourceCounts": { "continuityItem": 1 },
                "secretItemsOmitted": 0
            }
        },
        "runtime": {
            "runtimeKind": "local-stub",
            "providerId": "deterministic-stub",
            "modelId": "fixture-model",
            "networkUsed": false,
            "durationMs": 42
        },
        "limitations": ["one"]
    }"#;
    let result: Result<ProxyDraft, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn proxy_draft_rejects_wrong_classification() {
    let mut draft = sample_draft();
    draft.classification = "verified-answer".into();
    assert_eq!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::InvalidClassification)
    );
}

#[test]
fn proxy_draft_rejects_modified_authority_notice() {
    let mut draft = sample_draft();
    draft.authority_notice = "approved by human".into();
    assert_eq!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::InvalidAuthorityNotice)
    );
}

#[test]
fn proxy_draft_rejects_modified_execution_boundary() {
    let mut draft = sample_draft();
    draft.execution_boundary = "execute-actions".into();
    assert_eq!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::InvalidExecutionBoundary)
    );
}

#[test]
fn proxy_draft_rejects_invalid_generated_at() {
    let mut draft = sample_draft();
    draft.generated_at = "2026-07-18T10:00:00-05:00".into();
    assert!(matches!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::InvalidGeneratedAt(_))
    ));
}

#[test]
fn proxy_draft_accepts_exact_draft_byte_limit() {
    let mut draft = sample_draft();
    draft.draft_text = "x".repeat(MAX_PROXY_DRAFT_TEXT_BYTES);
    validate_proxy_draft(&draft).expect("exact draft byte limit");
}

#[test]
fn proxy_draft_rejects_oversized_draft() {
    let mut draft = sample_draft();
    draft.draft_text = "x".repeat(MAX_PROXY_DRAFT_TEXT_BYTES + 1);
    assert!(matches!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::DraftTextTooLong { .. })
    ));
}

#[test]
fn proxy_draft_rejects_too_many_limitations() {
    let mut draft = sample_draft();
    draft.limitations = (0..=MAX_PROXY_DRAFT_LIMITATIONS)
        .map(|i| format!("limit-{i}"))
        .collect();
    assert!(matches!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::TooManyLimitations { .. })
    ));
}

#[test]
fn proxy_draft_rejects_empty_limitation() {
    let mut draft = sample_draft();
    draft.limitations.push("   ".into());
    assert_eq!(
        validate_proxy_draft(&draft),
        Err(ProxyDraftValidationError::EmptyLimitation)
    );
}

#[test]
fn proxy_draft_contains_no_claims_or_citations() {
    let json = serde_json::to_string(&sample_draft()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    assert!(!obj.contains_key("claims"));
    assert!(!obj.contains_key("citations"));
}

#[test]
fn proxy_draft_contains_no_authority_decision_or_approval_fields() {
    let json = serde_json::to_string(&sample_draft()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    for forbidden in [
        "authorityDecision",
        "approvalResult",
        "executionPermission",
        "verifiedAnswer",
        "approved",
    ] {
        assert!(!obj.contains_key(forbidden), "must not contain {forbidden}");
    }
}

#[test]
fn proxy_draft_contains_no_execution_or_tool_fields() {
    let json = serde_json::to_string(&sample_draft()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    for forbidden in [
        "executed",
        "confirmedByHuman",
        "toolCalls",
        "tools",
        "actions",
    ] {
        assert!(!obj.contains_key(forbidden), "must not contain {forbidden}");
    }
}

#[test]
fn proxy_draft_contains_no_conversation_history() {
    let json = serde_json::to_string(&sample_draft()).expect("serialize");
    let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
    let obj = value.as_object().expect("object");
    assert!(!obj.contains_key("conversationHistory"));
}

#[test]
fn validators_are_deterministic() {
    let draft = sample_draft();
    let first = validate_proxy_draft(&draft);
    let second = validate_proxy_draft(&draft);
    assert_eq!(first, second);
    let question = sample_question();
    assert_eq!(
        validate_proxy_question(&question),
        validate_proxy_question(&question)
    );
    assert_eq!(
        normalize_proxy_question_text("  hello  "),
        normalize_proxy_question_text("  hello  ")
    );
}

#[test]
fn validators_perform_no_io() {
    let draft = sample_draft();
    let _ = validate_proxy_draft(&draft);
    let _ = validate_proxy_question(&sample_question());
    let _ = validate_proxy_prompt_bundle(&sample_prompt_bundle());
    let _ = validate_proxy_runtime_request(&sample_runtime_request());
    let _ = validate_proxy_runtime_output(&sample_runtime_output());
    let _ = validate_proxy_draft_trace_metadata(&sample_trace());
    let _ = validate_proxy_draft_runtime_metadata(&sample_runtime());
    let _ = validate_proxy_draft_evidence_summary(&sample_evidence_summary());
    let _ = is_supported_proxy_question_protocol(PROXY_QUESTION_PROTOCOL_VERSION);
    let _ = is_supported_proxy_draft_protocol(PROXY_DRAFT_PROTOCOL_VERSION);
    let _ = is_supported_proxy_draft_trace_metadata_protocol(
        PROXY_DRAFT_TRACE_METADATA_PROTOCOL_VERSION,
    );
    let _ = is_supported_proxy_prompt_bundle_protocol(PROXY_PROMPT_BUNDLE_PROTOCOL_VERSION);
}

#[test]
fn checkpoint_a_adds_no_prompt_composition() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/domain.rs")).expect("read domain");
    let lowered = domain.to_ascii_lowercase();
    for forbidden in [
        "proxypromptcontext",
        "compose_proxy_prompt",
        "map_pack_to_proxy_prompt_context",
        "bound_proxy_prompt_context",
        "processlocalrequestidentityprovider",
        "deterministicrequestidentityprovider",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "domain.rs must not contain prompt composition symbol {forbidden}"
        );
    }
}

#[test]
fn checkpoint_a_adds_no_runtime_behavior() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/domain.rs")).expect("read domain");
    for forbidden in [
        "trait ProxyDraftRuntime",
        "UnconfiguredProxyDraftRuntime",
        "DeterministicStubProxyDraftRuntime",
        "fn generate_draft",
        "ask_my_proxy",
        "resolve_production_proxy_draft_runtime",
    ] {
        assert!(
            !domain.contains(forbidden),
            "domain.rs must not contain runtime behavior symbol {forbidden}"
        );
    }
}

// Lifecycle amendment (Checkpoint E isolation patch): A contract guards scan non-E CLI
// modules only. Shared `main.rs` may register completed checkpoints; E-owned CLI
// files are excluded from this A guard and proven by Checkpoint E tests.
#[test]
fn checkpoint_a_adds_no_cli_behavior() {
    let cli_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../openmesh-cli/src");
    const CHECKPOINT_E_CLI_FILES: &[&str] = &["proxy.rs", "proxy_runtime_factory.rs", "main.rs"];
    if cli_root.exists() {
        for entry in fs::read_dir(&cli_root).expect("read cli src") {
            let path = entry.expect("entry").path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if CHECKPOINT_E_CLI_FILES.contains(&file_name) {
                continue;
            }
            let content = fs::read_to_string(&path).expect("read cli source");
            let lowered = content.to_ascii_lowercase();
            assert!(!lowered.contains("proxy ask"));
            assert!(!lowered.contains("ask_my_proxy"));
        }
    }
}

#[test]
fn checkpoint_a_does_not_start_0_1_7() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let domain = fs::read_to_string(root.join("src/domain.rs")).expect("read domain");
    for marker in [
        "pub struct ProxyClaim",
        "pub struct ProxyCitation",
        "verifiedAnswer",
    ] {
        assert!(
            !domain.contains(marker),
            "domain.rs must not start 0.1.7 type {marker}"
        );
    }
}

#[test]
fn proxy_runtime_output_validation_rejects_empty_draft_text() {
    let mut output = sample_runtime_output();
    output.draft_text = "   ".into();
    assert!(matches!(
        validate_proxy_runtime_output(&output),
        Err(ProxyRuntimeOutputValidationError::EmptyDraftText)
    ));
}

#[test]
fn proxy_question_normalization_trims_without_serialization() {
    let question = ProxyQuestion {
        protocol_version: PROXY_QUESTION_PROTOCOL_VERSION.to_string(),
        question_id: "proxy-q-1a2b3c4d5e6f7890-1a2b-3".into(),
        text: "  hello  ".into(),
    };
    validate_proxy_question(&question).expect("trimmed text validates");
    let json = serde_json::to_string(&question).expect("serialize");
    assert!(json.contains("  hello  "));
    assert!(!json.contains("normalizedText"));
    assert!(!json.contains("byteLength"));
}

#[test]
fn checkpoint_a_does_not_change_tauri_surface() {
    let tauri_lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/src/lib.rs");
    let content = fs::read_to_string(tauri_lib).expect("read tauri lib");
    let count = content.matches("#[tauri::command]").count();
    assert_eq!(count, 53, "Tauri command count must remain 53 (get_host_os)");
}
