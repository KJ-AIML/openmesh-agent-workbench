// Agent Session Adapter for Openmesh
// Abstracts agent CLI launching and session directory reading
// Phase 6: Real session scanning in Tauri, mock in web

import type { AdapterResult, ScannedSession } from "./types";
import { getRuntimeKind } from "./environment";
import { invoke } from "@tauri-apps/api/core";
import { store } from "../store";

interface ScanAgentSessionsResult {
	success: boolean;
	sessions: ScannedSession[];
	error?: string;
}

/**
 * Scan agent session directory for real sessions
 * Phase 6: Real scanning in Tauri, empty in web
 */
export async function scanAgentSessionDirectory(
	tool: string,
	directoryPath: string,
	limit?: number,
): Promise<AdapterResult<ScannedSession[]>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<ScanAgentSessionsResult>(
				"scan_agent_sessions",
				{
					tool,
					directoryPath,
					limit: limit || 100,
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

	// Web mock: empty array
	return {
		success: true,
		data: [],
		isMock: true,
	};
}

/**
 * List agent sessions for a project
 * Phase 1-2: Returns mock sessions from store
 * Future Tauri: Will scan real session directories (~/.codex/sessions/, etc.)
 */
export async function listAgentSessions(
	projectId?: string,
): Promise<AdapterResult<any[]>> {
	const runtime = getRuntimeKind();

	// For now, use existing store data
	const sessions = store.getAgentSessions();
	const filtered = projectId
		? sessions.filter((s) => s.projectId === projectId)
		: sessions;

	return {
		success: true,
		data: filtered,
		isMock: runtime === "web",
	};
}

/**
 * Get a specific agent session by ID
 * Phase 1-2: Returns session from store (mock data)
 * Future Tauri: Will read real session file
 */
export async function getAgentSession(
	sessionId: string,
): Promise<AdapterResult<any | null>> {
	const sessions = store.getAgentSessions();
	const session = sessions.find((s) => s.id === sessionId);

	if (!session) {
		return {
			success: false,
			error: "Session not found",
			data: null,
			isMock: true,
		};
	}

	return {
		success: true,
		data: session,
		isMock: true,
	};
}

/**
 * Summarize an agent session
 * Phase 1-2: Returns existing summary from store (mock)
 * Future Tauri: Will read and parse real session transcript
 */
export async function summarizeAgentSession(
	sessionId: string,
): Promise<AdapterResult<string>> {
	const sessions = store.getAgentSessions();
	const session = sessions.find((s) => s.id === sessionId);

	if (!session) {
		return {
			success: false,
			error: "Session not found",
			data: "",
			isMock: true,
		};
	}

	return {
		success: true,
		data: session.summary || "No summary available",
		isMock: true,
	};
}

/**
 * Attach a session to a task
 * Phase 1-2: Updates store (mock behavior)
 * Future Tauri: Same behavior, but with real session data
 */
export async function attachSessionToTask(
	sessionId: string,
	taskId: string,
): Promise<AdapterResult<void>> {
	const sessions = store.getAgentSessions();
	const idx = sessions.findIndex((s) => s.id === sessionId);

	if (idx >= 0) {
		sessions[idx].linkedTaskId = taskId;
		store.saveAgentSessions(sessions);
	}

	return {
		success: true,
		isMock: true,
	};
}
