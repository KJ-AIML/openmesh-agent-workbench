// Openmesh file-based storage layer (Tauri only)
// All data stored in ~/.openmesh/ (global) and <project>/.openmesh/ (per-project)
import { invokeTyped, legacyInvoke } from "./ipc";
import type {
	Project,
	Sprint,
	Task,
	RecentItem,
	AgentSession,
	CommandPreset,
	Settings,
	AppState,
} from "../types";

// Re-export FileEntry type from adapter types
export interface FileEntry {
	name: string;
	path: string;
	is_dir: boolean;
	size: number | null;
	modified_at: string | null;
}

export interface DocTreeNode {
	name: string;
	path: string;
	nodeType: "file" | "folder";
	children?: DocTreeNode[];
	size?: number | null;
	modifiedAt?: string | null;
}

// --- Global Store API (async, file-based) ---

export const store = {
	// Settings
	async getSettings(): Promise<Settings> {
		return invokeTyped<Settings>("get_settings");
	},
	async saveSettings(settings: Settings): Promise<void> {
		return invokeTyped("save_settings", { settings });
	},

	// Projects list
	async getProjectsList(): Promise<string[]> {
		return invokeTyped<string[]>("get_projects_list");
	},
	async addProjectToList(path: string): Promise<void> {
		return invokeTyped("add_project_to_list", { path });
	},
	async removeProjectFromList(path: string): Promise<void> {
		return invokeTyped("remove_project_from_list", { path });
	},

	// App state
	async getAppState(): Promise<AppState> {
		return invokeTyped<AppState>("get_app_state");
	},
	async saveAppState(state: AppState): Promise<void> {
		return invokeTyped("save_app_state", { state });
	},

	// Project init/read/delete
	async initProject(projectPath: string): Promise<void> {
		return invokeTyped("init_project_cmd", { projectPath });
	},
	async getProject(projectPath: string): Promise<Project | null> {
		return invokeTyped<Project | null>("get_project", { projectPath });
	},
	async saveProject(projectPath: string, project: Project): Promise<void> {
		return invokeTyped("save_project", { projectPath, project });
	},
	async deleteProjectData(projectPath: string): Promise<void> {
		return invokeTyped("delete_project_cmd", { projectPath });
	},

	// Project-scoped data
	async getSessions(projectPath: string): Promise<AgentSession[]> {
		return legacyInvoke<AgentSession[]>("get_sessions", { projectPath });
	},
	async saveSessions(projectPath: string, sessions: AgentSession[]): Promise<void> {
		return legacyInvoke("save_sessions", { projectPath, sessions });
	},
	async getSprint(projectPath: string): Promise<Sprint | null> {
		return legacyInvoke<Sprint | null>("get_sprint", { projectPath });
	},
	async saveSprint(projectPath: string, sprint: Sprint): Promise<void> {
		return legacyInvoke("save_sprint", { projectPath, sprint });
	},
	async getTasks(projectPath: string): Promise<Task[]> {
		return legacyInvoke<Task[]>("get_tasks", { projectPath });
	},
	async saveTasks(projectPath: string, tasks: Task[]): Promise<void> {
		return legacyInvoke("save_tasks", { projectPath, tasks });
	},
	async getPresets(projectPath: string): Promise<CommandPreset[]> {
		return legacyInvoke<CommandPreset[]>("get_presets", { projectPath });
	},
	async savePresets(projectPath: string, presets: CommandPreset[]): Promise<void> {
		return legacyInvoke("save_presets", { projectPath, presets });
	},
	async getRecent(projectPath: string): Promise<RecentItem[]> {
		return legacyInvoke<RecentItem[]>("get_recent", { projectPath });
	},
	async saveRecent(projectPath: string, items: RecentItem[]): Promise<void> {
		return legacyInvoke("save_recent", { projectPath, items });
	},

	// Docs (markdown files)
	async listDocs(projectPath: string): Promise<FileEntry[]> {
		return legacyInvoke<FileEntry[]>("list_docs", { projectPath });
	},
	async listDocsTree(projectPath: string): Promise<DocTreeNode[]> {
		return legacyInvoke<DocTreeNode[]>("list_docs_tree", { projectPath });
	},
	async readDoc(projectPath: string, filename: string): Promise<string> {
		return legacyInvoke<string>("read_doc", { projectPath, filename });
	},
	async writeDoc(projectPath: string, filename: string, content: string): Promise<void> {
		return legacyInvoke("write_doc", { projectPath, filename, content });
	},
	async deleteDoc(projectPath: string, filename: string): Promise<void> {
		return legacyInvoke("delete_doc", { projectPath, filename });
	},
	async createDocFolder(projectPath: string, folderName: string): Promise<void> {
		return legacyInvoke("create_doc_folder", { projectPath, folderName });
	},
	async renameDocFolder(projectPath: string, oldName: string, newName: string): Promise<void> {
		return legacyInvoke("rename_doc_folder", { projectPath, oldName, newName });
	},
	async deleteDocFolder(projectPath: string, folderName: string): Promise<void> {
		return legacyInvoke("delete_doc_folder", { projectPath, folderName });
	},
	async moveDoc(projectPath: string, filename: string, targetFolder: string): Promise<void> {
		return legacyInvoke("move_doc", { projectPath, filename, targetFolder });
	},
	async renameDoc(projectPath: string, oldFilename: string, newFilename: string): Promise<void> {
		return legacyInvoke("rename_doc", { projectPath, oldFilename, newFilename });
	},

	// Notes (markdown files)
	async listNotes(projectPath: string): Promise<FileEntry[]> {
		return legacyInvoke<FileEntry[]>("list_notes", { projectPath });
	},
	async readNote(projectPath: string, filename: string): Promise<string> {
		return legacyInvoke<string>("read_note", { projectPath, filename });
	},
	async writeNote(projectPath: string, filename: string, content: string): Promise<void> {
		return legacyInvoke("write_note", { projectPath, filename, content });
	},
	async deleteNote(projectPath: string, filename: string): Promise<void> {
		return legacyInvoke("delete_note", { projectPath, filename });
	},
	async renameNote(projectPath: string, oldFilename: string, newFilename: string): Promise<void> {
		return legacyInvoke("rename_note", { projectPath, oldFilename, newFilename });
	},
	async importFile(projectPath: string, folder: string, filename: string, content: string): Promise<void> {
		return legacyInvoke("import_file", { projectPath, folder, filename, content });
	},

	// Export
	async exportProject(projectPath: string): Promise<string> {
		return legacyInvoke<string>("export_project", { projectPath });
	},

	// Reset all data
	async resetAllData(): Promise<void> {
		return invokeTyped("reset_all_data_cmd");
	},

	// Work snapshot
	async writeSnapshot(projectPath: string, filename: string, content: string): Promise<{ success: boolean; filename?: string; error?: string }> {
		return legacyInvoke("write_snapshot", { projectPath, filename, content });
	},
};
