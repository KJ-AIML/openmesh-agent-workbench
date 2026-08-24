//! Local SQLite-based usage tracking.
//!
//! Database lives at `~/.openmesh/usage.sqlite`. Two tables: `agent_turns`
//! (recorded by the Agent Engine after each turn) and `proxy_requests`
//! (reserved for durable built-in proxy request history).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

fn usage_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory.".to_string())?;
    Ok(home.join(".openmesh").join("usage.sqlite"))
}

static DB: std::sync::OnceLock<Mutex<Connection>> = std::sync::OnceLock::new();

fn with_connection<R>(f: impl FnOnce(&Connection) -> Result<R, String>) -> Result<R, String> {
    let guard = DB
        .get()
        .ok_or_else(|| "Usage database is not initialized.".to_string())?
        .lock()
        .map_err(|_| "Usage database lock is poisoned.".to_string())?;
    f(&guard)
}

/// Create the usage database and tables if they do not exist. Safe to call
/// multiple times — subsequent calls are no-ops.
pub fn init_usage_db() -> Result<(), String> {
    if DB.get().is_some() {
        return Ok(());
    }
    let path = usage_db_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create usage database directory: {e}"))?;
    }
    let conn =
        Connection::open(&path).map_err(|e| format!("Could not open usage database: {e}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS agent_turns (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            project_path TEXT,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            input_tokens INTEGER DEFAULT 0,
            output_tokens INTEGER DEFAULT 0,
            total_tokens INTEGER DEFAULT 0,
            outcome TEXT NOT NULL,
            error_message TEXT
        );
        CREATE TABLE IF NOT EXISTS proxy_requests (
            id TEXT PRIMARY KEY,
            timestamp TEXT NOT NULL,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            input_tokens INTEGER DEFAULT 0,
            output_tokens INTEGER DEFAULT 0,
            latency_ms INTEGER DEFAULT 0,
            status_code INTEGER,
            account_label TEXT
        );",
    )
    .map_err(|e| format!("Could not initialize usage database tables: {e}"))?;
    let _ = DB.set(Mutex::new(conn));
    Ok(())
}

// ---------------------------------------------------------------------------
// Record types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnRecord {
    pub id: String,
    pub timestamp: String,
    pub project_path: Option<String>,
    pub provider: String,
    pub model: String,
    pub duration_ms: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub outcome: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct ProxyRequestRecord {
    pub id: String,
    pub timestamp: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub status_code: Option<u16>,
    pub account_label: Option<String>,
}

pub fn record_agent_turn(record: AgentTurnRecord) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO agent_turns (
                id, timestamp, project_path, provider, model,
                duration_ms, input_tokens, output_tokens, total_tokens,
                outcome, error_message
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                record.id,
                record.timestamp,
                record.project_path,
                record.provider,
                record.model,
                record.duration_ms as i64,
                record.input_tokens as i64,
                record.output_tokens as i64,
                record.total_tokens as i64,
                record.outcome,
                record.error_message,
            ],
        )
        .map_err(|e| format!("Could not record agent turn: {e}"))?;
        Ok(())
    })
}

#[allow(dead_code)]
pub fn record_proxy_request(record: ProxyRequestRecord) -> Result<(), String> {
    with_connection(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO proxy_requests (
                id, timestamp, provider, model,
                input_tokens, output_tokens, latency_ms,
                status_code, account_label
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.id,
                record.timestamp,
                record.provider,
                record.model,
                record.input_tokens as i64,
                record.output_tokens as i64,
                record.latency_ms as i64,
                record.status_code.map(|s| s as i64),
                record.account_label,
            ],
        )
        .map_err(|e| format!("Could not record proxy request: {e}"))?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Query types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageSummary {
    pub total_turns: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
    pub provider_breakdown: Vec<ProviderUsage>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider: String,
    pub turns: u64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageBucket {
    pub timestamp: String,
    pub turns: u64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogEntry {
    pub id: String,
    pub timestamp: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub latency_ms: u64,
    pub status_code: Option<u16>,
    pub account_label: Option<String>,
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

pub fn query_usage_summary(start: &str, end: &str) -> Result<UsageSummary, String> {
    with_connection(|conn| {
        let mut summary = UsageSummary::default();

        let mut stmt = conn
            .prepare(
                "SELECT COALESCE(SUM(duration_ms), 0),
                        COALESCE(SUM(input_tokens), 0),
                        COALESCE(SUM(output_tokens), 0),
                        COALESCE(SUM(total_tokens), 0),
                        COUNT(*)
                 FROM agent_turns
                 WHERE timestamp >= ?1 AND timestamp <= ?2",
            )
            .map_err(|e| format!("Could not prepare usage summary query: {e}"))?;
        let row = stmt
            .query_row(params![start, end], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    row.get::<_, i64>(1)? as u64,
                    row.get::<_, i64>(2)? as u64,
                    row.get::<_, i64>(3)? as u64,
                    row.get::<_, i64>(4)? as u64,
                ))
            })
            .map_err(|e| format!("Could not read usage summary: {e}"))?;
        summary.total_duration_ms = row.0;
        summary.total_input_tokens = row.1;
        summary.total_output_tokens = row.2;
        summary.total_tokens = row.3;
        summary.total_turns = row.4;

        let mut provider_stmt = conn
            .prepare(
                "SELECT provider,
                        COUNT(*),
                        COALESCE(SUM(total_tokens), 0),
                        COALESCE(SUM(duration_ms), 0)
                 FROM agent_turns
                 WHERE timestamp >= ?1 AND timestamp <= ?2
                 GROUP BY provider
                 ORDER BY COUNT(*) DESC",
            )
            .map_err(|e| format!("Could not prepare provider breakdown query: {e}"))?;
        let rows = provider_stmt
            .query_map(params![start, end], |row| {
                Ok(ProviderUsage {
                    provider: row.get(0)?,
                    turns: row.get::<_, i64>(1)? as u64,
                    total_tokens: row.get::<_, i64>(2)? as u64,
                    total_duration_ms: row.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| format!("Could not read provider breakdown: {e}"))?;
        for row in rows {
            match row {
                Ok(usage) => summary.provider_breakdown.push(usage),
                Err(_) => break,
            }
        }

        Ok(summary)
    })
}

pub fn query_usage_timeseries(
    start: &str,
    end: &str,
    interval: &str,
) -> Result<Vec<UsageBucket>, String> {
    let time_format = match interval {
        "hour" | "hourly" => "%Y-%m-%dT%H:00:00Z",
        "week" | "weekly" => "%Y-%W",
        "month" | "monthly" => "%Y-%m",
        _ => "%Y-%m-%dT00:00:00Z", // day
    };

    with_connection(|conn| {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT strftime('{time_format}', timestamp) AS bucket,
                        COUNT(*),
                        COALESCE(SUM(total_tokens), 0),
                        COALESCE(SUM(duration_ms), 0)
                 FROM agent_turns
                 WHERE timestamp >= ?1 AND timestamp <= ?2
                 GROUP BY bucket
                 ORDER BY bucket ASC"
            ))
            .map_err(|e| format!("Could not prepare timeseries query: {e}"))?;
        let rows = stmt
            .query_map(params![start, end], |row| {
                Ok(UsageBucket {
                    timestamp: row.get(0)?,
                    turns: row.get::<_, i64>(1)? as u64,
                    total_tokens: row.get::<_, i64>(2)? as u64,
                    total_duration_ms: row.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| format!("Could not read timeseries: {e}"))?;
        let mut buckets = Vec::new();
        for row in rows {
            match row {
                Ok(bucket) => buckets.push(bucket),
                Err(_) => break,
            }
        }
        Ok(buckets)
    })
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn usage_summary(start: String, end: String) -> Result<UsageSummary, String> {
    tauri::async_runtime::spawn_blocking(move || query_usage_summary(&start, &end))
        .await
        .map_err(|e| format!("usage summary failed to join: {e}"))?
}

#[tauri::command]
pub async fn usage_timeseries(
    start: String,
    end: String,
    interval: String,
) -> Result<Vec<UsageBucket>, String> {
    tauri::async_runtime::spawn_blocking(move || query_usage_timeseries(&start, &end, &interval))
        .await
        .map_err(|e| format!("usage timeseries failed to join: {e}"))?
}

#[tauri::command]
pub async fn usage_request_logs(
    limit: u32,
    offset: u32,
    provider_filter: Option<String>,
) -> Result<Vec<RequestLogEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        query_request_logs(limit, offset, provider_filter.as_deref())
    })
    .await
    .map_err(|e| format!("usage request logs failed to join: {e}"))?
}

pub fn query_request_logs(
    limit: u32,
    offset: u32,
    provider_filter: Option<&str>,
) -> Result<Vec<RequestLogEntry>, String> {
    with_connection(|conn| {
        let mut entries = Vec::new();

        if let Some(provider) = provider_filter {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, provider, model,
                            input_tokens, output_tokens, latency_ms,
                            status_code, account_label
                     FROM agent_turns
                     WHERE provider = ?1
                     ORDER BY timestamp DESC
                     LIMIT ?2 OFFSET ?3",
                )
                .map_err(|e| format!("Could not prepare request logs query: {e}"))?;
            let rows = stmt
                .query_map(params![provider, limit as i64, offset as i64], |row| {
                    Ok(RequestLogEntry {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        provider: row.get(2)?,
                        model: row.get(3)?,
                        input_tokens: row.get::<_, i64>(4)? as u64,
                        output_tokens: row.get::<_, i64>(5)? as u64,
                        latency_ms: row.get::<_, i64>(6)? as u64,
                        status_code: None,
                        account_label: None,
                    })
                })
                .map_err(|e| format!("Could not read request logs: {e}"))?;
            for row in rows {
                match row {
                    Ok(entry) => entries.push(entry),
                    Err(_) => break,
                }
            }
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, timestamp, provider, model,
                            input_tokens, output_tokens, latency_ms,
                            status_code, account_label
                     FROM agent_turns
                     ORDER BY timestamp DESC
                     LIMIT ?1 OFFSET ?2",
                )
                .map_err(|e| format!("Could not prepare request logs query: {e}"))?;
            let rows = stmt
                .query_map(params![limit as i64, offset as i64], |row| {
                    Ok(RequestLogEntry {
                        id: row.get(0)?,
                        timestamp: row.get(1)?,
                        provider: row.get(2)?,
                        model: row.get(3)?,
                        input_tokens: row.get::<_, i64>(4)? as u64,
                        output_tokens: row.get::<_, i64>(5)? as u64,
                        latency_ms: row.get::<_, i64>(6)? as u64,
                        status_code: None,
                        account_label: None,
                    })
                })
                .map_err(|e| format!("Could not read request logs: {e}"))?;
            for row in rows {
                match row {
                    Ok(entry) => entries.push(entry),
                    Err(_) => break,
                }
            }
        }

        Ok(entries)
    })
}
