// Git Adapter for Openmesh
// Abstracts git status reading
// Phase 4B: Real git status in Tauri using git2, mock in web

import type { GitStatus, AdapterResult } from "./types";
import { getRuntimeKind } from "./environment";
import { invoke } from "@tauri-apps/api/core";

interface GitStatusResult {
	success: boolean;
	is_repo: boolean;
	branch: string | null;
	dirty_count: number;
	staged_count: number;
	untracked_count: number;
	last_commit_hash: string | null;
	last_commit_message: string | null;
	error: string | null;
}

/**
 * Get git status for a repository
 * Phase 4B: Real git status in Tauri using git2, mock in web
 */
export async function getGitStatus(
	projectPath: string,
): Promise<AdapterResult<GitStatus>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		try {
			const result = await invoke<GitStatusResult>("get_git_status", {
				path: projectPath,
			});

			if (!result.success || !result.is_repo) {
				return {
					success: false,
					error: result.error || "Not a git repository",
					isMock: false,
					data: undefined,
				};
			}

			const isClean =
				result.dirty_count === 0 &&
				result.staged_count === 0 &&
				result.untracked_count === 0;

			return {
				success: true,
				data: {
					branch: result.branch || "HEAD",
					isClean,
					modifiedFiles: result.dirty_count,
					untrackedFiles: result.untracked_count,
					lastCommitHash: result.last_commit_hash || "",
					lastCommitMessage: result.last_commit_message || "",
				},
				isMock: false,
			};
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
				isMock: false,
				data: undefined,
			};
		}
	}

	// Web mock: clean status
	return {
		success: true,
		data: {
			branch: "main",
			isClean: true,
			modifiedFiles: 0,
			untrackedFiles: 0,
			lastCommitHash: "a1b2c3d",
			lastCommitMessage: "Initial commit",
		},
		isMock: true,
	};
}

/**
 * Get current branch name
 * Phase 4B: Real branch name in Tauri, mock in web
 */
export async function getCurrentBranch(
	_projectPath: string,
): Promise<AdapterResult<string>> {
	const runtime = getRuntimeKind();

	if (runtime === "tauri") {
		// Phase 4B: Not implemented yet - use getGitStatus instead
		return {
			success: false,
			error: "Use getGitStatus() instead",
			isMock: true,
			data: "main",
		};
	}

	// Web mock: main branch
	return {
		success: true,
		data: "main",
		isMock: true,
	};
}
