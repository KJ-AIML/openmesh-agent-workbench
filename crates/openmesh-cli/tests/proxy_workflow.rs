//! Dev Track 0.1.6 Checkpoint E — Ask My Proxy CLI workflow and injection tests.

#![allow(non_snake_case)]

use chrono::Utc;
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::context_pack_storage::{
    context_pack_projections_dir, proxy_context_pack_path, read_proxy_context_pack,
};
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::continuity::current_state_projection_path;
use openmesh_core::domain::{
    CatchUpWindow, EvidenceAttachment, EvidenceRef, ProxyDraft, ProxyRuntimeOutput,
    ProxyRuntimeRequest, WorkEvent, MAX_PROXY_DRAFT_TEXT_BYTES,
};
use openmesh_core::events::{append_event, ledger_dir};
use openmesh_core::profile::work_proxy_profile_path;
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, FixedProxyDraftClock, ProxyAskError, ProxyAskOptions,
};
use openmesh_core::proxy_question::{create_proxy_question, ProxyRequestIdentityProvider};
use openmesh_core::proxy_runtime::{
    DeterministicStubProxyDraftRuntime, ProxyDraftRuntime, ProxyDraftRuntimeError,
    UnconfiguredProxyDraftRuntime,
};
use openmesh_core::storage::init_project;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

static COUNTER: AtomicU64 = AtomicU64::new(0);

const API_KEY_CANARY: &str = "CANARY-API-KEY-9f3a2b1c";
const PROMPT_CANARY: &str = "CANARY-PROMPT-question-text";
const CONTEXT_CANARY: &str = "CANARY-CONTEXT-json-body";
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";
const WINDOW_UNTIL: &str = "2026-07-18T00:00:00Z";
const HUMAN_OUTPUT_HEADER: &str = "Local Work Proxy draft — not the human owner.";
const HUMAN_OUTPUT_ACTION_LINE: &str = "No action was performed.";

fn temp_project(label: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-proxy-workflow-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    dir
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn run(args: &[&str], project: &Path) -> std::process::Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--project").arg(project);
    cmd.output().expect("spawn cli")
}

fn init_profile(project: &Path) {
    let output = run(
        &[
            "profile",
            "init",
            "--owner-label",
            "Owner",
            "--role-label",
            "Role",
        ],
        project,
    );
    assert!(output.status.success());
}

fn seed_event(project: &Path) {
    let project_path = project.to_string_lossy();
    let workspace_id = fs::read_to_string(project.join(".openmesh/project.json"))
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.get("id").and_then(|id| id.as_str().map(str::to_string)))
        .expect("workspace id");
    let event = WorkEvent::new(
        "evt-proxy-workflow-seed",
        &workspace_id,
        "work.completed",
        CONTEXT_CANARY,
        vec![EvidenceAttachment {
            evidence_ref: EvidenceRef::FilePath("docs/overview.md".into()),
            observed_at: None,
        }],
        "2026-07-17T01:00:00Z",
    );
    append_event(&project_path, &event).expect("append");
}

fn fixed_window() -> CatchUpWindow {
    CatchUpWindow {
        since: WINDOW_SINCE.into(),
        until: WINDOW_UNTIL.into(),
    }
}

fn build_ephemeral_pack(project: &Path) -> openmesh_core::domain::ProxyContextPack {
    let project_path = project.to_string_lossy().to_string();
    let options = ProxyContextPackBuildOptions {
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        selection: Default::default(),
    };
    let pack = build_proxy_context_pack(&project_path, fixed_window(), options).expect("build");
    validate_proxy_context_pack_complete(&pack).expect("validate");
    pack
}

fn persist_pack(project: &Path) {
    let output = run(
        &[
            "context",
            "build",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
            "--write",
        ],
        project,
    );
    assert!(output.status.success());
}

struct TestSequenceIdentityProvider {
    sequence: AtomicUsize,
}

impl TestSequenceIdentityProvider {
    fn new() -> Self {
        Self {
            sequence: AtomicUsize::new(1),
        }
    }
}

impl ProxyRequestIdentityProvider for TestSequenceIdentityProvider {
    fn next_question_id(
        &self,
    ) -> Result<String, openmesh_core::proxy_question::ProxyQuestionIdentityError> {
        let value = self.sequence.fetch_add(1, Ordering::SeqCst);
        Ok(format!("proxy-q-deadbeef-{value:04x}-00"))
    }
}

struct CountingRuntime<R> {
    inner: R,
    calls: Arc<AtomicUsize>,
}

impl<R: ProxyDraftRuntime> ProxyDraftRuntime for CountingRuntime<R> {
    fn runtime_kind(&self) -> &'static str {
        self.inner.runtime_kind()
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.generate_draft(request)
    }
}

struct CapturingRuntime {
    captured: Arc<Mutex<Vec<String>>>,
    inner: DeterministicStubProxyDraftRuntime,
}

impl ProxyDraftRuntime for CapturingRuntime {
    fn runtime_kind(&self) -> &'static str {
        self.inner.runtime_kind()
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        self.captured
            .lock()
            .expect("lock")
            .push(serde_json::to_string(&request.prompt).expect("serialize"));
        self.inner.generate_draft(request)
    }
}

struct FixedOutputRuntime {
    output: ProxyRuntimeOutput,
}

impl ProxyDraftRuntime for FixedOutputRuntime {
    fn runtime_kind(&self) -> &'static str {
        "fixed-output-test"
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        openmesh_core::domain::validate_proxy_runtime_request(request)
            .map_err(|_| ProxyDraftRuntimeError::InvalidRequest)?;
        Ok(self.output.clone())
    }
}

struct FailingThenStubRuntime {
    calls: Arc<AtomicUsize>,
    inner: DeterministicStubProxyDraftRuntime,
}

impl ProxyDraftRuntime for FailingThenStubRuntime {
    fn runtime_kind(&self) -> &'static str {
        self.inner.runtime_kind()
    }

    fn generate_draft(
        &self,
        request: &ProxyRuntimeRequest,
    ) -> Result<ProxyRuntimeOutput, ProxyDraftRuntimeError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            return Err(ProxyDraftRuntimeError::ProviderFailure);
        }
        self.inner.generate_draft(request)
    }
}

fn ask_with_stub(
    pack: &openmesh_core::domain::ProxyContextPack,
    question_text: &str,
    runtime: &dyn ProxyDraftRuntime,
    identity: &dyn ProxyRequestIdentityProvider,
    clock: &FixedProxyDraftClock,
    timeout_ms: u64,
) -> Result<ProxyDraft, ProxyAskError> {
    let question = create_proxy_question(question_text, identity)
        .map_err(|_| ProxyAskError::InvalidQuestion)?;
    let options = ProxyAskOptions::new(timeout_ms, MAX_PROXY_DRAFT_TEXT_BYTES as u32);
    ask_my_proxy_local(pack, &question, &options, runtime, clock)
}

fn render_human_output(draft: &ProxyDraft) -> String {
    let mut out = String::new();
    out.push_str(HUMAN_OUTPUT_HEADER);
    out.push('\n');
    out.push_str(&draft.draft_text);
    if !draft.limitations.is_empty() {
        out.push_str("\nlimitations:");
        for limitation in &draft.limitations {
            out.push_str("\n- ");
            out.push_str(limitation);
        }
    }
    out.push_str(&format!(
        "\nruntime_kind={} provider_id={} model_id={} network_used={} duration_ms={}",
        draft.runtime.runtime_kind,
        draft.runtime.provider_id,
        draft.runtime.model_id,
        draft.runtime.network_used,
        draft.runtime.duration_ms
    ));
    out.push('\n');
    out.push_str(HUMAN_OUTPUT_ACTION_LINE);
    out
}

fn proxy_ask_args(question: &str, persisted: bool) -> Vec<String> {
    let mut args = vec![
        "proxy".into(),
        "ask".into(),
        "--question".into(),
        question.into(),
    ];
    if persisted {
        args.push("--from-persisted".into());
    } else {
        args.extend([
            "--since".into(),
            WINDOW_SINCE.into(),
            "--until".into(),
            WINDOW_UNTIL.into(),
        ]);
    }
    args
}

fn run_proxy_cli(args: &[String], project: &Path) -> std::process::Output {
    let mut cmd = cli();
    for arg in args {
        cmd.arg(arg);
    }
    cmd.arg("--project").arg(project);
    cmd.env_remove("OPENMESH_PROXY_PROVIDER");
    cmd.env_remove("OPENMESH_PROXY_MODEL");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.env_remove("DEEPSEEK_API_KEY");
    cmd.output().expect("spawn")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BucketSnapshot {
    pending: usize,
    processed: usize,
    quarantine: usize,
    duplicate: usize,
}

fn bucket_snapshot(signals_root: &Path) -> BucketSnapshot {
    let count = |bucket: &str| -> usize {
        let dir = signals_root.join(bucket);
        if dir.exists() {
            fs::read_dir(dir)
                .map(|entries| entries.count())
                .unwrap_or(0)
        } else {
            0
        }
    };
    BucketSnapshot {
        pending: count("pending"),
        processed: count("processed"),
        quarantine: count("quarantine"),
        duplicate: count("duplicate"),
    }
}

fn file_bytes(path: &Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}

fn proxy_source() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/proxy.rs")).unwrap()
}

fn factory_source() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/proxy_runtime_factory.rs"),
    )
    .unwrap()
}

// --- Workflow tests (40) ---

#[test]
fn ephemeral_valid_pack_and_stub_produce_valid_draft() {
    let project = temp_project("ephemeral-stub");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "What changed recently?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert!(!draft.draft_text.is_empty());
    openmesh_core::domain::validate_proxy_draft(&draft).expect("valid draft");
}

#[test]
fn persisted_valid_pack_and_stub_produce_valid_draft() {
    let project = temp_project("persisted-stub");
    init_profile(&project);
    seed_event(&project);
    persist_pack(&project);
    let pack = read_proxy_context_pack(&project.to_string_lossy()).expect("read");
    validate_proxy_context_pack_complete(&pack).expect("validate");
    let draft = ask_with_stub(
        &pack,
        "Summarize continuity.",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert!(!draft.draft_text.is_empty());
}

#[test]
fn ephemeral_mode_performs_zero_projection_writes() {
    let project = temp_project("ephemeral-no-write");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "hello",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert!(!context_pack_projections_dir(&project.to_string_lossy()).exists());
}

#[test]
fn persisted_mode_does_not_rewrite_pack() {
    let project = temp_project("persisted-no-rewrite");
    init_profile(&project);
    seed_event(&project);
    persist_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    let pack = read_proxy_context_pack(&project.to_string_lossy()).expect("read");
    let _ = ask_with_stub(
        &pack,
        "hello",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(before, file_bytes(&path));
}

#[test]
fn persisted_missing_pack_fails_without_ephemeral_fallback() {
    let project = temp_project("persisted-missing");
    init_profile(&project);
    let args = proxy_ask_args("hello", true);
    let output = run_proxy_cli(&args, &project);
    assert!(!output.status.success());
}

#[test]
fn invalid_persisted_pack_fails_before_runtime() {
    let project = temp_project("persisted-invalid");
    init_profile(&project);
    seed_event(&project);
    persist_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    fs::write(&path, b"{\"invalid\":true}").unwrap();
    let args = proxy_ask_args("hello", true);
    let output = run_proxy_cli(&args, &project);
    assert!(!output.status.success());
}

#[test]
fn invalid_question_fails_before_runtime() {
    let project = temp_project("invalid-question");
    init_profile(&project);
    seed_event(&project);
    let args = proxy_ask_args("   ", false);
    let output = run_proxy_cli(&args, &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("question"));
}

#[test]
fn unconfigured_production_runtime_returns_runtime_not_configured() {
    let project = temp_project("unconfigured");
    init_profile(&project);
    seed_event(&project);
    let args = proxy_ask_args("What is the status?", false);
    let output = run_proxy_cli(&args, &project);
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("runtime-not-configured") || combined.contains("not configured"));
}

#[test]
fn production_factory_is_used_by_production_dispatch() {
    let source = proxy_source();
    assert!(source.contains("resolve_production_proxy_draft_runtime"));
    assert!(source.contains("ProductionProxyRuntimeResolver"));
}

#[test]
fn deterministic_stub_is_used_only_by_test_injection() {
    let proxy = proxy_source();
    let factory = factory_source();
    assert!(!proxy.contains("DeterministicStubProxyDraftRuntime"));
    let production = factory.split("#[cfg(test)]").next().unwrap_or(&factory);
    assert!(!production.contains("DeterministicStubProxyDraftRuntime"));
    let _ = DeterministicStubProxyDraftRuntime::new_for_tests();
}

#[test]
fn runtime_is_invoked_exactly_once() {
    let pack = {
        let project = temp_project("once");
        init_profile(&project);
        seed_event(&project);
        build_ephemeral_pack(&project)
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = CountingRuntime {
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
        calls: Arc::clone(&calls),
    };
    let _ = ask_with_stub(
        &pack,
        "once?",
        &runtime,
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn no_retry_occurs() {
    let pack = {
        let project = temp_project("no-retry");
        init_profile(&project);
        seed_event(&project);
        build_ephemeral_pack(&project)
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let runtime = FailingThenStubRuntime {
        calls: Arc::clone(&calls),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let err = ask_with_stub(
        &pack,
        "retry?",
        &runtime,
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect_err("provider failure");
    assert!(matches!(err, ProxyAskError::ProviderFailure));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn no_fallback_occurs() {
    let pack = {
        let project = temp_project("no-fallback");
        init_profile(&project);
        seed_event(&project);
        build_ephemeral_pack(&project)
    };
    let err = ask_with_stub(
        &pack,
        "fallback?",
        &UnconfiguredProxyDraftRuntime::new(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect_err("unconfigured");
    assert!(matches!(err, ProxyAskError::RuntimeNotConfigured));
}

#[test]
fn question_is_not_persisted() {
    let project = temp_project("question-not-persisted");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let identity = TestSequenceIdentityProvider::new();
    let question = create_proxy_question(PROMPT_CANARY, &identity).expect("question");
    let before = serde_json::to_string(&question).expect("serialize");
    let _ = ask_with_stub(
        &pack,
        PROMPT_CANARY,
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &identity,
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(before, serde_json::to_string(&question).unwrap());
    assert!(!project
        .join(".openmesh")
        .join("proxy-question.json")
        .exists());
}

#[test]
fn draft_is_not_persisted() {
    let project = temp_project("draft-not-persisted");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "persist draft?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    for forbidden in [
        "proxy-draft",
        "proxy-response",
        "proxy-history",
        "draft.json",
    ] {
        assert!(
            !walk_contains(&project, forbidden),
            "unexpected persistence artifact: {forbidden}"
        );
    }
}

fn walk_contains(dir: &Path, needle: &str) -> bool {
    if !dir.exists() {
        return false;
    }
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains(needle))
        {
            return true;
        }
        if path.is_dir() && walk_contains(&path, needle) {
            return true;
        }
    }
    false
}

#[test]
fn no_response_history_is_created() {
    let project = temp_project("no-history");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "history?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert!(!walk_contains(&project, "history"));
}

#[test]
fn profile_bytes_are_unchanged() {
    let project = temp_project("profile-unchanged");
    init_profile(&project);
    seed_event(&project);
    let profile_path = work_proxy_profile_path(&project.to_string_lossy());
    let before = file_bytes(&profile_path);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "profile?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(before, file_bytes(&profile_path));
}

#[test]
fn persisted_context_pack_bytes_are_unchanged() {
    let project = temp_project("pack-unchanged");
    init_profile(&project);
    seed_event(&project);
    persist_pack(&project);
    let path = proxy_context_pack_path(&project.to_string_lossy());
    let before = file_bytes(&path);
    let pack = read_proxy_context_pack(&project.to_string_lossy()).expect("read");
    let _ = ask_with_stub(
        &pack,
        "pack?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(before, file_bytes(&path));
}

#[test]
fn continuity_bytes_are_unchanged() {
    let project = temp_project("continuity-unchanged");
    init_profile(&project);
    seed_event(&project);
    let events_before = fs::read_dir(ledger_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    let state_before = current_state_projection_path(&project.to_string_lossy());
    let state_existed = state_before.exists();
    let state_bytes = file_bytes(&state_before);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "continuity?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(
        events_before,
        fs::read_dir(ledger_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0)
    );
    assert_eq!(state_before.exists(), state_existed);
    if state_existed {
        assert_eq!(state_bytes, file_bytes(&state_before));
    }
}

#[test]
fn signal_buckets_are_unchanged() {
    let project = temp_project("signals-unchanged");
    init_profile(&project);
    seed_event(&project);
    let signals_root = project.join(".openmesh/signals");
    let before = bucket_snapshot(&signals_root);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "signals?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(before, bucket_snapshot(&signals_root));
}

#[test]
fn no_WorkEvent_is_created() {
    let project = temp_project("no-event");
    init_profile(&project);
    seed_event(&project);
    let before = fs::read_dir(ledger_dir(&project.to_string_lossy()))
        .map(|e| e.count())
        .unwrap_or(0);
    let pack = build_ephemeral_pack(&project);
    let _ = ask_with_stub(
        &pack,
        "events?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(
        before,
        fs::read_dir(ledger_dir(&project.to_string_lossy()))
            .map(|e| e.count())
            .unwrap_or(0)
    );
}

#[test]
fn no_authority_execution_occurs() {
    let source = proxy_source();
    for forbidden in ["resolve_profile_authority", "approve", "execute_authority"] {
        assert!(
            !source.contains(forbidden),
            "authority execution: {forbidden}"
        );
    }
}

#[test]
fn no_tool_execution_occurs() {
    let source = proxy_source();
    for forbidden in ["tool_call", "execute_tool", "run_tool"] {
        assert!(!source.contains(forbidden), "tool execution: {forbidden}");
    }
}

#[test]
fn fixed_clock_controls_generated_at() {
    let project = temp_project("clock");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "clock?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2030-01-02T03:04:05Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(draft.generated_at, "2030-01-02T03:04:05Z");
}

#[test]
fn deterministic_identity_provider_controls_question_id_in_tests() {
    let project = temp_project("identity");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let provider = TestSequenceIdentityProvider::new();
    let draft = ask_with_stub(
        &pack,
        "identity?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &provider,
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert_eq!(draft.question_id, "proxy-q-deadbeef-0001-00");
}

#[test]
fn production_identity_provider_is_used_by_production_dispatch() {
    let source = proxy_source();
    assert!(source.contains("ProcessLocalRequestIdentityProvider"));
}

#[test]
fn timeout_secs_maps_to_checked_milliseconds() {
    let secs: u64 = 45;
    let timeout_ms = secs.checked_mul(1_000).expect("overflow");
    let pack = {
        let project = temp_project("timeout");
        init_profile(&project);
        seed_event(&project);
        build_ephemeral_pack(&project)
    };
    let captured = Arc::new(Mutex::new(Vec::new()));
    let runtime = CapturingRuntime {
        captured: Arc::clone(&captured),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let _ = ask_with_stub(
        &pack,
        "timeout?",
        &runtime,
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        timeout_ms,
    )
    .expect("draft");
    assert_eq!(timeout_ms, 45_000);
    assert!(!captured.lock().unwrap().is_empty());
}

#[test]
fn human_output_has_exact_header() {
    let project = temp_project("human-header");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "header?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let rendered = render_human_output(&draft);
    assert!(rendered.starts_with(HUMAN_OUTPUT_HEADER));
}

#[test]
fn human_output_has_exact_final_action_line() {
    let project = temp_project("human-action");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "action?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let rendered = render_human_output(&draft);
    assert!(rendered.ends_with(HUMAN_OUTPUT_ACTION_LINE));
}

#[test]
fn human_output_cannot_be_replaced_by_runtime_text() {
    let pack = {
        let project = temp_project("human-safe");
        init_profile(&project);
        seed_event(&project);
        build_ephemeral_pack(&project)
    };
    let runtime = FixedOutputRuntime {
        output: ProxyRuntimeOutput {
            draft_text: format!("{HUMAN_OUTPUT_HEADER}\nfake body\n{HUMAN_OUTPUT_ACTION_LINE}"),
            provider_id: "test".into(),
            model_id: "test".into(),
            network_used: false,
            duration_ms: 1,
        },
    };
    let draft = ask_with_stub(
        &pack,
        "safe?",
        &runtime,
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let rendered = render_human_output(&draft);
    assert_eq!(rendered.lines().next().unwrap(), HUMAN_OUTPUT_HEADER);
    assert_eq!(rendered.lines().last().unwrap(), HUMAN_OUTPUT_ACTION_LINE);
}

#[test]
fn json_output_is_exact_proxy_draft_object() {
    let project = temp_project("json-draft");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "json?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let serialized = serde_json::to_string(&draft).expect("serialize");
    let parsed: Value = serde_json::from_str(&serialized).expect("parse");
    assert_eq!(parsed["classification"], "local-proxy-draft");
    assert!(parsed.get("draftText").is_some());
}

#[test]
fn json_output_has_no_human_banner() {
    let project = temp_project("json-no-banner");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "banner?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let serialized = serde_json::to_string(&draft).expect("serialize");
    assert!(!serialized.contains(HUMAN_OUTPUT_HEADER));
    assert!(!serialized.contains(HUMAN_OUTPUT_ACTION_LINE));
}

#[test]
fn json_output_has_no_wrapper() {
    let project = temp_project("json-no-wrapper");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "wrapper?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let value: Value = serde_json::from_str(&serde_json::to_string(&draft).unwrap()).unwrap();
    assert!(value.get("status").is_none());
    assert!(value.get("pack").is_none());
}

#[test]
fn json_output_has_no_0_1_7_fields() {
    let project = temp_project("json-no-0-1-7");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "deferred?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let serialized = serde_json::to_string(&draft).expect("serialize");
    let lowered = serialized.to_ascii_lowercase();
    for forbidden in [
        "\"claims\"",
        "\"citations\"",
        "authoritydecision",
        "approvalresult",
        "executionpermission",
        "verifiedanswer",
        "confirmedbyhuman",
        "claim_citation",
        "citation_sufficiency",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "ProxyDraft JSON must not contain 0.1.7 field `{forbidden}`"
        );
    }
}

#[test]
fn errors_emit_no_partial_success_json() {
    let project = temp_project("error-json");
    init_profile(&project);
    seed_event(&project);
    let mut args = proxy_ask_args("   ", false);
    args.push("--json".into());
    let output = run_proxy_cli(&args, &project);
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("\"classification\":\"local-proxy-draft\""));
    assert!(!stdout.contains("\"classification\": \"local-proxy-draft\""));
}

#[test]
fn Thai_question_and_draft_are_preserved() {
    let project = temp_project("thai");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let thai = "สถานะปัจจุบันคืออะไร";
    let draft = ask_with_stub(
        &pack,
        thai,
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    assert!(draft.draft_text.contains(thai));
}

#[test]
fn provider_and_model_metadata_are_safe() {
    let project = temp_project("metadata-safe");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        "metadata?",
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let serialized = serde_json::to_string(&draft).unwrap();
    assert!(!serialized.contains(API_KEY_CANARY));
    assert!(!serialized.is_empty());
    assert!(!draft.runtime.provider_id.is_empty());
    assert!(!draft.runtime.model_id.is_empty());
}

#[test]
fn API_key_canary_is_absent_from_output_and_errors() {
    let project = temp_project("api-canary");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let draft = ask_with_stub(
        &pack,
        PROMPT_CANARY,
        &DeterministicStubProxyDraftRuntime::new_for_tests(),
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let rendered = render_human_output(&draft);
    assert!(!rendered.contains(API_KEY_CANARY));
    let mut args = proxy_ask_args(PROMPT_CANARY, false);
    args.push("--json".into());
    let output = run_proxy_cli(&args, &project);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(API_KEY_CANARY));
}

#[test]
fn prompt_canary_is_absent_from_errors() {
    let project = temp_project("prompt-canary");
    init_profile(&project);
    seed_event(&project);
    let args = proxy_ask_args(PROMPT_CANARY, true);
    let output = run_proxy_cli(&args, &project);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(PROMPT_CANARY));
}

#[test]
fn context_canary_is_absent_from_errors() {
    let project = temp_project("context-canary");
    init_profile(&project);
    seed_event(&project);
    let args = proxy_ask_args("safe question", false);
    let output = run_proxy_cli(&args, &project);
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!combined.contains(CONTEXT_CANARY));
}

#[test]
fn project_path_canary_is_not_sent_to_runtime() {
    let project = temp_project("path-canary");
    init_profile(&project);
    seed_event(&project);
    let pack = build_ephemeral_pack(&project);
    let captured = Arc::new(Mutex::new(Vec::new()));
    let runtime = CapturingRuntime {
        captured: Arc::clone(&captured),
        inner: DeterministicStubProxyDraftRuntime::new_for_tests(),
    };
    let _ = ask_with_stub(
        &pack,
        "path?",
        &runtime,
        &TestSequenceIdentityProvider::new(),
        &FixedProxyDraftClock::new("2026-07-18T10:00:00Z"),
        60_000,
    )
    .expect("draft");
    let prompt = captured.lock().unwrap().join("\n");
    assert!(!prompt.contains(&project.to_string_lossy().to_string()));
    assert!(!prompt.contains(".openmesh"));
}
