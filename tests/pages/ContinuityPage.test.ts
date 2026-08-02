import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

vi.mock("@/lib/continuityClient", () => ({
  getContinuityHubSummary: vi.fn(),
  getPendingQuestions: vi.fn(),
  getReturnDigest: vi.fn(),
  listMeshPeers: vi.fn(),
  listMeshEnvelopes: vi.fn(),
  listRelayAudit: vi.fn(),
  getOnlineProxyStatus: vi.fn(),
  initOnlineProxy: vi.fn(),
  askOnlineProxy: vi.fn(),
}));

vi.mock("vue-router", () => ({
  useRoute: () => ({ query: {} }),
  useRouter: () => ({ push: vi.fn() }),
}));

const mockStore = {
  currentProject: ref({
    id: "p1",
    name: "Test Project",
    folderPath: "/tmp/test",
  } as any),
  currentProjectPath: ref("/tmp/test" as any),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import ContinuityPage from "@/pages/ContinuityPage.vue";
import {
  getContinuityHubSummary,
  getPendingQuestions,
  getOnlineProxyStatus,
} from "@/lib/continuityClient";

describe("ContinuityPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStore.currentProject.value = {
      id: "p1",
      name: "Test Project",
      folderPath: "/tmp/test",
    } as any;
    mockStore.currentProjectPath.value = "/tmp/test" as any;

    (getContinuityHubSummary as any).mockResolvedValue({
      openPendingCount: 2,
      peerCount: 1,
      envelopeCount: 0,
      auditEventCount: 0,
      onlineProxyInitialized: false,
    });
    (getPendingQuestions as any).mockResolvedValue({
      workspaceId: "p1",
      generatedAt: "2026-08-02T12:00:00Z",
      protocolVersion: "1.0",
      items: [
        {
          id: "q1",
          summary: "Need decision on API shape",
          source: "proxy-pending",
          sourceId: "pq-1",
          status: "open",
          severity: "high",
          createdAt: "2026-08-02T11:00:00Z",
          reason: "must ask",
        },
      ],
      openCount: 1,
      sourceCounts: {
        proxyPending: 1,
        continuityAttention: 0,
        unresolvedSignal: 0,
      },
      limitations: [],
    });
    (getOnlineProxyStatus as any).mockResolvedValue(null);
  });

  it("renders Continuity header and tabs", async () => {
    const wrapper = mount(ContinuityPage);
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Continuity");
    expect(wrapper.text()).toContain("Pending");
    expect(wrapper.text()).toContain("Digest");
    expect(wrapper.text()).toContain("Mesh");
    expect(wrapper.text()).toContain("Relay");
    expect(wrapper.text()).toContain("Online Proxy");
  });

  it("shows no-project state when none selected", async () => {
    mockStore.currentProject.value = null as any;
    mockStore.currentProjectPath.value = null as any;
    const wrapper = mount(ContinuityPage);
    await nextTick();
    expect(wrapper.text()).toContain("No project selected");
  });

  it("loads and shows pending items", async () => {
    const wrapper = mount(ContinuityPage);
    await flushPromises();
    await nextTick();
    expect(getPendingQuestions).toHaveBeenCalledWith("/tmp/test");
    expect(wrapper.text()).toContain("Need decision on API shape");
    expect(wrapper.text()).toContain("Pending open: 2");
  });
});
