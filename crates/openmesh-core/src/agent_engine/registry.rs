//! Built-in OpenMesh tool specs (read-mostly allowlist).

use super::types::ToolSpec;
use serde_json::json;

pub fn builtin_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "project_info".into(),
            description: "Return active workspace path and project metadata.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_docs".into(),
            description: "List documentation files under the project docs folder.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_notes".into(),
            description: "List notes in the project.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "continuity_summary".into(),
            description: "Continuity hub summary (pending, peers, envelopes, proxy).".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_mesh_peers".into(),
            description: "List registered mesh peers.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "pilot_status".into(),
            description: "Enterprise pilot readiness pack status.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "rc_status".into(),
            description: "1.0 RC program status.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "git_status".into(),
            description: "Read-only git status for the project workspace.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "search_context".into(),
            description: "Search project context index.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"],
                "additionalProperties": false
            }),
        },
    ]
}

pub fn filter_tools(allowlist: &[String]) -> Vec<ToolSpec> {
    let all = builtin_tool_specs();
    if allowlist.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|t| allowlist.iter().any(|a| a == &t.name))
        .collect()
}

/// Host-side tool execution.
pub trait ToolExecutor: Send + Sync {
    fn execute(&self, tool_name: &str, arguments_json: &str) -> Result<String, String>;
}

/// Deterministic stub executor for unit tests.
pub struct StubToolExecutor {
    pub responses: std::collections::BTreeMap<String, String>,
}

impl ToolExecutor for StubToolExecutor {
    fn execute(&self, tool_name: &str, _arguments_json: &str) -> Result<String, String> {
        self.responses
            .get(tool_name)
            .cloned()
            .ok_or_else(|| format!("unknown tool: {tool_name}"))
    }
}
