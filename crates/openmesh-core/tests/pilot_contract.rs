//! Dev Track 0.1.20 — pilot pack contract tests.

use openmesh_core::pilot::{
    validate_pilot_pack, PilotCheckItem, PilotCheckStatus, PilotPack, PILOT_PROTOCOL_VERSION,
};

fn sample_pack(fail: bool) -> PilotPack {
    let checks = vec![
        PilotCheckItem {
            id: "c1".into(),
            title: "ok".into(),
            status: PilotCheckStatus::Pass,
            evidence: "e".into(),
            detail: None,
        },
        PilotCheckItem {
            id: "c2".into(),
            title: "maybe".into(),
            status: if fail {
                PilotCheckStatus::Fail
            } else {
                PilotCheckStatus::Warn
            },
            evidence: "e".into(),
            detail: None,
        },
    ];
    let (pass, warn, fail_n, ready) = if fail {
        (1, 0, 1, false)
    } else {
        (1, 1, 0, true)
    };
    PilotPack {
        protocol_version: PILOT_PROTOCOL_VERSION.into(),
        workspace_id: "ws".into(),
        generated_at: "2026-08-03T00:00:00Z".into(),
        pilot_ready: ready,
        pass_count: pass,
        warn_count: warn,
        fail_count: fail_n,
        checks,
        threat_notes: vec![],
        runbook: vec![],
        limitations: vec![],
    }
}

#[test]
fn valid_ready_pack() {
    assert!(validate_pilot_pack(&sample_pack(false)).is_ok());
}

#[test]
fn valid_not_ready_pack() {
    assert!(validate_pilot_pack(&sample_pack(true)).is_ok());
}

#[test]
fn count_mismatch_fails() {
    let mut p = sample_pack(false);
    p.pass_count = 99;
    assert!(validate_pilot_pack(&p).is_err());
}

#[test]
fn serde_roundtrip() {
    let p = sample_pack(false);
    let json = serde_json::to_string(&p).unwrap();
    let back: PilotPack = serde_json::from_str(&json).unwrap();
    assert_eq!(p, back);
}
