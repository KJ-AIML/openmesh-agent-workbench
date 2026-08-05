import { describe, expect, it } from "vitest";
import {
  resolveToolsForMessage,
  listToolsHelp,
  isToolsHelpText,
  summarizeToolsHelp,
} from "../src/lib/agentChat/tools";
import { __test_resolve, __test_toolIds } from "../src/lib/agentChat/runner";

describe("agent chat tool routing", () => {
  it("exposes a stable tool inventory", () => {
    const ids = __test_toolIds();
    expect(ids).toContain("pilot_check");
    expect(ids).toContain("rc_check");
    expect(ids).toContain("team_status");
    expect(ids.length).toBeGreaterThanOrEqual(10);
  });

  it("routes slash commands", () => {
    expect(resolveToolsForMessage("/pilot").map((t) => t.id)).toEqual([
      "pilot_check",
    ]);
    expect(resolveToolsForMessage("/rc").map((t) => t.id)).toEqual(["rc_check"]);
    expect(resolveToolsForMessage("/team").map((t) => t.id)).toEqual([
      "team_status",
    ]);
    expect(resolveToolsForMessage("/search auth").map((t) => t.id)).toEqual([
      "context_search",
    ]);
    expect(resolveToolsForMessage("/git").map((t) => t.id)).toEqual([
      "git_status",
    ]);
    expect(resolveToolsForMessage("/read src/main.rs").map((t) => t.id)).toEqual([
      "read_file",
    ]);
    expect(resolveToolsForMessage("/grep openmesh").map((t) => t.id)).toEqual([
      "grep",
    ]);
    expect(resolveToolsForMessage("/diff --staged").map((t) => t.id)).toEqual([
      "git_diff",
    ]);
    expect(resolveToolsForMessage("/ls src").map((t) => t.id)).toEqual([
      "list_dir",
    ]);
    expect(resolveToolsForMessage("/patch show x").map((t) => t.id)).toEqual([
      "patch",
    ]);
    expect(resolveToolsForMessage("/verify list").map((t) => t.id)).toEqual([
      "verify",
    ]);
    expect(resolveToolsForMessage("/delegate codex").map((t) => t.id)).toEqual([
      "delegate",
    ]);
    expect(resolveToolsForMessage("/continue pending").map((t) => t.id)).toEqual([
      "continue",
    ]);
  });

  it("routes plain-language keywords", () => {
    expect(__test_resolve("please check pilot readiness")).toContain(
      "pilot_check",
    );
    expect(__test_resolve("show org graph")).toContain("org_graph");
  });

  it("prefers /continue for handoff continuity keywords", () => {
    expect(resolveToolsForMessage("create handoff for teammate").map((t) => t.id)[0]).toBe(
      "continue",
    );
    expect(resolveToolsForMessage("please continue from pending work").map((t) => t.id)).toContain(
      "continue",
    );
    expect(__test_resolve("create handoff")).toEqual(
      expect.arrayContaining(["continue"]),
    );
  });

  it("help text lists slash commands", () => {
    const help = listToolsHelp();
    expect(help).toContain("/pilot");
    expect(help).toContain("/rc");
    expect(help).toContain("/read");
    expect(help).toContain("/grep");
    expect(help).toContain("/diff");
    expect(help).toContain("/ls");
    expect(help).toContain("/patch");
    expect(help).toContain("/verify");
    expect(help).toContain("/delegate");
    expect(help).toContain("/continue");
    expect(isToolsHelpText(help)).toBe(true);
    expect(summarizeToolsHelp(help)).toMatch(/^\d+ tools ·/);
  });
});
