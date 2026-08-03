//! Format-aware readers for foreign AI-agent CLI session stores.
//!
//! These layouts are reverse-engineered from local provider storage (and match
//! the discovery rules used by Grok's bundled `session_reader.py`):
//!
//! | Tool       | Default root                         | Layout |
//! |------------|--------------------------------------|--------|
//! | Codex      | `~/.codex/sessions` (+ `state_*.sqlite`) | `YYYY/MM/DD/rollout-*.jsonl` |
//! | Claude     | `~/.claude/projects`                 | `<slugified-cwd>/*.jsonl` |
//! | Cursor     | `~/.cursor/projects`                 | `*/agent-transcripts/<id>/<id>.jsonl` |
//! | OpenCode   | `~/.local/share/opencode`            | `opencode.db` and/or `storage/session/**` |
//! | Gemini     | `~/.gemini/tmp`                      | `<project_hash>/chats/*.json` |
//! | Grok       | `~/.grok/sessions`                   | `<url-encoded-cwd>/<id>/summary.json` |
//!
//! Discovery is recursive and content-aware (title / cwd / preview). Transcript
//! bodies are treated as untrusted inert history — never executed.

mod discovery;
mod parse;
mod redact;

pub use discovery::{
    candidate_session_dirs, default_session_dir, detect_provider_roots, normalize_tool,
    normalize_workspace_path, scan_agent_sessions, scan_workspace_sessions,
    session_matches_workspace, DetectedProviderRoot, ScannedForeignSession, SessionScanOverrides,
};
pub use redact::redact_secrets;
