// Reactive Vue composable for the Openmesh file-based store
import { ref, computed, watch } from "vue";
import { store } from "./store";
import type {
	Project,
	Sprint,
	Task,
	RecentItem,
	AgentSession,
	CommandPreset,
	Settings,
} from "../types";
import type { FileEntry, DocTreeNode } from "./store";

// --- Global reactive state ---
const isLoading = ref(true);
const settings = ref<Settings>(defaultSettings());
const projectPaths = ref<string[]>([]);
const currentProjectPath = ref<string | null>(null);

// --- Project-scoped reactive state ---
const currentProject = ref<Project | null>(null);
const sprints = ref<Sprint[]>([]);
const tasks = ref<Task[]>([]);
const recentItems = ref<RecentItem[]>([]);
const agentSessions = ref<AgentSession[]>([]);
const commandPresets = ref<CommandPreset[]>([]);
const docs = ref<FileEntry[]>([]);
const docsTree = ref<DocTreeNode[]>([]);
const notes = ref<FileEntry[]>([]);

// --- Derived state ---
const projectSprint = computed(() => sprints.value[0] || null);

const projectCommandPresets = computed(() => commandPresets.value);

const projectNotes = computed(() => notes.value);

const projectDocs = computed(() => docs.value);

const projectSessions = computed(() => agentSessions.value);

const projectTasks = computed(() => tasks.value);

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

// --- Load all data ---
async function loadAll() {
	isLoading.value = true;
	try {
		// Load global data
		const [loadedSettings, loadedPaths, loadedAppState] = await Promise.all([
			store.getSettings(),
			store.getProjectsList(),
			store.getAppState(),
		]);

		settings.value = loadedSettings;
		projectPaths.value = loadedPaths;
		currentProjectPath.value = loadedAppState.currentProjectId || null;

		// Load project-scoped data if we have a current project
		if (currentProjectPath.value) {
			await loadProjectData(currentProjectPath.value);
		}
	} catch (e) {
		console.error("[useStore] Failed to load data:", e);
	} finally {
		isLoading.value = false;
	}
}

async function loadProjectData(projectPath: string) {
	console.log('[loadProjectData] Loading data for:', projectPath);
	try {
		const project = await store.getProject(projectPath);
		console.log('[loadProjectData] Project from backend:', project);
		
		if (!project) {
			console.error('[loadProjectData] Project is null! Check Rust backend.');
			currentProject.value = null;
			return;
		}

		const [loadedSprints, loadedTasks, loadedRecent, loadedSessions, loadedPresets, loadedDocs, loadedDocsTree, loadedNotes] =
			await Promise.all([
				store.getSprint(projectPath).then((s) => (s ? [s] : [])),
				store.getTasks(projectPath),
				store.getRecent(projectPath),
				store.getSessions(projectPath),
				store.getPresets(projectPath),
				store.listDocs(projectPath),
				store.listDocsTree(projectPath),
				store.listNotes(projectPath),
			]);

		console.log('[loadProjectData] All data loaded successfully');
		console.log('[loadProjectData] Tasks:', loadedTasks.length, 'Presets:', loadedPresets.length);

		currentProject.value = project;
		sprints.value = loadedSprints;
		tasks.value = loadedTasks;
		recentItems.value = loadedRecent;
		agentSessions.value = loadedSessions;
		commandPresets.value = loadedPresets;
		docs.value = loadedDocs;
		docsTree.value = loadedDocsTree;
		notes.value = loadedNotes;
	} catch (e) {
		console.error("[loadProjectData] Failed to load project data:", e);
		currentProject.value = null;
	}
}

// --- Watch project changes ---
watch(currentProjectPath, async (newPath, oldPath) => {
	console.log('[watch currentProjectPath] Changed from:', oldPath, 'to:', newPath);
	if (newPath && newPath !== oldPath) {
		console.log('[watch] Loading project data for:', newPath);
		await loadProjectData(newPath);
		await store.saveAppState({ currentProjectId: newPath });
		console.log('[watch] Project data loaded, currentProject:', currentProject.value);
	} else if (!newPath && oldPath) {
		console.log('[watch] Clearing project data');
		currentProject.value = null;
		sprints.value = [];
		tasks.value = [];
		recentItems.value = [];
		agentSessions.value = [];
		commandPresets.value = [];
		docs.value = [];
		docsTree.value = [];
		notes.value = [];
		await store.saveAppState({ currentProjectId: null });
	}
});

// --- Actions ---

async function selectProject(path: string | null) {
	console.log('[selectProject] Setting path to:', path);
	currentProjectPath.value = path;
	// The watch will handle loading the data
}

async function addProject(input: {
	name: string;
	folderPath: string;
	repoUrl?: string;
	defaultBranch?: string;
	docsFolder?: string;
	terminalDir?: string;
	defaultAgentCli?: "codex" | "claude-code" | "opencode" | null;
	notes?: string;
}): Promise<Project> {
	// Init .openmesh/ directory structure
	await store.initProject(input.folderPath);

	// Get the created project
	const project = await store.getProject(input.folderPath);
	if (!project) throw new Error("Failed to create project");

	// Update with user-provided values
	const updatedProject: Project = {
		...project,
		name: input.name,
		repoUrl: input.repoUrl || undefined,
		defaultBranch: input.defaultBranch || "main",
		docsFolder: input.docsFolder || undefined,
		terminalDir: input.terminalDir || undefined,
		defaultAgentCli: input.defaultAgentCli || undefined,
		notes: input.notes || undefined,
	};
	await store.saveProject(input.folderPath, updatedProject);

	// Add to projects list
	await store.addProjectToList(input.folderPath);
	projectPaths.value = await store.getProjectsList();

	// Create default command presets
	const defaultPresets: CommandPreset[] = [
		{
			id: crypto.randomUUID(),
			projectId: updatedProject.id,
			name: "npm run dev",
			command: "npm",
			args: ["run", "dev"],
			riskLevel: "safe",
			cwd: input.folderPath,
			createdAt: new Date().toISOString(),
		},
		{
			id: crypto.randomUUID(),
			projectId: updatedProject.id,
			name: "npm run build",
			command: "npm",
			args: ["run", "build"],
			riskLevel: "safe",
			cwd: input.folderPath,
			createdAt: new Date().toISOString(),
		},
		{
			id: crypto.randomUUID(),
			projectId: updatedProject.id,
			name: "npm test",
			command: "npm",
			args: ["test"],
			riskLevel: "safe",
			cwd: input.folderPath,
			createdAt: new Date().toISOString(),
		},
		{
			id: crypto.randomUUID(),
			projectId: updatedProject.id,
			name: "git status",
			command: "git",
			args: ["status"],
			riskLevel: "safe",
			cwd: input.folderPath,
			createdAt: new Date().toISOString(),
		},
	];
	await store.savePresets(input.folderPath, defaultPresets);
	commandPresets.value = defaultPresets;

	// Set as current project - the watcher will handle loading data
	currentProjectPath.value = input.folderPath;
	await store.saveAppState({ currentProjectId: input.folderPath });

	return updatedProject;
}

async function updateProject(updates: Partial<Project>) {
	if (!currentProject.value || !currentProjectPath.value) return;
	const updated = { ...currentProject.value, ...updates, updatedAt: new Date().toISOString() };
	await store.saveProject(currentProjectPath.value, updated);
	currentProject.value = updated;
}

async function deleteProject() {
	if (!currentProjectPath.value) return;
	await store.deleteProjectData(currentProjectPath.value);
	projectPaths.value = await store.getProjectsList();
	currentProjectPath.value = null;
}

async function saveSettings(updates: Partial<Settings>) {
	const newSettings = { ...settings.value, ...updates };
	await store.saveSettings(newSettings);
	settings.value = newSettings;
}

// --- Sprint actions ---
async function createMockSprint(name: string) {
	if (!currentProject.value || !currentProjectPath.value) return;
	const sprint: Sprint = {
		id: crypto.randomUUID(),
		projectId: currentProject.value.id,
		name,
		status: "active",
		source: "mock",
		createdAt: new Date().toISOString(),
		updatedAt: new Date().toISOString(),
	};
	await store.saveSprint(currentProjectPath.value, sprint);
	sprints.value = [sprint];
	return sprint;
}

// --- Task actions ---
async function updateTask(id: string, updates: Partial<Task>) {
	if (!currentProjectPath.value) return;
	const idx = tasks.value.findIndex((t) => t.id === id);
	if (idx >= 0) {
		tasks.value[idx] = { ...tasks.value[idx], ...updates, updatedAt: new Date().toISOString() };
		await store.saveTasks(currentProjectPath.value, tasks.value);
	}
}

// --- Recent items ---
async function addRecentItem(item: Omit<RecentItem, "id" | "lastOpenedAt" | "pinned">) {
	if (!currentProjectPath.value) return;
	const dedupKey = `${item.type}:${item.projectId || ""}:${item.sourceId || item.sourcePath || item.title}`;
	const existingIdx = recentItems.value.findIndex(
		(i) => `${i.type}:${i.projectId || ""}:${i.sourceId || i.sourcePath || i.title}` === dedupKey
	);
	if (existingIdx >= 0) {
		recentItems.value[existingIdx].lastOpenedAt = new Date().toISOString();
		recentItems.value[existingIdx].title = item.title;
	} else {
		recentItems.value.unshift({
			...item,
			id: crypto.randomUUID(),
			lastOpenedAt: new Date().toISOString(),
			pinned: false,
		});
	}
	// Prune to 50
	recentItems.value = recentItems.value.slice(0, 50);
	await store.saveRecent(currentProjectPath.value, recentItems.value);
}

// --- Agent sessions ---
async function deleteAgentSession(id: string) {
	if (!currentProjectPath.value) return;
	agentSessions.value = agentSessions.value.filter((s) => s.id !== id);
	await store.saveSessions(currentProjectPath.value, agentSessions.value);
}

async function updateAgentSession(id: string, updates: Partial<AgentSession>) {
	if (!currentProjectPath.value) return;
	const idx = agentSessions.value.findIndex((s) => s.id === id);
	if (idx >= 0) {
		agentSessions.value[idx] = { ...agentSessions.value[idx], ...updates, updatedAt: new Date().toISOString() };
		await store.saveSessions(currentProjectPath.value, agentSessions.value);
	}
}

// --- Command presets ---
async function addCommandPreset(preset: Omit<CommandPreset, "id" | "createdAt">) {
	if (!currentProjectPath.value) return;
	const newPreset: CommandPreset = {
		...preset,
		id: crypto.randomUUID(),
		createdAt: new Date().toISOString(),
	};
	commandPresets.value.push(newPreset);
	await store.savePresets(currentProjectPath.value, commandPresets.value);
	return newPreset;
}

async function deleteCommandPreset(id: string) {
	if (!currentProjectPath.value) return;
	commandPresets.value = commandPresets.value.filter((p) => p.id !== id);
	await store.savePresets(currentProjectPath.value, commandPresets.value);
}

// --- Docs ---
async function refreshDocs() {
	if (!currentProjectPath.value) return;
	docs.value = await store.listDocs(currentProjectPath.value);
	docsTree.value = await store.listDocsTree(currentProjectPath.value);
}

async function writeDoc(filename: string, content: string) {
	if (!currentProjectPath.value) return;
	await store.writeDoc(currentProjectPath.value, filename, content);
	await refreshDocs();
}

async function deleteDoc(filename: string) {
	if (!currentProjectPath.value) return;
	await store.deleteDoc(currentProjectPath.value, filename);
	await refreshDocs();
}

async function readDoc(filename: string): Promise<string> {
	if (!currentProjectPath.value) return "";
	return store.readDoc(currentProjectPath.value, filename);
}

async function createDocFolder(folderName: string) {
	if (!currentProjectPath.value) return;
	await store.createDocFolder(currentProjectPath.value, folderName);
	await refreshDocs();
}

async function renameDocFolder(oldName: string, newName: string) {
	if (!currentProjectPath.value) return;
	await store.renameDocFolder(currentProjectPath.value, oldName, newName);
	await refreshDocs();
}

async function deleteDocFolder(folderName: string) {
	if (!currentProjectPath.value) return;
	await store.deleteDocFolder(currentProjectPath.value, folderName);
	await refreshDocs();
}

async function moveDoc(filename: string, targetFolder: string) {
	if (!currentProjectPath.value) return;
	await store.moveDoc(currentProjectPath.value, filename, targetFolder);
	await refreshDocs();
}

async function renameDoc(oldFilename: string, newFilename: string) {
	if (!currentProjectPath.value) return;
	await store.renameDoc(currentProjectPath.value, oldFilename, newFilename);
	await refreshDocs();
}

// --- Notes ---
async function refreshNotes() {
	if (!currentProjectPath.value) return;
	notes.value = await store.listNotes(currentProjectPath.value);
}

async function writeNote(filename: string, content: string) {
	if (!currentProjectPath.value) return;
	await store.writeNote(currentProjectPath.value, filename, content);
	await refreshNotes();
}

async function deleteNote(filename: string) {
	if (!currentProjectPath.value) return;
	await store.deleteNote(currentProjectPath.value, filename);
	await refreshNotes();
}

async function renameNote(oldFilename: string, newFilename: string) {
	if (!currentProjectPath.value) return;
	await store.renameNote(currentProjectPath.value, oldFilename, newFilename);
	await refreshNotes();
}

async function readNote(filename: string): Promise<string> {
	if (!currentProjectPath.value) return "";
	return store.readNote(currentProjectPath.value, filename);
}

async function importFile(folder: string, filename: string, content: string) {
	if (!currentProjectPath.value) return;
	await store.importFile(currentProjectPath.value, folder, filename, content);
	if (folder === "docs") await refreshDocs();
	if (folder === "notes") await refreshNotes();
}

// --- Export ---
function getRecentItemsForProject(limit = 5): RecentItem[] {
	return recentItems.value.slice(0, limit);
}

// --- Reset (clear all file-based data) ---
async function resetAll() {
	// Get all project paths before resetting
	const paths = [...projectPaths.value];
	
	// Call Rust backend to delete all data from disk
	await store.resetAllData();
	
	// Clear in-memory state
	projectPaths.value = [];
	currentProjectPath.value = null;
	currentProject.value = null;
	sprints.value = [];
	tasks.value = [];
	recentItems.value = [];
	agentSessions.value = [];
	commandPresets.value = [];
	docs.value = [];
	notes.value = [];
	settings.value = defaultSettings();
	
	// Save default settings to recreate ~/.openmesh/settings.json
	await store.saveSettings(settings.value);
	await store.saveAppState({ currentProjectId: null });
}

// --- Init ---
loadAll();

export function useStore() {
	return {
		// Loading state
		isLoading,

		// Global state
		settings,
		projectPaths,
		currentProjectPath,

		// Project-scoped state
		currentProject,
		projectSprint,
		projectTasks,
		projectSessions,
		projectCommandPresets,
		projectDocs,
		projectNotes,
		docsTree,

		// Actions
		selectProject,
		addProject,
		updateProject,
		deleteProject,
		saveSettings,
		createMockSprint,
		updateTask,
		addRecentItem,
		deleteAgentSession,
		updateAgentSession,
		addCommandPreset,
		deleteCommandPreset,
		resetAll,

		// Docs
		refreshDocs,
		readDoc,
		writeDoc,
		deleteDoc,
		createDocFolder,
		renameDocFolder,
		deleteDocFolder,
		moveDoc,
		renameDoc,

		// Notes
		refreshNotes,
		readNote,
		writeNote,
		deleteNote,
		renameNote,
		importFile,

		// Helpers
		getRecentItemsForProject,

		// Raw store access
		store,
	};
}
