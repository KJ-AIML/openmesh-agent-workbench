import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));

import {
  getOAuthConnectionStatus,
  oauthExclusionsForOpenModels,
  oauthOpenModelIds,
  parseOAuthConfig,
  parseOAuthConnection,
  parseOAuthModelDefinitions,
  parseOAuthStartResult,
  OAUTH_PROVIDERS,
  setOAuthManagementSecret,
} from "@/lib/oauthClient";

const config = {
  managementPort: 8317,
  endpoint: "http://127.0.0.1:8317/v0/management",
  dataPlaneEndpoint: "http://127.0.0.1:8317/v1",
  sidecarEnabled: true,
  secretConfigured: true,
  sidecarClientKeyConfigured: true,
};

describe("OpenMesh OAuth compatibility client", () => {
  beforeEach(() => vi.clearAllMocks());

  it("accepts the local compatibility status and preserves the loopback contract", () => {
    expect(parseOAuthConfig(config)).toEqual(config);
    expect(parseOAuthConnection({
      ...config,
      dataPlaneStatus: "ready",
      dataPlaneError: null,
      status: "ready",
      error: null,
      providers: [{
        provider: "openai",
        total: 1,
        enabled: 1,
        disabled: 0,
        unavailable: 0,
        runtimeOnly: 0,
      }],
    }).providers[0].provider).toBe("openai");
  });

  it("rejects non-loopback compatibility endpoints", () => {
    expect(() => parseOAuthConfig({ ...config, endpoint: "https://remote.example/v0/management" })).toThrow();
  });

  it("uses the local Tauri status command and never accepts a native secret write", async () => {
    mocks.invoke.mockResolvedValue({
      ...config,
      dataPlaneStatus: "ready",
      dataPlaneError: null,
      status: "ready",
      error: null,
      providers: [],
    });
    await expect(getOAuthConnectionStatus()).resolves.toMatchObject({ status: "ready" });
    expect(mocks.invoke).toHaveBeenCalledWith("oauth_connection_status");
    await expect(setOAuthManagementSecret("secret")).rejects.toThrow("not available yet");
  });

  it("keeps model availability helpers deterministic for local provider catalogs", () => {
    const models = parseOAuthModelDefinitions([
      { id: "model-a", displayName: "Model A" },
      { id: "model-b" },
    ]);
    expect(oauthOpenModelIds(models, ["model-b"])).toEqual(new Set(["model-a"]));
    expect(oauthExclusionsForOpenModels([], models, ["model-a"])).toEqual(["model-b"]);
  });

  it("accepts device-flow metadata and exposes only the implemented native adapters", () => {
    expect(parseOAuthStartResult({
      url: "https://auth.example/device",
      state: "device-state",
      flow: "device",
      userCode: "ABCD-EFGH",
      verificationUri: "https://auth.example/device",
      pollIntervalSeconds: 5,
    })).toMatchObject({ flow: "device", userCode: "ABCD-EFGH", pollIntervalSeconds: 5 });
    expect(OAUTH_PROVIDERS.find((provider) => provider.id === "grok")).toMatchObject({
      callbackSupported: false,
      deviceSupported: true,
    });
    expect(OAUTH_PROVIDERS.find((provider) => provider.id === "kimi")).toMatchObject({
      callbackSupported: false,
      deviceSupported: true,
    });
    expect(OAUTH_PROVIDERS.find((provider) => provider.id === "gemini")).toMatchObject({
      callbackSupported: false,
      deviceSupported: false,
    });
  });
});
