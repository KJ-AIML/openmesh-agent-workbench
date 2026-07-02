// Reactive Vue composable for the Openmesh store
import { ref, computed } from "vue";
import { store } from "./store";
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
} from "../types";

// Global reactive state — shared across all components
const projects = ref<Project[]>(store.getProjects());
const currentProjectId = ref<string | null>(store.getCurrentProjectId());
const docSources = ref<DocSource[]>(store.getDocSources());
const sprints = ref<Sprint[]>(store.getSprints());
const tasks = ref<Task[]>(store.getTasks());
const recentItems = ref<RecentItem[]>(store.getRecentItems());
const agentSessions = ref<AgentSession[]>(store.getAgentSessions());
const terminalPresets = ref<TerminalPreset[]>(store.getTerminalPresets());
const commandPresets = ref<CommandPreset[]>(store.getCommandPresets());
const settings = ref<Settings>(store.getSettings());

// Derived
const currentProject = computed(() =>
	currentProjectId.value
		? projects.value.find((p) => p.id === currentProjectId.value)
		: undefined,
);

const projectDocSources = computed(() =>
	currentProjectId.value
		? docSources.value.filter((d) => d.projectId === currentProjectId.value)
		: [],
);

const projectSprint = computed(() =>
	currentProjectId.value
		? sprints.value.find((s) => s.projectId === currentProjectId.value)
		: undefined,
);

const projectTasks = computed(() =>
	currentProjectId.value
		? tasks.value.filter((t) => t.projectId === currentProjectId.value)
		: [],
);

const projectSessions = computed(() =>
	agentSessions.value.filter((s) =>
		currentProjectId.value ? s.projectId === currentProjectId.value : true,
	),
);

const projectPresets = computed(() =>
	currentProjectId.value
		? terminalPresets.value.filter(
				(p) => p.projectId === currentProjectId.value,
			)
		: [],
);

const projectCommandPresets = computed(() =>
	currentProjectId.value
		? commandPresets.value.filter((p) => p.projectId === currentProjectId.value)
		: [],
);

// Sync helpers — write to store, then update reactive refs
function syncProjects() {
	projects.value = store.getProjects();
}
function syncDocSources() {
	docSources.value = store.getDocSources();
}
function syncSprints() {
	sprints.value = store.getSprints();
}
function syncTasks() {
	tasks.value = store.getTasks();
}
function syncRecentItems() {
	recentItems.value = store.getRecentItems();
}
function syncAgentSessions() {
	agentSessions.value = store.getAgentSessions();
}
function syncTerminalPresets() {
	terminalPresets.value = store.getTerminalPresets();
}
function syncCommandPresets() {
	commandPresets.value = store.getCommandPresets();
}
function syncSettings() {
	settings.value = store.getSettings();
}

// Actions
function selectProject(id: string | null) {
	store.setCurrentProject(id);
	currentProjectId.value = id;
}

function addProject(input: Parameters<typeof store.addProject>[0]): Project {
	const project = store.addProject(input);
	syncProjects();
	currentProjectId.value = project.id;
	// Init default doc sources for the new project
	store.initDocSourcesForProject(project.id);
	syncDocSources();
	// Init mock agent sessions for the new project
	store.initMockSessions(project.id);
	syncAgentSessions();
	// Init default command presets
	store.addCommandPreset({
		projectId: project.id,
		name: "npm run dev",
		command: "npm",
		args: ["run", "dev"],
		riskLevel: "safe",
		cwd: project.folderPath,
	});
	store.addCommandPreset({
		projectId: project.id,
		name: "npm run build",
		command: "npm",
		args: ["run", "build"],
		riskLevel: "safe",
		cwd: project.folderPath,
	});
	store.addCommandPreset({
		projectId: project.id,
		name: "npm test",
		command: "npm",
		args: ["test"],
		riskLevel: "safe",
		cwd: project.folderPath,
	});
	store.addCommandPreset({
		projectId: project.id,
		name: "git status",
		command: "git",
		args: ["status"],
		riskLevel: "safe",
		cwd: project.folderPath,
	});
	syncCommandPresets();
	// Track recent
	store.addRecentItem({
		type: "project",
		title: project.name,
		projectId: project.id,
		sourceId: project.id,
	});
	syncRecentItems();
	return project;
}

function updateDocSource(id: string, updates: Partial<DocSource>) {
	store.updateDocSource(id, updates);
	syncDocSources();
	// Track recent when connecting a doc source
	if (updates.isConnected) {
		const source = store.getDocSources().find((s) => s.id === id);
		if (source) {
			store.addRecentItem({
				type: "doc",
				title: source.title,
				projectId: source.projectId,
				sourceId: source.id,
			});
			syncRecentItems();
		}
	}
}

function createMockSprint(projectId: string): Sprint {
	const sprint = store.createMockSprint(projectId);
	syncSprints();
	syncTasks();
	return sprint;
}

function updateTask(id: string, updates: Partial<Task>) {
	store.updateTask(id, updates);
	syncTasks();
	// Track recent when task status changes
	if (updates.status) {
		const task = store.getTasks().find((t) => t.id === id);
		if (task) {
			store.addRecentItem({
				type: "task",
				title: task.title,
				projectId: task.projectId,
				sourceId: task.id,
			});
			syncRecentItems();
		}
	}
}

function addRecentItem(
	item: Omit<RecentItem, "id" | "lastOpenedAt" | "pinned">,
) {
	store.addRecentItem(item);
	syncRecentItems();
}

function deleteAgentSession(id: string) {
	store.deleteAgentSession(id);
	syncAgentSessions();
}

function updateAgentSession(id: string, updates: Partial<AgentSession>) {
	store.updateAgentSession(id, updates);
	syncAgentSessions();
}

function initMockSessions(projectId: string) {
	store.initMockSessions(projectId);
	syncAgentSessions();
}

function addTerminalPreset(projectId: string, name: string, command: string) {
	const preset = store.addTerminalPreset(projectId, name, command);
	syncTerminalPresets();
	return preset;
}

function addCommandPreset(preset: Omit<CommandPreset, "id" | "createdAt">) {
	const newPreset = store.addCommandPreset(preset);
	syncCommandPresets();
	return newPreset;
}

function deleteCommandPreset(id: string) {
	store.deleteCommandPreset(id);
	syncCommandPresets();
}

function updateProject(id: string, updates: Partial<Project>) {
	store.updateProject(id, updates);
	syncProjects();
}

function deleteProject(id: string) {
	store.deleteProject(id);
	syncProjects();
	currentProjectId.value = store.getCurrentProjectId();
	syncDocSources();
	syncSprints();
	syncTasks();
	syncAgentSessions();
	syncCommandPresets();
	syncTerminalPresets();
}

function updateCommandPreset(id: string, updates: Partial<CommandPreset>) {
	store.updateCommandPreset(id, updates);
	syncCommandPresets();
}

function saveSettings(updates: Partial<Settings>) {
	store.updateSettings(updates);
	syncSettings();
}

function resetAll() {
	store.resetAll();
	projects.value = [];
	currentProjectId.value = null;
	docSources.value = [];
	sprints.value = [];
	tasks.value = [];
	recentItems.value = [];
	agentSessions.value = [];
	terminalPresets.value = [];
	settings.value = store.getSettings();
}

export function useStore() {
	return {
		// Reactive state
		projects,
		currentProjectId,
		currentProject,
		docSources,
		projectDocSources,
		sprints,
		projectSprint,
		tasks,
		projectTasks,
		recentItems,
		agentSessions,
		projectSessions,
		terminalPresets,
		projectPresets,
		commandPresets,
		projectCommandPresets,
		settings,

		// Computed helpers
		getRecentItemsForProject: (limit = 5) =>
			store.getRecentItemsForProject(currentProjectId.value, limit),

		// Actions
		selectProject,
		addProject,
		updateProject,
		deleteProject,
		updateDocSource,
		createMockSprint,
		updateTask,
		addRecentItem,
		deleteAgentSession,
		updateAgentSession,
		initMockSessions,
		addTerminalPreset,
		addCommandPreset,
		deleteCommandPreset,
		updateCommandPreset,
		saveSettings,
		resetAll,

		// Raw store access (for export/import)
		store,
	};
}
