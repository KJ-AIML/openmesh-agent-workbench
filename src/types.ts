// Openmesh core types

export type Project = {
	id: string;
	name: string;
	folderPath: string;
	repoUrl?: string;
	defaultBranch: string;
	sprintSource: "none" | "mock" | "azure-devops";
	docsFolder?: string;
	terminalDir?: string;
	defaultAgentCli?: "codex" | "claude-code" | "opencode" | null;
	notes?: string;
	status: "active" | "archived";
	createdAt: string;
	updatedAt: string;
};

export type DocSource = {
	id: string;
	projectId: string;
	title: string;
	description: string;
	category:
		| "specs"
		| "engineering"
		| "api"
		| "sprint"
		| "research"
		| "architecture"
		| "agent-instructions";
	connectedPath?: string;
	isConnected: boolean;
	fileCount?: number;
	agentContextEnabled: boolean;
	lastIndexedAt?: string;
	createdAt: string;
	updatedAt: string;
};

export type Sprint = {
	id: string;
	projectId: string;
	name: string;
	status: "planned" | "active" | "completed" | "archived";
	/** local = OpenMesh-owned board; mock kept for legacy files; azure-devops reserved */
	source: "local" | "mock" | "azure-devops";
	createdAt: string;
	updatedAt: string;
};

export type Task = {
	id: string;
	sprintId: string;
	projectId: string;
	title: string;
	description?: string;
	status: "pending" | "in-progress" | "blocked" | "completed";
	priority: "P0" | "P1" | "P2" | "P3";
	owner?: string;
	nextAction?: string;
	notes?: string;
	linkedDocIds: string[];
	linkedSessionIds: string[];
	createdAt: string;
	updatedAt: string;
};

export type RecentItem = {
	id: string;
	type:
		| "project"
		| "folder"
		| "doc"
		| "task"
		| "session"
		| "note"
		| "artifact"
		| "terminal"
		| "agent_session"
		| "command_preset";
	title: string;
	projectId?: string;
	sourceId?: string;
	sourcePath?: string;
	lastOpenedAt: string;
	pinned: boolean;
};

export type AgentSession = {
	id: string;
	tool: "codex" | "claude-code" | "opencode" | "cursor" | "gemini-cli" | "axga";
	title: string;
	projectId?: string;
	sourcePath?: string;
	status: "active" | "completed" | "archived";
	summary?: string;
	startedAt: string;
	lastActiveAt: string;
	endedAt?: string;
	changedFiles?: string[];
	linkedTaskId?: string;
	isImportant: boolean;
	createdAt: string;
	updatedAt: string;
};

export type Note = {
	id: string;
	projectId?: string;
	title: string;
	content: string;
	tags?: string[];
	createdAt: string;
	updatedAt: string;
};

export type ScannedSession = {
	id: string;
	toolName: string;
	title: string;
	sessionPath: string;
	fileName: string;
	createdAt: string;
	lastActiveAt: string;
	fileSizeBytes: number;
	summaryPreview?: string;
	projectHint?: string;
	isReal: true;
};

export type TerminalPreset = {
	id: string;
	projectId: string;
	name: string;
	command: string;
	description?: string;
	lastUsedAt?: string;
	createdAt: string;
};

export type CommandPreset = {
	id: string;
	projectId: string;
	name: string;
	command: string;
	args: string[];
	riskLevel: "safe" | "caution" | "dangerous";
	cwd?: string;
	description?: string;
	createdAt: string;
};

export type Settings = {
	workspace: {
		name?: string;
		defaultProjectId?: string;
		theme: "dark" | "light" | "system";
	};
	provider: {
		name?: string;
		apiKeyConfigured: boolean;
		defaultModel?: string;
		fallbackModel?: string;
		usageTrackingEnabled: boolean;
		/** OpenAI-compatible base URL (e.g. https://api.x.ai/v1). Key is never stored here. */
		apiBaseUrl?: string;
	};
	models: {
		codingModel?: string;
		researchModel?: string;
		summarizationModel?: string;
		localModelEnabled: boolean;
	};
	server: {
		mode: "local" | "cloud";
		apiBaseUrl: string;
		healthStatus: "unknown" | "healthy" | "unreachable";
		syncStatus: "unknown" | "synced" | "pending" | "error";
	};
	agentClis: {
		codexPath?: string;
		claudeCodePath?: string;
		opencodePath?: string;
		axgaPath?: string;
	};
	sessionDirs: {
		codexDir?: string;
		codexEnabled: boolean;
		claudeCodeDir?: string;
		claudeCodeEnabled: boolean;
		opencodeDir?: string;
		opencodeEnabled: boolean;
		cursorDir?: string;
		cursorEnabled: boolean;
		geminiDir?: string;
		geminiEnabled: boolean;
		grokDir?: string;
		grokEnabled: boolean;
	};
	localPaths: {
		defaultProjectsDir?: string;
		defaultTerminalDir?: string;
		dataStorageDir?: string;
	};
	appearance: {
		theme: "dark" | "light" | "system";
		fontSize: "small" | "medium" | "large";
	};
	/** Skills / hooks / plugins enable maps (opt-out; missing ⇒ enabled). */
	extensions?: {
		skills: Record<string, boolean>;
		hooks: Record<string, boolean>;
		plugins: Record<string, boolean>;
	};
	/**
	 * Voice STT prefs — separate from chat LLM (`provider.defaultModel`).
	 * Cloud transcription uses OpenRouter/OpenAI audio APIs.
	 */
	voice?: {
		/** e.g. openai/whisper-large-v3, openai/whisper-1 */
		sttModel?: string;
		/** ISO-639-1 hint: en, th, … Empty = auto */
		sttLanguage?: string;
	};
};

export type AppState = {
	currentProjectId: string | null;
};

// Default source categories for docs
export const DOC_SOURCE_CATEGORIES: {
	category: DocSource["category"];
	title: string;
	description: string;
}[] = [
	{
		category: "specs",
		title: "Product Specs",
		description: "Product requirements, feature specs, PRDs",
	},
	{
		category: "engineering",
		title: "Engineering Guidelines",
		description: "Coding standards, architecture decisions",
	},
	{
		category: "api",
		title: "API References",
		description: "API documentation, endpoint specs",
	},
	{
		category: "sprint",
		title: "Sprint Notes",
		description: "Sprint planning, retros, task notes",
	},
	{
		category: "research",
		title: "Research",
		description: "Research notes, explorations, experiments",
	},
	{
		category: "architecture",
		title: "Architecture",
		description: "System architecture, diagrams, ADRs",
	},
	{
		category: "agent-instructions",
		title: "Agent Instructions",
		description: "AGENTS.md, CLAUDE.md, system prompts",
	},
];

// --- Legacy usage types (kept for Usage page) ---

export type ModelMetrics = {
	model: string;
	requests: number;
	tokens: number;
	latency: number;
	ttft: number;
	prefill: number;
	gen: number;
};

export type DailyBucket = {
	date: string;
	label: string;
	values: Record<string, number>;
	total: number;
	totalTokens: number;
};

export type UsageSummary = {
	totalTokens: number;
	totalRequests: number;
	sessions: number;
	users: number;
	promptTokens: number;
	completionTokens: number;
	requests24h: number;
	requestsLastHour: number;
	avgTokens: number;
	avgTokensIn: number;
	avgTokensOut: number;
};

export type DashboardData = {
	summary: UsageSummary;
	daily: DailyBucket[];
	totals: { totalTokens: number; totalRequests: number; peakBucket: number };
	models: ModelMetrics[];
};

export const EMPTY_USAGE: DashboardData = {
	summary: {
		totalTokens: 0,
		totalRequests: 0,
		sessions: 0,
		users: 0,
		promptTokens: 0,
		completionTokens: 0,
		requests24h: 0,
		requestsLastHour: 0,
		avgTokens: 0,
		avgTokensIn: 0,
		avgTokensOut: 0,
	},
	daily: [],
	totals: { totalTokens: 0, totalRequests: 0, peakBucket: 0 },
	models: [],
};

export const MODEL_COLORS: Record<string, string> = {
	"glm-5.2": "#4A90E2",
	"deepseek-v4-flash": "#50C878",
	"minimax-m3": "#9370DB",
	"nex-n2-pro": "#FF6B6B",
	"nemotron-3-ultra": "#FFA500",
	"step-3.7-flash": "#20B2AA",
	"qwen3.6-27b": "#FF69B4",
};

export const FALLBACK_COLORS = [
	"#4A90E2",
	"#50C878",
	"#9370DB",
	"#FF6B6B",
	"#FFA500",
	"#20B2AA",
	"#FF69B4",
	"#F4D03F",
	"#5DADE2",
	"#48C9B0",
];

export function colorForModel(model: string, idx: number): string {
	return MODEL_COLORS[model] ?? FALLBACK_COLORS[idx % FALLBACK_COLORS.length];
}
