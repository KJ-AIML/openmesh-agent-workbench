import type { Settings } from "../types";
import type { ScannedSession } from "./adapters/types";
import { invoke } from "@tauri-apps/api/core";

type SessionScanOverrides = {
	codexDir?: string | null;
	claudeCodeDir?: string | null;
	opencodeDir?: string | null;
	cursorDir?: string | null;
	geminiDir?: string | null;
	grokDir?: string | null;
};

type ScanWorkspaceResult = {
	success: boolean;
	sessions: ScannedSession[];
	error?: string;
};

export type ScanConfiguredSessionsResult =
	| { ok: true; sessions: ScannedSession[] }
	| { ok: false; sessions: []; error: string };

/** Optional path overrides only — empty means auto-detect on this OS/device. */
export function sessionDirOverrides(
	sessionDirs: Settings["sessionDirs"] | undefined,
): SessionScanOverrides {
	if (!sessionDirs) return {};
	const pick = (v?: string) => {
		const t = v?.trim();
		return t ? t : null;
	};
	return {
		codexDir: pick(sessionDirs.codexDir),
		claudeCodeDir: pick(sessionDirs.claudeCodeDir),
		opencodeDir: pick(sessionDirs.opencodeDir),
		cursorDir: pick(sessionDirs.cursorDir),
		geminiDir: pick(sessionDirs.geminiDir),
		grokDir: pick(sessionDirs.grokDir),
	};
}

/** Most-recent-first preview for Home / compact lists. */
export function pickRecentAgentSessions(
	sessions: ScannedSession[],
	limit = 4,
): ScannedSession[] {
	return [...sessions]
		.sort(
			(a, b) =>
				new Date(b.lastActiveAt).getTime() - new Date(a.lastActiveAt).getTime(),
		)
		.slice(0, Math.max(0, limit));
}

/**
 * Auto-detect every agent provider root that exists on this machine/OS,
 * then return sessions for the open project folder (with ok/error).
 *
 * Settings paths are optional overrides only — no per-provider enable flags
 * required. Missing providers are skipped automatically.
 */
export async function scanConfiguredSessionsResult(
	sessionDirs: Settings["sessionDirs"] | undefined,
	limit = 100,
	workspaceCwd?: string | null,
): Promise<ScanConfiguredSessionsResult> {
	if (!workspaceCwd?.trim()) {
		return { ok: true, sessions: [] };
	}

	try {
		const result = await invoke<ScanWorkspaceResult>(
			"scan_workspace_agent_sessions",
			{
				workspaceCwd: workspaceCwd.trim(),
				limit,
				overrides: sessionDirOverrides(sessionDirs),
			},
		);
		if (!result.success) {
			const error = result.error?.trim() || "Session scan failed";
			console.warn("[sessions] workspace scan:", error);
			return { ok: false, sessions: [], error };
		}
		return { ok: true, sessions: result.sessions ?? [] };
	} catch (error) {
		const message =
			error instanceof Error ? error.message : "Session scan failed";
		console.error("[sessions] workspace scan failed:", error);
		return { ok: false, sessions: [], error: message };
	}
}

/**
 * Auto-detect every agent provider root that exists on this machine/OS,
 * then return sessions for the open project folder.
 *
 * Settings paths are optional overrides only — no per-provider enable flags
 * required. Missing providers are skipped automatically.
 */
export async function scanConfiguredSessions(
	sessionDirs: Settings["sessionDirs"] | undefined,
	limit = 100,
	workspaceCwd?: string | null,
): Promise<ScannedSession[]> {
	const result = await scanConfiguredSessionsResult(
		sessionDirs,
		limit,
		workspaceCwd,
	);
	return result.sessions;
}

export function hasConfiguredSessionDir(
	_sessionDirs?: Settings["sessionDirs"],
): boolean {
	// Auto-detect always available once a project is open.
	return true;
}
