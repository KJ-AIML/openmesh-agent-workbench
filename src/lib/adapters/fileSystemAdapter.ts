// File System Adapter for Openmesh
// Abstracts file system operations (folder picker, path validation, file reading)
// Phase 3: Native folder picker and path validation in Tauri, web fallback in browser

import type { PathValidation, FileEntry, AdapterResult } from "./types";
import { getRuntimeKind } from "./environment";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

/**
 * Result shape for folder picker operations
 */
interface PickFolderResult {
	success: boolean;
	cancelled?: boolean;
	path?: string;
	isMock: boolean;
	runtime: "web" | "tauri";
	error?: string;
}

/**
 * Open native folder picker dialog
 * Phase 3: Real native dialog in Tauri, prompt() fallback in web
 */
export async function pickFolder(): Promise<PickFolderResult> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const selected = await open({
				directory: true,
				multiple: false,
				title: "Select Folder",
			});

			if (selected === null) {
				// User cancelled
				return {
					success: true,
					cancelled: true,
					isMock: false,
					runtime: "tauri",
				};
			}

			return {
				success: true,
				path: selected as string,
				isMock: false,
				runtime: "tauri",
			};
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
				isMock: false,
				runtime: "tauri",
			};
		}
	}

	// Web fallback: use prompt()
	const path = prompt("Enter folder path:");
	return {
		success: true,
		path: path || undefined,
		cancelled: path === null,
		isMock: true,
		runtime: "web",
	};
}

/**
 * Validate if a path exists and is a directory
 * Phase 3: Real validation in Tauri via Rust command, mock in web
 */
export async function validatePath(
	path: string,
): Promise<AdapterResult<PathValidation>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<PathValidation>("validate_path", { path });
			return {
				success: true,
				data: result,
				isMock: false,
			};
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
				isMock: false,
				data: {
					exists: false,
					isDirectory: false,
					isFile: false,
				},
			};
		}
	}

	// Web mock: always valid
	return {
		success: true,
		data: {
			exists: true,
			isDirectory: true,
			isFile: false,
		},
		isMock: true,
	};
}

/**
 * Open folder in system file browser
 * Phase 4A: Real implementation in Tauri, mock in web
 */
export async function openFolder(path: string): Promise<AdapterResult<void>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<{ success: boolean; error?: string }>(
				"open_folder",
				{ path },
			);

			if (result.success) {
				return {
					success: true,
					isMock: false,
				};
			} else {
				return {
					success: false,
					error: result.error || "Failed to open folder",
					isMock: false,
				};
			}
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
				isMock: false,
			};
		}
	}

	// Web mock: show alert
	alert(`Mock: would open folder at ${path}`);
	return {
		success: true,
		isMock: true,
	};
}

/**
 * Read directory contents
 * Phase 1-2: Returns empty array (mock)
 * Future Tauri: Will use std::fs::read_dir()
 */
export async function readDir(
	_path: string,
): Promise<AdapterResult<FileEntry[]>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		// Phase 1-2: Not implemented yet
		return {
			success: false,
			error: "Read directory not implemented in Phase 1-2",
			isMock: true,
			data: [],
		};
	}

	// Web mock: empty array
	return {
		success: true,
		data: [],
		isMock: true,
	};
}

/**
 * Count files in directory
 * Phase 1-2: Returns random number (mock)
 * Future Tauri: Will count actual files
 */
export async function countFiles(
	_path: string,
): Promise<AdapterResult<number>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		// Phase 1-2: Not implemented yet
		return {
			success: false,
			error: "Count files not implemented in Phase 1-2",
			isMock: true,
			data: 0,
		};
	}

	// Web mock: random count
	return {
		success: true,
		data: Math.floor(Math.random() * 15) + 3,
		isMock: true,
	};
}
