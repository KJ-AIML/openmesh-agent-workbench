//! Dev Track 0.1.6 Checkpoint E — production runtime factory and CLI parser boundary tests.
//!
//! Factory tests compile `../src/proxy_runtime_factory.rs` via `#[path]`, which is the
//! same source file wired by `main.rs`. The factory `#[cfg(test)]` block is omitted
//! from the production binary; production dispatch is source-guarded in `proxy_workflow`.

#[path = "../src/proxy_runtime_factory.rs"]
mod proxy_runtime_factory;

use openmesh_core::proxy_runtime::{
    DeterministicStubProxyDraftRuntime, ProxyDraftRuntime, PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED,
};
use openmesh_core::proxy_runtime_axga::{
    PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC, PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK,
    PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI,
};
use proxy_runtime_factory::{
    expected_runtime_kind_for_provider, resolve_production_proxy_draft_runtime_with_env,
    MapProxyRuntimeEnvironment, ProxyRuntimeEnvironment, ProxyRuntimeFactoryError,
    ENV_ANTHROPIC_API_KEY, ENV_DEEPSEEK_API_KEY, ENV_OPENAI_API_KEY, ENV_OPENMESH_PROXY_MODEL,
    ENV_OPENMESH_PROXY_OPENAI_BASE_URL, ENV_OPENMESH_PROXY_PROVIDER,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use openmesh_core::storage::init_project;
use std::sync::atomic::{AtomicU64, Ordering};

static PARSER_PROJECT_COUNTER: AtomicU64 = AtomicU64::new(0);

fn init_profile_for_parser(project: &PathBuf) {
    let output = cli()
        .args([
            "profile",
            "init",
            "--owner-label",
            "Owner",
            "--role-label",
            "Role",
            "--project",
            &project.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
}

fn temp_project_for_parser(label: &str) -> PathBuf {
    let n = PARSER_PROJECT_COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!(
        "openmesh-cli-proxy-boundary-{label}-{}-{n}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    init_project(&dir.to_string_lossy()).expect("init");
    dir
}
const API_KEY_CANARY: &str = "CANARY-API-KEY-9f3a2b1c";
const WINDOW_SINCE: &str = "2026-07-15T00:00:00Z";
const WINDOW_UNTIL: &str = "2026-07-18T00:00:00Z";

struct NonUnicodeSignalingEnvironment {
    inner: MapProxyRuntimeEnvironment,
    non_unicode_keys: BTreeSet<String>,
}

impl NonUnicodeSignalingEnvironment {
    fn new() -> Self {
        Self {
            inner: MapProxyRuntimeEnvironment::new(),
            non_unicode_keys: BTreeSet::new(),
        }
    }

    fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner = self.inner.set(key, value);
        self
    }

    fn mark_non_unicode(mut self, key: impl Into<String>) -> Self {
        self.non_unicode_keys.insert(key.into());
        self
    }
}

impl ProxyRuntimeEnvironment for NonUnicodeSignalingEnvironment {
    fn get(&self, key: &str) -> Option<String> {
        if self.non_unicode_keys.contains(key) {
            return None;
        }
        self.inner.get(key)
    }

    fn has_non_unicode_value(&self, key: &str) -> bool {
        self.non_unicode_keys.contains(key)
    }
}

fn openai_env() -> MapProxyRuntimeEnvironment {
    MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY)
}

fn anthropic_env() -> MapProxyRuntimeEnvironment {
    MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "anthropic")
        .set(ENV_OPENMESH_PROXY_MODEL, "claude-3-5-sonnet-20241022")
        .set(ENV_ANTHROPIC_API_KEY, API_KEY_CANARY)
}

fn deepseek_env() -> MapProxyRuntimeEnvironment {
    MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "deepseek")
        .set(ENV_OPENMESH_PROXY_MODEL, "deepseek-chat")
        .set(ENV_DEEPSEEK_API_KEY, API_KEY_CANARY)
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_openmesh-cli"))
}

fn proxy_help() -> String {
    String::from_utf8_lossy(&cli().args(["proxy", "--help"]).output().unwrap().stdout).into_owned()
}

fn proxy_ask_help() -> String {
    String::from_utf8_lossy(
        &cli()
            .args(["proxy", "ask", "--help"])
            .output()
            .unwrap()
            .stdout,
    )
    .into_owned()
}

fn proxy_source() -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/proxy.rs"))
        .expect("read proxy.rs")
}

fn factory_source() -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/proxy_runtime_factory.rs"),
    )
    .expect("read proxy_runtime_factory.rs")
}

fn factory_err(env: &dyn ProxyRuntimeEnvironment) -> ProxyRuntimeFactoryError {
    match resolve_production_proxy_draft_runtime_with_env(env) {
        Err(err) => err,
        Ok(_) => panic!("expected factory error"),
    }
}

fn assert_error_display_safe(err: ProxyRuntimeFactoryError, secret: &str, leaked: &str) {
    let message = format!("{err}");
    assert!(!message.contains(secret), "error leaked secret: {message}");
    if !leaked.is_empty() {
        assert!(!message.contains(leaked), "error leaked value: {message}");
    }
}

// --- Factory tests (30) ---

#[test]
fn fully_absent_selector_configuration_returns_unconfigured_runtime() {
    let runtime =
        resolve_production_proxy_draft_runtime_with_env(&MapProxyRuntimeEnvironment::new())
            .expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
    );
}

#[test]
fn generic_api_keys_alone_do_not_auto_configure_runtime() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY)
        .set(ENV_ANTHROPIC_API_KEY, API_KEY_CANARY)
        .set(ENV_DEEPSEEK_API_KEY, API_KEY_CANARY);
    let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
    );
}

#[test]
fn provider_without_model_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn model_without_provider_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new().set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini");
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn base_url_without_provider_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new().set(
        ENV_OPENMESH_PROXY_OPENAI_BASE_URL,
        "https://api.openai.com/v1",
    );
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn unsupported_provider_is_rejected_safely() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "gemini-secret-provider")
        .set(ENV_OPENMESH_PROXY_MODEL, "model-x")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::UnsupportedProvider);
    assert_error_display_safe(err, API_KEY_CANARY, "gemini-secret-provider");
}

#[test]
fn openai_requires_openai_api_key() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini");
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn anthropic_requires_anthropic_api_key() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "anthropic")
        .set(ENV_OPENMESH_PROXY_MODEL, "claude-3-5-sonnet-20241022");
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn deepseek_requires_deepseek_api_key() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "deepseek")
        .set(ENV_OPENMESH_PROXY_MODEL, "deepseek-chat");
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn openai_does_not_use_anthropic_or_deepseek_key() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_ANTHROPIC_API_KEY, API_KEY_CANARY)
        .set(ENV_DEEPSEEK_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn anthropic_does_not_use_openai_or_deepseek_key() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "anthropic")
        .set(ENV_OPENMESH_PROXY_MODEL, "claude-3-5-sonnet-20241022")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY)
        .set(ENV_DEEPSEEK_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn deepseek_does_not_use_openai_env_fallback() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "deepseek")
        .set(ENV_OPENMESH_PROXY_MODEL, "deepseek-chat")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::PartialConfiguration);
}

#[test]
fn model_is_never_defaulted() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert!(
        matches!(
            err,
            ProxyRuntimeFactoryError::PartialConfiguration | ProxyRuntimeFactoryError::MissingModel
        ),
        "unexpected error: {err:?}"
    );
}

#[test]
fn provider_is_never_inferred_from_keys() {
    let env = MapProxyRuntimeEnvironment::new().set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_UNCONFIGURED
    );
}

#[test]
fn provider_value_is_case_normalized() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "OpenAI")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
    assert_eq!(runtime.runtime_kind(), PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI);
    assert_eq!(
        expected_runtime_kind_for_provider("OPENAI"),
        Some(PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI)
    );
}

#[test]
fn whitespace_only_model_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "   ")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::MissingModel);
}

#[test]
fn whitespace_only_key_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_OPENAI_API_KEY, "   ");
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::MissingCredential);
}

#[test]
fn openai_base_url_is_optional() {
    let runtime = resolve_production_proxy_draft_runtime_with_env(&openai_env()).expect("runtime");
    assert_eq!(runtime.runtime_kind(), PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI);
}

#[test]
fn deepseek_base_url_is_optional() {
    let runtime =
        resolve_production_proxy_draft_runtime_with_env(&deepseek_env()).expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK
    );
}

#[test]
fn anthropic_custom_base_url_is_rejected() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "anthropic")
        .set(ENV_OPENMESH_PROXY_MODEL, "claude-3-5-sonnet-20241022")
        .set(ENV_ANTHROPIC_API_KEY, API_KEY_CANARY)
        .set(
            ENV_OPENMESH_PROXY_OPENAI_BASE_URL,
            "https://api.openai.com/v1",
        );
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::InvalidBaseUrl);
}

#[test]
fn base_url_credentials_are_rejected() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY)
        .set(
            ENV_OPENMESH_PROXY_OPENAI_BASE_URL,
            "https://user:pass@api.openai.com/v1",
        );
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::InvalidBaseUrl);
}

#[test]
fn invalid_base_url_error_does_not_echo_value() {
    let leaked = "not-a-valid-base-url-scheme";
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY)
        .set(ENV_OPENMESH_PROXY_OPENAI_BASE_URL, leaked);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::InvalidBaseUrl);
    assert_error_display_safe(err, API_KEY_CANARY, leaked);
}

#[test]
fn non_unicode_configuration_is_rejected_safely() {
    let env = NonUnicodeSignalingEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "openai")
        .set(ENV_OPENMESH_PROXY_MODEL, "gpt-4o-mini")
        .mark_non_unicode(ENV_OPENAI_API_KEY);
    let err = factory_err(&env);
    assert_eq!(err, ProxyRuntimeFactoryError::NonUnicodeConfiguration);
    assert_error_display_safe(err, API_KEY_CANARY, "");
}

#[test]
fn complete_openai_configuration_creates_axga_openai_runtime() {
    let runtime = resolve_production_proxy_draft_runtime_with_env(&openai_env()).expect("runtime");
    assert_eq!(runtime.runtime_kind(), PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI);
}

#[test]
fn complete_anthropic_configuration_creates_axga_anthropic_runtime() {
    let runtime =
        resolve_production_proxy_draft_runtime_with_env(&anthropic_env()).expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_AXGA_ANTHROPIC
    );
}

#[test]
fn complete_deepseek_configuration_creates_axga_deepseek_runtime() {
    let runtime =
        resolve_production_proxy_draft_runtime_with_env(&deepseek_env()).expect("runtime");
    assert_eq!(
        runtime.runtime_kind(),
        PROXY_DRAFT_RUNTIME_KIND_AXGA_DEEPSEEK
    );
}

#[test]
fn production_factory_never_returns_deterministic_stub() {
    let env = MapProxyRuntimeEnvironment::new();
    let runtime = resolve_production_proxy_draft_runtime_with_env(&env).expect("runtime");
    assert_ne!(
        runtime.runtime_kind(),
        DeterministicStubProxyDraftRuntime::new_for_tests().runtime_kind()
    );
    let configured =
        resolve_production_proxy_draft_runtime_with_env(&openai_env()).expect("runtime");
    assert_ne!(
        configured.runtime_kind(),
        DeterministicStubProxyDraftRuntime::new_for_tests().runtime_kind()
    );
}

#[test]
fn factory_performs_no_provider_request() {
    let runtime = resolve_production_proxy_draft_runtime_with_env(&openai_env()).expect("runtime");
    assert_eq!(runtime.runtime_kind(), PROXY_DRAFT_RUNTIME_KIND_AXGA_OPENAI);
}

#[test]
fn factory_does_not_read_project_files() {
    let source = factory_source();
    for forbidden in ["read_to_string", "read_dir", "File::open", "std::fs::read"] {
        assert!(
            !source.contains(forbidden),
            "factory must not read project files (`{forbidden}`)"
        );
    }
}

#[test]
fn factory_error_messages_do_not_expose_credentials() {
    let env = MapProxyRuntimeEnvironment::new()
        .set(ENV_OPENMESH_PROXY_PROVIDER, "unsupported-vendor")
        .set(ENV_OPENMESH_PROXY_MODEL, "secret-model-name")
        .set(ENV_OPENAI_API_KEY, API_KEY_CANARY);
    let err = factory_err(&env);
    assert_error_display_safe(err, API_KEY_CANARY, "secret-model-name");
}

// --- Parser tests (20) ---

#[test]
fn proxy_ask_command_is_registered() {
    let help = proxy_help().to_ascii_lowercase();
    assert!(help.contains("ask"));
    let ask_help = proxy_ask_help().to_ascii_lowercase();
    assert!(ask_help.contains("question"));
}

#[test]
fn question_is_required() {
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn timeout_secs_accepts_positive_integer() {
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--timeout-secs",
            "30",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
            "--project",
            env!("CARGO_MANIFEST_DIR"),
        ])
        .output()
        .unwrap();
    assert_ne!(
        output.status.code(),
        Some(2),
        "parser rejected positive timeout"
    );
}

#[test]
fn timeout_secs_rejects_zero() {
    let output = cli()
        .args(["proxy", "ask", "--question", "hello", "--timeout-secs", "0"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn timeout_secs_overflow_is_rejected() {
    let project = temp_project_for_parser("timeout-overflow");
    init_profile_for_parser(&project);
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--timeout-secs",
            "18446744073709551615",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
            "--project",
            &project.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("overflow") || output.status.code() == Some(2),
        "overflow must be rejected safely: {combined}"
    );
}

#[test]
fn from_persisted_conflicts_with_since() {
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--from-persisted",
            "--since",
            WINDOW_SINCE,
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn from_persisted_conflicts_with_until() {
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--from-persisted",
            "--until",
            WINDOW_UNTIL,
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
}

#[test]
fn since_until_follow_existing_context_window_rules() {
    let project = temp_project_for_parser("since-until-window");
    init_profile_for_parser(&project);
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--project",
            &project.to_string_lossy(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(combined.contains("since") || combined.contains("until"));
}

#[test]
fn project_flag_uses_existing_project_resolution() {
    let project = temp_project_for_parser("project-resolution");
    init_profile_for_parser(&project);
    let output = cli()
        .args([
            "proxy",
            "ask",
            "--question",
            "hello",
            "--since",
            WINDOW_SINCE,
            "--until",
            WINDOW_UNTIL,
            "--project",
            &project.to_string_lossy(),
        ])
        .output()
        .unwrap();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !combined.contains("no OpenMesh project found at explicit --project path"),
        "explicit project should resolve: {combined}"
    );
}

#[test]
fn json_flag_is_supported() {
    let help = proxy_ask_help();
    assert!(help.contains("--json") || help.contains("json"));
}

#[test]
fn write_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--write"));
}

#[test]
fn history_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--history"));
}

#[test]
fn provider_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--provider"));
}

#[test]
fn model_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--model"));
}

#[test]
fn api_key_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--api-key"));
}

#[test]
fn stub_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--stub"));
}

#[test]
fn fake_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--fake"));
}

#[test]
fn test_runtime_flag_is_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    assert!(!help.contains("--test-runtime"));
}

#[test]
fn authority_and_execute_flags_are_absent() {
    let help = proxy_ask_help().to_ascii_lowercase();
    for forbidden in ["--authority", "--approve", "--execute"] {
        assert!(!help.contains(forbidden), "forbidden flag: {forbidden}");
    }
}

#[test]
fn no_hidden_alias_selects_stub() {
    for source in [proxy_help(), proxy_ask_help(), proxy_source()] {
        let lower = source.to_ascii_lowercase();
        for forbidden in ["--stub", "--fake", "--test-runtime"] {
            assert!(
                !lower.contains(forbidden),
                "hidden stub surface `{forbidden}`"
            );
        }
    }
    let factory = factory_source();
    let production = factory.split("#[cfg(test)]").next().unwrap_or(&factory);
    assert!(!production.contains("DeterministicStubProxyDraftRuntime"));
}
