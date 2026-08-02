//! Team workspace CLI — Dev Track 0.1.15.

use chrono::Utc;
use clap::{Args, Subcommand, ValueEnum};
use openmesh_core::authority_policy::FreshnessTier;
use openmesh_core::mesh::{query_remote_peer_proxy, MeshRemoteQueryRequest};
use openmesh_core::profile::read_work_proxy_profile;
use openmesh_core::team::{
    add_team_member, init_team_workspace, list_team_members, read_team_workspace, remove_team_member,
    TeamMember, TeamMemberRole, TeamStorageError,
};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum TeamCommand {
    /// Initialize a local team workspace registry.
    Init(TeamInitArgs),
    /// Show the team workspace.
    Show(TeamShowArgs),
    /// Manage team members.
    #[command(subcommand)]
    Member(TeamMemberCommand),
    /// Query a team member's offline proxy (read-only; uses linked mesh peer).
    Query(TeamQueryArgs),
}

#[derive(Subcommand, Debug)]
pub enum TeamMemberCommand {
    Add(TeamMemberAddArgs),
    List(TeamMemberListArgs),
    Remove(TeamMemberRemoveArgs),
}

#[derive(Args, Debug, Clone)]
pub struct TeamInitArgs {
    #[arg(long)]
    pub name: String,
    #[arg(long = "owner-label")]
    pub owner_label: Option<String>,
    #[arg(long = "team-id")]
    pub team_id: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TeamShowArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum TeamRoleArg {
    Owner,
    #[default]
    Member,
    Observer,
}

impl From<TeamRoleArg> for TeamMemberRole {
    fn from(v: TeamRoleArg) -> Self {
        match v {
            TeamRoleArg::Owner => TeamMemberRole::Owner,
            TeamRoleArg::Member => TeamMemberRole::Member,
            TeamRoleArg::Observer => TeamMemberRole::Observer,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct TeamMemberAddArgs {
    #[arg(long)]
    pub label: String,
    #[arg(long = "id")]
    pub member_id: Option<String>,
    #[arg(long, value_enum, default_value_t = TeamRoleArg::Member)]
    pub role: TeamRoleArg,
    #[arg(long = "peer")]
    pub mesh_peer_id: Option<String>,
    #[arg(long = "profile-id")]
    pub proxy_profile_id: Option<String>,
    #[arg(long = "workspace-id")]
    pub remote_workspace_id: Option<String>,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TeamMemberListArgs {
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TeamMemberRemoveArgs {
    #[arg(long = "id")]
    pub member_id: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct TeamQueryArgs {
    /// Member id or label.
    #[arg(long)]
    pub member: String,
    #[arg(long)]
    pub question: String,
    #[arg(long, default_value = "low-impact")]
    pub tier: String,
    #[arg(long)]
    pub project: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub fn run_team(command: TeamCommand, cwd: &Path) -> i32 {
    match command {
        TeamCommand::Init(a) => run_init(&a, cwd),
        TeamCommand::Show(a) => run_show(&a, cwd),
        TeamCommand::Member(TeamMemberCommand::Add(a)) => run_member_add(&a, cwd),
        TeamCommand::Member(TeamMemberCommand::List(a)) => run_member_list(&a, cwd),
        TeamCommand::Member(TeamMemberCommand::Remove(a)) => run_member_remove(&a, cwd),
        TeamCommand::Query(a) => run_query(&a, cwd),
    }
}

fn run_init(args: &TeamInitArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let owner = args
        .owner_label
        .clone()
        .or_else(|| read_work_proxy_profile(&project_path).ok().map(|p| p.owner_label))
        .unwrap_or_else(|| "local-owner".into());
    match init_team_workspace(
        &project_path,
        &args.name,
        &owner,
        args.team_id.clone(),
    ) {
        Ok(ws) => {
            if args.json {
                println!("{}", serde_json::to_value(&ws).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("team_id={}", ws.team_id);
                println!("display_name={}", ws.display_name);
                println!("members={}", ws.members.len());
            }
            0
        }
        Err(e) => err_team(e, args.json),
    }
}

fn run_show(args: &TeamShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match read_team_workspace(&project_path) {
        Ok(ws) => {
            if args.json {
                println!("{}", serde_json::to_value(&ws).unwrap_or(json!({})));
            } else {
                println!("team_id={}", ws.team_id);
                println!("display_name={}", ws.display_name);
                println!("host_workspace_id={}", ws.host_workspace_id);
                println!("members={}", ws.members.len());
                for m in &ws.members {
                    println!(
                        "  {} | {} | {:?} | peer={}",
                        m.member_id,
                        m.label,
                        m.role,
                        m.mesh_peer_id.as_deref().unwrap_or("-")
                    );
                }
            }
            0
        }
        Err(e) => err_team(e, args.json),
    }
}

fn run_member_add(args: &TeamMemberAddArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let now = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let member_id = args
        .member_id
        .clone()
        .unwrap_or_else(|| format!("m-{}", slug(&args.label)));
    let member = TeamMember {
        member_id,
        label: args.label.clone(),
        role: args.role.into(),
        mesh_peer_id: args.mesh_peer_id.clone(),
        proxy_profile_id: args.proxy_profile_id.clone(),
        remote_workspace_id: args.remote_workspace_id.clone(),
        joined_at: now,
    };
    match add_team_member(&project_path, member) {
        Ok(ws) => {
            if args.json {
                println!("{}", serde_json::to_value(&ws).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("members={}", ws.members.len());
            }
            0
        }
        Err(e) => err_team(e, args.json),
    }
}

fn run_member_list(args: &TeamMemberListArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match list_team_members(&project_path) {
        Ok(members) => {
            if args.json {
                println!("{}", serde_json::to_value(&members).unwrap_or(json!([])));
            } else if members.is_empty() {
                println!("(no members)");
            } else {
                for m in members {
                    println!(
                        "{} | {} | {:?} | peer={}",
                        m.member_id,
                        m.label,
                        m.role,
                        m.mesh_peer_id.as_deref().unwrap_or("-")
                    );
                }
            }
            0
        }
        Err(e) => err_team(e, args.json),
    }
}

fn run_member_remove(args: &TeamMemberRemoveArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    match remove_team_member(&project_path, &args.member_id) {
        Ok(ws) => {
            if args.json {
                println!("{}", serde_json::to_value(&ws).unwrap_or(json!({})));
            } else {
                println!("status=ok");
                println!("members={}", ws.members.len());
            }
            0
        }
        Err(e) => err_team(e, args.json),
    }
}

fn run_query(args: &TeamQueryArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(r) => r,
        Err(e) => return output::print_project_resolution_error(&e.describe(), args.json),
    };
    let project_path = resolved.path.to_string_lossy().to_string();
    let ws = match read_team_workspace(&project_path) {
        Ok(w) => w,
        Err(e) => return err_team(e, args.json),
    };
    let key = args.member.trim().to_ascii_lowercase();
    let member = match ws.members.iter().find(|m| {
        m.member_id.to_ascii_lowercase() == key || m.label.to_ascii_lowercase() == key
    }) {
        Some(m) => m,
        None => {
            return err_msg(args.json, "member", &format!("member not found: {}", args.member));
        }
    };
    let peer = match &member.mesh_peer_id {
        Some(p) => p.clone(),
        None => {
            return err_msg(
                args.json,
                "peer",
                "member has no mesh_peer_id; link with team member add --peer",
            );
        }
    };
    let tier = match args.tier.as_str() {
        "low-impact" | "LowImpact" => FreshnessTier::LowImpact,
        "critical" | "Critical" => FreshnessTier::Critical,
        _ => FreshnessTier::Standard,
    };
    let now = Utc::now();
    let req = MeshRemoteQueryRequest {
        peer,
        question: args.question.clone(),
        query_id: format!("tq-{}", now.format("%Y%m%dT%H%M%SZ")),
        now,
        freshness_tier: tier,
        include_relay_received: true,
    };
    match query_remote_peer_proxy(&project_path, &req, true) {
        Ok(ans) => {
            if args.json {
                println!("{}", serde_json::to_value(&ans).unwrap_or(json!({})));
            } else {
                println!("team_id={}", ws.team_id);
                println!("member={} ({})", member.label, member.member_id);
                println!("read_only={}", ans.read_only);
                println!("refused={}", ans.refused);
                println!("freshness={}", ans.freshness.statement);
                println!("---");
                println!("{}", ans.answer_text);
            }
            0
        }
        Err(e) => err_msg(args.json, "query", &e.to_string()),
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .chars()
        .take(32)
        .collect()
}

fn err_team(e: TeamStorageError, json: bool) -> i32 {
    err_msg(json, "team", &e.to_string())
}

fn err_msg(json: bool, category: &str, message: &str) -> i32 {
    if json {
        println!(
            "{}",
            json!({"status":"error","category":category,"message":message})
        );
    } else {
        eprintln!("ERROR {category}: {message}");
    }
    3
}
