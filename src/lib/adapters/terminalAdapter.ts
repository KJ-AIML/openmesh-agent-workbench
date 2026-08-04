// Terminal Adapter for Openmesh
// Abstracts terminal launching and command execution
// Phase 5: Real terminal and agent CLI launching in Tauri, web fallback in browser

import type { TerminalOptions, AdapterResult } from "./types";
import { getRuntimeKind } from "./environment";
import { invoke } from "@tauri-apps/api/core";

interface TerminalLaunchResult {
	success: boolean;
	error?: string;
}

interface AgentCliLaunchResult {
	success: boolean;
	error?: string;
}

/**
 * Open terminal at specified working directory
 * Phase 5: Real terminal launch in Tauri, mock in web
 */
export async function openTerminal(
	options: TerminalOptions,
): Promise<AdapterResult<void>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<TerminalLaunchResult>("open_terminal", {
				cwd: options.workingDir,
			});

			if (result.success) {
				return {
					success: true,
					isMock: false,
				};
			} else {
				return {
					success: false,
					error: result.error || "Failed to open terminal",
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
	alert(`Mock: would open terminal at ${options.workingDir}`);
	return {
		success: true,
		isMock: true,
	};
}

/**
 * Open agent CLI at specified working directory
 * Phase 5: Real agent CLI launch in Tauri, mock in web
 */
export async function openAgentCli(
	tool: string,
	cwd: string,
	cliPath?: string,
	opts?: {
		resumeSessionId?: string;
		extraArgs?: string[];
	},
): Promise<AdapterResult<void>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<AgentCliLaunchResult>("open_agent_cli", {
				tool,
				cwd,
				cliPath: cliPath || null,
				resumeSessionId: opts?.resumeSessionId || null,
				extraArgs: opts?.extraArgs || null,
			});

			if (result.success) {
				return {
					success: true,
					isMock: false,
				};
			} else {
				return {
					success: false,
					error: result.error || `Failed to launch ${tool}`,
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
	alert(
		`Mock: would launch ${tool} at ${cwd}` +
			(opts?.resumeSessionId ? ` (resume ${opts.resumeSessionId})` : ""),
	);
	return {
		success: true,
		isMock: true,
	};
}

/**
 * List terminal presets for a project
 * Phase 1-2: Returns empty array (mock)
 * Future Tauri: Will read from storage
 */
export async function listTerminalPresets(
	_projectId?: string,
): Promise<AdapterResult<any[]>> {
	// For now, just return empty array
	// Future: will integrate with storage adapter
	return {
		success: true,
		data: [],
		isMock: true,
	};
}

/**
 * Run a command preset
 * Phase 6: Real command execution in Tauri, mock in web
 */
export async function runCommandPreset(
	command: string,
	args: string[],
	cwd: string,
): Promise<AdapterResult<void>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<{ success: boolean; error?: string }>(
				"run_command_preset",
				{
					command,
					args,
					cwd,
				},
			);

			if (result.success) {
				return {
					success: true,
					isMock: false,
				};
			} else {
				return {
					success: false,
					error: result.error || "Failed to run command",
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
	alert(`Mock: would run ${command} ${args.join(" ")} in ${cwd}`);
	return {
		success: true,
		isMock: true,
	};
}

/**
 * Validate terminal configuration
 * Phase 5: Check if terminal can be launched
 */
export async function validateTerminalConfig(): Promise<AdapterResult<void>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		// For now, assume terminal is available
		// Future: could check if terminal exists
		return {
			success: true,
			isMock: false,
		};
	}

	// Web mock: always valid
	return {
		success: true,
		isMock: true,
	};
}
