//! Trust, Privacy & Admin CLI — Dev Track 0.1.17.

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::trust_admin::{
    append_audit_event, init_trust_policy, list_audit_events, read_trust_policy, update_trust_policy,
    AdminAuditEvent, AuditAction, QueryAllowEntry, QueryAllowlistMode, TrustAdminStorageError,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum TrustAdminCommand {
    /// Initialize trust/privacy/admin policy (requires team init).
    Init(TrustInitArgs),
    /// Show the current policy snapshot.
    Show(TrustShowArgs),
    /// Set query allowlist mode.
    #[command(name = "set-query-mode")]
    SetQueryMode(SetQueryModeArgs),
    /// Enable or disable remote query entirely.
    #[command(name = "set-remote-query")]
    SetRemoteQuery(SetRemoteQueryArgs),
    /// Manage query allowlist.
    #[command(subcommand)]
    Allowlist(AllowlistCommand),
    /// List admin audit events.
    Audit(AuditListArgs),
}

#[derive(Subcommand, Debug)]
pub enum AllowlistCommand {
    Add(AllowlistAddArgs),
    Remove(AllowlistRemoveArgs),
    List(AllowlistListArgs),
}

#[derive(Args, Debug, Clone)]
pub struct TrustInitArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TrustShowArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum QueryModeArg {
    #[default]
    #[value(name = "allow-all")]
    AllowAll,
    #[value(name = "allowlist-only")]
    AllowlistOnly,
    #[value(name = "deny-all")]
    DenyAll,
}

impl From<QueryModeArg> for QueryAllowlistMode {
    fn from(v: QueryModeArg) -> Self {
        match v {
            QueryModeArg::AllowAll => QueryAllowlistMode::AllowAll,
            QueryModeArg::AllowlistOnly => QueryAllowlistMode::AllowlistOnly,
            QueryModeArg::DenyAll => QueryAllowlistMode::DenyAll,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct SetQueryModeArgs {
    #[arg(long, value_enum)]
    pub mode: QueryModeArg,
    #[arg(long = "actor", default_value = "owner-local")]
    pub actor: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct SetRemoteQueryArgs {
    #[arg(long)]
    pub enabled: bool,
    #[arg(long = "actor", default_value = "owner-local")]
    pub actor: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AllowlistAddArgs {
    #[arg(long = "member-id")]
    pub member_id: Option<String>,
    #[arg(long = "peer")]
    pub mesh_peer_id: Option<String>,
    #[arg(long)]
    pub note: Option<String>,
    #[arg(long = "actor", default_value = "owner-local")]
    pub actor: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AllowlistRemoveArgs {
    #[arg(long = "member-id")]
    pub member_id: Option<String>,
    #[arg(long = "peer")]
    pub mesh_peer_id: Option<String>,
    #[arg(long = "actor", default_value = "owner-local")]
    pub actor: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AllowlistListArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct AuditListArgs {
    #[arg(long, default_value_t = 50)]
    pub limit: usize,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_trust_admin(command: TrustAdminCommand, cwd: &Path) -> i32 {
    match command {
        TrustAdminCommand::Init(a) => run_init(&a, cwd),
        TrustAdminCommand::Show(a) => run_show(&a, cwd),
        TrustAdminCommand::SetQueryMode(a) => run_set_mode(&a, cwd),
        TrustAdminCommand::SetRemoteQuery(a) => run_set_remote(&a, cwd),
        TrustAdminCommand::Allowlist(AllowlistCommand::Add(a)) => run_allow_add(&a, cwd),
        TrustAdminCommand::Allowlist(AllowlistCommand::Remove(a)) => run_allow_remove(&a, cwd),
        TrustAdminCommand::Allowlist(AllowlistCommand::List(a)) => run_allow_list(&a, cwd),
        TrustAdminCommand::Audit(a) => run_audit(&a, cwd),
    }
}

fn run_init(args: &TrustInitArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match init_trust_policy(&project_path) {
        Ok(p) => {
            let _ = audit(
                &project_path,
                &p.team_id,
                "owner-local",
                AuditAction::PolicyInit,
                "policy initialized",
            );
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_show(args: &TrustShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_trust_policy(&project_path) {
        Ok(p) => {
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_set_mode(args: &SetQueryModeArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let mut p = match read_trust_policy(&project_path) {
        Ok(p) => p,
        Err(e) => return err_store(e, args.json),
    };
    p.query_allowlist_mode = args.mode.into();
    match update_trust_policy(&project_path, p) {
        Ok(p) => {
            let _ = audit(
                &project_path,
                &p.team_id,
                &args.actor,
                AuditAction::PolicyUpdate,
                &format!("query_allowlist_mode={:?}", p.query_allowlist_mode),
            );
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_set_remote(args: &SetRemoteQueryArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let mut p = match read_trust_policy(&project_path) {
        Ok(p) => p,
        Err(e) => return err_store(e, args.json),
    };
    p.remote_query_enabled = args.enabled;
    match update_trust_policy(&project_path, p) {
        Ok(p) => {
            let _ = audit(
                &project_path,
                &p.team_id,
                &args.actor,
                AuditAction::PolicyUpdate,
                &format!("remote_query_enabled={}", p.remote_query_enabled),
            );
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_allow_add(args: &AllowlistAddArgs, cwd: &Path) -> i32 {
    if args.member_id.is_none() && args.mesh_peer_id.is_none() {
        return err_msg(args.json, "allowlist", "need --member-id and/or --peer");
    }
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let mut p = match read_trust_policy(&project_path) {
        Ok(p) => p,
        Err(e) => return err_store(e, args.json),
    };
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    p.query_allowlist.push(QueryAllowEntry {
        member_id: args.member_id.clone(),
        mesh_peer_id: args.mesh_peer_id.clone(),
        note: args.note.clone(),
        added_at: now,
    });
    match update_trust_policy(&project_path, p) {
        Ok(p) => {
            let _ = audit(
                &project_path,
                &p.team_id,
                &args.actor,
                AuditAction::AllowlistAdd,
                &format!(
                    "member={:?} peer={:?}",
                    args.member_id, args.mesh_peer_id
                ),
            );
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_allow_remove(args: &AllowlistRemoveArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let mut p = match read_trust_policy(&project_path) {
        Ok(p) => p,
        Err(e) => return err_store(e, args.json),
    };
    let before = p.query_allowlist.len();
    p.query_allowlist.retain(|e| {
        let member_match = match (&args.member_id, &e.member_id) {
            (Some(want), Some(have)) => want == have,
            (Some(_), None) => false,
            (None, _) => false,
        };
        let peer_match = match (&args.mesh_peer_id, &e.mesh_peer_id) {
            (Some(want), Some(have)) => want == have,
            (Some(_), None) => false,
            (None, _) => false,
        };
        // keep if neither specified match hit
        !(member_match || peer_match)
    });
    if p.query_allowlist.len() == before {
        return err_msg(args.json, "allowlist", "no matching entry removed");
    }
    match update_trust_policy(&project_path, p) {
        Ok(p) => {
            let _ = audit(
                &project_path,
                &p.team_id,
                &args.actor,
                AuditAction::AllowlistRemove,
                &format!(
                    "member={:?} peer={:?}",
                    args.member_id, args.mesh_peer_id
                ),
            );
            print_policy(&p, args.json);
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_allow_list(args: &AllowlistListArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_trust_policy(&project_path) {
        Ok(p) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::to_value(&p.query_allowlist).unwrap_or(json!([]))
                );
            } else if p.query_allowlist.is_empty() {
                println!("(empty allowlist) mode={:?}", p.query_allowlist_mode);
            } else {
                println!("mode={:?}", p.query_allowlist_mode);
                for e in &p.query_allowlist {
                    println!(
                        "  member={} peer={} note={}",
                        e.member_id.as_deref().unwrap_or("-"),
                        e.mesh_peer_id.as_deref().unwrap_or("-"),
                        e.note.as_deref().unwrap_or("-")
                    );
                }
            }
            0
        }
        Err(e) => err_store(e, args.json),
    }
}

fn run_audit(args: &AuditListArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match list_audit_events(&project_path, Some(args.limit)) {
        Ok(events) => {
            if args.json {
                println!("{}", serde_json::to_value(&events).unwrap_or(json!([])));
            } else if events.is_empty() {
                println!("(no audit events)");
            } else {
                for e in events {
                    println!(
                        "{} | {:?} | actor={} | {}",
                        e.at, e.action, e.actor_member_id, e.detail
                    );
                }
            }
            0
        }
        Err(e) => err_msg(args.json, "audit", &e.to_string()),
    }
}

fn print_policy(p: &openmesh_core::trust_admin::TeamTrustPolicy, json: bool) {
    if json {
        println!("{}", serde_json::to_value(p).unwrap_or(json!({})));
    } else {
        println!("team_id={}", p.team_id);
        println!("remote_query_enabled={}", p.remote_query_enabled);
        println!("query_allowlist_mode={:?}", p.query_allowlist_mode);
        println!("allowlist_entries={}", p.query_allowlist.len());
        println!("secret_topics_fail_closed={}", p.secret_topics_fail_closed);
        println!("allow_secret_export={}", p.allow_secret_export);
        println!("sync_require_selective={}", p.sync_require_selective);
        println!("admins={}", p.admin_member_ids.join(","));
    }
}

fn audit(
    project_path: &str,
    team_id: &str,
    actor: &str,
    action: AuditAction,
    detail: &str,
) -> Result<(), String> {
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let ev = AdminAuditEvent {
        event_id: format!("ta-{}", now.replace(':', "")),
        team_id: team_id.into(),
        actor_member_id: actor.into(),
        action,
        detail: detail.into(),
        at: now,
    };
    append_audit_event(project_path, &ev).map_err(|e| e.to_string())
}

fn err_store(e: TrustAdminStorageError, json: bool) -> i32 {
    err_msg(json, "trust_admin", &e.to_string())
}

fn err_msg(json: bool, category: &str, message: &str) -> i32 {
    if json {
        println!(
            "{}",
            json!({ "error": { "category": category, "message": message } })
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    1
}
