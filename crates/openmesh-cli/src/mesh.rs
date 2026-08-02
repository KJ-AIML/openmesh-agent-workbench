// ============================================================================
// Mesh commands — Dev Track 0.1.10 Checkpoints B–C (peers + export)
// ============================================================================

use chrono::{Duration, Utc};
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::continuity::load_continuity_input_snapshot;
use openmesh_core::domain::CatchUpWindow;
use openmesh_core::mesh::{
    add_peer, export_mesh_envelope_to_outbox, list_peers, peer_id_from_label, read_peer,
    to_peer_from_registry, BuildMeshExportRequest, MeshExportError, MeshPeerError, MeshPeerRecord,
    MeshPeerRef, MeshSensitivityMax, MESH_PEER_RECORD_PROTOCOL_VERSION,
};
use openmesh_core::profile::{profile_exists, read_work_proxy_profile};
use openmesh_core::storage::read_project;
use openmesh_core::storage::Project;
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;
use crate::state::load_current_state_projection;

#[derive(Subcommand, Debug)]
pub enum MeshCommand {
    /// Manage local mesh peers (Checkpoint B).
    #[command(subcommand)]
    Peer(MeshPeerCommand),
    /// Export a local mesh envelope to outbox (Checkpoint C).
    Export(MeshExportArgs),
}

#[derive(Subcommand, Debug)]
pub enum MeshPeerCommand {
    /// Register a local peer label for mesh exchange.
    Add(MeshPeerAddArgs),
    /// List registered mesh peers.
    List(MeshPeerListArgs),
    /// Show one registered mesh peer.
    Show(MeshPeerShowArgs),
}

#[derive(Args, Debug, Clone)]
pub struct MeshPeerAddArgs {
    #[arg(long)]
    pub label: String,

    #[arg(long = "id")]
    pub peer_id: Option<String>,

    #[arg(long = "profile-id")]
    pub profile_id: Option<String>,

    #[arg(long = "workspace-id")]
    pub workspace_id: Option<String>,

    #[arg(long)]
    pub notes: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct MeshPeerListArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct MeshPeerShowArgs {
    #[arg(long = "id")]
    pub peer_id: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum MeshSensitivityArg {
    Public,
    Team,
    #[default]
    Private,
}

impl From<MeshSensitivityArg> for MeshSensitivityMax {
    fn from(value: MeshSensitivityArg) -> Self {
        match value {
            MeshSensitivityArg::Public => MeshSensitivityMax::Public,
            MeshSensitivityArg::Team => MeshSensitivityMax::Team,
            MeshSensitivityArg::Private => MeshSensitivityMax::Private,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct MeshExportArgs {
    /// Registered peer id to address (required).
    #[arg(long)]
    pub peer: String,

    /// Optional RFC 3339 UTC window start (default: now - 24h). With --no-window, omit window.
    #[arg(long)]
    pub since: Option<String>,

    /// Export without a catch-up window (current-state sections only).
    #[arg(long = "no-window")]
    pub no_window: bool,

    /// Optional envelope id (default: env-<timestamp>).
    #[arg(long = "envelope-id")]
    pub envelope_id: Option<String>,

    /// Max sensitivity for the envelope body.
    #[arg(long, value_enum, default_value_t = MeshSensitivityArg::Private)]
    pub sensitivity: MeshSensitivityArg,

    /// Include local handoff note ids in the envelope.
    #[arg(long = "include-handoffs", default_value_t = true)]
    pub include_handoffs: bool,

    /// Skip including handoff ids.
    #[arg(long = "no-handoffs")]
    pub no_handoffs: bool,

    /// From-peer label override (default: profile owner_label or "local").
    #[arg(long = "from-label")]
    pub from_label: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_mesh(command: MeshCommand, cwd: &Path) -> i32 {
    match command {
        MeshCommand::Peer(peer) => match peer {
            MeshPeerCommand::Add(args) => run_peer_add(&args, cwd),
            MeshPeerCommand::List(args) => run_peer_list(&args, cwd),
            MeshPeerCommand::Show(args) => run_peer_show(&args, cwd),
        },
        MeshCommand::Export(args) => run_export(&args, cwd),
    }
}

fn run_peer_add(args: &MeshPeerAddArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let peer_id = args
        .peer_id
        .clone()
        .unwrap_or_else(|| peer_id_from_label(&args.label));
    let record = MeshPeerRecord {
        protocol_version: MESH_PEER_RECORD_PROTOCOL_VERSION.into(),
        peer_id: peer_id.clone(),
        label: args.label.clone(),
        proxy_profile_id: args.profile_id.clone(),
        remote_workspace_id: args.workspace_id.clone(),
        notes: args.notes.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    match add_peer(&project_path, &record) {
        Ok(stored) => {
            if args.json {
                println!("{}", serde_json::to_value(&stored).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("peer_id={}", stored.peer_id);
                println!("label={}", stored.label);
                println!("path=.openmesh/mesh/peers/{}.json", stored.peer_id);
            }
            0
        }
        Err(err) => print_mesh_peer_error(&err, args.json),
    }
}

fn run_peer_list(args: &MeshPeerListArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match list_peers(&project_path) {
        Ok(peers) => {
            if args.json {
                println!("{}", serde_json::to_value(&peers).unwrap_or(json!([])));
            } else if peers.is_empty() {
                println!("(no mesh peers registered)");
            } else {
                for peer in peers {
                    println!(
                        "{} | {} | workspace={}",
                        peer.peer_id,
                        peer.label,
                        peer.remote_workspace_id.as_deref().unwrap_or("-")
                    );
                }
            }
            0
        }
        Err(err) => print_mesh_peer_error(&err, args.json),
    }
}

fn run_peer_show(args: &MeshPeerShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_peer(&project_path, &args.peer_id) {
        Ok(peer) => {
            if args.json {
                println!("{}", serde_json::to_value(&peer).unwrap_or(json!({})));
            } else {
                println!("peer_id={}", peer.peer_id);
                println!("label={}", peer.label);
                println!(
                    "proxy_profile_id={}",
                    peer.proxy_profile_id.as_deref().unwrap_or("-")
                );
                println!(
                    "remote_workspace_id={}",
                    peer.remote_workspace_id.as_deref().unwrap_or("-")
                );
                println!("notes={}", peer.notes.as_deref().unwrap_or("-"));
                println!("created_at={}", peer.created_at);
                println!("updated_at={}", peer.updated_at);
            }
            0
        }
        Err(err) => print_mesh_peer_error(&err, args.json),
    }
}

fn run_export(args: &MeshExportArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();

    let project: Project = match read_project(&project_path, "project.json") {
        Some(p) => p,
        None => {
            return print_export_error(&MeshExportError::ProjectNotInitialized, args.json);
        }
    };

    let to_peer = match to_peer_from_registry(&project_path, &args.peer) {
        Ok(p) => p,
        Err(err) => return print_export_error(&err, args.json),
    };

    let from_peer = resolve_from_peer(&project_path, &project.id, args.from_label.as_deref());
    let now = Utc::now();
    let now_str = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let envelope_id = args
        .envelope_id
        .clone()
        .unwrap_or_else(|| format!("env-{}", now.format("%Y%m%dT%H%M%SZ")));

    let window = if args.no_window {
        None
    } else {
        match build_window(args.since.as_deref(), &now_str) {
            Ok(w) => Some(w),
            Err(msg) => {
                if args.json {
                    println!(
                        "{}",
                        json!({"status":"error","category":"validation","message":msg})
                    );
                } else {
                    eprintln!("ERROR validation: {msg}");
                }
                return 3;
            }
        }
    };

    let snapshot = match load_continuity_input_snapshot(&project_path) {
        Ok(s) => s,
        Err(err) => {
            return print_export_error(
                &MeshExportError::Continuity(err.to_string()),
                args.json,
            );
        }
    };
    let current_state = match load_current_state_projection(&project_path, false) {
        Ok(s) => s,
        Err(err) => {
            return print_export_error(
                &MeshExportError::Continuity(err.to_string()),
                args.json,
            );
        }
    };

    let include_handoffs = args.include_handoffs && !args.no_handoffs;
    let request = BuildMeshExportRequest {
        workspace_id: project.id,
        from_peer,
        to_peer: Some(to_peer),
        window,
        now_rfc3339: now_str,
        envelope_id,
        sensitivity_max: args.sensitivity.into(),
        include_handoff_ids: include_handoffs,
    };

    match export_mesh_envelope_to_outbox(&project_path, &snapshot, &current_state, &request) {
        Ok(envelope) => {
            if args.json {
                println!("{}", serde_json::to_value(&envelope).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("envelope_id={}", envelope.envelope_id);
                println!(
                    "path=.openmesh/mesh/outbox/{}.json",
                    envelope.envelope_id
                );
                println!("evidence_items={}", envelope.evidence_items.len());
                println!("handoff_ids={}", envelope.handoff_ids.len());
                println!("limitations={}", envelope.limitations.len());
            }
            0
        }
        Err(err) => print_export_error(&err, args.json),
    }
}

fn resolve_from_peer(project_path: &str, workspace_id: &str, label_override: Option<&str>) -> MeshPeerRef {
    let label = if let Some(l) = label_override {
        l.to_string()
    } else if profile_exists(project_path).unwrap_or(false) {
        read_work_proxy_profile(project_path)
            .map(|p| p.owner_label)
            .unwrap_or_else(|_| "local".into())
    } else {
        "local".into()
    };
    let profile_id = read_work_proxy_profile(project_path)
        .ok()
        .map(|p| p.profile_id);
    MeshPeerRef {
        label,
        proxy_profile_id: profile_id,
        workspace_id: Some(workspace_id.to_string()),
    }
}

fn build_window(since_override: Option<&str>, until: &str) -> Result<CatchUpWindow, String> {
    let since = match since_override {
        Some(raw) => {
            validate_utc(raw)?;
            raw.to_string()
        }
        None => {
            let until_dt = chrono::DateTime::parse_from_rfc3339(until)
                .map_err(|e| e.to_string())?
                .with_timezone(&Utc);
            let since_dt = until_dt - Duration::hours(24);
            since_dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        }
    };
    Ok(CatchUpWindow {
        since,
        until: until.to_string(),
    })
}

fn validate_utc(raw: &str) -> Result<(), String> {
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| format!("invalid --since value `{raw}` (expected RFC 3339 UTC)"))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!(
            "invalid --since value `{raw}` (timestamp must use UTC Z offset)"
        ));
    }
    Ok(())
}

fn print_mesh_peer_error(err: &MeshPeerError, json_mode: bool) -> i32 {
    let (code, category) = match err {
        MeshPeerError::ProjectNotInitialized => (1, "project"),
        MeshPeerError::NotFound => (3, "not-found"),
        MeshPeerError::AlreadyExists(_) => (3, "conflict"),
        MeshPeerError::ValidationFailed(_) | MeshPeerError::MalformedJson => (3, "validation"),
        MeshPeerError::ReadFailed
        | MeshPeerError::WriteFailed
        | MeshPeerError::AtomicReplaceFailed => (4, "io"),
    };
    let message = err.to_string();
    if json_mode {
        println!(
            "{}",
            json!({"status":"error","category":category,"message":message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    code
}

fn print_export_error(err: &MeshExportError, json_mode: bool) -> i32 {
    let (code, category) = match err {
        MeshExportError::ProjectNotInitialized => (1, "project"),
        MeshExportError::NotFound => (3, "not-found"),
        MeshExportError::AlreadyExists(_) => (3, "conflict"),
        MeshExportError::WorkspaceMismatch | MeshExportError::Validation(_) => (3, "validation"),
        MeshExportError::Peer(p) => return print_mesh_peer_error(p, json_mode),
        MeshExportError::Continuity(_) | MeshExportError::Handoff(_) => (4, "read-failed"),
        MeshExportError::WriteFailed | MeshExportError::AtomicReplaceFailed => (4, "io"),
    };
    let message = err.to_string();
    if json_mode {
        println!(
            "{}",
            json!({"status":"error","category":category,"message":message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    code
}
