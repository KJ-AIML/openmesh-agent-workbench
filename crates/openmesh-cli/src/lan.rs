// ============================================================================
// LAN commands — Dev Track 0.1.22 LAN Relay + Live Ask
// ============================================================================

use clap::{Args, Subcommand};
use openmesh_core::lan::{
    ask_peer, lan_serve_status_for_project, listen_beacons, parse_host_port, send_package_to_peer,
    start_lan_serve, stop_lan_serve, PeerTable, DEFAULT_HTTP_PORT, DEFAULT_UDP_PORT, LAN_PROTOCOL,
};
use openmesh_core::relay::{
    is_package_approved, read_approved_package, RelayTransportError,
};
use openmesh_core::storage::{read_project, Project};
use serde_json::json;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum LanCommand {
    /// Start UDP beacon + HTTP listener (blocks until Enter / --seconds).
    Serve(LanServeArgs),
    /// Listen for LAN beacons and print peer table.
    Discover(LanDiscoverArgs),
    /// POST an approved relay package to a peer host:port.
    Send(LanSendArgs),
    /// Live-ask a peer's local Work Proxy over LAN (read-only draft).
    Ask(LanAskArgs),
    /// Show local LAN serve status.
    Status(LanStatusArgs),
}

#[derive(Args, Debug, Clone)]
pub struct LanServeArgs {
    /// HTTP bind host (default 0.0.0.0).
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Preferred HTTP port (falls back to ephemeral if busy).
    #[arg(long, default_value_t = DEFAULT_HTTP_PORT)]
    pub http_port: u16,

    /// UDP beacon port.
    #[arg(long, default_value_t = DEFAULT_UDP_PORT)]
    pub udp_port: u16,

    /// Optional owner label override for beacons.
    #[arg(long)]
    pub owner: Option<String>,

    /// Auto-stop after N seconds (for tests / dogfood scripts).
    #[arg(long)]
    pub seconds: Option<u64>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LanDiscoverArgs {
    /// Seconds to listen for beacons.
    #[arg(long, default_value_t = 3)]
    pub seconds: u64,

    #[arg(long, default_value_t = DEFAULT_UDP_PORT)]
    pub udp_port: u16,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LanSendArgs {
    #[arg(long = "id")]
    pub package_id: String,

    /// Peer address as host:port.
    #[arg(long = "to")]
    pub to: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LanAskArgs {
    /// Peer address as host:port.
    #[arg(long = "to")]
    pub to: String,

    #[arg(long)]
    pub question: String,

    #[arg(long)]
    pub tier: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct LanStatusArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_lan(command: LanCommand, cwd: &Path) -> i32 {
    match command {
        LanCommand::Serve(a) => run_serve(&a, cwd),
        LanCommand::Discover(a) => run_discover(&a, cwd),
        LanCommand::Send(a) => run_send(&a, cwd),
        LanCommand::Ask(a) => run_ask(&a, cwd),
        LanCommand::Status(a) => run_status(&a, cwd),
    }
}

fn run_serve(args: &LanServeArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    if read_project::<Project>(&project_path, "project.json").is_none() {
        return err_json(args.json, 1, "project", "project not initialized");
    }

    let handle = match start_lan_serve(
        &project_path,
        &args.host,
        args.http_port,
        args.udp_port,
        args.owner.as_deref(),
    ) {
        Ok(h) => h,
        Err(e) => return err_json(args.json, 3, "serve", &e.to_string()),
    };

    if args.json {
        println!(
            "{}",
            serde_json::to_value(&handle.status).unwrap_or(json!({}))
        );
    } else {
        println!("status=ok");
        println!("protocol={}", LAN_PROTOCOL);
        println!(
            "http={}:{}",
            handle.status.http_host.as_deref().unwrap_or(&args.host),
            handle.status.http_port.unwrap_or(0)
        );
        println!("udp_port={}", handle.status.udp_port.unwrap_or(args.udp_port));
        println!(
            "peer_id={}",
            handle.status.peer_id.as_deref().unwrap_or("-")
        );
        println!(
            "owner={}",
            handle.status.owner_label.as_deref().unwrap_or("-")
        );
        if let Some(note) = &handle.status.note {
            println!("note={note}");
        }
        if args.seconds.is_none() {
            println!("hint=Press Enter to stop (macOS may prompt for firewall on first bind).");
        }
    }
    let _ = io::stdout().flush();

    if let Some(secs) = args.seconds {
        thread::sleep(Duration::from_secs(secs.max(1)));
    } else {
        wait_for_enter_or_signal();
    }

    match stop_lan_serve() {
        Ok(st) => {
            if args.json {
                // already printed start status; print stop as second line object when json
                eprintln!("{}", serde_json::to_value(&st).unwrap_or(json!({})));
            } else {
                println!("stopped=true");
            }
            0
        }
        Err(e) => err_json(args.json, 3, "stop", &e.to_string()),
    }
}

fn wait_for_enter_or_signal() {
    let stop = Arc::new(AtomicBool::new(false));
    let stop2 = stop.clone();
    thread::spawn(move || {
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        stop2.store(true, Ordering::SeqCst);
    });
    while !stop.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
}

fn run_discover(args: &LanDiscoverArgs, cwd: &Path) -> i32 {
    // project optional — discover is network-local; resolve only to ignore self if present
    let ignore = resolve_project(args.project.as_deref(), cwd)
        .ok()
        .and_then(|r| {
            let path = r.path.to_string_lossy().to_string();
            read_project::<Project>(&path, "project.json").map(|p| format!("lan-{}", p.id))
        });

    let table = PeerTable::new();
    match listen_beacons(
        &table,
        args.udp_port,
        args.seconds,
        ignore.as_deref(),
    ) {
        Ok(peers) => {
            if args.json {
                println!("{}", serde_json::to_value(&peers).unwrap_or(json!([])));
            } else {
                println!("status=ok");
                println!("protocol={}", LAN_PROTOCOL);
                println!("peers={}", peers.len());
                for p in peers {
                    println!(
                        "peer\t{}\t{}\t{}\t{}",
                        p.owner_label, p.peer_id, p.address, p.last_seen_at
                    );
                }
            }
            0
        }
        Err(e) => err_json(args.json, 3, "discover", &e.to_string()),
    }
}

fn run_send(args: &LanSendArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let (host, port) = match parse_host_port(&args.to) {
        Ok(v) => v,
        Err(e) => return err_json(args.json, 2, "address", &e.to_string()),
    };
    let pkg = match read_approved_package(&project_path, &args.package_id) {
        Ok(p) => p,
        Err(e) => return err_json(args.json, 3, "package", &e.to_string()),
    };
    if !is_package_approved(&pkg) {
        return err_json(
            args.json,
            3,
            "package",
            &RelayTransportError::NotApproved.to_string(),
        );
    }
    match send_package_to_peer(&host, port, &pkg) {
        Ok(v) => {
            if args.json {
                println!("{v}");
            } else {
                println!("status=ok");
                println!("package_id={}", args.package_id);
                println!("to={}:{}", host, port);
                println!("transport=lan-http");
            }
            0
        }
        Err(e) => err_json(args.json, 3, "send", &e.to_string()),
    }
}

fn run_ask(args: &LanAskArgs, cwd: &Path) -> i32 {
    let _ = resolve_project(args.project.as_deref(), cwd); // optional project context
    let (host, port) = match parse_host_port(&args.to) {
        Ok(v) => v,
        Err(e) => return err_json(args.json, 2, "address", &e.to_string()),
    };
    match ask_peer(&host, port, &args.question, args.tier.as_deref()) {
        Ok(answer) => {
            if args.json {
                println!("{}", serde_json::to_value(&answer).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("peer_id={}", answer.peer_id);
                println!("peer_label={}", answer.peer_label);
                println!("read_only={}", answer.read_only);
                println!("refused={}", answer.refused);
                println!("freshness={}", answer.freshness.statement);
                println!("---");
                println!("{}", answer.answer_text);
            }
            0
        }
        Err(e) => err_json(args.json, 3, "ask", &e.to_string()),
    }
}

fn run_status(args: &LanStatusArgs, cwd: &Path) -> i32 {
    let status = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => {
            let project_path = r.path.to_string_lossy().to_string();
            lan_serve_status_for_project(&project_path)
        }
        Err(_) => openmesh_core::lan::current_lan_serve_status(),
    };
    if args.json {
        println!("{}", serde_json::to_value(&status).unwrap_or(json!({})));
    } else {
        println!("status=ok");
        println!("running={}", status.running);
        println!("protocol={}", status.protocol);
        if let Some(p) = &status.http_port {
            println!(
                "http={}:{}",
                status.http_host.as_deref().unwrap_or("0.0.0.0"),
                p
            );
        }
        if let Some(u) = status.udp_port {
            println!("udp_port={u}");
        }
        if let Some(id) = &status.peer_id {
            println!("peer_id={id}");
        }
        if let Some(n) = &status.note {
            println!("note={n}");
        }
    }
    0
}

fn err_json(json_mode: bool, code: i32, kind: &str, msg: &str) -> i32 {
    if json_mode {
        println!("{}", json!({ "error": { "kind": kind, "message": msg } }));
    } else {
        eprintln!("error[{kind}]={msg}");
    }
    code
}
