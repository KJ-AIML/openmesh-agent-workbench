// Openmesh file-based storage layer (Tauri only)
// All data stored in ~/.openmesh/ (global) and <project>/.openmesh/ (per-project)
import { invoke } from "@tauri-apps/api/core";
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

// --- Global Store API (async, file-based) ---

export const store = {
	// Settings
	async getSettings(): Promise<Settings> {
		return invoke<Settings>("get_settings");
	},
	async saveSettings(settings: Settings): Promise<void> {
		return invoke("save_settings", { settings });
	},

	// Projects list
	async getProjectsList(): Promise<string[]> {
		return invoke<string[]>("get_projects_list");
	},
	async addProjectToList(path: string): Promise<void> {
		return invoke("add_project_to_list", { path });
	},
	async removeProjectFromList(path: string): Promise<void> {
		return invoke("remove_project_from_list", { path });
	},

	// App state
	async getAppState(): Promise<AppState> {
		return invoke<AppState>("get_app_state");
	},
	async saveAppState(state: AppState): Promise<void> {
		return invoke("save_app_state", { state });
	},

	// Project init/read/delete
	async initProject(projectPath: string): Promise<void> {
		return invoke("init_project_cmd", { projectPath });
	},
	async getProject(projectPath: string): Promise<Project | null> {
		return invoke<Project | null>("get_project", { projectPath });
	},
	async saveProject(projectPath: string, project: Project): Promise<void> {
		return invoke("save_project", { projectPath, project });
	},
	async deleteProjectData(projectPath: string): Promise<void> {
		return invoke("delete_project_cmd", { projectPath });
	},

	// Project-scoped data
	async getSessions(projectPath: string): Promise<AgentSession[]> {
		return invoke<AgentSession[]>("get_sessions", { projectPath });
	},
	async saveSessions(projectPath: string, sessions: AgentSession[]): Promise<void> {
		return invoke("save_sessions", { projectPath, sessions });
	},
	async getSprint(projectPath: string): Promise<Sprint | null> {
		return invoke<Sprint | null>("get_sprint", { projectPath });
	},
	async saveSprint(projectPath: string, sprint: Sprint): Promise<void> {
		return invoke("save_sprint", { projectPath, sprint });
	},
	async getTasks(projectPath: string): Promise<Task[]> {
		return invoke<Task[]>("get_tasks", { projectPath });
	},
	async saveTasks(projectPath: string, tasks: Task[]): Promise<void> {
		return invoke("save_tasks", { projectPath, tasks });
	},
	async getPresets(projectPath: string): Promise<CommandPreset[]> {
		return invoke<CommandPreset[]>("get_presets", { projectPath });
	},
	async savePresets(projectPath: string, presets: CommandPreset[]): Promise<void> {
		return invoke("save_presets", { projectPath, presets });
	},
	async getRecent(projectPath: string): Promise<RecentItem[]> {
		return invoke<RecentItem[]>("get_recent", { projectPath });
	},
	async saveRecent(projectPath: string, items: RecentItem[]): Promise<void> {
		return invoke("save_recent", { projectPath, items });
	},

	// Docs (markdown files)
	async listDocs(projectPath: string): Promise<FileEntry[]> {
		return invoke<FileEntry[]>("list_docs", { projectPath });
	},
	async readDoc(projectPath: string, filename: string): Promise<string> {
		return invoke<string>("read_doc", { projectPath, filename });
	},
	async writeDoc(projectPath: string, filename: string, content: string): Promise<void> {
		return invoke("write_doc", { projectPath, filename, content });
	},
	async deleteDoc(projectPath: string, filename: string): Promise<void> {
		return invoke("delete_doc", { projectPath, filename });
	},

	// Notes (markdown files)
	async listNotes(projectPath: string): Promise<FileEntry[]> {
		return invoke<FileEntry[]>("list_notes", { projectPath });
	},
	async readNote(projectPath: string, filename: string): Promise<string> {
		return invoke<string>("read_note", { projectPath, filename });
	},
	async writeNote(projectPath: string, filename: string, content: string): Promise<void> {
		return invoke("write_note", { projectPath, filename, content });
	},
	async deleteNote(projectPath: string, filename: string): Promise<void> {
		return invoke("delete_note", { projectPath, filename });
	},
	async importFile(projectPath: string, folder: string, filename: string, content: string): Promise<void> {
		return invoke("import_file", { projectPath, folder, filename, content });
	},

	// Export
	async exportProject(projectPath: string): Promise<string> {
		return invoke<string>("export_project", { projectPath });
	},

	// Reset all data
	async resetAllData(): Promise<void> {
		return invoke("reset_all_data_cmd");
	},
};
