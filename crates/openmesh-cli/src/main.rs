// ============================================================================
// OpenMesh CLI — Dev Track 0.1.3.3
// ============================================================================
// The official programmable integration spine into the file-backed Signal
// Inbox (Dev Track 0.1.3.2). See the approved execution plan:
//   .heli-harness/state/reports/openmesh-0.1.3.3-execution-plan.md
//
// Checkpoint A: argument-parsing skeleton only. All 11 `signal <kind>`
// subcommands are recognized; no project resolution, signal construction, or
// `write_signal` call happens yet (Checkpoints B/C).
// ============================================================================

mod catch_up;
mod collect;
mod context;
mod digest;
mod event;
mod handoff;
mod init;
mod mesh;
mod online_proxy;
mod output;
mod pending;
mod profile;
mod project;
mod proxy;
mod proxy_runtime_factory;
mod proxy_verify;
mod relay;
mod signal;
mod state;
mod team;
mod trust_admin;
mod connector;
mod org;

use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "openmesh-cli",
    version,
    about = "OpenMesh CLI — report WorkSignals into the file-backed Signal Inbox"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize an OpenMesh project marker (`.openmesh/`) for CLI workflows.
    Init(init::InitArgs),
    /// Report a WorkSignal of a specific semantic kind.
    #[command(subcommand)]
    Signal(SignalKindCommand),
    /// Collect local evidence into the Signal Inbox (Dev Track 0.1.3.6).
    #[command(subcommand)]
    Collect(CollectCommand),
    /// Show the rebuildable Current State projection (Dev Track 0.1.3.7).
    State(state::StateArgs),
    /// Build an on-demand local Catch-up view (Dev Track 0.1.3.7).
    CatchUp(catch_up::CatchUpArgs),
    /// Inspect or correct canonical WorkEvents (evidence correction CLI).
    #[command(subcommand)]
    Event(EventCommand),
    /// Manage the local Work Proxy Profile (Dev Track 0.1.4).
    #[command(subcommand)]
    Profile(profile::ProfileCommand),
    /// Build, show, or validate the local Proxy Context Pack (Dev Track 0.1.5).
    #[command(subcommand)]
    Context(context::ContextCommand),
    /// Ask the local Work Proxy for a draft answer (Dev Track 0.1.6).
    #[command(subcommand)]
    Proxy(proxy::ProxyCommand),
    /// Create, approve, show, or export local handoff notes (Dev Track 0.1.8).
    #[command(subcommand)]
    Handoff(handoff::HandoffCommand),
    /// List unified pending questions that need a person (Dev Track 0.1.9).
    Pending(pending::PendingArgs),
    /// Build a return digest for an absence window (Dev Track 0.1.9).
    Digest(digest::DigestArgs),
    /// Local two-person mesh (Dev Track 0.1.10) — peer registry first.
    #[command(subcommand)]
    Mesh(mesh::MeshCommand),
    /// Private relay selective egress (Dev Track 0.1.11).
    #[command(subcommand)]
    Relay(relay::RelayCommand),
    /// Always-online Work Proxy alpha (Dev Track 0.1.12).
    #[command(subcommand, name = "online-proxy")]
    OnlineProxy(online_proxy::OnlineProxyCommand),
    /// Team workspace foundation (Dev Track 0.1.15).
    #[command(subcommand)]
    Team(team::TeamCommand),
    /// Trust, privacy & admin beta (0.1.17).
    #[command(subcommand, name = "trust-admin")]
    TrustAdmin(trust_admin::TrustAdminCommand),
    /// Connector Layer evidence producers (0.1.18).
    #[command(subcommand)]
    Connector(connector::ConnectorCommand),
    /// Organization graph preview (0.1.19).
    #[command(subcommand)]
    Org(org::OrgCommand),
}

#[derive(Subcommand, Debug)]
pub enum EventCommand {
    /// Inspect a WorkEvent, its correction chain, and effective presentation.
    Inspect(event::EventInspectArgs),
    /// Append an append-only correction WorkEvent for a target event.
    Correct(event::EventCorrectArgs),
}

#[derive(Subcommand, Debug)]
pub enum CollectCommand {
    /// Collect Git repository state into a WorkSignal.
    Git(collect::CollectArgs),
    /// Collect Heli harness state into a WorkSignal.
    Heli(collect::CollectArgs),
}

/// One subcommand per `WorkSignalKind` variant (kebab-case, mirroring the
/// protocol's own wire form exactly — approved plan §5). No other top-level
/// command exists in this binary.
#[derive(Subcommand, Debug)]
pub enum SignalKindCommand {
    /// Meaningful implementation progress.
    Progress(SignalArgs),
    /// An architectural/product decision.
    Decision(SignalArgs),
    /// A blocker was discovered.
    Blocker(SignalArgs),
    /// A previously reported blocker was resolved.
    BlockerResolved(SignalArgs),
    /// A material scope change.
    ScopeChange(SignalArgs),
    /// A meaningful checkpoint/milestone.
    Milestone(SignalArgs),
    /// Work requires human review.
    ReviewRequired(SignalArgs),
    /// An unresolved question that could stall continuation.
    UnresolvedQuestion(SignalArgs),
    /// A handoff checkpoint.
    Handoff(SignalArgs),
    /// The session is ending.
    SessionEnd(SignalArgs),
    /// The producing agent/provider is switching.
    AgentSwitch(SignalArgs),
}

impl SignalKindCommand {
    pub fn args(&self) -> &SignalArgs {
        match self {
            SignalKindCommand::Progress(a)
            | SignalKindCommand::Decision(a)
            | SignalKindCommand::Blocker(a)
            | SignalKindCommand::BlockerResolved(a)
            | SignalKindCommand::ScopeChange(a)
            | SignalKindCommand::Milestone(a)
            | SignalKindCommand::ReviewRequired(a)
            | SignalKindCommand::UnresolvedQuestion(a)
            | SignalKindCommand::Handoff(a)
            | SignalKindCommand::SessionEnd(a)
            | SignalKindCommand::AgentSwitch(a) => a,
        }
    }
}

/// Flags shared by every `signal <kind>` subcommand (approved plan §5).
#[derive(Args, Debug, Clone)]
pub struct SignalArgs {
    /// Human-readable claim/summary (required).
    #[arg(long)]
    pub summary: String,

    /// Explicit project path. If omitted, resolved by upward directory
    /// search from the current working directory (§6).
    #[arg(long)]
    pub project: Option<String>,

    /// Producer name. Defaults to `Reporter("cli")` when omitted (§8).
    #[arg(long)]
    pub producer: Option<String>,

    /// Actor spec: `person:<name>` | `device:<name>` | `proxy:<name>` |
    /// `unknown`. Defaults to `unknown` when omitted (§8).
    #[arg(long, value_parser = parse_actor_arg, default_value = "unknown")]
    pub actor: ActorArg,

    /// Repeatable file-path evidence reference.
    #[arg(long = "evidence")]
    pub evidence: Vec<String>,

    /// Repeatable prior-signal evidence reference (by `signal_id`).
    #[arg(long = "evidence-signal")]
    pub evidence_signal: Vec<String>,

    /// Optional correlation hint.
    #[arg(long = "correlation-hint")]
    pub correlation_hint: Option<String>,

    /// Sensitivity: `public` | `team` | `private` | `secret`. Defaults to
    /// `private`, matching `Sensitivity::default()`.
    #[arg(long, value_parser = parse_sensitivity_arg, default_value = "private")]
    pub sensitivity: SensitivityArg,

    /// Explicit `signal_id` override. Defaults to a generated id (§7).
    #[arg(long = "signal-id")]
    pub signal_id: Option<String>,

    /// Explicit RFC 3339 UTC timestamp override. Defaults to "now" (§6/§7).
    #[arg(long)]
    pub timestamp: Option<String>,

    /// Emit machine-readable JSON output instead of human-readable text (§9).
    #[arg(long)]
    pub json: bool,
}

/// A parsed `--actor` value (approved plan §8). Never authentication — a
/// label the producer chose to assert.
#[derive(Clone, Debug)]
pub enum ActorArg {
    Person(String),
    Device(String),
    Proxy(String),
    Unknown,
}

fn parse_actor_arg(s: &str) -> Result<ActorArg, String> {
    if s == "unknown" {
        return Ok(ActorArg::Unknown);
    }
    let (prefix, rest) = s.split_once(':').ok_or_else(|| {
        format!(
            "invalid --actor value `{s}` (expected person:<name>, device:<name>, proxy:<name>, or unknown)"
        )
    })?;
    if rest.is_empty() {
        return Err(format!("--actor `{prefix}:` requires a non-empty name"));
    }
    match prefix {
        "person" => Ok(ActorArg::Person(rest.to_string())),
        "device" => Ok(ActorArg::Device(rest.to_string())),
        "proxy" => Ok(ActorArg::Proxy(rest.to_string())),
        _ => Err(format!(
            "invalid --actor prefix `{prefix}` (expected person, device, proxy, or unknown)"
        )),
    }
}

/// A parsed `--sensitivity` value (approved plan §8), defaulting to `private`.
#[derive(Clone, Debug)]
pub enum SensitivityArg {
    Public,
    Team,
    Private,
    Secret,
}

fn parse_sensitivity_arg(s: &str) -> Result<SensitivityArg, String> {
    match s {
        "public" => Ok(SensitivityArg::Public),
        "team" => Ok(SensitivityArg::Team),
        "private" => Ok(SensitivityArg::Private),
        "secret" => Ok(SensitivityArg::Secret),
        other => Err(format!(
            "invalid --sensitivity value `{other}` (expected public, team, private, or secret)"
        )),
    }
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let cli = Cli::parse();
    let cwd = std::env::current_dir().expect("failed to read current working directory");
    match cli.command {
        Commands::Init(args) => init::run_init(&args, &cwd),
        Commands::Signal(kind) => run_signal(kind),
        Commands::Collect(cmd) => match cmd {
            CollectCommand::Git(args) => collect::run_collect_git(&args, &cwd),
            CollectCommand::Heli(args) => collect::run_collect_heli(&args, &cwd),
        },
        Commands::State(args) => state::run_state(&args, &cwd),
        Commands::CatchUp(args) => catch_up::run_catch_up(&args, &cwd),
        Commands::Event(cmd) => match cmd {
            EventCommand::Inspect(args) => event::run_event_inspect(&args, &cwd),
            EventCommand::Correct(args) => event::run_event_correct(&args, &cwd),
        },
        Commands::Profile(cmd) => profile::run_profile(cmd, &cwd),
        Commands::Context(cmd) => context::run_context(cmd, &cwd),
        Commands::Proxy(cmd) => proxy::run_proxy(cmd, &cwd),
        Commands::Handoff(cmd) => handoff::run_handoff(cmd, &cwd),
        Commands::Pending(args) => pending::run_pending(&args, &cwd),
        Commands::Digest(args) => digest::run_digest(&args, &cwd),
        Commands::Mesh(cmd) => mesh::run_mesh(cmd, &cwd),
        Commands::Relay(cmd) => relay::run_relay(cmd, &cwd),
        Commands::OnlineProxy(cmd) => online_proxy::run_online_proxy(cmd, &cwd),
        Commands::Team(cmd) => team::run_team(cmd, &cwd),
        Commands::TrustAdmin(cmd) => trust_admin::run_trust_admin(cmd, &cwd),
        Commands::Connector(cmd) => connector::run_connector(cmd, &cwd),
        Commands::Org(cmd) => org::run_org(cmd, &cwd),
    }
}

fn run_signal(kind: SignalKindCommand) -> i32 {
    let args = kind.args();
    let json_mode = args.json;

    let cwd = std::env::current_dir().expect("failed to read current working directory");
    let resolved = match project::resolve_project(args.project.as_deref(), &cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), json_mode),
    };

    let signal = signal::build_work_signal(&kind, &resolved);
    let project_path = resolved.path.to_string_lossy().to_string();

    match openmesh_core::signals::write_signal(&project_path, &signal) {
        Ok(()) => {
            output::print_success(&signal, &project_path, json_mode);
            0
        }
        Err(err) => output::print_signal_error(&err, json_mode),
    }
}
