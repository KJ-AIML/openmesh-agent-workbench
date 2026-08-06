import { describe, expect, it } from "vitest";
import {
  STARTER_SLASHES,
  buildMentionMenuItems,
  buildSlashMenuItems,
  filterMentionMenuItems,
  filterSlashMenuItems,
  matchMentionToken,
  matchSlashToken,
  replaceToken,
} from "@/lib/agentChat/composerMenus";
import { AGENT_TOOLS } from "@/lib/agentChat/tools";
import { createSessionRun } from "@/lib/agentChat/sessionRuns";
import { createShellTab } from "@/lib/agentChat/shellTabs";

describe("composerMenus slash", () => {
  it("lists starters first then all real AGENT_TOOLS plus help", () => {
    const items = buildSlashMenuItems();
    const slashes = items.map((i) => i.slash);
    expect(slashes.slice(0, STARTER_SLASHES.length)).toEqual([...STARTER_SLASHES]);
    for (const t of AGENT_TOOLS) {
      expect(slashes).toContain(t.slash);
    }
    expect(slashes).toContain("/tools");
    expect(slashes).toContain("/help");
    // No Cursor-only marketplace fakes
    expect(slashes.join(" ")).not.toMatch(/browser|skills?/i);
  });

  it("each row has label + description", () => {
    for (const item of buildSlashMenuItems()) {
      expect(item.label.trim().length).toBeGreaterThan(0);
      expect(item.description.trim().length).toBeGreaterThan(0);
    }
  });

  it("filters by query", () => {
    const items = buildSlashMenuItems();
    const hits = filterSlashMenuItems(items, "/pil");
    expect(hits.some((h) => h.slash === "/pilot")).toBe(true);
    expect(hits.every((h) => /pil|readiness|pilot/i.test(`${h.slash} ${h.label} ${h.description}`))).toBe(
      true,
    );
  });

  it("matches trailing slash tokens", () => {
    expect(matchSlashToken("/")).toEqual({ start: 0, query: "/" });
    expect(matchSlashToken("/read")).toEqual({ start: 0, query: "/read" });
    expect(matchSlashToken("hello /ver")).toEqual({ start: 6, query: "/ver" });
    expect(matchSlashToken("nope")).toBeNull();
  });
});

describe("composerMenus mentions", () => {
  it("builds only supported context kinds", () => {
    const run = createSessionRun({
      id: "term:1",
      kind: "terminal",
      title: "Verify",
      command: "npm test",
    });
    const shell = createShellTab({ id: "shell-1", cwd: "/tmp/p", label: "zsh" });
    const items = buildMentionMenuItems({
      projectPath: "/tmp/p",
      projectName: "Demo",
      files: ["src/main.ts", "README.md"],
      docs: [{ name: "guide.md", path: "guide.md" }],
      notes: [{ name: "todo.md", path: "todo.md" }],
      terminalRuns: [run],
      shellTabs: [shell],
      showCanvas: true,
    });
    const kinds = new Set(items.map((i) => i.kind));
    expect(kinds.has("project")).toBe(true);
    expect(kinds.has("file")).toBe(true);
    expect(kinds.has("doc")).toBe(true);
    expect(kinds.has("note")).toBe(true);
    expect(kinds.has("terminal")).toBe(true);
    expect(kinds.has("shell")).toBe(true);
    expect(kinds.has("canvas")).toBe(true);
    expect(items.some((i) => i.insert === "/read src/main.ts")).toBe(true);
    expect(items.some((i) => /browser|branch|chats-from-cursor/i.test(i.label))).toBe(
      false,
    );
  });

  it("filters mentions and matches @ tokens", () => {
    const items = buildMentionMenuItems({
      projectPath: "/tmp/p",
      files: ["src/a.ts", "src/b.ts"],
      showCanvas: true,
    });
    const filtered = filterMentionMenuItems(items, "@src");
    expect(filtered.every((i) => /src/i.test(`${i.label} ${i.description}`))).toBe(
      true,
    );
    expect(matchMentionToken("@")).toEqual({ start: 0, query: "@" });
    expect(matchMentionToken("see @fi")).toEqual({ start: 4, query: "@fi" });
  });

  it("replaceToken swaps the active token", () => {
    expect(replaceToken("/pi", 0, 3, "/pilot ")).toBe("/pilot ");
    expect(replaceToken("x @a", 2, 4, "/read f")).toBe("x /read f");
  });
});
