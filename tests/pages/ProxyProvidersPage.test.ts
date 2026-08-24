import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

const mocks = vi.hoisted(() => ({
  getBuiltInProxyManagementConfig: vi.fn(),
  updateBuiltInProxyManagementConfig: vi.fn(),
  confirm: vi.fn(() => true),
}));

vi.mock("@/lib/builtinProxyClient", () => ({
  getBuiltInProxyManagementConfig: mocks.getBuiltInProxyManagementConfig,
  updateBuiltInProxyManagementConfig: mocks.updateBuiltInProxyManagementConfig,
}));

import ProxyProvidersPage from "@/pages/ProxyProvidersPage.vue";

const makeConfig = () => ({
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
    priority: 10,
    accountId: "account-a",
    models: [{ id: "model-a", ownedBy: "openmesh", capabilities: ["chat"] }],
  }],
  modelAliases: {},
  modelFallbacks: {},
});

describe("ProxyProvidersPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.confirm.mockReset();
    mocks.confirm.mockReturnValue(true);
    mocks.getBuiltInProxyManagementConfig.mockResolvedValue(makeConfig());
    mocks.updateBuiltInProxyManagementConfig.mockResolvedValue(makeConfig());
    vi.stubGlobal("confirm", mocks.confirm);
  });

  it("renders a safe OpenMesh-owned inventory", async () => {
    const wrapper = mount(ProxyProvidersPage);
    await flushPromises();
    expect(wrapper.text()).toContain("gateway");
    expect(wrapper.text()).toContain("Key configured");
    expect(wrapper.text()).not.toContain("CLIProxyAPI");
    expect(wrapper.text()).not.toContain("write-only-key");
  });

  it("adds an upstream through the local management command", async () => {
    const wrapper = mount(ProxyProvidersPage);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text().includes("Add upstream"))!.trigger("click");
    const inputs = wrapper.findAll("input");
    await inputs[0].setValue("new-gateway");
    await inputs[1].setValue("https://new.example.com/v1");
    await wrapper.get('input[placeholder="Never displayed after save"]').setValue("new-key");
    await wrapper.get(".proxy-providers-page__models").setValue("new-model");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();
    expect(mocks.updateBuiltInProxyManagementConfig).toHaveBeenCalledWith(expect.objectContaining({
      upstreams: expect.arrayContaining([expect.objectContaining({ id: "new-gateway", apiKey: "new-key" })]),
    }));
    expect(wrapper.find("form").exists()).toBe(false);
  });

  it("preserves an empty edit key and updates routing locally", async () => {
    const wrapper = mount(ProxyProvidersPage);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text() === "Edit")!.trigger("click");
    await wrapper.get("form").trigger("submit.prevent");
    await flushPromises();
    const update = mocks.updateBuiltInProxyManagementConfig.mock.calls[0][0];
    expect(update.upstreams[0]).not.toHaveProperty("apiKey");

    const select = wrapper.get("select");
    await select.setValue("fill-first");
    await flushPromises();
    expect(mocks.updateBuiltInProxyManagementConfig).toHaveBeenCalledWith({ routingStrategy: "fill-first" });
  });

  it("confirms disable and remove operations", async () => {
    const wrapper = mount(ProxyProvidersPage);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text() === "Disable")!.trigger("click");
    await flushPromises();
    expect(mocks.updateBuiltInProxyManagementConfig).toHaveBeenCalledWith(expect.objectContaining({
      upstreams: [expect.objectContaining({ id: "gateway", enabled: false })],
    }));
    await wrapper.findAll("button").find((button) => button.text().includes("Remove"))!.trigger("click");
    await flushPromises();
    expect(mocks.updateBuiltInProxyManagementConfig).toHaveBeenCalledWith({ upstreams: [] });
    expect(mocks.confirm).toHaveBeenCalled();
  });

  it("does not mutate when confirmation is declined", async () => {
    mocks.confirm.mockReturnValue(false);
    const wrapper = mount(ProxyProvidersPage);
    await flushPromises();
    await wrapper.findAll("button").find((button) => button.text() === "Disable")!.trigger("click");
    await flushPromises();
    expect(mocks.updateBuiltInProxyManagementConfig).not.toHaveBeenCalled();
  });
});
