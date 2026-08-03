//! Dev Track 0.1.12 — online proxy contract tests.

use openmesh_core::authority_freshness::ConfidenceLabel;
use openmesh_core::authority_policy::FreshnessTier;
use openmesh_core::online_proxy::{
    validate_evidence_freshness_statement, validate_online_proxy_answer,
    validate_online_proxy_config, EvidenceFreshnessStatement, OnlineProxyAnswer, OnlineProxyConfig,
    OnlineProxyMode, ONLINE_PROXY_PROTOCOL_VERSION,
};

fn sample_config() -> OnlineProxyConfig {
    OnlineProxyConfig {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        proxy_id: "online-ws-1".into(),
        workspace_id: "ws-1".into(),
        owner_label: "Ter".into(),
        mode: OnlineProxyMode::LocalScaffold,
        default_freshness_tier: FreshnessTier::Standard,
        use_relay_received: true,
        created_at: "2026-08-02T19:00:00Z".into(),
        updated_at: "2026-08-02T19:00:00Z".into(),
    }
}

fn sample_freshness(sufficient: bool) -> EvidenceFreshnessStatement {
    EvidenceFreshnessStatement {
        statement: if sufficient {
            "Evidence freshness: fresh enough for tier Standard (oldest age 60s, confidence High)."
                .into()
        } else {
            "Evidence freshness: stale for tier Critical (oldest age 999999s, confidence Insufficient)."
                .into()
        },
        evaluated_at: "2026-08-02T19:00:00Z".into(),
        tier: if sufficient {
            FreshnessTier::Standard
        } else {
            FreshnessTier::Critical
        },
        is_sufficient: sufficient,
        confidence_label: if sufficient {
            ConfidenceLabel::High
        } else {
            ConfidenceLabel::Insufficient
        },
        oldest_evidence_age_seconds: if sufficient { 60 } else { 999_999 },
        stale_warnings: if sufficient {
            vec![]
        } else {
            vec!["pack snapshot exceeds freshness tier".into()]
        },
        evidence_source_ids: vec!["ref-001".into()],
    }
}

#[test]
fn config_validates() {
    validate_online_proxy_config(&sample_config()).expect("ok");
}

#[test]
fn answer_requires_freshness_words() {
    let mut f = sample_freshness(true);
    f.statement = "all good".into();
    assert!(validate_evidence_freshness_statement(&f).is_err());
}

#[test]
fn insufficient_must_refuse() {
    let ans = OnlineProxyAnswer {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        answer_id: "ans-1".into(),
        proxy_id: "online-ws-1".into(),
        workspace_id: "ws-1".into(),
        question: "status?".into(),
        answer_text: "should refuse".into(),
        generated_at: "2026-08-02T19:00:00Z".into(),
        freshness: sample_freshness(false),
        refused: false,
        mode: OnlineProxyMode::LocalScaffold,
        live_engine: false,
    };
    assert!(validate_online_proxy_answer(&ans).is_err());
}

#[test]
fn live_engine_may_answer_with_stale_disclosure() {
    let ans = OnlineProxyAnswer {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        answer_id: "ans-live".into(),
        proxy_id: "online-ws-1".into(),
        workspace_id: "ws-1".into(),
        question: "status?".into(),
        answer_text: "Live Continuity Proxy answer (Agent Engine): working.".into(),
        generated_at: "2026-08-02T19:00:00Z".into(),
        freshness: sample_freshness(false),
        refused: false,
        mode: OnlineProxyMode::LocalScaffold,
        live_engine: true,
    };
    validate_online_proxy_answer(&ans).expect("live engine soft-warn ok");
}

#[test]
fn refused_stale_answer_ok() {
    let ans = OnlineProxyAnswer {
        protocol_version: ONLINE_PROXY_PROTOCOL_VERSION.into(),
        answer_id: "ans-2".into(),
        proxy_id: "online-ws-1".into(),
        workspace_id: "ws-1".into(),
        question: "status?".into(),
        answer_text: "Cannot answer: evidence is not fresh enough.".into(),
        generated_at: "2026-08-02T19:00:00Z".into(),
        freshness: sample_freshness(false),
        refused: true,
        mode: OnlineProxyMode::LocalScaffold,
        live_engine: false,
    };
    validate_online_proxy_answer(&ans).expect("refused ok");
}
