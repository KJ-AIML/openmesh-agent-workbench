// Storage Adapter for Openmesh
// File-based storage in ~/.openmesh/ and <project>/.openmesh/
// Tauri only - no web mode

import type { StorageStatus, AdapterResult } from "./types";
import { store } from "../store";
import { useStore } from "../useStore";

/**
 * Get storage status and metadata
 * Returns file-based storage info
 */
export async function getStorageStatus(): Promise<
	AdapterResult<StorageStatus>
> {
	// File-based storage - return placeholder info
	return {
		success: true,
		data: {
			storageType: "file-based",
			storageSize: 0, // TODO: Calculate actual size from ~/.openmesh/
			version: "0.3.0",
		},
		isMock: false,
	};
}

/**
 * Export current project state as JSON
 */
export async function exportState(): Promise<AdapterResult<string>> {
	try {
		const { currentProject } = useStore();
		if (!currentProject.value) {
			return {
				success: false,
				error: "No project selected",
				isMock: false,
			};
		}

		const json = await store.exportProject(currentProject.value.folderPath);
		return {
			success: true,
			data: json,
			isMock: false,
		};
	} catch (e) {
		return {
			success: false,
			error: (e as Error).message,
			isMock: false,
		};
	}
}

/**
 * Import state from JSON
 * Note: Import is not yet implemented for file-based storage
 */
export async function importState(_json: string): Promise<AdapterResult<void>> {
	return {
		success: false,
		error: "Import not yet implemented for file-based storage",
		isMock: false,
	};
}

/**
 * Reset all state
 * Clears in-memory state (file deletion not yet implemented)
 */
export async function resetState(): Promise<AdapterResult<void>> {
	try {
		const { resetAll } = useStore();
		await resetAll();
		return {
			success: true,
			isMock: false,
		};
	} catch (e) {
		return {
			success: false,
			error: (e as Error).message,
			isMock: false,
		};
	}
}
