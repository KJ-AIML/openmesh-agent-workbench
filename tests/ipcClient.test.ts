import { afterEach, describe, expect, it, vi } from "vitest";
import {
  IpcError,
  TYPED_COMMAND_NAMES,
  invokeTyped,
  legacyInvoke,
  setIpcTransport,
} from "../src/lib/ipc";

describe("typed IPC client", () => {
  afterEach(() => {
    setIpcTransport(null);
  });

  it("invokes the catalog command name for an agent turn", async () => {
    const invoke = vi.fn(async () => ({ assistantText: "ok", toolSteps: [] }));
    setIpcTransport({ invoke });
    await invokeTyped("agent_engine_turn", {
      projectPath: "/tmp/proj",
      request: { question: "hi", messages: [], mode: "ask" },
    });
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith(
      "agent_engine_turn",
      expect.objectContaining({
        projectPath: "/tmp/proj",
        request: expect.objectContaining({ question: "hi", mode: "ask" }),
      }),
    );
  });

  it("rejects legacyInvoke for a typed command", async () => {
    await expect(legacyInvoke("agent_patch_apply", { projectPath: "/x", patchId: "p" })).rejects.toMatchObject({
      name: "IpcError",
      code: "contract_conflict",
    });
  });

  it("allows legacyInvoke for an unmigrated command", async () => {
    const invoke = vi.fn(async () => []);
    setIpcTransport({ invoke });
    await legacyInvoke("list_docs", { projectPath: "/tmp" });
    expect(invoke).toHaveBeenCalledWith("list_docs", { projectPath: "/tmp" });
  });

  it("normalizes transport failures without treating them as success", async () => {
    setIpcTransport({
      invoke: async () => {
        throw new Error("API key not configured");
      },
    });
    await expect(invokeTyped("agent_secret_status")).rejects.toBeInstanceOf(IpcError);
    await expect(invokeTyped("agent_secret_status")).rejects.toMatchObject({
      code: "missing_credential",
    });
  });

  it("has unique typed command identities", () => {
    expect(new Set(TYPED_COMMAND_NAMES).size).toBe(TYPED_COMMAND_NAMES.length);
  });

  it("keeps typed command identity closed", () => {
    expect(TYPED_COMMAND_NAMES.includes("agent_engine_turn")).toBe(true);
    expect(
      (TYPED_COMMAND_NAMES as readonly string[]).includes("not_a_real_command"),
    ).toBe(false);
  });
});
