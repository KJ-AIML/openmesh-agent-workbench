import { beforeEach, describe, expect, it, vi } from "vitest";

const runAgentEngineTurn = vi.fn();
const listAgentRecipes = vi.fn();
const runAgentRecipe = vi.fn();
const applyAgentPatch = vi.fn();
const listMeshPeers = vi.fn();

vi.mock("../src/lib/agentEngineClient", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/lib/agentEngineClient")>();
  return {
    ...actual,
    runAgentEngineTurn: (...args: unknown[]) => runAgentEngineTurn(...args),
    listAgentRecipes: (...args: unknown[]) => listAgentRecipes(...args),
    runAgentRecipe: (...args: unknown[]) => runAgentRecipe(...args),
    applyAgentPatch: (...args: unknown[]) => applyAgentPatch(...args),
  };
});

vi.mock("../src/lib/continuityClient", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../src/lib/continuityClient")>();
  return {
    ...actual,
    listMeshPeers: (...args: unknown[]) => listMeshPeers(...args),
  };
});

import { runAgentChatTurn } from "../src/lib/agentChat/runner";

describe("runAgentChatTurn routing", () => {
  beforeEach(() => {
    runAgentEngineTurn.mockReset();
    listAgentRecipes.mockReset();
    runAgentRecipe.mockReset();
    applyAgentPatch.mockReset();
    listMeshPeers.mockReset();
    listMeshPeers.mockResolvedValue([{ label: "yo", peerId: "peer-1" }]);
    listAgentRecipes.mockResolvedValue([
      { id: "echo-hi", title: "echo", argv: ["echo", "hi"] },
    ]);
    runAgentRecipe.mockResolvedValue({
      ok: true,
      recipeId: "echo-hi",
      exitCode: 0,
      durationMs: 4,
      stdout: "",
      stderr: "",
      timedOut: false,
      cancelled: false,
      runId: "run-1",
    });
    runAgentEngineTurn.mockResolvedValue({
      assistantText: "ENGINE",
      toolSteps: [],
      iterations: 1,
      model: "test-model",
      provider: "test",
      refused: false,
      error: null,
      route: {
        transport: "direct-provider",
        providerLabel: "test",
        endpointKind: "provider-default",
        outcome: "completed",
      },
    });
  });

  it("sends ordinary language to Agent Engine", async () => {
    const result = await runAgentChatTurn("/tmp/proj", "Explain mesh");
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
    expect(runAgentEngineTurn.mock.calls[0][1]).toBe("Explain mesh");
    expect(runAgentEngineTurn.mock.calls[0][2]?.mode ?? "ask").toBe("ask");
    expect(result.assistantText).toBe("ENGINE");
    expect(result.toolCalls).toEqual([]);
  });

  it("does not execute /peers for a question about peers", async () => {
    await runAgentChatTurn("/tmp/proj", "Which peers are active?");
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
  });

  it("does not execute /verify for a verification question", async () => {
    await runAgentChatTurn("/tmp/proj", "Can you verify this approach?");
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
  });

  it("returns unknown-command without calling Agent Engine", async () => {
    const result = await runAgentChatTurn("/tmp/proj", "/does-not-exist");
    expect(runAgentEngineTurn).not.toHaveBeenCalled();
    expect(result.assistantText).toMatch(/Unknown command/);
    expect(result.toolCalls).toEqual([]);
  });

  it("voice skipLocalTools sends slash text to Agent Engine", async () => {
    const result = await runAgentChatTurn("/tmp/proj", "/peers", {
      skipLocalTools: true,
    });
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
    expect(runAgentEngineTurn.mock.calls[0][1]).toBe("/peers");
    expect(result.assistantText).toBe("ENGINE");
  });

  it("voice skipLocalTools sends ordinary transcripts to Agent Engine", async () => {
    await runAgentChatTurn("/tmp/proj", "Show me the sprint board", {
      skipLocalTools: true,
    });
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
  });

  it("executes /mesh and /peers locally", async () => {
    const mesh = await runAgentChatTurn("/tmp/proj", "/mesh");
    expect(runAgentEngineTurn).not.toHaveBeenCalled();
    expect(listMeshPeers).toHaveBeenCalledWith("/tmp/proj");
    expect(mesh.toolCalls[0]?.toolId).toBe("mesh_peers");
    expect(mesh.assistantText).toMatch(/yo/);

    listMeshPeers.mockClear();
    const peers = await runAgentChatTurn("/tmp/proj", "/peers");
    expect(listMeshPeers).toHaveBeenCalledTimes(1);
    expect(peers.toolCalls[0]?.toolId).toBe("mesh_peers");
  });

  it("executes /verify through the recipe path, not Agent Engine", async () => {
    const listed = await runAgentChatTurn("/tmp/proj", "/verify");
    expect(runAgentEngineTurn).not.toHaveBeenCalled();
    expect(listAgentRecipes).toHaveBeenCalled();
    expect(listed.toolCalls[0]?.toolId).toBe("verify");

    const ran = await runAgentChatTurn("/tmp/proj", "/verify echo-hi");
    expect(runAgentEngineTurn).not.toHaveBeenCalled();
    expect(runAgentRecipe).toHaveBeenCalledWith(
      "/tmp/proj",
      "echo-hi",
      "/tmp/proj:echo-hi",
    );
    expect(ran.toolCalls[0]?.toolId).toBe("verify");
    expect(applyAgentPatch).not.toHaveBeenCalled();
  });

  it("does not grant patch-apply via natural language", async () => {
    await runAgentChatTurn("/tmp/proj", "please apply the patch");
    expect(applyAgentPatch).not.toHaveBeenCalled();
    expect(runAgentEngineTurn).toHaveBeenCalledTimes(1);
  });
});
