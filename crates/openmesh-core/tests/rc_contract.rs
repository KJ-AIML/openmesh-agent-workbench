//! Dev Track 0.1.21 — RC pack contract tests.

use openmesh_core::rc::{
    validate_rc_pack, RcCheckItem, RcCheckStatus, RcFreezePolicy, RcPack, RcSeverity,
    RC_PROTOCOL_VERSION,
};

fn freeze() -> RcFreezePolicy {
    RcFreezePolicy {
        features_frozen: true,
        allowed: vec!["bugfix".into()],
        forbidden: vec!["features".into()],
        summary: "frozen".into(),
    }
}

fn sample(ready: bool) -> RcPack {
    let status = if ready {
        RcCheckStatus::Pass
    } else {
        RcCheckStatus::Fail
    };
    RcPack {
        protocol_version: RC_PROTOCOL_VERSION.into(),
        workspace_id: "ws".into(),
        generated_at: "2026-08-03T00:00:00Z".into(),
        rc_ready: ready,
        p0_fail_count: if ready { 0 } else { 1 },
        p1_fail_count: 0,
        open_count: 0,
        checks: vec![RcCheckItem {
            id: "c1".into(),
            title: "x".into(),
            severity: RcSeverity::P0,
            status,
            evidence: "e".into(),
            detail: None,
        }],
        regression_matrix: vec![],
        freeze_policy: freeze(),
        limitations: vec![],
    }
}

#[test]
fn ready_pack_ok() {
    assert!(validate_rc_pack(&sample(true)).is_ok());
}

#[test]
fn not_ready_pack_ok() {
    assert!(validate_rc_pack(&sample(false)).is_ok());
}

#[test]
fn freeze_required() {
    let mut p = sample(true);
    p.freeze_policy.features_frozen = false;
    assert!(validate_rc_pack(&p).is_err());
}

#[test]
fn ready_mismatch() {
    let mut p = sample(true);
    p.rc_ready = false;
    assert!(validate_rc_pack(&p).is_err());
}

#[test]
fn serde_roundtrip() {
    let p = sample(true);
    let json = serde_json::to_string(&p).unwrap();
    let back: RcPack = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}
