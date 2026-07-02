// Openmesh localStorage persistence layer
import type {
	Project,
	DocSource,
	Sprint,
	Task,
	RecentItem,
	AgentSession,
	TerminalPreset,
	CommandPreset,
	Settings,
	AppState,
} from "../types";

const KEY_PREFIX = "openmesh:";
const DATA_VERSION = "1.0.0";

// --- Helpers ---

function uid(): string {
	return (
		crypto.randomUUID?.() ??
		`${Date.now()}-${Math.random().toString(36).slice(2, 9)}`
	);
}

function now(): string {
	return new Date().toISOString();
}

function read<T>(key: string, fallback: T): T {
	try {
		const raw = localStorage.getItem(KEY_PREFIX + key);
		if (!raw) return fallback;
		return JSON.parse(raw) as T;
	} catch {
		return fallback;
	}
}

function write<T>(key: string, value: T): void {
	try {
		localStorage.setItem(KEY_PREFIX + key, JSON.stringify(value));
	} catch (e) {
		console.error(`[store] Failed to write ${key}:`, e);
	}
}

// --- Default factories ---

function defaultSettings(): Settings {
	return {
		workspace: { theme: "dark" },
		provider: { apiKeyConfigured: false, usageTrackingEnabled: false },
		models: { localModelEnabled: false },
		server: {
			mode: "local",
			apiBaseUrl: "http://localhost:3000",
			healthStatus: "unknown",
			syncStatus: "unknown",
		},
		agentClis: {},
		sessionDirs: {
			codexEnabled: false,
			claudeCodeEnabled: false,
			opencodeEnabled: false,
		},
		localPaths: {},
		appearance: { theme: "dark", fontSize: "medium" },
	};
}

function defaultAppState(): AppState {
	return { currentProjectId: null };
}

// --- Store API ---

export const store = {
	// Projects
	getProjects(): Project[] {
		return read<Project[]>("projects", []);
	},
	saveProjects(projects: Project[]): void {
		write("projects", projects);
	},
	addProject(
		input: Omit<
			Project,
			| "id"
			| "createdAt"
			| "updatedAt"
			| "status"
			| "defaultBranch"
			| "sprintSource"
		> &
			Partial<Project>,
	): Project {
		const projects = this.getProjects();
		const project: Project = {
			id: uid(),
			name: input.name,
			folderPath: input.folderPath,
			repoUrl: input.repoUrl,
			defaultBranch: input.defaultBranch ?? "main",
			sprintSource: input.sprintSource ?? "none",
			docsFolder: input.docsFolder,
			terminalDir: input.terminalDir,
			defaultAgentCli: input.defaultAgentCli ?? null,
			notes: input.notes,
			status: "active",
			createdAt: now(),
			updatedAt: now(),
		};
		projects.push(project);
		this.saveProjects(projects);
		this.setCurrentProject(project.id);
		return project;
	},
	getProject(id: string): Project | undefined {
		return this.getProjects().find((p) => p.id === id);
	},
	updateProject(id: string, updates: Partial<Project>): void {
		const projects = this.getProjects();
		const idx = projects.findIndex((p) => p.id === id);
		if (idx >= 0) {
			projects[idx] = { ...projects[idx], ...updates, updatedAt: now() };
			this.saveProjects(projects);
		}
	},
	deleteProject(id: string): void {
		const projects = this.getProjects().filter((p) => p.id !== id);
		this.saveProjects(projects);
		// Clean up related data
		const docSources = this.getDocSources().filter((d) => d.projectId !== id);
		this.saveDocSources(docSources);
		const sprints = this.getSprints().filter((s) => s.projectId !== id);
		this.saveSprints(sprints);
		const tasks = this.getTasks().filter((t) => t.projectId !== id);
		this.saveTasks(tasks);
		const sessions = this.getAgentSessions().filter((s) => s.projectId !== id);
		this.saveAgentSessions(sessions);
		const presets = this.getCommandPresets().filter((p) => p.projectId !== id);
		write("command-presets", presets);
		const termPresets = this.getTerminalPresets().filter((p) => p.projectId !== id);
		write("terminal-presets", termPresets);
		// Clear current project if deleted
		const appState = this.getAppState();
		if (appState.currentProjectId === id) {
			appState.currentProjectId = null;
			write("app-state", appState);
		}
	},

	// App State (current project)
	getAppState(): AppState {
		return read<AppState>("app-state", defaultAppState());
	},
	setCurrentProject(projectId: string | null): void {
		const state = this.getAppState();
		state.currentProjectId = projectId;
		write("app-state", state);
	},
	getCurrentProjectId(): string | null {
		return this.getAppState().currentProjectId;
	},
	getCurrentProject(): Project | undefined {
		const id = this.getCurrentProjectId();
		return id ? this.getProject(id) : undefined;
	},

	// Doc Sources
	getDocSources(): DocSource[] {
		return read<DocSource[]>("doc-sources", []);
	},
	getDocSourcesForProject(projectId: string): DocSource[] {
		return this.getDocSources().filter((d) => d.projectId === projectId);
	},
	saveDocSources(sources: DocSource[]): void {
		write("doc-sources", sources);
	},
	initDocSourcesForProject(projectId: string): DocSource[] {
		const categories = [
			{
				category: "specs" as const,
				title: "Product Specs",
				description: "Product requirements, feature specs, PRDs",
			},
			{
				category: "engineering" as const,
				title: "Engineering Guidelines",
				description: "Coding standards, architecture decisions",
			},
			{
				category: "api" as const,
				title: "API References",
				description: "API documentation, endpoint specs",
			},
			{
				category: "sprint" as const,
				title: "Sprint Notes",
				description: "Sprint planning, retros, task notes",
			},
			{
				category: "research" as const,
				title: "Research",
				description: "Research notes, explorations, experiments",
			},
			{
				category: "architecture" as const,
				title: "Architecture",
				description: "System architecture, diagrams, ADRs",
			},
			{
				category: "agent-instructions" as const,
				title: "Agent Instructions",
				description: "AGENTS.md, CLAUDE.md, system prompts",
			},
		];
		const sources: DocSource[] = categories.map((c) => ({
			id: uid(),
			projectId,
			title: c.title,
			description: c.description,
			category: c.category,
			isConnected: false,
			agentContextEnabled: false,
			createdAt: now(),
			updatedAt: now(),
		}));
		const existing = this.getDocSources().filter(
			(d) => d.projectId !== projectId,
		);
		this.saveDocSources([...existing, ...sources]);
		return sources;
	},
	updateDocSource(id: string, updates: Partial<DocSource>): void {
		const sources = this.getDocSources();
		const idx = sources.findIndex((s) => s.id === id);
		if (idx >= 0) {
			sources[idx] = { ...sources[idx], ...updates, updatedAt: now() };
			this.saveDocSources(sources);
		}
	},

	// Sprints
	getSprints(): Sprint[] {
		return read<Sprint[]>("sprints", []);
	},
	getSprintForProject(projectId: string): Sprint | undefined {
		return this.getSprints().find((s) => s.projectId === projectId);
	},
	saveSprints(sprints: Sprint[]): void {
		write("sprints", sprints);
	},
	createMockSprint(projectId: string): Sprint {
		const sprint: Sprint = {
			id: uid(),
			projectId,
			name: "Sprint 1 — POC Foundation",
			status: "active",
			source: "mock",
			createdAt: now(),
			updatedAt: now(),
		};
		const sprints = this.getSprints().filter((s) => s.projectId !== projectId);
		sprints.push(sprint);
		this.saveSprints(sprints);
		// Create mock tasks
		this.createMockTasks(sprint.id, projectId);
		return sprint;
	},

	// Tasks
	getTasks(): Task[] {
		return read<Task[]>("tasks", []);
	},
	getTasksForProject(projectId: string): Task[] {
		return this.getTasks().filter((t) => t.projectId === projectId);
	},
	getTasksForSprint(sprintId: string): Task[] {
		return this.getTasks().filter((t) => t.sprintId === sprintId);
	},
	saveTasks(tasks: Task[]): void {
		write("tasks", tasks);
	},
	updateTask(id: string, updates: Partial<Task>): void {
		const tasks = this.getTasks();
		const idx = tasks.findIndex((t) => t.id === id);
		if (idx >= 0) {
			tasks[idx] = { ...tasks[idx], ...updates, updatedAt: now() };
			this.saveTasks(tasks);
		}
	},
	createMockTasks(sprintId: string, projectId: string): void {
		const mockTasks: Omit<Task, "id" | "createdAt" | "updatedAt">[] = [
			{
				sprintId,
				projectId,
				title: "Set up project structure",
				status: "completed",
				priority: "P0",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Add project feature",
				status: "in-progress",
				priority: "P0",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Connect docs sources",
				status: "pending",
				priority: "P1",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Configure agent sessions",
				status: "pending",
				priority: "P1",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Build Home dashboard",
				status: "in-progress",
				priority: "P0",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Set up local persistence",
				status: "pending",
				priority: "P1",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Design sidebar navigation",
				status: "completed",
				priority: "P2",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
			{
				sprintId,
				projectId,
				title: "Write feature specification",
				status: "in-progress",
				priority: "P1",
				linkedDocIds: [],
				linkedSessionIds: [],
			},
		];
		const tasks: Task[] = mockTasks.map((t) => ({
			...t,
			id: uid(),
			createdAt: now(),
			updatedAt: now(),
		}));
		const existing = this.getTasks().filter((t) => t.projectId !== projectId);
		this.saveTasks([...existing, ...tasks]);
	},

	// Recent Items
	getRecentItems(): RecentItem[] {
		return read<RecentItem[]>("recent-items", []);
	},
	getRecentItemsForProject(projectId: string | null, limit = 5): RecentItem[] {
		let items = this.getRecentItems();
		if (projectId) items = items.filter((i) => i.projectId === projectId);
		return items
			.sort((a, b) => b.lastOpenedAt.localeCompare(a.lastOpenedAt))
			.slice(0, limit);
	},
	addRecentItem(
		item: Omit<RecentItem, "id" | "lastOpenedAt" | "pinned">,
	): void {
		const items = this.getRecentItems();
		// Dedup key: type + projectId + (sourceId || sourcePath || title).
		// This catches actions like "open terminal" (no sourceId, but same path)
		// and "run preset" (no sourceId, but same title) instead of spawning duplicates.
		const dedupKey =
			`${item.type}:${item.projectId || ""}:${item.sourceId || item.sourcePath || item.title}`;
		const existingIdx = items.findIndex((i) =>
			`${i.type}:${i.projectId || ""}:${i.sourceId || i.sourcePath || i.title}` === dedupKey,
		);
		if (existingIdx >= 0) {
			items[existingIdx].lastOpenedAt = now();
			items[existingIdx].title = item.title;
			if (item.sourcePath) items[existingIdx].sourcePath = item.sourcePath;
		} else {
			items.unshift({ ...item, id: uid(), lastOpenedAt: now(), pinned: false });
		}
		// Prune to 50
		const pruned = items.slice(0, 50);
		write("recent-items", pruned);
	},

	// Agent Sessions
	getAgentSessions(): AgentSession[] {
		return read<AgentSession[]>("agent-sessions", []);
	},
	getAgentSessionsForProject(projectId: string | null): AgentSession[] {
		let sessions = this.getAgentSessions();
		if (projectId) sessions = sessions.filter((s) => s.projectId === projectId);
		return sessions.sort((a, b) =>
			b.lastActiveAt.localeCompare(a.lastActiveAt),
		);
	},
	saveAgentSessions(sessions: AgentSession[]): void {
		write("agent-sessions", sessions);
	},
	deleteAgentSession(id: string): void {
		const sessions = this.getAgentSessions().filter((s) => s.id !== id);
		this.saveAgentSessions(sessions);
	},
	updateAgentSession(id: string, updates: Partial<AgentSession>): void {
		const sessions = this.getAgentSessions();
		const idx = sessions.findIndex((s) => s.id === id);
		if (idx >= 0) {
			sessions[idx] = { ...sessions[idx], ...updates, updatedAt: now() };
			this.saveAgentSessions(sessions);
		}
	},
	initMockSessions(projectId: string): void {
		const mockSessions: AgentSession[] = [
			{
				id: uid(),
				tool: "codex",
				title: "Implement auth flow",
				projectId,
				status: "completed",
				summary:
					"Built JWT authentication with refresh tokens. Updated 12 files.",
				startedAt: now(),
				lastActiveAt: now(),
				changedFiles: ["src/auth/jwt.ts", "src/middleware/auth.ts"],
				isImportant: false,
				createdAt: now(),
				updatedAt: now(),
			},
			{
				id: uid(),
				tool: "claude-code",
				title: "Refactor database layer",
				projectId,
				status: "completed",
				summary:
					"Migrated from raw SQL to query builder. Improved testability.",
				startedAt: now(),
				lastActiveAt: now(),
				changedFiles: ["src/db/client.ts", "src/db/queries.ts"],
				isImportant: true,
				createdAt: now(),
				updatedAt: now(),
			},
			{
				id: uid(),
				tool: "opencode",
				title: "Write API documentation",
				projectId,
				status: "active",
				summary: "Documenting REST endpoints for v2 API.",
				startedAt: now(),
				lastActiveAt: now(),
				isImportant: false,
				createdAt: now(),
				updatedAt: now(),
			},
			{
				id: uid(),
				tool: "codex",
				title: "Fix CI pipeline",
				projectId,
				status: "completed",
				summary: "Resolved flaky tests and updated GitHub Actions config.",
				startedAt: now(),
				lastActiveAt: now(),
				changedFiles: [".github/workflows/ci.yml"],
				isImportant: false,
				createdAt: now(),
				updatedAt: now(),
			},
			{
				id: uid(),
				tool: "claude-code",
				title: "Design system setup",
				projectId,
				status: "completed",
				summary:
					"Created color tokens, typography scale, and component primitives.",
				startedAt: now(),
				lastActiveAt: now(),
				changedFiles: ["src/styles/tokens.css", "src/components/Button.tsx"],
				isImportant: false,
				createdAt: now(),
				updatedAt: now(),
			},
		];
		// Keep existing sessions for other projects
		const existing = this.getAgentSessions().filter(
			(s) => s.projectId !== projectId,
		);
		this.saveAgentSessions([...existing, ...mockSessions]);
	},

	// Terminal Presets
	getTerminalPresets(): TerminalPreset[] {
		return read<TerminalPreset[]>("terminal-presets", []);
	},
	getTerminalPresetsForProject(projectId: string): TerminalPreset[] {
		return this.getTerminalPresets().filter((p) => p.projectId === projectId);
	},
	addTerminalPreset(
		projectId: string,
		name: string,
		command: string,
	): TerminalPreset {
		const presets = this.getTerminalPresets();
		const preset: TerminalPreset = {
			id: uid(),
			projectId,
			name,
			command,
			createdAt: now(),
		};
		presets.push(preset);
		write("terminal-presets", presets);
		return preset;
	},

	// Command Presets (Phase 6)
	getCommandPresets(): CommandPreset[] {
		return read<CommandPreset[]>("command-presets", []);
	},
	getCommandPresetsForProject(projectId: string): CommandPreset[] {
		return this.getCommandPresets().filter((p) => p.projectId === projectId);
	},
	addCommandPreset(
		preset: Omit<CommandPreset, "id" | "createdAt">,
	): CommandPreset {
		const presets = this.getCommandPresets();
		const newPreset: CommandPreset = {
			...preset,
			id: uid(),
			createdAt: now(),
		};
		presets.push(newPreset);
		write("command-presets", presets);
		return newPreset;
	},
	deleteCommandPreset(id: string): void {
		const presets = this.getCommandPresets().filter((p) => p.id !== id);
		write("command-presets", presets);
	},
	updateCommandPreset(id: string, updates: Partial<CommandPreset>): void {
		const presets = this.getCommandPresets();
		const idx = presets.findIndex((p) => p.id === id);
		if (idx >= 0) {
			presets[idx] = { ...presets[idx], ...updates };
			write("command-presets", presets);
		}
	},

	// Settings
	getSettings(): Settings {
		return read<Settings>("settings", defaultSettings());
	},
	saveSettings(settings: Settings): void {
		write("settings", settings);
	},
	updateSettings(updates: Partial<Settings>): void {
		const settings = this.getSettings();
		write("settings", { ...settings, ...updates });
	},

	// Export / Import / Reset
	exportAll(): string {
		const data: Record<string, unknown> = { _version: DATA_VERSION };
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key?.startsWith(KEY_PREFIX)) {
				try {
					data[key] = JSON.parse(localStorage.getItem(key)!);
				} catch {
					data[key] = localStorage.getItem(key);
				}
			}
		}
		return JSON.stringify(data, null, 2);
	},
	importAll(json: string): { success: boolean; error?: string; warnings?: string[] } {
		try {
			const data = JSON.parse(json);
			if (typeof data !== "object" || data === null || Array.isArray(data)) {
				return { success: false, error: "Import data must be a JSON object" };
			}
			// Check version if present. Older exports without _version are still
			// accepted (backward-compat with Phase 1-6 data).
			const warnings: string[] = [];
			if (data._version && data._version !== DATA_VERSION) {
				warnings.push(`Data version ${data._version} differs from current ${DATA_VERSION}. Importing anyway.`);
			}
			// Basic structural validation for critical keys to prevent corrupt state
			const arrayKeys = [
				"projects",
				"doc-sources",
				"sprints",
				"tasks",
				"recent-items",
				"agent-sessions",
				"terminal-presets",
				"command-presets",
			];
			for (const key of arrayKeys) {
				const fullKey = KEY_PREFIX + key;
				if (data[fullKey] !== undefined && !Array.isArray(data[fullKey])) {
					warnings.push(`Skipping invalid ${key}: expected array`);
					delete data[fullKey];
				}
			}
			const objectKeys = ["settings", "app-state"];
			for (const key of objectKeys) {
				const fullKey = KEY_PREFIX + key;
				if (data[fullKey] !== undefined && (typeof data[fullKey] !== "object" || data[fullKey] === null || Array.isArray(data[fullKey]))) {
					warnings.push(`Skipping invalid ${key}: expected object`);
					delete data[fullKey];
				}
			}
			let imported = 0;
			for (const [key, value] of Object.entries(data)) {
				if (key === "_version") continue;
				if (!key.startsWith(KEY_PREFIX)) continue;
				try {
					localStorage.setItem(key, JSON.stringify(value));
					imported++;
				} catch (e) {
					warnings.push(`Failed to import ${key}: ${(e as Error).message}`);
				}
			}
			if (imported === 0) {
				return { success: false, error: "No openmesh data found in import file", warnings };
			}
			return { success: true, warnings };
		} catch (e) {
			return { success: false, error: (e as Error).message };
		}
	},
	resetAll(): void {
		const keys: string[] = [];
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key?.startsWith(KEY_PREFIX)) keys.push(key);
		}
		keys.forEach((k) => localStorage.removeItem(k));
	},

	// Storage size estimate
	getStorageSize(): number {
		let size = 0;
		for (let i = 0; i < localStorage.length; i++) {
			const key = localStorage.key(i);
			if (key?.startsWith(KEY_PREFIX)) {
				size += (localStorage.getItem(key) ?? "").length;
			}
		}
		return size;
	},
};
