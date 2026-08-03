/** Canonical agent CLI / session tool ids used across the UI. */
export type AgentToolId =
	| "codex"
	| "claude"
	| "claude-code"
	| "opencode"
	| "cursor"
	| "gemini"
	| "gemini-cli"
	| "grok";

export type AgentToolKey =
	| "codex"
	| "claude"
	| "opencode"
	| "cursor"
	| "gemini"
	| "grok";

/** Normalize scanner / settings aliases to a single icon key. */
export function normalizeAgentTool(tool: string): AgentToolKey | null {
	const t = tool.trim().toLowerCase();
	if (t === "codex" || t === "openai" || t === "openai-codex") return "codex";
	if (t === "claude" || t === "claude-code" || t === "anthropic") return "claude";
	if (t === "opencode" || t === "open-code") return "opencode";
	if (t === "cursor") return "cursor";
	if (t === "gemini" || t === "gemini-cli" || t === "google-gemini") return "gemini";
	if (t === "grok" || t === "xai" || t === "x-ai") return "grok";
	return null;
}

export function agentToolLabel(tool: string): string {
	const key = normalizeAgentTool(tool);
	switch (key) {
		case "codex":
			return "Codex";
		case "claude":
			return "Claude";
		case "opencode":
			return "OpenCode";
		case "cursor":
			return "Cursor";
		case "gemini":
			return "Gemini";
		case "grok":
			return "Grok";
		default:
			return tool;
	}
}

/** Brand accent used when `colored` is true (falls back to currentColor). */
export const AGENT_TOOL_BRAND_COLOR: Record<AgentToolKey, string> = {
	codex: "currentColor",
	claude: "#D97757",
	opencode: "#38BDF8",
	cursor: "currentColor",
	gemini: "#8E75B2",
	grok: "currentColor",
};

export const AGENT_TOOL_FILTERS: Array<"all" | AgentToolKey> = [
	"all",
	"codex",
	"claude",
	"opencode",
	"cursor",
	"gemini",
	"grok",
];
