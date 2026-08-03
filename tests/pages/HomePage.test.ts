import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/", query: {} }),
  useRouter: () => ({ push: vi.fn() }),
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

const mockStore = {
  currentProject: ref(null as any),
  projectPaths: ref<string[]>([]),
  settings: ref({
    workspace: { theme: "dark" },
    provider: { apiKeyConfigured: false, usageTrackingEnabled: false },
    models: { localModelEnabled: false },
    agentClis: {},
  } as any),
  projectSprint: ref(null as any),
  projectTasks: ref([] as any[]),
  projectSessions: ref([] as any[]),
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
});
