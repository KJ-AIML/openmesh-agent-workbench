import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

const push = vi.fn();

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/", query: {} }),
  useRouter: () => ({ push }),
}));

vi.mock("@/lib/adapters/gitAdapter", () => ({
  getGitStatus: vi.fn().mockResolvedValue({
    success: true,
    data: { branch: "main", dirty: false, ahead: 0, behind: 0 },
    isMock: true,
  }),
}));

vi.mock("@/lib/adapters/fileSystemAdapter", () => ({
  openFolder: vi.fn(),
}));

vi.mock("@/lib/adapters/terminalAdapter", () => ({
  openAgentCli: vi.fn(),
}));

const scanConfiguredSessionsResult = vi.fn();

vi.mock("@/lib/scanConfiguredSessions", async () => {
  const actual = await vi.importActual<
    typeof import("@/lib/scanConfiguredSessions")
  >("@/lib/scanConfiguredSessions");
  return {
    ...actual,
    scanConfiguredSessionsResult: (...args: unknown[]) =>
      scanConfiguredSessionsResult(...args),
  };
});

const mockStore = {
  currentProject: ref(null as any),
  projectPaths: ref<string[]>([]),
  settings: ref({
    workspace: { theme: "dark" },
    provider: { apiKeyConfigured: false, usageTrackingEnabled: false },
    models: { localModelEnabled: false },
    agentClis: {},
    sessionDirs: {},
  } as any),
  projectSprint: ref(null as any),
  projectTasks: ref([] as any[]),
  projectDocs: ref([] as any[]),
  getRecentItemsForProject: vi.fn(() => []),
  addRecentItem: vi.fn(),
  createSprint: vi.fn(),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import HomePage from "@/pages/HomePage.vue";

describe("HomePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStore.currentProject.value = null;
    mockStore.projectPaths.value = [];
    scanConfiguredSessionsResult.mockResolvedValue({ ok: true, sessions: [] });
  });

  it("shows no-project shell when none selected", async () => {
    const wrapper = mount(HomePage);
    await nextTick();
    expect(wrapper.text()).toContain("No project selected");
  });

  it("renders project home shell with primary sections", async () => {
    mockStore.currentProject.value = {
      id: "p1",
      name: "Workbench Demo",
      folderPath: "/tmp/demo",
    };
    const wrapper = mount(HomePage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Workbench Demo");
    expect(wrapper.text()).toContain("Current Sprint");
    expect(wrapper.text()).toContain("Agent Sessions");
  });

  it("loads scanned agent sessions into the Home card", async () => {
    mockStore.currentProject.value = {
      id: "p1",
      name: "Workbench Demo",
      folderPath: "/tmp/demo",
    };
    scanConfiguredSessionsResult.mockResolvedValue({
      ok: true,
      sessions: [
        {
          id: "sess-1",
          toolName: "codex",
          title: "Fix login flow",
          sessionPath: "/tmp/sess-1.jsonl",
          fileName: "sess-1.jsonl",
          createdAt: "2026-06-01T00:00:00.000Z",
          lastActiveAt: "2026-06-02T00:00:00.000Z",
          fileSizeBytes: 120,
          isReal: true,
        },
      ],
    });

    const wrapper = mount(HomePage);
    await flushPromises();
    await nextTick();

    expect(scanConfiguredSessionsResult).toHaveBeenCalledWith(
      mockStore.settings.value.sessionDirs,
      100,
      "/tmp/demo",
    );
    expect(wrapper.text()).toContain("Fix login flow");
    expect(wrapper.text()).toContain("codex");
    expect(wrapper.text()).not.toContain("No agent sessions for this project yet");
  });

  it("navigates to Agent Sessions with the selected session id", async () => {
    mockStore.currentProject.value = {
      id: "p1",
      name: "Workbench Demo",
      folderPath: "/tmp/demo",
    };
    scanConfiguredSessionsResult.mockResolvedValue({
      ok: true,
      sessions: [
        {
          id: "sess-1",
          toolName: "claude",
          title: "Refactor auth",
          sessionPath: "/tmp/sess-1.jsonl",
          fileName: "sess-1.jsonl",
          createdAt: "2026-06-01T00:00:00.000Z",
          lastActiveAt: "2026-06-02T00:00:00.000Z",
          fileSizeBytes: 120,
          isReal: true,
        },
      ],
    });

    const wrapper = mount(HomePage);
    await flushPromises();
    await nextTick();

    const row = wrapper
      .findAll("button")
      .find((b) => b.text().includes("Refactor auth"));
    expect(row).toBeTruthy();
    await row!.trigger("click");

    expect(push).toHaveBeenCalledWith({
      path: "/agent-sessions",
      query: { session: "sess-1" },
    });
  });

  it("shows honest empty state when scan returns zero sessions", async () => {
    mockStore.currentProject.value = {
      id: "p1",
      name: "Workbench Demo",
      folderPath: "/tmp/demo",
    };
    scanConfiguredSessionsResult.mockResolvedValue({ ok: true, sessions: [] });

    const wrapper = mount(HomePage);
    await flushPromises();
    await nextTick();

    expect(wrapper.text()).toContain("No agent sessions for this project yet");
  });

  it("shows error state when scan fails", async () => {
    mockStore.currentProject.value = {
      id: "p1",
      name: "Workbench Demo",
      folderPath: "/tmp/demo",
    };
    scanConfiguredSessionsResult.mockResolvedValue({
      ok: false,
      sessions: [],
      error: "IPC unavailable",
    });

    const wrapper = mount(HomePage);
    await flushPromises();
    await nextTick();

    expect(wrapper.text()).toContain("IPC unavailable");
    expect(wrapper.text()).toContain("Retry");
  });
});
