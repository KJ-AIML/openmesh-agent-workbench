// Environment detection for Openmesh
// Determines whether running in web browser or Tauri desktop environment

import type { RuntimeKind } from "./types";

/**
 * Check if running in Tauri runtime
 * Tauri injects window.__TAURI__ object when running in desktop mode
 */
export function isTauriRuntime(): boolean {
	return typeof window !== "undefined" && "__TAURI__" in window;
}

/**
 * Get the current runtime kind
 * Returns 'tauri' if running in Tauri desktop app, 'web' otherwise
 */
export function getRuntimeKind(): RuntimeKind {
	return isTauriRuntime() ? "tauri" : "web";
}

/**
 * Check if a specific native feature is available.
 * Returns true only when running inside Tauri.
 */
export function hasNativeFeature(feature: string): boolean {
	if (!isTauriRuntime()) return false;
	// All core features (folder picker, path validation, open folder,
	// git status, terminal launch, agent CLI launch, session scanning,
	// command preset execution) are implemented in the Tauri backend.
	const supported = new Set([
		"folder-picker",
		"path-validation",
		"open-folder",
		"git-status",
		"terminal",
		"agent-cli",
		"session-scanning",
		"command-preset",
	]);
	return supported.has(feature);
}
