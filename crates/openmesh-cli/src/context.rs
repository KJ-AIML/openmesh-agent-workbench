// ============================================================================
// Context Pack commands — Dev Track 0.1.5 Checkpoint E
// ============================================================================

use chrono::Utc;
use clap::{Args, Subcommand};
use openmesh_core::context_pack::{
    build_proxy_context_pack, ContextPackBuildError, ProxyContextPackBuildOptions,
};
use openmesh_core::context_pack_storage::{
    proxy_context_pack_path, read_proxy_context_pack, read_proxy_context_pack_file,
    write_proxy_context_pack, ContextPackStorageError,
};
use openmesh_core::domain::{CatchUpWindow, ProxyContextPack, SourceCounts};
use openmesh_core::storage::read_project;
use openmesh_core::storage::Project;
use serde_json::json;
use std::path::{Path, PathBuf};

use crate::output;
use crate::project::resolve_project;

#[derive(Subcommand, Debug)]
pub enum ContextCommand {
    /// Build a Proxy Context Pack for a fixed catch-up window (ephemeral by default).
    Build(ContextBuildArgs),
    /// Show the persisted Proxy Context Pack (read-only).
    Show(ContextShowArgs),
    /// Validate a persisted or explicit Proxy Context Pack file (read-only).
    Validate(ContextValidateArgs),
}

#[derive(Args, Debug, Clone)]
pub struct ContextBuildArgs {
    #[arg(long)]
    pub since: Option<String>,

    #[arg(long)]
    pub until: Option<String>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub write: bool,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ContextShowArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug, Clone)]
pub struct ContextValidateArgs {
    #[arg(long)]
    pub project: Option<String>,

    #[arg(long)]
    pub file: Option<String>,

    #[arg(long)]
    pub json: bool,
}

pub fn run_context(cmd: ContextCommand, cwd: &Path) -> i32 {
    match cmd {
        ContextCommand::Build(args) => run_context_build(&args, cwd),
        ContextCommand::Show(args) => run_context_show(&args, cwd),
        ContextCommand::Validate(args) => run_context_validate(&args, cwd),
    }
}

pub fn run_context_build(args: &ContextBuildArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let window = match build_fixed_window(args.since.as_deref(), args.until.as_deref()) {
        Ok(window) => window,
        Err(message) => {
            return print_invalid_context_request(&message, "invalid-window", args.json)
        }
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    let options = ProxyContextPackBuildOptions {
        generated_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        selection: Default::default(),
    };

    let pack = match build_proxy_context_pack(&project_path, window, options) {
        Ok(pack) => pack,
        Err(err) => return print_context_build_error(&err, args.json),
    };

    if args.write {
        if let Err(err) = write_proxy_context_pack(&project_path, &pack) {
            return print_context_storage_error(&err, args.json);
        }
    }

    print_context_pack_success(&pack, &project_path, args.write, args.json);
    0
}

pub fn run_context_show(args: &ContextShowArgs, cwd: &Path) -> i32 {
    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_proxy_context_pack(&project_path) {
        Ok(pack) => {
            print_context_pack_success(&pack, &project_path, true, args.json);
            0
        }
        Err(err) => print_context_storage_error(&err, args.json),
    }
}

pub fn run_context_validate(args: &ContextValidateArgs, cwd: &Path) -> i32 {
    if let Some(file) = args.file.as_deref() {
        let path = PathBuf::from(file);
        let pack = match read_proxy_context_pack_file(&path) {
            Ok(pack) => pack,
            Err(err) => return print_context_storage_error(&err, args.json),
        };

        if let Some(project_arg) = args.project.as_deref() {
            let resolved = match resolve_project(Some(project_arg), cwd) {
                Ok(resolved) => resolved,
                Err(err) => {
                    return output::print_project_resolution_error(&err.describe(), args.json)
                }
            };
            let project_path = resolved.path.to_string_lossy().to_string();
            let project = match read_project::<Project>(&project_path, "project.json") {
                Some(project) => project,
                None => {
                    return print_context_storage_error(
                        &ContextPackStorageError::ProjectNotInitialized,
                        args.json,
                    )
                }
            };
            if pack.workspace_id != project.id {
                return print_context_storage_error(
                    &ContextPackStorageError::WorkspaceMismatch,
                    args.json,
                );
            }
            print_validate_success(&pack, Some(&project_path), args.json);
        } else {
            print_validate_success(&pack, None, args.json);
        }
        return 0;
    }

    let resolved = match resolve_project(args.project.as_deref(), cwd) {
        Ok(resolved) => resolved,
        Err(err) => return output::print_project_resolution_error(&err.describe(), args.json),
    };

    let project_path = resolved.path.to_string_lossy().to_string();
    match read_proxy_context_pack(&project_path) {
        Ok(pack) => {
            print_validate_success(&pack, Some(&project_path), args.json);
            0
        }
        Err(err) => print_context_storage_error(&err, args.json),
    }
}

pub fn build_fixed_window(
    since: Option<&str>,
    until: Option<&str>,
) -> Result<CatchUpWindow, String> {
    let since =
        since.ok_or_else(|| "missing required --since (RFC 3339 UTC timestamp)".to_string())?;
    let until =
        until.ok_or_else(|| "missing required --until (RFC 3339 UTC timestamp)".to_string())?;
    validate_window_timestamp(since, "--since")?;
    validate_window_timestamp(until, "--until")?;

    let since_dt = chrono::DateTime::parse_from_rfc3339(since)
        .map_err(|_| "invalid --since value (expected RFC 3339 UTC timestamp)".to_string())?;
    let until_dt = chrono::DateTime::parse_from_rfc3339(until)
        .map_err(|_| "invalid --until value (expected RFC 3339 UTC timestamp)".to_string())?;
    if since_dt > until_dt {
        return Err("invalid fixed window: since must be <= until".into());
    }

    Ok(CatchUpWindow {
        since: since.to_string(),
        until: until.to_string(),
    })
}

fn validate_window_timestamp(raw: &str, flag: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        return Err(format!("missing required {flag} (RFC 3339 UTC timestamp)"));
    }
    let parsed = chrono::DateTime::parse_from_rfc3339(raw)
        .map_err(|_| format!("invalid {flag} value (expected RFC 3339 UTC timestamp)"))?;
    if parsed.offset().local_minus_utc() != 0 {
        return Err(format!(
            "invalid {flag} value (timestamp must use UTC `Z` offset)"
        ));
    }
    Ok(())
}

fn print_context_pack_success(
    pack: &ProxyContextPack,
    project_path: &str,
    persisted: bool,
    json_mode: bool,
) {
    if json_mode {
        let mut payload = json!({
            "status": "ok",
            "project": project_path,
            "pack": pack,
        });
        if persisted {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert(
                    "packPath".into(),
                    json!(proxy_context_pack_path(project_path)),
                );
            }
        }
        println!(
            "{}",
            serde_json::to_string(&payload).expect("serialize json")
        );
        return;
    }

    print_human_summary(pack, project_path, persisted);
}

fn print_validate_success(pack: &ProxyContextPack, project_path: Option<&str>, json_mode: bool) {
    let redaction = &pack.redaction_summary;
    if json_mode {
        let mut payload = json!({
            "status": "ok",
            "valid": true,
            "contextPackId": pack.context_pack_id,
            "protocolVersion": pack.protocol_version,
            "workspaceId": pack.workspace_id,
            "profileId": pack.profile_id,
            "requestedWindow": pack.requested_window,
            "evidenceCount": pack.evidence_index.len(),
            "redactionSummary": redaction,
            "limitationsCount": pack.limitations.len(),
            "buildInputsHash": pack.build_inputs_hash,
        });
        if let Some(project) = project_path {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("project".into(), json!(project));
            }
        }
        println!(
            "{}",
            serde_json::to_string(&payload).expect("serialize json")
        );
        return;
    }

    println!("valid=true");
    println!("context_pack_id={}", pack.context_pack_id);
    println!("protocol_version={}", pack.protocol_version);
    println!("workspace_id={}", pack.workspace_id);
    println!("profile_id={}", pack.profile_id);
    println!(
        "requested_window_since={} requested_window_until={}",
        pack.requested_window.since, pack.requested_window.until
    );
    println!("evidence_count={}", pack.evidence_index.len());
    println!(
        "redaction_secret_items_omitted={} redaction_policy_restricted_items_omitted={} redaction_malformed_items_omitted={} redaction_quarantined_items_omitted={} redaction_bounds_truncated_items={}",
        redaction.secret_items_omitted,
        redaction.policy_restricted_items_omitted,
        redaction.malformed_items_omitted,
        redaction.quarantined_items_omitted,
        redaction.bounds_truncated_items
    );
    println!("limitations_count={}", pack.limitations.len());
    println!("build_inputs_hash={}", pack.build_inputs_hash);
    if let Some(project) = project_path {
        println!("project={project}");
    }
}

fn print_human_summary(pack: &ProxyContextPack, project_path: &str, persisted: bool) {
    let redaction = &pack.redaction_summary;
    println!("context_pack_id={}", pack.context_pack_id);
    println!("workspace_id={}", pack.workspace_id);
    println!("profile_id={}", pack.profile_id);
    println!("profile_version={}", pack.profile_version);
    println!("protocol_version={}", pack.protocol_version);
    println!(
        "requested_window_since={} requested_window_until={}",
        pack.requested_window.since, pack.requested_window.until
    );
    println!("evidence_index_count={}", pack.evidence_index.len());
    println!(
        "redaction_secret_items_omitted={} redaction_policy_restricted_items_omitted={} redaction_malformed_items_omitted={} redaction_quarantined_items_omitted={} redaction_bounds_truncated_items={}",
        redaction.secret_items_omitted,
        redaction.policy_restricted_items_omitted,
        redaction.malformed_items_omitted,
        redaction.quarantined_items_omitted,
        redaction.bounds_truncated_items
    );
    println!("diagnostics_count={}", pack.diagnostics.len());
    println!("limitations_count={}", pack.limitations.len());
    println!("unresolved_count={}", pack.unresolved_items.len());
    print_source_counts(&pack.source_counts);
    println!("build_inputs_hash={}", pack.build_inputs_hash);
    if persisted {
        println!(
            "pack_path={}",
            proxy_context_pack_path(project_path).display()
        );
    }
    println!("note=local context package only; not an answer and does not execute proxy authority");
}

fn print_source_counts(counts: &SourceCounts) {
    let value = serde_json::to_value(counts).expect("source counts");
    println!(
        "source_counts_work_events={} source_counts_processed_signals={} source_counts_pending_signals={} source_counts_audit_records={}",
        value["workEvents"].as_u64().unwrap_or(0),
        value["processedSignals"].as_u64().unwrap_or(0),
        value["pendingSignals"].as_u64().unwrap_or(0),
        value["promotionAuditRecords"].as_u64().unwrap_or(0),
    );
}

pub fn print_invalid_context_request(message: &str, category: &str, json_mode: bool) -> i32 {
    print_context_error(message, category, 3, json_mode)
}

pub fn print_context_error(message: &str, category: &str, code: i32, json_mode: bool) -> i32 {
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

pub fn print_context_build_error(err: &ContextPackBuildError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_context_build_error(err);
    let message = safe_build_error_message(err);
    print_context_error(&message, category, code, json_mode)
}

pub fn print_context_storage_error(err: &ContextPackStorageError, json_mode: bool) -> i32 {
    let (code, category) = exit_code_for_context_storage_error(err);
    let message = safe_storage_error_message(err);
    print_context_error(&message, category, code, json_mode)
}

fn safe_build_error_message(err: &ContextPackBuildError) -> String {
    match err {
        ContextPackBuildError::ProjectNotInitialized(_) => "project not initialized".into(),
        ContextPackBuildError::ProfileMissing => "work proxy profile is missing".into(),
        ContextPackBuildError::InvalidWindow(_) => "invalid catch-up window".into(),
        ContextPackBuildError::PackValidation(category) => {
            format!("context pack validation failed ({category})")
        }
        ContextPackBuildError::Profile(_) => "profile error".into(),
        ContextPackBuildError::ContinuitySnapshot(_) => "continuity snapshot error".into(),
        ContextPackBuildError::CurrentStateBuild(_) => "current state build failed".into(),
        ContextPackBuildError::CatchUpBuild(_) => "catch-up build failed".into(),
        ContextPackBuildError::Selection(_) => "context selection failed".into(),
        ContextPackBuildError::Serialization(_) => "serialization failed".into(),
    }
}

fn safe_storage_error_message(err: &ContextPackStorageError) -> String {
    match err {
        ContextPackStorageError::ValidationFailed { category } => {
            format!("context pack validation failed ({category})")
        }
        other => other.to_string(),
    }
}

pub fn exit_code_for_context_build_error(err: &ContextPackBuildError) -> (i32, &'static str) {
    match err {
        ContextPackBuildError::ProjectNotInitialized(_) => (1, "project-resolution"),
        ContextPackBuildError::ProfileMissing => (3, "profile-missing"),
        ContextPackBuildError::InvalidWindow(_) => (3, "invalid-window"),
        ContextPackBuildError::PackValidation(_) => (3, "invalid-context-pack"),
        ContextPackBuildError::Profile(_)
        | ContextPackBuildError::ContinuitySnapshot(_)
        | ContextPackBuildError::CurrentStateBuild(_)
        | ContextPackBuildError::CatchUpBuild(_)
        | ContextPackBuildError::Selection(_) => (3, "context-build-failed"),
        ContextPackBuildError::Serialization(_) => (4, "write-failed"),
    }
}

pub fn exit_code_for_context_storage_error(err: &ContextPackStorageError) -> (i32, &'static str) {
    match err {
        ContextPackStorageError::ProjectNotInitialized => (1, "project-not-initialized"),
        ContextPackStorageError::PackNotFound => (3, "pack-not-found"),
        ContextPackStorageError::MalformedJson
        | ContextPackStorageError::UnsupportedProtocolVersion
        | ContextPackStorageError::ValidationFailed { .. }
        | ContextPackStorageError::WorkspaceMismatch => (3, "invalid-context-pack"),
        ContextPackStorageError::ReadFailed
        | ContextPackStorageError::WriteFailed
        | ContextPackStorageError::AtomicReplaceFailed => (4, "write-failed"),
    }
}
