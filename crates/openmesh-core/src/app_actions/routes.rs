//! Shared UI route allowlist — single source of truth for navigate actions.

use serde_json::json;

/// Canonical Vue routes the agent / voice layer may navigate to.
pub const ALLOWED_UI_ROUTES: &[&str] = &[
    "/",
    "/agent-chat",
    "/docs",
    "/sprint",
    "/notes",
    "/settings",
    "/continuity",
    "/agent-sessions",
    "/context",
    "/canvas",
];

/// Normalize a free-form destination into `(route, label)`, or `None`.
pub fn normalize_ui_route(raw: &str) -> Option<(&'static str, &'static str)> {
    let key = raw
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
        .replace('_', "-")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    match key.as_str() {
        "" | "work" | "home" => Some(("/", "Work")),
        "agent-chat" | "chat" | "agent" => Some(("/agent-chat", "Chat")),
        "docs" | "documentation" => Some(("/docs", "Docs")),
        "sprint" | "sprints" => Some(("/sprint", "Sprint")),
        "notes" | "note" => Some(("/notes", "Notes")),
        "settings" => Some(("/settings", "Settings")),
        "continuity" => Some(("/continuity", "Continuity")),
        "agent-sessions" | "sessions" | "session" => Some(("/agent-sessions", "Agent Sessions")),
        "context" => Some(("/context", "Context")),
        "canvas" | "board" | "graph" => Some(("/canvas", "Canvas")),
        _ => None,
    }
}

pub fn label_for_route(route: &str) -> &'static str {
    normalize_ui_route(route)
        .map(|(_, label)| label)
        .unwrap_or("Page")
}

/// JSON payload for tool `summary` — legacy fields + typed AppAction.
pub fn ui_navigate_json(raw: &str) -> Result<String, String> {
    let (route, label) = normalize_ui_route(raw)
        .ok_or_else(|| format!("unsupported ui route: {raw}"))?;
    Ok(serde_json::to_string_pretty(&json!({
        "ok": true,
        "action": "ui_navigate",
        "route": route,
        "label": label,
        "appAction": {
            "type": "navigate",
            "route": route,
        }
    }))
    .unwrap_or_else(|_| "{}".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_aliases() {
        assert_eq!(normalize_ui_route("notes"), Some(("/notes", "Notes")));
        assert_eq!(
            normalize_ui_route("Agent Chat"),
            Some(("/agent-chat", "Chat"))
        );
        assert_eq!(
            normalize_ui_route("/settings"),
            Some(("/settings", "Settings"))
        );
    }

    #[test]
    fn rejects_unknown() {
        assert!(normalize_ui_route("javascript:alert(1)").is_none());
        assert!(normalize_ui_route("/evil").is_none());
    }
}
