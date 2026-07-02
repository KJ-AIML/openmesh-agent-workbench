// Work Snapshot and Agent Context Prompt generators
// Rule-based markdown generation — no AI summarization

import type { Project, Settings, Task, Sprint, RecentItem, CommandPreset, AgentSession } from "../types";
import type { GitStatus } from "./adapters/types";

export interface SnapshotContext {
  project: Project | null;
  settings: Settings;
  gitStatus: GitStatus | null;
  recentItems: RecentItem[];
  tasks: Task[];
  sprint: Sprint | null;
  sessions: AgentSession[];
  presets: CommandPreset[];
}

/**
 * Generate work snapshot markdown content
 */
export function generateSnapshotMarkdown(ctx: SnapshotContext): string {
  const lines: string[] = [];
  const now = new Date();
  const timestamp = now.toISOString();

  lines.push(`# Work Snapshot — ${ctx.project?.name || "Unknown Project"}`);
  lines.push("");
  lines.push(`Created: ${timestamp}`);
  lines.push("");

  // Project section
  lines.push("## Project");
  if (ctx.project) {
    lines.push(`- Name: ${ctx.project.name}`);
    lines.push(`- Path: ${ctx.project.folderPath}`);
    if (ctx.project.repoUrl) lines.push(`- Repo: ${ctx.project.repoUrl}`);
    if (ctx.project.defaultBranch) lines.push(`- Default branch: ${ctx.project.defaultBranch}`);
  } else {
    lines.push("- No project selected");
  }
  lines.push("");

  // Git section
  lines.push("## Git");
  if (ctx.gitStatus) {
    lines.push(`- Branch: ${ctx.gitStatus.branch || "unknown"}`);
    lines.push(`- Status: ${ctx.gitStatus.isClean ? "Clean" : "Modified"}`);
    if (!ctx.gitStatus.isClean) {
      lines.push(`- Modified: ${ctx.gitStatus.modifiedFiles}`);
      lines.push(`- Untracked: ${ctx.gitStatus.untrackedFiles}`);
    }
    if (ctx.gitStatus.lastCommitHash) {
      lines.push(`- Last commit: ${ctx.gitStatus.lastCommitHash.slice(0, 7)} — ${ctx.gitStatus.lastCommitMessage || "No message"}`);
    }
  } else {
    lines.push("- Git status unavailable");
  }
  lines.push("");

  // Recent Work section
  lines.push("## Recent Work");
  if (ctx.recentItems.length > 0) {
    for (const item of ctx.recentItems.slice(0, 10)) {
      const timeAgo = getTimeAgo(item.lastOpenedAt);
      lines.push(`- [${timeAgo}] ${item.title} (${item.type})`);
    }
  } else {
    lines.push("- No recent work yet");
  }
  lines.push("");

  // Sprint / Tasks section
  lines.push("## Sprint / Tasks");
  if (ctx.sprint) {
    lines.push(`- Sprint: ${ctx.sprint.name} (${ctx.sprint.status})`);
  } else {
    lines.push("- No sprint configured");
  }
  if (ctx.tasks.length > 0) {
    const activeTasks = ctx.tasks.filter((t) => t.status !== "completed").slice(0, 5);
    if (activeTasks.length > 0) {
      lines.push("- Active tasks:");
      for (const task of activeTasks) {
        lines.push(`  - [${task.status}] ${task.title} (${task.priority})`);
      }
    } else {
      lines.push("- All tasks completed");
    }
  } else {
    lines.push("- No tasks");
  }
  lines.push("");

  // Agent Sessions section
  lines.push("## Agent Sessions");
  if (ctx.sessions.length > 0) {
    for (const session of ctx.sessions.slice(0, 5)) {
      const timeAgo = getTimeAgo(session.lastActiveAt);
      lines.push(`- [${timeAgo}] ${session.title} (${session.tool}) — ${session.status}`);
    }
  } else {
    lines.push("- No sessions yet");
  }
  lines.push("");

  // Commands / Presets section
  lines.push("## Commands / Presets");
  if (ctx.presets.length > 0) {
    for (const preset of ctx.presets) {
      lines.push(`- ${preset.name}: ${preset.command} ${preset.args.join(" ")} [${preset.riskLevel}]`);
    }
  } else {
    lines.push("- No command presets configured");
  }
  lines.push("");

  // Notes section
  lines.push("## Notes");
  lines.push("- Check .openmesh/notes/ for project notes");
  lines.push("");

  // Suggested Next Actions
  lines.push("## Suggested Next Actions");
  lines.push("- Open terminal in project folder");
  lines.push("- Launch agent CLI (Codex / Claude Code / OpenCode)");
  if (ctx.tasks.length > 0) {
    const nextTask = ctx.tasks.find((t) => t.status === "in-progress" || t.status === "pending");
    if (nextTask) {
      lines.push(`- Continue current task: ${nextTask.title}`);
    }
  }
  lines.push("- Scan agent sessions for recent context");
  lines.push("");

  return lines.join("\n");
}

/**
 * Generate agent context prompt for Codex / Claude / OpenCode
 */
export function generateAgentContextPrompt(ctx: SnapshotContext): string {
  const lines: string[] = [];

  lines.push("You are working in a local AI agent workbench.");
  lines.push("");

  // Project info
  lines.push("## Project Context");
  if (ctx.project) {
    lines.push(`- Name: ${ctx.project.name}`);
    lines.push(`- Path: ${ctx.project.folderPath}`);
  } else {
    lines.push("- No project selected");
  }
  lines.push("");

  // Current goal
  lines.push("## Current Goal");
  if (ctx.sprint && ctx.tasks.length > 0) {
    const activeTask = ctx.tasks.find((t) => t.status === "in-progress");
    if (activeTask) {
      lines.push(`- Sprint: ${ctx.sprint.name}`);
      lines.push(`- Active task: ${activeTask.title}`);
      if (activeTask.description) {
        lines.push(`- Description: ${activeTask.description}`);
      }
    } else {
      lines.push(`- Sprint: ${ctx.sprint.name} (no active task)`);
    }
  } else {
    lines.push("- No sprint or tasks configured");
  }
  lines.push("");

  // Git status
  lines.push("## Git Status");
  if (ctx.gitStatus) {
    lines.push(`- Branch: ${ctx.gitStatus.branch || "unknown"}`);
    lines.push(`- Status: ${ctx.gitStatus.isClean ? "Clean" : "Modified"}`);
    if (!ctx.gitStatus.isClean) {
      lines.push(`- Changes: ${ctx.gitStatus.modifiedFiles} modified, ${ctx.gitStatus.untrackedFiles} untracked`);
    }
  } else {
    lines.push("- Git status unavailable");
  }
  lines.push("");

  // Recent work
  lines.push("## Recent Work");
  if (ctx.recentItems.length > 0) {
    for (const item of ctx.recentItems.slice(0, 5)) {
      lines.push(`- ${item.title}`);
    }
  } else {
    lines.push("- No recent work");
  }
  lines.push("");

  // Agent/session context
  lines.push("## Agent/Session Context");
  if (ctx.sessions.length > 0) {
    for (const session of ctx.sessions.slice(0, 3)) {
      lines.push(`- ${session.title} (${session.tool}) — ${session.status}`);
      if (session.summary) {
        lines.push(`  Summary: ${session.summary}`);
      }
    }
  } else {
    lines.push("- No recent sessions");
  }
  lines.push("");

  // Instruction
  lines.push("## Instructions");
  lines.push("Please inspect the repository, infer the current state, and continue from this context.");
  lines.push("Ask before taking destructive actions.");
  lines.push("");

  return lines.join("\n");
}

/**
 * Generate snapshot filename with timestamp
 */
export function generateSnapshotFilename(): string {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, "0");
  const day = String(now.getDate()).padStart(2, "0");
  const hour = String(now.getHours()).padStart(2, "0");
  const minute = String(now.getMinutes()).padStart(2, "0");
  return `snapshot-${year}-${month}-${day}-${hour}-${minute}.md`;
}

/**
 * Simple time-ago formatter
 */
function getTimeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  const days = Math.floor(hrs / 24);
  return `${days}d ago`;
}
