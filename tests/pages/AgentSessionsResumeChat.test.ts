import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick } from "vue";

const {
  mockQuery,
  routerPush,
  scanned,
  persistSessionsAsync,
  loadSessionsAsync,
  readForeignSessionTranscript,
  mockStore,
} = vi.hoisted(() => {
  const scanned = [
    {
      id: "cursor-1",
      toolName: "cursor",
      title: "Research sessions",
      sessionPath: "/tmp/cursor/abc.jsonl",
      fileName: "abc.jsonl",
      createdAt: "2026-08-06T00:00:00.000Z",
      lastActiveAt: "2026-08-06T00:00:00.000Z",
      fileSizeBytes: 1200,
      summaryPreview: "User asked about resume into chat",
      isReal: true,
    },
  ];
  return {
    mockQuery: { value: {} as Record<string, string> },
    routerPush: vi.fn(),
    scanned,
    persistSessionsAsync: vi.fn().mockResolvedValue(undefined),
    loadSessionsAsync: vi.fn().mockResolvedValue([]),
    readForeignSessionTranscript: vi.fn().mockResolvedValue({
      success: true,
      data: {
        tool: "cursor",
        path: "/tmp/cursor/abc.jsonl",
        title: "Research sessions",
        messages: [
          { role: "user", text: "Wire resume" },
          { role: "assistant", text: "On it" },
        ],
        truncated: false,
        previewOnly: false,
      },
      isMock: false,
    }),
    mockStore: {
      currentProject: {
        value: {
          id: "p1",
          name: "Test Project",
          folderPath: "/tmp/test",
        } as any,
      },
      projectSessions: { value: [] as any[] },
      projectTasks: { value: [] as any[] },
      deleteAgentSession: vi.fn(),
      updateAgentSession: vi.fn(),
      addRecentItem: vi.fn(),
      settings: {
        value: {
          sessionDirs: {},
          agentClis: {},
        } as any,
      },
    },
  };
});

vi.mock("vue-router", () => ({
  useRoute: () => ({
    get query() {
      return mockQuery.value;
    },
    path: "/agent-sessions",
  }),
  useRouter: () => ({ push: routerPush, replace: vi.fn() }),
}));

vi.mock("@/lib/scanConfiguredSessions", () => ({
  scanConfiguredSessions: vi.fn().mockResolvedValue(scanned),
}));

vi.mock("@/lib/adapters/agentSessionAdapter", () => ({
  readForeignSessionTranscript,
}));

vi.mock("@/lib/agentChat/chatSessions", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/agentChat/chatSessions")
  >("@/lib/agentChat/chatSessions");
  return {
    ...actual,
    loadSessionsAsync: (...args: unknown[]) => loadSessionsAsync(...args),
    persistSessionsAsync: (...args: unknown[]) => persistSessionsAsync(...args),
  };
});

vi.mock("@/lib/adapters/terminalAdapter", () => ({
  openAgentCli: vi.fn(),
}));

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import AgentSessionsPage from "@/pages/AgentSessionsPage.vue";

describe("Agent Sessions Continue in Chat", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
    persistSessionsAsync.mockResolvedValue(undefined);
    loadSessionsAsync.mockResolvedValue([]);
    readForeignSessionTranscript.mockResolvedValue({
      success: true,
      data: {
        tool: "cursor",
        path: "/tmp/cursor/abc.jsonl",
        title: "Research sessions",
        messages: [
          { role: "user", text: "Wire resume" },
          { role: "assistant", text: "On it" },
        ],
        truncated: false,
        previewOnly: false,
      },
      isMock: false,
    });
  });

  it("shows Continue in Chat and opens choice modal", async () => {
    const wrapper = mount(AgentSessionsPage);
    await flushPromises();
    await nextTick();

    const card = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Research sessions"));
    expect(card).toBeTruthy();
    await card!.trigger("click");
    await nextTick();

    expect(wrapper.text()).toContain("Continue in Chat");
    const continueBtn = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Continue in Chat"));
    await continueBtn!.trigger("click");
    await nextTick();

    expect(wrapper.text()).toContain("Continue in OpenMesh Chat");
    expect(wrapper.text()).toContain("Summarize & continue");
    expect(wrapper.text()).toContain("Import full");
    expect(wrapper.text()).toContain("Cancel / Not now");
    expect(wrapper.text()).toContain("stays untouched");
  });

  it("summarize path persists a chat and navigates with ?chat=", async () => {
    const wrapper = mount(AgentSessionsPage);
    await flushPromises();
    await nextTick();

    const card = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Research sessions"));
    await card!.trigger("click");
    await nextTick();

    const continueBtn = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Continue in Chat"));
    await continueBtn!.trigger("click");
    await nextTick();

    const summarize = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Summarize & continue"));
    await summarize!.trigger("click");
    await flushPromises();

    expect(persistSessionsAsync).toHaveBeenCalled();
    const [, sessions] = persistSessionsAsync.mock.calls[0];
    expect(sessions[0].importedFrom).toMatchObject({
      source: "cursor",
      id: "cursor-1",
    });
    expect(routerPush).toHaveBeenCalledWith(
      expect.objectContaining({
        path: "/agent-chat",
        query: { chat: sessions[0].id },
      }),
    );
  });

  it("cancel dismisses modal without persisting", async () => {
    const wrapper = mount(AgentSessionsPage);
    await flushPromises();
    await nextTick();

    const card = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Research sessions"));
    await card!.trigger("click");
    await nextTick();
    await wrapper
      .findAll("button")
      .find((b) => b.text().includes("Continue in Chat"))!
      .trigger("click");
    await nextTick();

    await wrapper
      .findAll("button")
      .find((b) => b.text().includes("Cancel / Not now"))!
      .trigger("click");
    await nextTick();

    expect(wrapper.text()).not.toContain("Summarize & continue");
    expect(persistSessionsAsync).not.toHaveBeenCalled();
    expect(routerPush).not.toHaveBeenCalled();
  });
});
