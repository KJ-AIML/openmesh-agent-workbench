use openmesh_core::authority_gate::{
    build_authority_policy_input, run_pre_provider_authority_gate, AuthorityGateOutcome,
};
use openmesh_core::domain::default_work_proxy_profile;

#[test]
fn secret_question_must_ask_before_provider() {
    let profile = default_work_proxy_profile(
        "ws-test",
        "profile-ws-test",
        "owner",
        "dev",
        "2026-07-24T10:00:00Z",
    );
    let outcome = run_pre_provider_authority_gate("What is the API secret?", &profile, "secret");
    assert!(matches!(outcome, AuthorityGateOutcome::MustAsk { .. }));
}

#[test]
fn status_question_proceeds() {
    let profile = default_work_proxy_profile(
        "ws-test",
        "profile-ws-test",
        "owner",
        "dev",
        "2026-07-24T10:00:00Z",
    );
    let outcome =
        run_pre_provider_authority_gate("What is the current project status?", &profile, "status");
    assert!(matches!(outcome, AuthorityGateOutcome::Proceed { .. }));
}

#[test]
fn policy_input_marks_secret_risk() {
    let input = build_authority_policy_input("password rotation plan", "security");
    assert!(input.involves_secret_topic);
}

#[test]
fn commitment_question_must_ask_before_provider() {
    let profile = default_work_proxy_profile(
        "ws-test",
        "profile-ws-test",
        "owner",
        "dev",
        "2026-07-24T10:00:00Z",
    );
    let outcome =
        run_pre_provider_authority_gate("Can we deploy to production now?", &profile, "ops");
    assert!(matches!(outcome, AuthorityGateOutcome::MustAsk { .. }));
}
