// ============================================================================
// Profile commands — Dev Track 0.1.4 Checkpoint D
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::domain::{
    default_work_proxy_profile, deterministic_work_proxy_profile_id, ProxyAuthorityLevel,
    UnsupportedClaimBehavior, WorkProxyProfile,
};
use openmesh_core::profile::{
    profile_exists, read_work_proxy_profile, work_proxy_profile_path, write_work_proxy_profile,
    ProfileError,
};
use openmesh_core::profile_validation::{resolve_profile_authority, ProfileEvaluationContext};
use serde_json::json;
use std::path::Path;

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum ProfileCommand {
    /// Create a conservative local Work Proxy Profile for the project.
    Init(ProfileInitArgs),
    /// Show the stored Work Proxy Profile (read-only).
    Show(ProfileShowArgs),
    /// Validate the stored Work Proxy Profile without writing.
    Validate(ProfileValidateArgs),
    /// Update selected profile metadata fields.
    Update(ProfileUpdateArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ProfileInitArgs {
    #[arg(long)]
    pub owner_label: String,

    #[arg(long)]
    pub role_label: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ProfileShowArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ProfileValidateArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ProfileUpdateArgs {
    #[arg(long)]
    pub owner_label: Option<String>,

    #[arg(long)]
    pub role_label: Option<String>,

    #[arg(long)]
    pub working_style: Option<String>,

    #[arg(long)]
    pub communication_style: Option<String>,

    #[arg(long)]
    pub decision_preferences: Option<String>,

    #[arg(long)]
    pub limitations: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_profile(command: ProfileCommand, cwd: &Path) -> i32 {
    match command {
        ProfileCommand::Init(args) => run_profile_init(&args, cwd),
        ProfileCommand::Show(args) => run_profile_show(&args, cwd),
        ProfileCommand::Validate(args) => run_profile_validate(&args, cwd),
        ProfileCommand::Update(args) => run_profile_update(&args, cwd),
    }
}

pub fn run_profile_init(args: &ProfileInitArgs, cwd: &Path) -> i32 {
    let owner_label = args.owner_label.trim();
    let role_label = args.role_label.trim();
    if owner_label.is_empty() {
        return print_invalid_profile_request("owner_label is empty after trim", args.json);
    }
    if role_label.is_empty() {
        return print_invalid_profile_request("role_label is empty after trim", args.json);
    }

    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match profile_exists(&project_path) {
        Ok(true) => {
            return print_profile_error(
                "work proxy profile already exists",
                "profile-already-exists",
                3,
                args.json,
            );
        }
        Ok(false) => {}
        Err(err) => return print_core_profile_error(&err, args.json),
    }

    let timestamp = utc_now_rfc3339();
    let profile_id = deterministic_work_proxy_profile_id(&resolved.project.id);
    let profile = default_work_proxy_profile(
        resolved.project.id.clone(),
        profile_id,
        owner_label,
        role_label,
        timestamp,
    );

    match write_work_proxy_profile(&project_path, &profile) {
        Ok(()) => {
            print_init_success(&profile, &project_path, args.json);
            0
        }
        Err(err) => print_core_profile_error(&err, args.json),
    }
}

pub fn run_profile_show(args: &ProfileShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_work_proxy_profile(&project_path) {
        Ok(profile) => {
            print_show_success(&profile, &project_path, args.json);
            0
        }
        Err(err) => print_core_profile_error(&err, args.json),
    }
}

pub fn run_profile_validate(args: &ProfileValidateArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_work_proxy_profile(&project_path) {
        Ok(profile) => {
            print_validate_success(&profile, &project_path, args.json);
            0
        }
        Err(err) => print_core_profile_error(&err, args.json),
    }
}

pub fn run_profile_update(args: &ProfileUpdateArgs, cwd: &Path) -> i32 {
    if !update_fields_present(args) {
        return print_invalid_profile_request("at least one update field is required", args.json);
    }

    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let mut profile = match read_work_proxy_profile(&project_path) {
        Ok(profile) => profile,
        Err(err) => return print_core_profile_error(&err, args.json),
    };

    if let Some(owner_label) = &args.owner_label {
        let trimmed = owner_label.trim();
        if trimmed.is_empty() {
            return print_invalid_profile_request("owner_label is empty after trim", args.json);
        }
        profile.owner_label = trimmed.to_string();
    }
    if let Some(role_label) = &args.role_label {
        let trimmed = role_label.trim();
        if trimmed.is_empty() {
            return print_invalid_profile_request("role_label is empty after trim", args.json);
        }
        profile.role_label = trimmed.to_string();
    }
    if let Some(working_style) = &args.working_style {
        profile.working_style = working_style.clone();
    }
    if let Some(communication_style) = &args.communication_style {
        profile.communication_style = communication_style.clone();
    }
    if let Some(decision_preferences) = &args.decision_preferences {
        profile.decision_preferences.decision_style = decision_preferences.clone();
    }
    if let Some(limitations) = &args.limitations {
        let trimmed = limitations.trim();
        if trimmed.is_empty() {
            return print_invalid_profile_request("limitations is empty after trim", args.json);
        }
        profile.limitations = vec![trimmed.to_string()];
    }

    profile.last_updated_at = utc_now_rfc3339();

    match write_work_proxy_profile(&project_path, &profile) {
        Ok(()) => {
            print_update_success(&profile, &project_path, args.json);
            0
        }
        Err(err) => print_core_profile_error(&err, args.json),
    }
}

fn update_fields_present(args: &ProfileUpdateArgs) -> bool {
    args.owner_label.is_some()
        || args.role_label.is_some()
        || args.working_style.is_some()
        || args.communication_style.is_some()
        || args.decision_preferences.is_some()
        || args.limitations.is_some()
}

pub fn utc_now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

pub fn authority_wire_value(level: ProxyAuthorityLevel) -> &'static str {
    match level {
        ProxyAuthorityLevel::CanAnswer => "can-answer",
        ProxyAuthorityLevel::CanSuggest => "can-suggest",
        ProxyAuthorityLevel::CanDraft => "can-draft",
        ProxyAuthorityLevel::MustAskHuman => "must-ask-human",
        ProxyAuthorityLevel::CannotAnswer => "cannot-answer",
    }
}

fn unsupported_claim_behavior_wire_value(behavior: UnsupportedClaimBehavior) -> &'static str {
    match behavior {
        UnsupportedClaimBehavior::Refuse => "refuse",
        UnsupportedClaimBehavior::AskHuman => "ask-human",
        UnsupportedClaimBehavior::SayUnknown => "say-unknown",
    }
}

fn resolved_safe_default_authority(profile: &WorkProxyProfile) -> ProxyAuthorityLevel {
    resolve_profile_authority(profile, "*", &ProfileEvaluationContext::default()).resolved_authority
}

fn print_init_success(profile: &WorkProxyProfile, project_path: &str, json_mode: bool) {
    let profile_path = work_proxy_profile_path(project_path);
    if json_mode {
        let payload = json!({
            "status": "ok",
            "project": project_path,
            "profilePath": profile_path,
            "profile": profile,
        });
        println!("{payload}");
        return;
    }

    println!(
        "OK  profile_id={}  workspace_id={}  project={}",
        profile.profile_id, profile.workspace_id, project_path
    );
    println!("profile_path={}", profile_path.display());
    println!("owner_label={}", profile.owner_label);
    println!("role_label={}", profile.role_label);
    println!("profile_version={}", profile.profile_version);
    println!(
        "note=local policy profile metadata only; not the human owner and not an answering runtime"
    );
}

fn print_show_success(profile: &WorkProxyProfile, project_path: &str, json_mode: bool) {
    if json_mode {
        println!(
            "{}",
            serde_json::to_string(profile).expect("serialize profile")
        );
        return;
    }

    let profile_path = work_proxy_profile_path(project_path);
    let evidence = &profile.evidence_policy;
    println!("profile_id={}", profile.profile_id);
    println!("workspace_id={}", profile.workspace_id);
    println!("owner_label={}", profile.owner_label);
    println!("role_label={}", profile.role_label);
    println!("profile_version={}", profile.profile_version);
    println!("authority_rules={}", profile.authority_rules.len());
    println!("privacy_rules={}", profile.privacy_rules.len());
    println!(
        "default_refusal_rules={}",
        profile.default_refusal_rules.len()
    );
    println!(
        "evidence_policy=require_evidence_for_claims={} answer_without_evidence={} expose_limitations={} unsupported_claim_behavior={}",
        evidence.require_evidence_for_claims,
        evidence.answer_without_evidence,
        evidence.expose_limitations,
        unsupported_claim_behavior_wire_value(evidence.unsupported_claim_behavior)
    );
    println!("limitations={}", profile.limitations.len());
    println!("last_updated_at={}", profile.last_updated_at);
    println!("profile_path={}", profile_path.display());
    println!(
        "note=local policy profile metadata only; not the human owner and not an answering runtime"
    );
}

fn print_validate_success(profile: &WorkProxyProfile, project_path: &str, json_mode: bool) {
    let resolved = resolved_safe_default_authority(profile);
    if json_mode {
        let payload = json!({
            "status": "ok",
            "valid": true,
            "project": project_path,
            "profileId": profile.profile_id,
            "profileVersion": profile.profile_version,
            "resolvedSafeDefaultAuthority": authority_wire_value(resolved),
            "limitationsCount": profile.limitations.len(),
        });
        println!("{payload}");
        return;
    }

    println!("valid=true");
    println!("profile_id={}", profile.profile_id);
    println!("profile_version={}", profile.profile_version);
    println!(
        "resolved_safe_default_authority={}",
        authority_wire_value(resolved)
    );
    println!("limitations={}", profile.limitations.len());
    println!("project={project_path}");
}

fn print_update_success(profile: &WorkProxyProfile, project_path: &str, json_mode: bool) {
    if json_mode {
        let payload = json!({
            "status": "ok",
            "project": project_path,
            "profile": profile,
        });
        println!("{payload}");
        return;
    }

    println!(
        "OK  profile_id={}  project={}  last_updated_at={}",
        profile.profile_id, project_path, profile.last_updated_at
    );
}

pub fn print_invalid_profile_request(message: &str, json_mode: bool) -> i32 {
    print_profile_error(message, "invalid-profile", 3, json_mode)
}

pub fn print_profile_error(message: &str, category: &str, code: i32, json_mode: bool) -> i32 {
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

pub fn print_core_profile_error(err: &ProfileError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_profile_error(err);
    let message = err.to_string();
    print_profile_error(&message, category, code, json_mode)
}

pub fn exit_code_for_profile_error(err: &ProfileError) -> (i32, &'static str) {
    match err {
        ProfileError::ProjectNotInitialized(_) => (1, "project-resolution"),
        ProfileError::ProfileMissing => (3, "profile-missing"),
        ProfileError::MalformedJson(_)
        | ProfileError::UnsupportedVersion { .. }
        | ProfileError::ValidationFailure(_)
        | ProfileError::WorkspaceMismatch { .. } => (3, "invalid-profile"),
        ProfileError::Io(_) | ProfileError::Json(_) | ProfileError::AtomicReplaceFailed(_) => {
            (4, "write-failed")
        }
    }
}
