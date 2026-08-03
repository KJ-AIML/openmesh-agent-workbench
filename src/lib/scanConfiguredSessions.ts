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
	if (!workspaceCwd?.trim()) return [];

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
			console.warn("[sessions] workspace scan:", result.error);
			return [];
		}
		return result.sessions ?? [];
	} catch (error) {
		console.error("[sessions] workspace scan failed:", error);
		return [];
	}
}

export function hasConfiguredSessionDir(
	_sessionDirs?: Settings["sessionDirs"],
): boolean {
	// Auto-detect always available once a project is open.
	return true;
}
