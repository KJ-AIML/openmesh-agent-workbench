//! Built-in OpenMesh tool specs. Empty allowlist = Ask (read-only) tools only.

use super::types::ToolSpec;
use serde_json::json;

/// Ask mode — inspect only (no propose / metadata writes).
/// Kept lean so models don't thrash status/mesh tools on every chat turn.
pub fn ask_tool_names() -> &'static [&'static str] {
    &[
        "project_info",
        "list_docs",
        "list_notes",
        "list_dir",
        "read_file",
        "grep",
        "git_diff",
        "git_status",
        "search_context",
        "continuity_summary",
        "pending_questions",
        "ui_navigate",
        "app_get_context",
        "app_propose_action",
        // Ask: no canvas_upsert_auto_ui — writes belong in Plan/Act
    ]
}

/// Plan mode — gather evidence and propose patches / handoff drafts.
pub fn plan_tool_names() -> &'static [&'static str] {
    &[
        "project_info",
        "list_docs",
        "list_notes",
        "list_dir",
        "read_file",
        "grep",
        "git_diff",
        "git_status",
        "search_context",
        "continuity_summary",
        "list_mesh_peers",
        "pilot_status",
        "rc_status",
        "pending_questions",
        "list_recipes",
        "ui_navigate",
        "app_get_context",
        "app_propose_action",
        "canvas_upsert_auto_ui",
        "propose_patch",
        "create_handoff_draft",
    ]
}

/// Act mode — plan tools plus OpenMesh task/session continuity writes.
/// Source-file apply remains human-gated IPC (not a model tool).
pub fn act_tool_names() -> &'static [&'static str] {
    &[
        "project_info",
        "list_docs",
        "list_notes",
        "list_dir",
        "read_file",
        "grep",
        "git_diff",
        "git_status",
        "search_context",
        "continuity_summary",
        "list_mesh_peers",
        "pilot_status",
        "rc_status",
        "pending_questions",
        "list_recipes",
        "ui_navigate",
        "app_get_context",
        "app_propose_action",
        "canvas_upsert_auto_ui",
        "propose_patch",
        "create_handoff_draft",
        "update_task",
        "link_session",
        "mesh_query",
    ]
}

/// Delegate mode — inspect + recipes; launch happens via slash/IPC, not model apply.
pub fn delegate_tool_names() -> &'static [&'static str] {
    ask_tool_names()
}

/// Tools available when `tool_allowlist` is empty (Ask).
pub fn default_tool_names() -> &'static [&'static str] {
    ask_tool_names()
}

/// Resolve a chat mode name to an allowlist.
pub fn tools_for_mode(mode: &str) -> Vec<String> {
    let names = match mode.trim().to_ascii_lowercase().as_str() {
        "plan" => plan_tool_names(),
        "act" => act_tool_names(),
        "delegate" => delegate_tool_names(),
        _ => ask_tool_names(),
    };
    names.iter().map(|s| (*s).to_string()).collect()
}

/// Explicitly human-gated / not model-callable by default.
pub fn human_only_tool_names() -> &'static [&'static str] {
    &["approve_handoff"]
}

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
            description: "List documentation files under <project>/.openmesh/docs.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_notes".into(),
            description: "List notes under <project>/.openmesh/notes.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_dir".into(),
            description: "List entries in a directory under the active workspace root (relative path).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative directory path (empty or \".\" for workspace root)"
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "read_file".into(),
            description: "Read a UTF-8 text file under the active workspace root (relative path). Bounded size.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Relative file path from the workspace root"
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "grep".into(),
            description: "Search file contents under the active workspace (ripgrep when available).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (literal/regex as accepted by rg)"
                    },
                    "glob": {
                        "type": "string",
                        "description": "Optional glob filter, e.g. \"*.rs\" or \"src/**/*.ts\""
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "git_diff".into(),
            description: "Read-only git diff for the workspace (optional path filter).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Optional relative path to limit the diff"
                    },
                    "staged": {
                        "type": "boolean",
                        "description": "If true, show staged (--cached) diff"
                    }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "propose_patch".into(),
            description: "Propose a workspace file change for human approval. Does NOT apply. Provide files[].path and files[].newContent.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string", "description": "Short description of the change" },
                    "files": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "newContent": { "type": "string" }
                            },
                            "required": ["path", "newContent"]
                        }
                    }
                },
                "required": ["files"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "list_recipes".into(),
            description: "List approved verify recipes (cargo/npm checks).".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "ui_navigate".into(),
            description: "Open an in-app page for the user (Work, Chat, Docs, Sprint, Notes, Settings, Continuity, Agent Sessions, Context). Does not click arbitrary UI.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "route": {
                        "type": "string",
                        "description": "App route or alias, e.g. \"/docs\", \"chat\", \"work\", \"sprint\", \"settings\""
                    }
                },
                "required": ["route"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "app_get_context".into(),
            description: "Read lightweight UI context: current project path, allowed navigate routes, and chat mode hints. Use before navigating when unsure where the user is.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "app_propose_action".into(),
            description: "Propose a typed in-app AppAction (createNote, createSprint, runRecipe, openCanvas, canvasAddNode, canvasConnect, etc.). Write/external/destructive actions require user confirmation — never silent.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "object",
                        "description": "Tagged AppAction JSON, e.g. {\"type\":\"createNote\",\"title\":\"Ideas\"} or {\"type\":\"canvasAddNode\",\"label\":\"Machine A\",\"kind\":\"machine\"}"
                    }
                },
                "required": ["action"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "canvas_upsert_auto_ui".into(),
            description: "Create or update an OpenMesh Auto UI canvas artifact (safe JSON UI, schema openmesh.canvas/1). Persists under .openmesh/canvases/auto-ui/ and shows on Canvas → Auto UI. Prefer this over dumping long markdown tables.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "document": {
                        "type": "object",
                        "description": "Auto UI document: { schema:\"openmesh.canvas/1\", id, title, summary?, blocks:[{type, ...}] }. Block types: h1, h2, text, callout, stat, stats, table, pills, todo, code, divider."
                    }
                },
                "required": ["document"],
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
            name: "pending_questions".into(),
            description: "List pending questions that need a person.".into(),
            parameters: json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "create_handoff_draft".into(),
            description: "Create a Continuity handoff draft for a recipient.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "recipient": { "type": "string" },
                    "role": { "type": "string" }
                },
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "approve_handoff".into(),
            description: "Approve a draft handoff note (human-gated; usually via slash/IPC).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "handoffId": { "type": "string" }
                },
                "required": ["handoffId"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "update_task".into(),
            description: "Update a sprint task (status, notes, nextAction).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "taskId": { "type": "string" },
                    "status": { "type": "string" },
                    "title": { "type": "string" },
                    "notes": { "type": "string" },
                    "nextAction": { "type": "string" }
                },
                "required": ["taskId"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "link_session".into(),
            description: "Link an OpenMesh chat session to a foreign agent session id.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "chatSessionId": { "type": "string" },
                    "foreignTool": { "type": "string" },
                    "foreignSessionId": { "type": "string" },
                    "foreignSessionPath": { "type": "string" }
                },
                "required": ["chatSessionId", "foreignSessionId"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "mesh_query".into(),
            description: "Read-only mesh peer query (trust-policy gated, fail-closed).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "peer": { "type": "string" },
                    "question": { "type": "string" }
                },
                "required": ["peer", "question"],
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

/// Empty allowlist means default tools only (not every built-in).
pub fn filter_tools(allowlist: &[String]) -> Vec<ToolSpec> {
    let all = builtin_tool_specs();
    if allowlist.is_empty() {
        let defaults = default_tool_names();
        return all
            .into_iter()
            .filter(|t| defaults.iter().any(|n| *n == t.name))
            .collect();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_allowlist_is_ask_read_only() {
        let tools = filter_tools(&[]);
        assert!(tools.iter().any(|t| t.name == "read_file"));
        assert!(tools.iter().any(|t| t.name == "ui_navigate"));
        assert!(!tools.iter().any(|t| t.name == "propose_patch"));
        assert!(!tools.iter().any(|t| t.name == "approve_handoff"));
        assert!(!tools.iter().any(|t| t.name == "update_task"));
        assert!(
            !tools.iter().any(|t| t.name == "canvas_upsert_auto_ui"),
            "Ask must not persist Auto UI writes"
        );
    }

    #[test]
    fn act_mode_includes_propose_not_approve() {
        let tools = filter_tools(&tools_for_mode("act"));
        assert!(tools.iter().any(|t| t.name == "propose_patch"));
        assert!(tools.iter().any(|t| t.name == "update_task"));
        assert!(!tools.iter().any(|t| t.name == "approve_handoff"));
    }
}
