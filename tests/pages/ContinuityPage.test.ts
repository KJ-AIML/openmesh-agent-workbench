import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

vi.mock("@/lib/continuityClient", () => ({
  getContinuityHubSummary: vi.fn(),
  getPendingQuestions: vi.fn(),
  getReturnDigest: vi.fn(),
  listMeshPeers: vi.fn(),
  listMeshEnvelopes: vi.fn(),
  queryMeshPeer: vi.fn(),
  addMeshPeer: vi.fn(),
  listRelayAudit: vi.fn(),
  getOnlineProxyStatus: vi.fn(),
  initOnlineProxy: vi.fn(),
  askOnlineProxy: vi.fn(),
  getTeamWorkspace: vi.fn(),
  getTeamTrustPolicy: vi.fn(),
  initTeamWorkspace: vi.fn(),
  addTeamMember: vi.fn(),
  initTeamTrustPolicy: vi.fn(),
  setTeamTrustRemoteQuery: vi.fn(),
  setTeamTrustQueryMode: vi.fn(),
  addTeamTrustAllowlist: vi.fn(),
  listTeamTrustAudit: vi.fn(),
  listConnectors: vi.fn(),
  getOrgGraph: vi.fn(),
  getPilotStatus: vi.fn(),
  getRcStatus: vi.fn(),
  lanServeStart: vi.fn(),
  lanServeStop: vi.fn(),
  lanServeStatus: vi.fn(),
  lanDiscover: vi.fn(),
  lanListApprovedPackages: vi.fn(),
  lanSendPackage: vi.fn(),
  lanAskPeer: vi.fn(),
  lanProbePresence: vi.fn(),
  lanProbeAddress: vi.fn(),
  lanChatSend: vi.fn(),
  lanChatList: vi.fn(),
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
  lanServeStatus,
  lanDiscover,
  lanListApprovedPackages,
  lanProbePresence,
  lanChatList,
  listMeshPeers,
  listMeshEnvelopes,
  listRelayAudit,
  listTeamTrustAudit,
  getTeamWorkspace,
  getTeamTrustPolicy,
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
    (lanServeStatus as any).mockResolvedValue({
      running: false,
      httpHost: "127.0.0.1",
      httpPort: 41778,
      note: null,
    });
    (lanDiscover as any).mockResolvedValue([]);
    (lanListApprovedPackages as any).mockResolvedValue([]);
    (lanProbePresence as any).mockResolvedValue([]);
    (lanChatList as any).mockResolvedValue([]);
    (listMeshPeers as any).mockResolvedValue([]);
    (listMeshEnvelopes as any).mockResolvedValue([]);
    (listRelayAudit as any).mockResolvedValue([]);
    (listTeamTrustAudit as any).mockResolvedValue([]);
    (getTeamWorkspace as any).mockResolvedValue(null);
    (getTeamTrustPolicy as any).mockResolvedValue(null);
  });

  it("renders Continuity header and section groups", async () => {
    const wrapper = mount(ContinuityPage);
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Continuity");
    // Grouped nav (You / Team / Mesh / Gate) — not the old flat tab strip.
    expect(wrapper.text()).toContain("You");
    expect(wrapper.text()).toContain("Team");
    expect(wrapper.text()).toContain("Mesh");
    expect(wrapper.text()).toContain("Gate");
    // Default group "You" shows Pending + Digest.
    expect(wrapper.text()).toContain("Pending");
    expect(wrapper.text()).toContain("Digest");
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
    // Status line uses summary.openPendingCount.
    expect(wrapper.text()).toMatch(/2 pending/);
    expect(wrapper.text()).toContain("1 open");
  });

  it("Mesh group exposes Peers, LAN, Chat, Relay, and Proxy tabs", async () => {
    const wrapper = mount(ContinuityPage);
    await flushPromises();

    const meshGroup = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "Mesh");
    expect(meshGroup).toBeTruthy();
    await meshGroup!.trigger("click");
    await nextTick();

    const text = wrapper.text();
    expect(text).toContain("Peers");
    expect(text).toContain("Relay");
    expect(text).toContain("Proxy");
    expect(text).toContain("LAN");
    expect(text).toContain("Chat");
  });

  it("LAN tab shows listener controls and presence probe UI", async () => {
    (lanDiscover as any).mockResolvedValue([
      {
        protocol: "openmesh-lan/0.1",
        projectId: "p2",
        ownerLabel: "Yo",
        peerId: "lan-yo",
        host: "127.0.0.1",
        httpPort: 41778,
        startedAt: "2026-08-06T01:00:00Z",
        lastSeenAt: "2026-08-06T01:00:00Z",
        address: "127.0.0.1:41778",
      },
    ]);
    (lanProbePresence as any).mockResolvedValue([
      {
        address: "127.0.0.1:41778",
        state: "live",
        probedAt: "2026-08-06T01:00:05Z",
        latencyMs: 12,
      },
    ]);

    const wrapper = mount(ContinuityPage);
    await flushPromises();

    await wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "Mesh")!
      .trigger("click");
    await nextTick();
    await wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().includes("LAN"))!
      .trigger("click");
    await flushPromises();
    await nextTick();

    expect(lanServeStatus).toHaveBeenCalledWith("/tmp/test");
    expect(lanProbePresence).toHaveBeenCalled();
    expect(wrapper.text()).toContain("LAN listener");
    expect(wrapper.text()).toContain("Start listener");
    expect(wrapper.text()).toContain("Refresh discover");
    expect(wrapper.text()).toContain("Listener is stopped");
    expect(wrapper.text()).toContain("Manual host:port probe");
    expect(wrapper.text()).toContain("Yo");
    expect(wrapper.text()).toMatch(/live/i);
  });

  it("Proxy tab shows initialize control when not configured", async () => {
    const wrapper = mount(ContinuityPage);
    await flushPromises();

    await wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "Mesh")!
      .trigger("click");
    await nextTick();
    await wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().includes("Proxy"))!
      .trigger("click");
    await flushPromises();
    await nextTick();

    expect(getOnlineProxyStatus).toHaveBeenCalledWith("/tmp/test");
    expect(wrapper.text()).toContain("Initialize Continuity Proxy");
  });
});
