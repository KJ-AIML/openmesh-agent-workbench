import { describe, it, expect, vi } from "vitest";
import { getCommands, type CommandContext } from "@/lib/commands";

const baseCtx: CommandContext = {
  currentProject: { id: "p1", name: "Test", folderPath: "/tmp/test" } as any,
  projectPaths: [],
  settings: {
    workspace: { theme: "dark" },
    provider: { apiKeyConfigured: false, usageTrackingEnabled: false },
    models: { localModelEnabled: false },
    server: {
      mode: "local",
      apiBaseUrl: "http://localhost:3000",
      healthStatus: "unknown",
      syncStatus: "unknown",
    },
    agentClis: {},
    sessionDirs: {
      codexEnabled: false,
      claudeCodeEnabled: false,
      opencodeEnabled: false,
    },
    localPaths: {},
    appearance: { theme: "dark", fontSize: "medium" },
  } as any,
  commandPresets: [],
  addRecentItem: vi.fn(),
  openFolder: vi.fn(),
  refreshGitStatus: vi.fn(),
  scanSessions: vi.fn(),
  createNote: vi.fn(),
  createSnapshot: vi.fn(),
  copyAgentContext: vi.fn(),
  launchAgentWithContext: vi.fn(),
};

describe("CommandPalette", () => {
  it("includes Search Context action in Workspace group", () => {
    const commands = getCommands(baseCtx);
    const searchContext = commands.find((c) => c.id === "workspace-context");
    expect(searchContext).toBeDefined();
    expect(searchContext?.title).toBe("Search Context");
    expect(searchContext?.group).toBe("Workspace");
  });

  it("Search Context command is always available (no project required)", () => {
    const commandsNoProject = getCommands({ ...baseCtx, currentProject: null });
    const searchContext = commandsNoProject.find((c) => c.id === "workspace-context");
    expect(searchContext).toBeDefined();
    expect(searchContext?.available).toBe(true);
  });

  it("Search Context command is discoverable by title", () => {
    const commands = getCommands(baseCtx);
    const titles = commands.map((c) => c.title);
    expect(titles).toContain("Search Context");
  });

  it("Search Context calls openFolder with /context route when executed", async () => {
    const openFolder = vi.fn();
    const commands = getCommands({ ...baseCtx, openFolder });
    const searchContext = commands.find((c) => c.id === "workspace-context");
    expect(searchContext).toBeDefined();
    await searchContext!.run();
    expect(openFolder).toHaveBeenCalledWith("/context?focus=search");
  });

  it("Search Context uses the Search icon", () => {
    const commands = getCommands(baseCtx);
    const searchContext = commands.find((c) => c.id === "workspace-context");
    expect(searchContext?.icon).toBe("search");
  });

  it("Search Context is placed before Settings in the Workspace group", () => {
    const commands = getCommands(baseCtx);
    const workspaceCommands = commands.filter((c) => c.group === "Workspace");
    const searchIdx = workspaceCommands.findIndex((c) => c.id === "workspace-context");
    const settingsIdx = workspaceCommands.findIndex((c) => c.id === "workspace-settings");
    expect(searchIdx).toBeGreaterThanOrEqual(0);
    expect(searchIdx).toBeLessThan(settingsIdx);
  });
});
