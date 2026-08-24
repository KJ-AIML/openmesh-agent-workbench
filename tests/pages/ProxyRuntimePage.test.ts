import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";

const mocks = vi.hoisted(() => ({
  getBuiltInProxyStatus: vi.fn(),
  startBuiltInProxyFromSettings: vi.fn(),
  stopBuiltInProxy: vi.fn(),
}));

vi.mock("@/lib/builtinProxyClient", () => ({
  getBuiltInProxyStatus: mocks.getBuiltInProxyStatus,
  startBuiltInProxyFromSettings: mocks.startBuiltInProxyFromSettings,
  stopBuiltInProxy: mocks.stopBuiltInProxy,
  builtInProxyStatusLabel: (status: { running: boolean; error?: string | null }) =>
    status.error ? "Runtime error" : status.running ? "Running" : "Stopped",
}));

import ProxyRuntimePage from "@/pages/ProxyRuntimePage.vue";

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

describe("ProxyRuntimePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getBuiltInProxyStatus.mockResolvedValue(runningStatus);
    mocks.stopBuiltInProxy.mockResolvedValue({ ...runningStatus, running: false, endpoint: null });
  });

  it("renders the OpenMesh-owned runtime contract", async () => {
    const wrapper = mount(ProxyRuntimePage, {
      global: {
        stubs: { RouterLink: { template: "<a><slot /></a>" } },
      },
    });
    await flushPromises();

    expect(wrapper.find('[data-testid="proxy-runtime-status"]').text()).toContain("Running");
    expect(wrapper.text()).toContain("built-in");
    expect(wrapper.text()).toContain("managed");
    expect(wrapper.text()).toContain("OpenMesh-owned server");
    expect(wrapper.text()).toContain("Chat completions");
    expect(wrapper.text()).not.toContain("sidecar");
    expect(wrapper.text()).not.toContain("CLIProxyAPI");
  });

  it("stops the built-in runtime through the lifecycle command", async () => {
    const wrapper = mount(ProxyRuntimePage, {
      global: {
        stubs: { RouterLink: { template: "<a><slot /></a>" } },
      },
    });
    await flushPromises();
    await wrapper.get("button.btn-primary").trigger("click");

    expect(mocks.stopBuiltInProxy).toHaveBeenCalledOnce();
    expect(mocks.startBuiltInProxyFromSettings).not.toHaveBeenCalled();
  });

  it("starts a stopped runtime from saved provider settings", async () => {
    mocks.getBuiltInProxyStatus.mockResolvedValueOnce({
      ...runningStatus,
      running: false,
      endpoint: null,
    });
    mocks.startBuiltInProxyFromSettings.mockResolvedValueOnce(runningStatus);
    const wrapper = mount(ProxyRuntimePage, {
      global: {
        stubs: { RouterLink: { template: "<a><slot /></a>" } },
      },
    });
    await flushPromises();
    await wrapper.get("button.btn-primary").trigger("click");

    expect(mocks.startBuiltInProxyFromSettings).toHaveBeenCalledOnce();
  });

  it("does not render backend error text verbatim", async () => {
    mocks.getBuiltInProxyStatus.mockRejectedValueOnce(new Error("provider secret leaked"));
    const wrapper = mount(ProxyRuntimePage, {
      global: {
        stubs: { RouterLink: { template: "<a><slot /></a>" } },
      },
    });
    await flushPromises();

    expect(wrapper.text()).not.toContain("provider secret leaked");
    expect(wrapper.text()).toContain("Unable to load the OpenMesh built-in proxy status.");
  });
});
