import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

const mocks = vi.hoisted(() => ({
  getBuiltInProxyStatus: vi.fn(),
  getBuiltInProxyManagementConfig: vi.fn(),
  startBuiltInProxyFromSettings: vi.fn(),
  stopBuiltInProxy: vi.fn(),
}));

vi.mock("@/lib/builtinProxyClient", () => ({
  getBuiltInProxyStatus: mocks.getBuiltInProxyStatus,
  getBuiltInProxyManagementConfig: mocks.getBuiltInProxyManagementConfig,
  startBuiltInProxyFromSettings: mocks.startBuiltInProxyFromSettings,
  stopBuiltInProxy: mocks.stopBuiltInProxy,
  builtInProxyStatusLabel: (status: { running: boolean; error?: string | null }) =>
    status.error ? "Runtime error" : status.running ? "Running" : "Stopped",
}));

import OAuthPage from "@/pages/OAuthPage.vue";

const runningStatus = {
  ownership: "built-in",
  mode: "managed",
  running: true,
  bindHost: "127.0.0.1",
  port: 8317,
  endpoint: "http://127.0.0.1:8317",
  apiKeyConfigured: true,
  upstreamCount: 1,
  modelCount: 1,
  error: null,
};

const managementConfig = {
  bindHost: "127.0.0.1",
  port: 8317,
  allowUnauthenticated: false,
  requestTimeoutSecs: 120,
  routingStrategy: "round-robin",
  maxRetries: 2,
  apiKeyCount: 1,
  upstreamCount: 1,
  modelAliasCount: 0,
  upstreams: [{
    id: "gateway",
    baseUrl: "https://api.example.com/v1",
    protocol: "open-ai-compatible",
    apiKeyConfigured: true,
    enabled: true,
    priority: 0,
    accountId: null,
    models: [{ id: "model-a", ownedBy: "openmesh", capabilities: ["chat"] }],
  }],
  modelAliases: {},
  modelFallbacks: {},
};

describe("OAuthPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getBuiltInProxyStatus.mockResolvedValue(runningStatus);
    mocks.getBuiltInProxyManagementConfig.mockResolvedValue(managementConfig);
    mocks.stopBuiltInProxy.mockResolvedValue({ ...runningStatus, running: false, endpoint: null });
  });

  it("renders the local runtime and explicit native OAuth capability status", async () => {
    const wrapper = mount(OAuthPage, {
      global: { stubs: { RouterLink: { template: "<a><slot /></a>" } } },
    });
    await flushPromises();

    expect(wrapper.find('[data-testid="oauth-runtime-status"]').text()).toContain("Running");
    expect(wrapper.text()).toContain("OpenMesh-owned proxy");
    expect(wrapper.text()).toContain("OAuth capability map");
    expect(wrapper.text()).toContain("Native PKCE OAuth is available; device login is available from the CLI.");
    expect(wrapper.text()).toContain("GrokNative device authorization is available. Device login");
    expect(wrapper.text()).toContain("KimiNative device authorization is available. Device login");
    expect(wrapper.text()).toContain("Endpoint required");
    expect(wrapper.text()).not.toContain("CLIProxyAPI");
    expect(wrapper.text()).not.toContain("sidecar");
  });

  it("stops the built-in runtime without invoking an external process", async () => {
    const wrapper = mount(OAuthPage, {
      global: { stubs: { RouterLink: { template: "<a><slot /></a>" } } },
    });
    await flushPromises();
    await wrapper.get("button.btn-primary").trigger("click");

    expect(mocks.stopBuiltInProxy).toHaveBeenCalledOnce();
    expect(mocks.startBuiltInProxyFromSettings).not.toHaveBeenCalled();
  });
});
