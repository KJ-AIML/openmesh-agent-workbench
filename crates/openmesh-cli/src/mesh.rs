// ============================================================================
// Mesh commands — Dev Track 0.1.10 Checkpoint B (peer registry)
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::mesh::{
    add_peer, list_peers, peer_id_from_label, read_peer, MeshPeerError, MeshPeerRecord,
    MESH_PEER_RECORD_PROTOCOL_VERSION,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum MeshCommand {
    /// Manage local mesh peers (Checkpoint B).
    #[command(subcommand)]
    Peer(MeshPeerCommand),
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
    /// Human-readable peer label (required).
    #[arg(long)]
    pub label: String,

    /// Optional stable peer id (default: slug from label).
    #[arg(long = "id")]
    pub peer_id: Option<String>,

    /// Optional remote Work Proxy profile id.
    #[arg(long = "profile-id")]
    pub profile_id: Option<String>,

    /// Optional remote workspace id when known.
    #[arg(long = "workspace-id")]
    pub workspace_id: Option<String>,

    /// Optional free-text notes.
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

pub fn run_mesh(command: MeshCommand, cwd: &Path) -> i32 {
    match command {
        MeshCommand::Peer(peer) => match peer {
            MeshPeerCommand::Add(args) => run_peer_add(&args, cwd),
            MeshPeerCommand::List(args) => run_peer_list(&args, cwd),
            MeshPeerCommand::Show(args) => run_peer_show(&args, cwd),
        },
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
        Err(err) => print_mesh_error(&err, args.json),
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
        Err(err) => print_mesh_error(&err, args.json),
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
        Err(err) => print_mesh_error(&err, args.json),
    }
}

fn print_mesh_error(err: &MeshPeerError, json_mode: bool) -> i32 {
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
