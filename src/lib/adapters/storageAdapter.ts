// Storage Adapter for Openmesh
// Abstracts persistence layer (localStorage now, SQLite later)
// Phase 1-2: Wraps existing localStorage/store behavior

import type { StorageStatus, AdapterResult } from "./types";
import { getRuntimeKind } from "./environment";
import { store } from "../store";

/**
 * Get storage status and metadata
 * Phase 1-2: Returns localStorage info
 * Future Tauri: Will return SQLite/database info
 */
export async function getStorageStatus(): Promise<
	AdapterResult<StorageStatus>
> {
	const runtime = getRuntimeKind();

	const size = store.getStorageSize();

	return {
		success: true,
		data: {
			storageType: "localStorage",
			storageSize: size,
			version: "0.3.0",
		},
		isMock: runtime === "web",
	};
}

/**
 * Export all state as JSON
 * Phase 1-2: Uses existing store export
 * Future Tauri: Same behavior, but from SQLite
 */
export async function exportState(): Promise<AdapterResult<string>> {
	try {
		const json = store.exportAll();
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
 * Phase 1-2: Uses existing store import
 * Future Tauri: Same behavior, but to SQLite
 */
export async function importState(json: string): Promise<AdapterResult<void>> {
	try {
		const result = store.importAll(json);
		if (result.success) {
			return {
				success: true,
				isMock: false,
			};
		} else {
			return {
				success: false,
				error: result.error,
				isMock: false,
			};
		}
	} catch (e) {
		return {
			success: false,
			error: (e as Error).message,
			isMock: false,
		};
	}
}

/**
 * Reset all state
 * Phase 1-2: Uses existing store reset
 * Future Tauri: Will clear SQLite database
 */
export async function resetState(): Promise<AdapterResult<void>> {
	try {
		store.resetAll();
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
