// Agent Session Adapter for Openmesh
// Handles scanning agent session directories
// File-based storage: sessions are stored in <project>/.openmesh/sessions.json

import type { AdapterResult, ScannedSession } from "./types";
import { invoke } from "@tauri-apps/api/core";

interface ScanAgentSessionsResult {
	success: boolean;
	sessions: ScannedSession[];
	error?: string;
}

/**
 * Scan agent session directory for real sessions.
 * When workspaceCwd is set, only sessions for that project path are returned.
 */
export async function scanAgentSessionDirectory(
	tool: string,
	directoryPath: string,
	limit?: number,
	workspaceCwd?: string,
): Promise<AdapterResult<ScannedSession[]>> {
	try {
		const result = await invoke<ScanAgentSessionsResult>(
			"scan_agent_sessions",
			{
				tool,
				directoryPath,
				limit: limit || 100,
				workspaceCwd: workspaceCwd || null,
			},
		);

		if (result.success) {
			return {
				success: true,
				data: result.sessions,
				isMock: false,
			};
		} else {
			return {
				success: false,
				error: result.error || "Failed to scan sessions",
				isMock: false,
				data: [],
			};
		}
	} catch (error) {
		return {
			success: false,
			error: error instanceof Error ? error.message : "Unknown error",
			isMock: false,
			data: [],
		};
	}
}
