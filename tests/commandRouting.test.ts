import { describe, expect, it } from "vitest";
import {
  isLocalChatCommand,
  parseChatInput,
  tokenizeCommandArgs,
  unknownCommandMessage,
} from "../src/lib/agentChat/commandRouting";
import { resolveToolsForMessage } from "../src/lib/agentChat/tools";
import { __test_resolve } from "../src/lib/agentChat/runner";

describe("parseChatInput", () => {
  it("treats /mesh and /peers as the mesh peers command", () => {
    const mesh = parseChatInput("/mesh");
    const peers = parseChatInput("/peers");
    expect(mesh.kind).toBe("command");
    expect(peers.kind).toBe("command");
    if (mesh.kind === "command" && mesh.tool !== "help") {
      expect(mesh.tool.id).toBe("mesh_peers");
      expect(mesh.name).toBe("peers");
      expect(mesh.args).toEqual([]);
    }
    if (peers.kind === "command" && peers.tool !== "help") {
      expect(peers.tool.id).toBe("mesh_peers");
    }
  });

  it("parses /verify arguments without shell grammar", () => {
    const parsed = parseChatInput("/verify backend");
    expect(parsed.kind).toBe("command");
    if (parsed.kind === "command" && parsed.tool !== "help") {
      expect(parsed.tool.id).toBe("verify");
      expect(parsed.args).toEqual(["backend"]);
    }
    expect(tokenizeCommandArgs("cargo-test-core extra")).toEqual([
      "cargo-test-core",
      "extra",
    ]);
  });

  it("sends ordinary language to the agent", () => {
    for (const text of [
      "Explain mesh",
      "Which peers are active?",
      "Can you verify this approach?",
      "show me the mesh",
      "please check pilot readiness",
      "create handoff for teammate",
      "help",
      "what can you do",
    ]) {
      expect(parseChatInput(text).kind).toBe("agent");
      expect(resolveToolsForMessage(text)).toEqual([]);
      expect(__test_resolve(text)).toEqual([]);
      expect(isLocalChatCommand(text)).toBe(false);
    }
  });

  it("does not let mid-sentence keywords steal the turn", () => {
    const text = "Before we ship, verify the mesh peers and git status please.";
    expect(parseChatInput(text)).toEqual({ kind: "agent", text });
    expect(resolveToolsForMessage(text)).toEqual([]);
  });

  it("treats unknown slash input as an explicit error namespace", () => {
    const parsed = parseChatInput("/does-not-exist");
    expect(parsed.kind).toBe("unknown-command");
    if (parsed.kind === "unknown-command") {
      expect(parsed.name).toBe("does-not-exist");
      expect(unknownCommandMessage(parsed.name)).toMatch(/Unknown command/);
      expect(unknownCommandMessage(parsed.name)).toContain("/peers");
      expect(unknownCommandMessage(parsed.name)).toContain("/verify");
    }
    expect(isLocalChatCommand("/does-not-exist")).toBe(true);
    expect(resolveToolsForMessage("/does-not-exist")).toEqual([]);
  });

  it("leaves @ mentions as agent text, not commands", () => {
    expect(parseChatInput("@docs/readme.md").kind).toBe("agent");
    expect(parseChatInput("see @file src/main.rs").kind).toBe("agent");
  });

  it("does not treat /patch-free prose as patch apply", () => {
    expect(parseChatInput("please apply this patch now").kind).toBe("agent");
    const cmd = parseChatInput("/patch apply patch-1");
    expect(cmd.kind).toBe("command");
    if (cmd.kind === "command") {
      expect(cmd.args).toEqual(["apply", "patch-1"]);
    }
  });
});
