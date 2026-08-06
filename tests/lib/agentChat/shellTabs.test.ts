import { describe, expect, it } from "vitest";
import {
  createShellTab,
  defaultShellLabel,
  removeShellTab,
  resolveTerminalCwd,
  shortCwdLabel,
  upsertShellTab,
} from "@/lib/agentChat/shellTabs";

describe("shellTabs", () => {
  it("resolves project cwd first", () => {
    expect(resolveTerminalCwd("/tmp/proj")).toBe("/tmp/proj");
    expect(resolveTerminalCwd("  /tmp/proj  ")).toBe("/tmp/proj");
  });

  it("falls back to HOME when no project", () => {
    const home = process.env.HOME || process.env.USERPROFILE;
    if (!home) return;
    expect(resolveTerminalCwd(null)).toBe(home);
    expect(resolveTerminalCwd("")).toBe(home);
  });

  it("creates and upserts tabs", () => {
    const a = createShellTab({ cwd: "/tmp/a", label: "zsh" });
    expect(a.external).toBeUndefined();
    expect(a.status).toBe("launching");
    expect(a.label).toBe("zsh");
    const b = { ...a, status: "open" as const };
    const tabs = upsertShellTab([a], b);
    expect(tabs).toHaveLength(1);
    expect(tabs[0]!.status).toBe("open");
  });

  it("removes a tab and picks a neighbor", () => {
    const a = createShellTab({ id: "a", cwd: "/a" });
    const b = createShellTab({ id: "b", cwd: "/b" });
    const c = createShellTab({ id: "c", cwd: "/c" });
    const { tabs, nextActiveId } = removeShellTab([a, b, c], "b");
    expect(tabs.map((t) => t.id)).toEqual(["a", "c"]);
    expect(nextActiveId).toBe("c");
  });

  it("shortens cwd labels", () => {
    expect(shortCwdLabel("")).toBe("(no cwd)");
    expect(shortCwdLabel("/short")).toBe("/short");
    expect(shortCwdLabel("/very/long/path/that/exceeds/limit", 12).startsWith("…")).toBe(
      true,
    );
  });

  it("exposes a default shell label", () => {
    expect(typeof defaultShellLabel()).toBe("string");
    expect(defaultShellLabel().length).toBeGreaterThan(0);
  });
});
