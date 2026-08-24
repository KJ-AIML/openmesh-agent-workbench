import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import {
  getBuiltInProxyStatus,
  parseBuiltInProxyStatus,
  startBuiltInProxyFromSettings,
  stopBuiltInProxy,
} from "@/lib/builtinProxyClient";

const status = {
  ownership: "built-in",
  mode: "managed",
  running: false,
  bindHost: "127.0.0.1",
  port: 8317,
  endpoint: null,
  apiKeyConfigured: true,
  upstreamCount: 1,
  modelCount: 2,
  error: null,
};

describe("built-in proxy client", () => {
  beforeEach(() => vi.clearAllMocks());

  it("parses safe lifecycle status", () => {
    expect(parseBuiltInProxyStatus(status)).toEqual(status);
  });

  it("rejects a runtime that claims external ownership", () => {
    expect(() => parseBuiltInProxyStatus({ ...status, ownership: "external-sidecar" })).toThrow(
      "OpenMesh built-in proxy status is invalid.",
    );
  });

  it("uses the built-in Tauri lifecycle commands", async () => {
    mocks.invoke
      .mockResolvedValueOnce(status)
      .mockResolvedValueOnce({ ...status, running: true, endpoint: "http://127.0.0.1:8317" })
      .mockResolvedValueOnce(status);

    await getBuiltInProxyStatus();
    await startBuiltInProxyFromSettings();
    await stopBuiltInProxy();

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, "proxy_runtime_status");
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, "proxy_runtime_start_default");
    expect(mocks.invoke).toHaveBeenNthCalledWith(3, "proxy_runtime_stop");
  });
});
