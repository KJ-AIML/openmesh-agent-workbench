// ============================================================================
// Proxy commands — Dev Track 0.1.6 Checkpoint E + 0.1.7 Authority Gate
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::answer_receipt::{write_answer_receipt, AnswerReceipt};
use openmesh_core::authority_gate::{
    run_pre_provider_authority_gate, AuthorityGateOutcome, AuthorityOutcomeLabel,
};
use openmesh_core::authority_policy::{classify_question_risk, AuthorityPolicyDecision};
use openmesh_core::context_pack::{build_proxy_context_pack, ProxyContextPackBuildOptions};
use openmesh_core::context_pack_storage::read_proxy_context_pack;
use openmesh_core::context_pack_validation::validate_proxy_context_pack_complete;
use openmesh_core::domain::{ProxyContextPack, ProxyDraft, MAX_PROXY_DRAFT_TEXT_BYTES};
use openmesh_core::oauth::{
    adapter_for, generate_state, OAuthCallback, OAuthCallbackServer, OAuthError,
    OAuthProviderAdapter, OAuthProviderId, OAuthTokenStore, PkceCodes,
    OPENMESH_OAUTH_KEYRING_SERVICE,
};
use openmesh_core::pending_proxy_question::write_pending_proxy_question;
use openmesh_core::profile::read_work_proxy_profile;
use openmesh_core::proxy_ask::{
    ask_my_proxy_local, ProxyAskError, ProxyAskOptions, ProxyDraftClock, SystemProxyDraftClock,
};
use openmesh_core::proxy_post_verify::apply_post_provider_verification;
use openmesh_core::proxy_question::{
    create_proxy_question, ProcessLocalRequestIdentityProvider, ProxyQuestionConstructionError,
    ProxyRequestIdentityProvider,
};
use openmesh_core::proxy_runtime::ProxyDraftRuntime;
use openmesh_core::proxy_server::{
    read_proxy_config, ProxyModelConfig, ProxyOAuthCredential, ProxyOAuthCredentialResolver,
    ProxyProviderProtocol, ProxyServer, ProxyServerConfig, ProxyUpstreamConfig, DEFAULT_PROXY_HOST,
    DEFAULT_PROXY_PORT, DEFAULT_REQUEST_TIMEOUT_SECS,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::context::build_fixed_window;
use crate::output;
use crate::project::resolve_project;
use crate::proxy_runtime_factory::{
    resolve_production_proxy_draft_runtime, ProxyRuntimeFactoryError,
};
use crate::proxy_verify::ProxyVerifyArgs;

pub const HUMAN_OUTPUT_HEADER: &str = "Local Work Proxy draft — not the human owner.";
pub const HUMAN_OUTPUT_ACTION_LINE: &str = "No action was performed.";

#[derive(Subcommand, Debug)]
pub enum ProxyCommand {
    /// Ask the local Work Proxy for a draft answer.
    Ask(ProxyAskArgs),
    /// Verify draft claims against persisted context pack evidence (read-only).
    Verify(ProxyVerifyArgs),
    /// Serve the OpenMesh-owned OpenAI-compatible proxy runtime.
    Serve(ProxyServeArgs),
    /// Manage native OpenMesh OAuth credentials.
    #[command(subcommand)]
    Auth(ProxyAuthCommand),
}

#[derive(Subcommand, Debug)]
pub enum ProxyAuthCommand {
    /// Start a provider OAuth flow and save its token in the OS credential manager.
    Login(ProxyAuthLoginArgs),
    /// Show whether a provider/account token is available in the OS credential manager.
    Status(ProxyAuthAccountArgs),
    /// Delete a provider/account token from the OS credential manager.
    Logout(ProxyAuthAccountArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ProxyAuthLoginArgs {
    /// Provider ID: codex, claude, antigravity, grok, or kimi in this native slice.
    pub provider: String,

    /// Use a provider device-code flow instead of its loopback browser callback where supported.
    #[arg(long)]
    pub device: bool,

    /// Do not ask the operating system to open the authorization URL.
    #[arg(long)]
    pub no_browser: bool,

    /// Maximum time to wait for the browser/device authorization to complete.
    #[arg(long, default_value_t = 600, value_parser = parse_positive_timeout_secs)]
    pub timeout_secs: u64,
}

#[derive(Args, Debug, Clone)]
pub struct ProxyAuthAccountArgs {
    /// Provider ID: codex, claude, antigravity, grok, or kimi in this native slice.
    pub provider: String,

    /// Account ID printed after a successful login.
    #[arg(long = "account-id")]
    pub account_id: String,
}

#[derive(Args, Debug, Clone)]
pub struct ProxyServeArgs {
    /// Load the OpenMesh YAML config instead of constructing one from flags.
    #[arg(long, value_name = "PATH")]
    pub config: Option<PathBuf>,

    /// Bind host for the built-in proxy.
    #[arg(long, default_value = DEFAULT_PROXY_HOST)]
    pub bind_host: String,

    /// Listen port for the built-in proxy.
    #[arg(long, default_value_t = DEFAULT_PROXY_PORT)]
    pub port: u16,

    /// Client API key. Repeat for multiple accepted keys.
    #[arg(
        long = "api-key",
        env = "OPENMESH_PROXY_API_KEY",
        value_delimiter = ','
    )]
    pub api_keys: Vec<String>,

    /// Explicitly disable client API-key enforcement for a trusted local test.
    #[arg(long)]
    pub allow_unauthenticated: bool,

    /// OpenAI-compatible upstream base URL, such as `https://api.openai.com/v1`.
    #[arg(long, env = "OPENMESH_PROXY_UPSTREAM_URL")]
    pub upstream_url: Option<String>,

    /// API key sent to the configured upstream.
    #[arg(long, env = "OPENMESH_PROXY_UPSTREAM_API_KEY")]
    pub upstream_api_key: Option<String>,

    /// Model exposed by the upstream. Repeat for a model catalog.
    #[arg(long = "model", default_value = "gpt-4o-mini")]
    pub models: Vec<String>,

    /// Model alias in `alias=target` form. Repeat as needed.
    #[arg(long = "alias")]
    pub aliases: Vec<String>,

    /// Request timeout applied to upstream calls.
    #[arg(long, default_value_t = DEFAULT_REQUEST_TIMEOUT_SECS)]
    pub request_timeout_secs: u64,
}

#[derive(Args, Debug, Clone)]
pub struct ProxyAskArgs {
    /// Question text for the local proxy draft.
    #[arg(long)]
    pub question: String,

    #[arg(long)]
    pub since: Option<String>,

    #[arg(long)]
    pub until: Option<String>,

    #[arg(long, conflicts_with_all = ["since", "until"])]
    pub from_persisted: bool,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,

    #[arg(long, value_parser = parse_positive_timeout_secs)]
    pub timeout_secs: Option<u64>,
}

fn parse_positive_timeout_secs(raw: &str) -> Result<u64, String> {
    let value = raw
        .parse::<u64>()
        .map_err(|_| "timeout_secs must be a positive integer".to_string())?;
    if value == 0 {
        return Err("timeout_secs must be greater than zero".to_string());
    }
    Ok(value)
}

pub trait ProxyRuntimeResolver: Send + Sync {
    fn resolve(&self) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ProductionProxyRuntimeResolver;

impl ProxyRuntimeResolver for ProductionProxyRuntimeResolver {
    fn resolve(&self) -> Result<Box<dyn ProxyDraftRuntime>, ProxyRuntimeFactoryError> {
        resolve_production_proxy_draft_runtime()
    }
}

/// Injectable execution harness for automated CLI tests.
pub struct ProxyAskHarness<'a> {
    pub runtime_resolver: &'a dyn ProxyRuntimeResolver,
    pub identity_provider: &'a dyn ProxyRequestIdentityProvider,
    pub clock: &'a dyn ProxyDraftClock,
}

pub fn run_proxy(cmd: ProxyCommand, cwd: &Path) -> i32 {
    match cmd {
        ProxyCommand::Ask(args) => run_proxy_ask(&args, cwd),
        ProxyCommand::Verify(args) => crate::proxy_verify::run_proxy_verify(&args, cwd),
        ProxyCommand::Serve(args) => run_proxy_serve(&args),
        ProxyCommand::Auth(command) => run_proxy_auth(command),
    }
}

pub fn run_proxy_serve(args: &ProxyServeArgs) -> i32 {
    if let Some(path) = args.config.as_ref() {
        let config = match read_proxy_config(path) {
            Ok(config) => config,
            Err(error) => {
                eprintln!("ERROR proxy-config: {error}");
                return 3;
            }
        };
        return serve_proxy_config(config, Some(path));
    }

    let api_keys = args.api_keys.clone();
    let upstream_url = match args.upstream_url.clone() {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            eprintln!(
                "ERROR proxy-config: --upstream-url or OPENMESH_PROXY_UPSTREAM_URL is required"
            );
            return 3;
        }
    };
    let upstream_api_key = args
        .upstream_api_key
        .clone()
        .filter(|value| !value.trim().is_empty());
    let models = args
        .models
        .iter()
        .filter(|model| !model.trim().is_empty())
        .map(|model| ProxyModelConfig {
            id: model.trim().to_string(),
            owned_by: "openmesh-upstream".to_string(),
            capabilities: vec![
                "chat".to_string(),
                "responses".to_string(),
                "embeddings".to_string(),
            ],
        })
        .collect();
    let aliases = match parse_aliases(&args.aliases) {
        Ok(aliases) => aliases,
        Err(error) => {
            eprintln!("ERROR proxy-config: {error}");
            return 3;
        }
    };
    let config = ProxyServerConfig {
        bind_host: args.bind_host.clone(),
        port: args.port,
        api_keys,
        allow_unauthenticated: args.allow_unauthenticated,
        request_timeout_secs: args.request_timeout_secs,
        routing_strategy: Default::default(),
        max_retries: openmesh_core::proxy_server::DEFAULT_MAX_RETRIES,
        upstreams: vec![ProxyUpstreamConfig {
            id: "default".to_string(),
            base_url: upstream_url,
            protocol: ProxyProviderProtocol::OpenAiCompatible,
            api_key: upstream_api_key,
            enabled: true,
            priority: 0,
            account_id: None,
            oauth_provider: None,
            models,
        }],
        model_aliases: aliases,
        model_fallbacks: Default::default(),
    };
    serve_proxy_config(config, None)
}

fn serve_proxy_config(config: ProxyServerConfig, config_path: Option<&Path>) -> i32 {
    let server = match ProxyServer::new_with_config_path(config, config_path.map(Path::to_path_buf))
    {
        Ok(server) => server,
        Err(error) => {
            eprintln!("ERROR proxy-config: {error}");
            return 3;
        }
    };
    server.set_oauth_credential_resolver(Arc::new(CliOAuthCredentialResolver::default()));
    if let Err(error) = server.persist_config() {
        eprintln!("ERROR proxy-config: {error}");
        return 3;
    }
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("ERROR proxy-runtime: could not start async runtime: {error}");
            return 4;
        }
    };
    runtime.block_on(async move {
        let listener = match server.bind().await {
            Ok(listener) => listener,
            Err(error) => {
                eprintln!("ERROR proxy-listener: {error}");
                return 4;
            }
        };
        let address = listener
            .local_addr()
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "configured address".to_string());
        println!("OpenMesh built-in proxy listening on http://{address}");
        let shutdown = async {
            let _ = tokio::signal::ctrl_c().await;
        };
        match server.serve(listener, shutdown).await {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("ERROR proxy-runtime: {error}");
                4
            }
        }
    })
}

#[derive(Clone)]
struct CliOAuthCredentialResolver {
    token_store: openmesh_core::oauth::KeyringTokenStore,
    client: reqwest::Client,
}

impl Default for CliOAuthCredentialResolver {
    fn default() -> Self {
        Self {
            token_store: openmesh_core::oauth::KeyringTokenStore::new(
                OPENMESH_OAUTH_KEYRING_SERVICE,
            )
            .expect("static OAuth keyring service name is valid"),
            client: reqwest::Client::builder()
                .user_agent("OpenMesh/0.1 built-in OAuth")
                .build()
                .expect("default OAuth HTTP client should build"),
        }
    }
}

#[async_trait::async_trait]
impl ProxyOAuthCredentialResolver for CliOAuthCredentialResolver {
    async fn resolve(
        &self,
        provider: OAuthProviderId,
        account_id: &str,
    ) -> Result<Option<ProxyOAuthCredential>, String> {
        let Some(mut token) = self
            .token_store
            .load(provider, account_id)
            .map_err(|error| error.to_string())?
        else {
            return Ok(None);
        };
        let adapter = adapter_for(provider, self.client.clone()).map_err(display_oauth_error)?;
        if token.needs_refresh(Utc::now(), adapter.refresh_lead()) {
            token = adapter.refresh(&token).await.map_err(display_oauth_error)?;
            self.token_store
                .save(&token)
                .map_err(|_| "OAuth token could not be stored securely".to_string())?;
        }
        Ok(Some(ProxyOAuthCredential {
            access_token: token.access_token,
            token_type: token.token_type,
            metadata: token
                .metadata
                .iter()
                .filter_map(|(key, value)| {
                    value.as_str().map(|value| (key.clone(), value.to_owned()))
                })
                .collect(),
        }))
    }
}

fn run_proxy_auth(command: ProxyAuthCommand) -> i32 {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("ERROR oauth-runtime: could not start async runtime: {error}");
            return 4;
        }
    };
    match runtime.block_on(async move {
        match command {
            ProxyAuthCommand::Login(args) => run_proxy_auth_login(args).await,
            ProxyAuthCommand::Status(args) => run_proxy_auth_status(args),
            ProxyAuthCommand::Logout(args) => run_proxy_auth_logout(args),
        }
    }) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("ERROR oauth: {}", display_oauth_error(error));
            3
        }
    }
}

async fn run_proxy_auth_login(args: ProxyAuthLoginArgs) -> Result<(), OAuthError> {
    let provider = args
        .provider
        .parse::<OAuthProviderId>()
        .map_err(|error| error)?;
    let client = reqwest::Client::builder()
        .user_agent("OpenMesh/0.1 built-in OAuth")
        .build()
        .map_err(|source| OAuthError::Http { provider, source })?;
    let token_store = openmesh_core::oauth::KeyringTokenStore::new(OPENMESH_OAUTH_KEYRING_SERVICE)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    let adapter = adapter_for(provider, client).map_err(|error| error)?;
    if args.device {
        return run_device_login(&*adapter, &token_store, &args).await;
    }
    let pkce = PkceCodes::generate()?;
    let state = generate_state()?;
    let redirect_uri = adapter.default_redirect_uri().to_owned();
    let redirect = reqwest::Url::parse(&redirect_uri).map_err(|_| {
        OAuthError::InvalidConfiguration(
            "OAuth adapter returned an invalid redirect URI".to_owned(),
        )
    })?;
    let callback_port = redirect.port_or_known_default().ok_or_else(|| {
        OAuthError::InvalidConfiguration("OAuth redirect URI has no port".to_owned())
    })?;
    let mut callback_server = OAuthCallbackServer::bind(callback_port, redirect.path()).await?;
    let authorization = adapter.authorization_url(&redirect_uri, &state, &pkce)?;
    println!(
        "OpenMesh OAuth authorization URL:\n{}",
        authorization.authorization_url
    );
    if !args.no_browser && !open_browser(authorization.authorization_url.as_str()) {
        println!("Browser launch was unavailable; open the URL above manually.");
    }
    println!("Waiting for the {provider} callback...");
    let callback = tokio::time::timeout(
        std::time::Duration::from_secs(args.timeout_secs),
        callback_server.wait_for_callback(),
    )
    .await
    .map_err(|_| OAuthError::Request("OAuth authorization timed out".to_owned()))??;
    if !callback_matches_state(&callback, &state) {
        return Err(OAuthError::StateMismatch);
    }
    let token = adapter
        .exchange_code(&callback, &redirect_uri, &pkce.verifier)
        .await?;
    let account_id = token.account_id.clone();
    token_store
        .save(&token)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    println!("OpenMesh OAuth login succeeded: provider={provider} account_id={account_id}");
    Ok(())
}

async fn run_device_login(
    adapter: &dyn OAuthProviderAdapter,
    token_store: &openmesh_core::oauth::KeyringTokenStore,
    args: &ProxyAuthLoginArgs,
) -> Result<(), OAuthError> {
    let device = adapter.start_device_flow().await?;
    let verification_url = device
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&device.verification_uri);
    println!(
        "OpenMesh {} device login: visit {} and enter code {}",
        adapter.provider(),
        verification_url,
        device.user_code
    );
    if !args.no_browser && !open_browser(verification_url) {
        println!("Browser launch was unavailable; open the URL above manually.");
    }
    println!("Waiting for {} device authorization...", adapter.provider());
    let deadline = std::time::Instant::now()
        .checked_add(std::time::Duration::from_secs(args.timeout_secs))
        .ok_or_else(|| OAuthError::Request("OAuth authorization timeout is invalid".to_owned()))?;
    let interval = std::time::Duration::from_secs(device.interval_seconds.max(1));
    loop {
        if std::time::Instant::now() >= deadline || Utc::now() >= device.expires_at {
            return Err(OAuthError::Request(format!(
                "{} device authorization timed out",
                adapter.provider()
            )));
        }
        tokio::time::sleep(interval).await;
        if let Some(token) = adapter.poll_device_flow(&device).await? {
            let account_id = token.account_id.clone();
            token_store
                .save(&token)
                .map_err(|error| OAuthError::Storage(error.to_string()))?;
            println!(
                "OpenMesh OAuth login succeeded: provider={} account_id={account_id}",
                adapter.provider()
            );
            return Ok(());
        }
    }
}

fn run_proxy_auth_status(args: ProxyAuthAccountArgs) -> Result<(), OAuthError> {
    let provider = args.provider.parse::<OAuthProviderId>()?;
    let store = openmesh_core::oauth::KeyringTokenStore::new(OPENMESH_OAUTH_KEYRING_SERVICE)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    let token = store
        .load(provider, &args.account_id)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    match token {
        Some(token) => {
            println!(
                "provider={} account_id={} status=stored expires_at={}",
                provider,
                token.account_id,
                token
                    .expires_at
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_owned())
            );
        }
        None => println!(
            "provider={} account_id={} status=missing",
            provider, args.account_id
        ),
    }
    Ok(())
}

fn run_proxy_auth_logout(args: ProxyAuthAccountArgs) -> Result<(), OAuthError> {
    let provider = args.provider.parse::<OAuthProviderId>()?;
    let store = openmesh_core::oauth::KeyringTokenStore::new(OPENMESH_OAUTH_KEYRING_SERVICE)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    store
        .delete(provider, &args.account_id)
        .map_err(|error| OAuthError::Storage(error.to_string()))?;
    println!(
        "OpenMesh OAuth credential removed: provider={} account_id={}",
        provider, args.account_id
    );
    Ok(())
}

fn callback_matches_state(callback: &OAuthCallback, expected: &str) -> bool {
    callback.state.as_deref() == Some(expected)
        || callback
            .code
            .as_deref()
            .and_then(|code| code.split_once('#').map(|(_, state)| state))
            == Some(expected)
}

fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    let command = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let command = ("cmd", vec!["/C", "start", "", url]);
    #[cfg(target_os = "linux")]
    let command = ("xdg-open", vec![url]);
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return false;

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    std::process::Command::new(command.0)
        .args(command.1)
        .status()
        .is_ok_and(|status| status.success())
}

fn display_oauth_error(error: OAuthError) -> String {
    match error {
        OAuthError::Http { .. } => "OAuth provider request failed".to_owned(),
        OAuthError::Provider { status, .. } => {
            format!("OAuth provider rejected the request (HTTP {status})")
        }
        OAuthError::Storage(_) => "OAuth token could not be stored securely".to_owned(),
        other => other.to_string(),
    }
}

fn parse_aliases(values: &[String]) -> Result<BTreeMap<String, String>, String> {
    let mut aliases = BTreeMap::new();
    for value in values {
        let (alias, target) = value
            .split_once('=')
            .ok_or_else(|| format!("alias `{value}` must use alias=target form"))?;
        if alias.trim().is_empty() || target.trim().is_empty() {
            return Err(format!("alias `{value}` must have non-empty names"));
        }
        aliases.insert(alias.trim().to_string(), target.trim().to_string());
    }
    Ok(aliases)
}

pub fn run_proxy_ask(args: &ProxyAskArgs, cwd: &Path) -> i32 {
    let harness = ProxyAskHarness {
        runtime_resolver: &ProductionProxyRuntimeResolver,
        identity_provider: &ProcessLocalRequestIdentityProvider::new(),
        clock: &SystemProxyDraftClock,
    };
    run_proxy_ask_with_harness(args, cwd, &harness)
}

pub fn run_proxy_ask_with_harness(
    args: &ProxyAskArgs,
    cwd: &Path,
    harness: &ProxyAskHarness<'_>,
) -> i32 {
    let question_text = args.question.trim();
    if question_text.is_empty() {
        return print_proxy_error(
            "question must not be empty",
            "invalid-question",
            3,
            args.json,
        );
    }

    if args.from_persisted && (args.since.is_some() || args.until.is_some()) {
        return print_proxy_error(
            "--from-persisted cannot be combined with --since or --until",
            "invalid-request",
            3,
            args.json,
        );
    }

    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();

    let profile = match read_work_proxy_profile(&project_path) {
        Ok(profile) => profile,
        Err(_) => {
            return print_proxy_error(
                "work proxy profile is required for authority gate",
                "profile-missing",
                3,
                args.json,
            )
        }
    };

    let gate = run_pre_provider_authority_gate(question_text, &profile, "local-proxy-ask");
    let decision = match &gate {
        AuthorityGateOutcome::Proceed { decision, .. }
        | AuthorityGateOutcome::MustAsk { decision, .. }
        | AuthorityGateOutcome::Denied { decision, .. } => decision.clone(),
    };

    match &gate {
        AuthorityGateOutcome::MustAsk { message, .. } => {
            let _ = write_pending_for_question(&project_path, question_text, &decision);
            return print_proxy_error(message, "must-ask-human", 2, args.json);
        }
        AuthorityGateOutcome::Denied { message, .. } => {
            let _ = write_pending_for_question(&project_path, question_text, &decision);
            return print_proxy_error(message, "authority-denied", 2, args.json);
        }
        AuthorityGateOutcome::Proceed { .. } => {}
    }

    let pack = match load_context_pack(args, &project_path) {
        Ok(pack) => pack,
        Err(code) => return code,
    };

    // Pre-provider freshness gate for critical tiers — fail closed before provider.
    let risk = classify_question_risk(question_text);
    let tier = openmesh_core::authority_policy::map_risk_to_freshness_tier(risk);
    let freshness_pre =
        openmesh_core::authority_freshness::evaluate_evidence_freshness(&pack, tier, Utc::now());
    if matches!(
        tier,
        openmesh_core::authority_policy::FreshnessTier::Critical
    ) && !freshness_pre.is_sufficient
    {
        let _ = write_pending_for_question(&project_path, question_text, &decision);
        return print_proxy_error(
            "critical question requires fresher evidence; must ask human",
            "freshness-insufficient",
            2,
            args.json,
        );
    }

    let question = match create_proxy_question(question_text, harness.identity_provider) {
        Ok(question) => question,
        Err(err) => return print_question_construction_error(&err, args.json),
    };

    let runtime = match harness.runtime_resolver.resolve() {
        Ok(runtime) => runtime,
        Err(err) => return print_factory_error(&err, args.json),
    };

    let timeout_ms = match timeout_ms_from_args(args.timeout_secs) {
        Ok(timeout_ms) => timeout_ms,
        Err(message) => return print_proxy_error(&message, "invalid-request", 3, args.json),
    };
    let options = ProxyAskOptions::new(timeout_ms, MAX_PROXY_DRAFT_TEXT_BYTES as u32);

    match ask_my_proxy_local(&pack, &question, &options, runtime.as_ref(), harness.clock) {
        Ok(mut draft) => {
            let post =
                apply_post_provider_verification(&mut draft, &pack, &question.text, Utc::now());
            let _ = write_receipt(&project_path, &question, &pack, &draft, &decision, &post);
            let label = match gate {
                AuthorityGateOutcome::Proceed { label, .. } => label,
                _ => AuthorityOutcomeLabel::Proceed,
            };
            print_proxy_draft_success(&draft, &label, &post, args.json);
            if post.hard_fail {
                2
            } else {
                0
            }
        }
        Err(err) => print_proxy_ask_error(&err, args.json),
    }
}

fn write_pending_for_question(
    project_path: &str,
    question_text: &str,
    decision: &AuthorityPolicyDecision,
) -> Result<(), ()> {
    let risk = classify_question_risk(question_text);
    let created_at = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    write_pending_proxy_question(project_path, question_text, risk, decision, &created_at)
        .map(|_| ())
        .map_err(|_| ())
}

fn write_receipt(
    project_path: &str,
    question: &openmesh_core::domain::ProxyQuestion,
    pack: &ProxyContextPack,
    draft: &ProxyDraft,
    decision: &AuthorityPolicyDecision,
    post: &openmesh_core::proxy_post_verify::PostVerifyResult,
) -> Result<(), ()> {
    let receipt = AnswerReceipt {
        receipt_id: format!("receipt-{}", question.question_id),
        question_id: question.question_id.clone(),
        question_text: question.text.clone(),
        resolved_authority: decision.resolved_authority,
        authority_decision_reason: decision.decision_reason.clone(),
        context_pack_id: pack.context_pack_id.clone(),
        draft_text: draft.draft_text.clone(),
        claims_json: serde_json::to_string(&post.citations).unwrap_or_else(|_| "[]".to_string()),
        freshness_summary: serde_json::to_string(&post.freshness).unwrap_or_default(),
        generated_at: draft.generated_at.clone(),
        correction_of: None,
    };
    write_answer_receipt(project_path, &receipt).map_err(|_| ())
}

fn load_context_pack(args: &ProxyAskArgs, project_path: &str) -> Result<ProxyContextPack, i32> {
    if args.from_persisted {
        let pack = match read_proxy_context_pack(project_path) {
            Ok(pack) => pack,
            Err(err) => return Err(crate::context::print_context_storage_error(&err, args.json)),
        };
        if validate_proxy_context_pack_complete(&pack).is_err() {
            return Err(print_proxy_error(
                "persisted context pack failed validation",
                "invalid-context-pack",
                3,
                args.json,
            ));
        }
        return Ok(pack);
    }

    let window = match build_fixed_window(args.since.as_deref(), args.until.as_deref()) {
        Ok(window) => window,
        Err(message) => return Err(print_proxy_error(&message, "invalid-window", 3, args.json)),
    };

    let options = ProxyContextPackBuildOptions {
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        selection: Default::default(),
    };

    let pack = match build_proxy_context_pack(project_path, window, options) {
        Ok(pack) => pack,
        Err(err) => return Err(crate::context::print_context_build_error(&err, args.json)),
    };
    if validate_proxy_context_pack_complete(&pack).is_err() {
        return Err(print_proxy_error(
            "context pack failed validation",
            "invalid-context-pack",
            3,
            args.json,
        ));
    }
    Ok(pack)
}

pub fn timeout_ms_from_args(timeout_secs: Option<u64>) -> Result<u64, String> {
    let secs = timeout_secs.unwrap_or(60);
    secs.checked_mul(1_000)
        .ok_or_else(|| "timeout_secs overflow".to_string())
}

fn outcome_label_wire(label: &AuthorityOutcomeLabel) -> &'static str {
    match label {
        AuthorityOutcomeLabel::Proceed => "proceed",
        AuthorityOutcomeLabel::MustAskHuman => "must-ask-human",
        AuthorityOutcomeLabel::CannotAnswer => "cannot-answer",
        AuthorityOutcomeLabel::DeniedBeforeProvider => "denied-before-provider",
    }
}

fn print_proxy_draft_success(
    draft: &ProxyDraft,
    label: &AuthorityOutcomeLabel,
    post: &openmesh_core::proxy_post_verify::PostVerifyResult,
    json_mode: bool,
) {
    if json_mode {
        // Keep wire compatible with frozen ProxyDraft JSON (no wrapper keys).
        println!(
            "{}",
            serde_json::to_string(draft).expect("serialize proxy draft")
        );
        return;
    }

    println!("{HUMAN_OUTPUT_HEADER}");
    println!("authority_outcome={}", outcome_label_wire(label));
    println!(
        "coverage_ok={} freshness_ok={} confidence={:?}",
        post.coverage_ok, post.freshness.is_sufficient, post.freshness.confidence_label
    );
    if post.must_ask {
        println!("outcome=must-ask-human");
    }
    println!("{}", draft.draft_text);
    if !draft.limitations.is_empty() {
        println!("limitations:");
        for limitation in &draft.limitations {
            println!("- {limitation}");
        }
    }
    println!(
        "runtime_kind={} provider_id={} model_id={} network_used={} duration_ms={}",
        draft.runtime.runtime_kind,
        draft.runtime.provider_id,
        draft.runtime.model_id,
        draft.runtime.network_used,
        draft.runtime.duration_ms
    );
    println!("{HUMAN_OUTPUT_ACTION_LINE}");
}

pub fn print_proxy_error(message: &str, category: &str, code: i32, json_mode: bool) -> i32 {
    if json_mode {
        println!(
            "{}",
            json!({"status": "error", "category": category, "message": message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    code
}

fn print_factory_error(err: &ProxyRuntimeFactoryError, json_mode: bool) -> i32 {
    let message = format!("{err}");
    print_proxy_error(&message, "runtime-configuration", 3, json_mode)
}

fn print_question_construction_error(err: &ProxyQuestionConstructionError, json_mode: bool) -> i32 {
    let message = match err {
        ProxyQuestionConstructionError::InvalidText(_) => "question text is invalid",
        ProxyQuestionConstructionError::IdentityGenerationFailed(_) => {
            "question identity generation failed"
        }
    };
    print_proxy_error(message, "invalid-question", 3, json_mode)
}

fn print_proxy_ask_error(err: &ProxyAskError, json_mode: bool) -> i32 {
    let (code, category, message) = match err {
        ProxyAskError::InvalidOptions => (3, "invalid-request", "proxy ask options are invalid"),
        ProxyAskError::InvalidQuestion => (3, "invalid-question", "proxy question is invalid"),
        ProxyAskError::InvalidContextPack => (3, "invalid-context-pack", "context pack is invalid"),
        ProxyAskError::PromptCompositionFailed => {
            (3, "prompt-composition-failed", "prompt composition failed")
        }
        ProxyAskError::TraceConstructionFailed => {
            (3, "trace-construction-failed", "trace construction failed")
        }
        ProxyAskError::InvalidRuntimeRequest => {
            (3, "invalid-runtime-request", "runtime request is invalid")
        }
        ProxyAskError::RuntimeNotConfigured => (
            3,
            "runtime-not-configured",
            "proxy draft runtime is not configured",
        ),
        ProxyAskError::RuntimeTimeout => (3, "runtime-timeout", "proxy draft runtime timed out"),
        ProxyAskError::RuntimeUnavailable => (
            3,
            "runtime-unavailable",
            "proxy draft runtime is unavailable",
        ),
        ProxyAskError::ProviderFailure => {
            (3, "provider-failure", "proxy draft runtime provider failed")
        }
        ProxyAskError::InvalidRuntimeOutput => {
            (3, "invalid-runtime-output", "runtime output is invalid")
        }
        ProxyAskError::UnsafeDraft => (
            3,
            "unsafe-draft",
            "generated draft failed safety validation",
        ),
        ProxyAskError::ClockFailure => (4, "clock-failure", "proxy draft clock is unavailable"),
        ProxyAskError::InvalidProxyDraft => {
            (3, "invalid-proxy-draft", "assembled proxy draft is invalid")
        }
    };
    print_proxy_error(message, category, code, json_mode)
}
