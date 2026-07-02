// Adapter type definitions for Openmesh Tauri migration
// These types define the contract between Vue components and native/web implementations

export type RuntimeKind = "web" | "tauri";

export interface PathValidation {
	exists: boolean;
	isDirectory: boolean;
	isFile: boolean;
	normalizedPath?: string;
	error?: string;
}

export interface FileEntry {
	name: string;
	path: string;
	isDir: boolean;
	size?: number;
}

export interface GitStatus {
	branch: string;
	isClean: boolean;
	modifiedFiles: number;
	untrackedFiles: number;
	lastCommitHash: string;
	lastCommitMessage: string;
}

export interface TerminalOptions {
	workingDir: string;
	shell?: string;
}

export interface StorageStatus {
	storageType: "localStorage" | "sqlite" | "json" | "file-based";
	storagePath?: string;
	storageSize: number;
	version: string;
}

export interface AdapterResult<T> {
	success: boolean;
	data?: T;
	error?: string;
	isMock?: boolean;
}

export interface ScannedSession {
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
}
