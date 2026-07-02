// Command registry for OpenMesh command palette
// Commands are created dynamically based on current store state

import type { Project, Settings, CommandPreset } from "../types";
import * as fileSystemAdapter from "./adapters/fileSystemAdapter";
import * as terminalAdapter from "./adapters/terminalAdapter";
import * as agentSessionAdapter from "./adapters/agentSessionAdapter";

export type CommandGroup =
  | "Project"
  | "Workspace"
  | "Agents"
  | "Dev"
  | "Notes"
  | "System";

export interface Command {
  id: string;
  group: CommandGroup;
  title: string;
  description: string;
  icon: string;
  shortcut?: string;
  available: boolean;
  disabledReason?: string;
  run: () => Promise<void>;
}

export interface CommandContext {
  currentProject: Project | null;
  projectPaths: string[];
  settings: Settings;
  commandPresets: CommandPreset[];
  addRecentItem: (item: {
    type: "project" | "folder" | "doc" | "task" | "session" | "note" | "artifact" | "terminal" | "agent_session" | "command_preset";
    title: string;
    projectId?: string;
    sourcePath?: string;
  }) => Promise<void>;
  openFolder: (path: string) => void;
  refreshGitStatus: () => Promise<void>;
  scanSessions: () => Promise<void>;
  createNote: () => Promise<void>;
  createSnapshot: () => Promise<void>;
  copyAgentContext: () => Promise<void>;
}

export function getCommands(ctx: CommandContext): Command[] {
  const commands: Command[] = [];
  const project = ctx.currentProject;
  const cwd = project?.terminalDir || project?.folderPath;

  // ─── Project Commands ───────────────────────────────────────────────
  if (project && cwd) {
    commands.push({
      id: "project-open-folder",
      group: "Project",
      title: "Open Project Folder",
      description: `Open ${project.name} in file explorer`,
      icon: "folder",
      available: true,
      async run() {
        await fileSystemAdapter.openFolder(project.folderPath);
        await ctx.addRecentItem({
          type: "folder",
          title: `Opened: ${project.name}`,
          projectId: project.id,
          sourcePath: project.folderPath,
        });
      },
    });

    commands.push({
      id: "project-open-terminal",
      group: "Project",
      title: "Open Terminal",
      description: `Open terminal in ${project.name}`,
      icon: "terminal",
      available: true,
      async run() {
        await terminalAdapter.openTerminal({ workingDir: cwd });
        await ctx.addRecentItem({
          type: "terminal",
          title: `Terminal: ${project.name}`,
          projectId: project.id,
          sourcePath: cwd,
        });
      },
    });

    commands.push({
      id: "project-refresh-git",
      group: "Project",
      title: "Refresh Git Status",
      description: "Reload git branch and changes",
      icon: "git",
      available: true,
      async run() {
        await ctx.refreshGitStatus();
        await ctx.addRecentItem({
          type: "command_preset",
          title: "Refreshed Git status",
          projectId: project.id,
        });
      },
    });
  } else {
    commands.push({
      id: "project-open-folder",
      group: "Project",
      title: "Open Project Folder",
      description: "No current project",
      icon: "folder",
      available: false,
      disabledReason: "No current project",
      run: async () => {},
    });
    commands.push({
      id: "project-open-terminal",
      group: "Project",
      title: "Open Terminal",
      description: "No current project",
      icon: "terminal",
      available: false,
      disabledReason: "No current project",
      run: async () => {},
    });
    commands.push({
      id: "project-refresh-git",
      group: "Project",
      title: "Refresh Git Status",
      description: "No current project",
      icon: "git",
      available: false,
      disabledReason: "No current project",
      run: async () => {},
    });
  }

  // ─── Agent Commands ─────────────────────────────────────────────────
  const agentClis = [
    {
      id: "agent-codex",
      title: "Launch Codex",
      icon: "zap",
      path: ctx.settings.agentClis?.codexPath,
      tool: "codex",
      label: "Codex",
    },
    {
      id: "agent-claude",
      title: "Launch Claude Code",
      icon: "bot",
      path: ctx.settings.agentClis?.claudeCodePath,
      tool: "claude-code",
      label: "Claude Code",
    },
    {
      id: "agent-opencode",
      title: "Launch OpenCode",
      icon: "code",
      path: ctx.settings.agentClis?.opencodePath,
      tool: "opencode",
      label: "OpenCode",
    },
  ];

  for (const agent of agentClis) {
    commands.push({
      id: agent.id,
      group: "Agents",
      title: agent.title,
      description: agent.path
        ? `Launch ${agent.label} in current project`
        : `${agent.label} CLI not configured`,
      icon: agent.icon,
      available: !!agent.path && !!project,
      disabledReason: !agent.path
        ? `${agent.label} CLI not configured`
        : !project
        ? "No current project"
        : undefined,
      async run() {
        if (!agent.path || !project) return;
        await terminalAdapter.openAgentCli(agent.tool, cwd!, agent.path);
        await ctx.addRecentItem({
          type: "agent_session",
          title: `${agent.label}: ${project.name}`,
          projectId: project.id,
          sourcePath: cwd,
        });
      },
    });
  }

  // Scan sessions
  const hasSessionDir = !!(
    (ctx.settings.sessionDirs?.codexEnabled &&
      ctx.settings.sessionDirs?.codexDir) ||
    (ctx.settings.sessionDirs?.claudeCodeEnabled &&
      ctx.settings.sessionDirs?.claudeCodeDir) ||
    (ctx.settings.sessionDirs?.opencodeEnabled &&
      ctx.settings.sessionDirs?.opencodeDir)
  );

  commands.push({
    id: "agent-scan-sessions",
    group: "Agents",
    title: "Scan Agent Sessions",
    description: hasSessionDir
      ? "Scan configured session directories"
      : "No session directory configured",
    icon: "scan",
    available: hasSessionDir,
    disabledReason: hasSessionDir ? undefined : "No session directory configured",
    async run() {
      await ctx.scanSessions();
      await ctx.addRecentItem({
        type: "agent_session",
        title: "Scanned agent sessions",
        sourcePath: "scan",
      });
    },
  });

  // ─── Dev Commands ───────────────────────────────────────────────────
  const presetCommands = [
    { name: "npm run dev", command: "npm", args: ["run", "dev"] },
    { name: "npm run build", command: "npm", args: ["run", "build"] },
    { name: "npm test", command: "npm", args: ["test"] },
    { name: "git status", command: "git", args: ["status"] },
  ];

  for (const preset of presetCommands) {
    commands.push({
      id: `dev-preset-${preset.name.replace(/\s+/g, "-")}`,
      group: "Dev",
      title: `Run: ${preset.name}`,
      description: project
        ? `Execute ${preset.name} in ${project.name}`
        : "No current project",
      icon: "play",
      available: !!project,
      disabledReason: project ? undefined : "No current project",
      async run() {
        if (!project) return;
        await terminalAdapter.runCommandPreset(
          preset.command,
          preset.args,
          cwd!,
        );
        await ctx.addRecentItem({
          type: "command_preset",
          title: `Preset: ${preset.name}`,
          projectId: project.id,
          sourcePath: cwd,
        });
      },
    });
  }

  commands.push({
    id: "dev-connector",
    group: "Dev",
    title: "Open Dev Connector",
    description: "Terminal launcher and command presets",
    icon: "terminal",
    available: true,
    async run() {
      ctx.openFolder("/dev-connector");
      await ctx.addRecentItem({
        type: "command_preset",
        title: "Opened Dev Connector",
        projectId: project?.id,
      });
    },
  });

  // ─── Workspace Commands ─────────────────────────────────────────────
  const workspacePages = [
    { id: "workspace-docs", title: "Open Docs", icon: "file-text", route: "/docs" },
    { id: "workspace-notes", title: "Open Notes", icon: "file-edit", route: "/notes" },
    { id: "workspace-sprint", title: "Open Sprint", icon: "list-todo", route: "/sprint" },
    {
      id: "workspace-sessions",
      title: "Open Agent Sessions",
      icon: "bot",
      route: "/agent-sessions",
    },
    { id: "workspace-settings", title: "Open Settings", icon: "settings", route: "/settings" },
  ];

  for (const page of workspacePages) {
    commands.push({
      id: page.id,
      group: "Workspace",
      title: page.title,
      description: `Navigate to ${page.title.replace("Open ", "")}`,
      icon: page.icon,
      available: true,
      async run() {
        ctx.openFolder(page.route);
      },
    });
  }

  // ── Notes Commands ─────────────────────────────────────────────────
  commands.push({
    id: "notes-create",
    group: "Notes",
    title: "Create New Note",
    description: project
      ? `Create a new note in ${project.name}`
      : "No current project",
    icon: "plus",
    available: !!project,
    disabledReason: project ? undefined : "No current project",
    async run() {
      await ctx.createNote();
      await ctx.addRecentItem({
        type: "note",
        title: "Created new note",
        projectId: project?.id,
      });
    },
  });

  commands.push({
    id: "notes-snapshot",
    group: "Notes",
    title: "Create Work Snapshot",
    description: project
      ? `Generate markdown snapshot of ${project.name} workspace`
      : "No current project",
    icon: "camera",
    available: !!project,
    disabledReason: project ? undefined : "No current project",
    async run() {
      await ctx.createSnapshot();
      await ctx.addRecentItem({
        type: "note",
        title: `Created work snapshot for ${project?.name || "project"}`,
        projectId: project?.id,
      });
    },
  });

  commands.push({
    id: "notes-copy-context",
    group: "Notes",
    title: "Copy Agent Context Prompt",
    description: project
      ? `Copy context prompt for ${project.name} to clipboard`
      : "No current project",
    icon: "copy",
    available: !!project,
    disabledReason: project ? undefined : "No current project",
    async run() {
      await ctx.copyAgentContext();
      await ctx.addRecentItem({
        type: "note",
        title: "Copied agent context prompt",
        projectId: project?.id,
      });
    },
  });

  // ── System Commands ────────────────────────────────────────────────
  commands.push({
    id: "system-settings",
    group: "System",
    title: "Open Settings",
    description: "Configure workspace, providers, and tools",
    icon: "settings",
    available: true,
    async run() {
      ctx.openFolder("/settings");
    },
  });

  return commands;
}

export const GROUP_ORDER: CommandGroup[] = [
  "Project",
  "Agents",
  "Dev",
  "Workspace",
  "Notes",
  "System",
];
