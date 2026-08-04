import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/agent-chat", query: {} }),
  useRouter: () => ({ push: vi.fn() }),
}));

const runAgentChatTurn = vi.fn();

vi.mock("@/lib/agentChat/runner", () => ({
  runAgentChatTurn: (...args: unknown[]) => runAgentChatTurn(...args),
}));

vi.mock("@/lib/agentEngineClient", () => ({
  getAgentSecretStatus: vi.fn(async () => ({
    configured: true,
    store: "/tmp/mock-secret",
  })),
}));

const mockStore = {
  currentProjectPath: ref("/tmp/test" as string | null),
  currentProject: ref({
    id: "p1",
    name: "Test Project",
    folderPath: "/tmp/test",
  } as any),
  settings: ref({
    workspace: { theme: "dark" },
    provider: {
      name: "MockProvider",
      apiKeyConfigured: true,
      usageTrackingEnabled: false,
      defaultModel: "mock-model",
    },
    models: { localModelEnabled: false },
    server: {
      mode: "local",
      apiBaseUrl: "",
      healthStatus: "unknown",
      syncStatus: "unknown",
    },
    agentClis: {},
    sessionDirs: {
      codexEnabled: false,
      claudeCodeEnabled: false,
      opencodeEnabled: false,
      cursorEnabled: false,
      geminiEnabled: false,
      grokEnabled: false,
    },
    localPaths: {},
    appearance: { theme: "dark" },
  } as any),
  saveSettings: vi.fn(async () => {}),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import AgentChatPage from "@/pages/AgentChatPage.vue";

describe("AgentChatPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockStore.currentProjectPath.value = "/tmp/test";
    mockStore.currentProject.value = {
      id: "p1",
      name: "Test Project",
      folderPath: "/tmp/test",
    };
    mockStore.settings.value.provider = {
      name: "MockProvider",
      apiKeyConfigured: true,
      usageTrackingEnabled: false,
      defaultModel: "mock-model",
    };
    // Fresh storage per test — chatSessions falls back to memory when needed.
    try {
      localStorage.clear();
    } catch {
      /* happy-dom may warn; memory fallback still isolates via empty load */
    }
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows provider gate when chat is not ready", async () => {
    mockStore.settings.value.provider = {
      name: "",
      apiKeyConfigured: false,
      usageTrackingEnabled: false,
      defaultModel: "",
    };
    const wrapper = mount(AgentChatPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Set up provider before chat");
  });

  it("renders Chat shell with composer when ready", async () => {
    const wrapper = mount(AgentChatPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Chat");
    expect(wrapper.text()).toContain("New chat");
    expect(wrapper.find("textarea").exists()).toBe(true);
    expect(wrapper.text()).toContain("Send");
    expect(wrapper.text()).toMatch(/OpenMesh Agent Engine|MockProvider|mock-model/);
    // Starter chips live in the thread column (not a full-width header dump).
    expect(wrapper.text()).toContain("/pilot");
    expect(wrapper.text()).toContain("More…");
    expect(wrapper.text()).toMatch(/Slash = local/);
    expect(wrapper.text()).not.toContain("DashScope Coding Plan");
  });

  it("optimistic send shows user message before mocked agent resolves", async () => {
    const deferred: {
      resolve:
        | ((v: { assistantText: string; toolCalls: unknown[] }) => void)
        | null;
    } = { resolve: null };

    runAgentChatTurn.mockImplementation(
      () =>
        new Promise((resolve) => {
          deferred.resolve = resolve;
        }),
    );

    const wrapper = mount(AgentChatPage);
    await flushPromises();
    await nextTick();

    const textarea = wrapper.find("textarea");
    await textarea.setValue("hello from e2e");
    await wrapper.find("button.chat-composer__send").trigger("click");
    await nextTick();
    // User bubble should paint before the agent promise settles.
    expect(wrapper.text()).toContain("hello from e2e");
    expect(wrapper.text()).toMatch(/Thinking|Working/);

    // send() awaits rAF + setTimeout(0) before calling the runner.
    await vi.waitFor(() => {
      expect(deferred.resolve).toBeTruthy();
    });

    deferred.resolve!({ assistantText: "mock reply", toolCalls: [] });
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("mock reply");
    expect(runAgentChatTurn).toHaveBeenCalled();
    // No real OpenRouter / agent_engine_turn — fully mocked.
    const callArgs = runAgentChatTurn.mock.calls[0];
    expect(callArgs[0]).toBe("/tmp/test");
    expect(callArgs[1]).toBe("hello from e2e");
  });

  it("exposes copy and fork actions on user/assistant bubbles", async () => {
    runAgentChatTurn.mockResolvedValue({
      assistantText: "assistant says hi",
      toolCalls: [],
    });

    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: { writeText },
    });

    const wrapper = mount(AgentChatPage);
    await flushPromises();
    await nextTick();

    const textarea = wrapper.find("textarea");
    await textarea.setValue("forkable prompt");
    await wrapper.find("button.chat-composer__send").trigger("click");
    await vi.waitFor(() => {
      expect(wrapper.text()).toContain("assistant says hi");
    });

    const copyBtns = wrapper.findAll('button[aria-label="Copy message"]');
    const forkBtns = wrapper.findAll(
      'button[aria-label="Fork chat from here"]',
    );
    // System welcome has no actions; user + assistant each have a pair.
    expect(copyBtns.length).toBeGreaterThanOrEqual(2);
    expect(forkBtns.length).toBe(copyBtns.length);

    await copyBtns[0].trigger("click");
    await flushPromises();
    expect(writeText).toHaveBeenCalledWith("forkable prompt");
    expect(wrapper.text()).toContain("Copied");

    const sessionsBefore = wrapper.findAll(".chat__rail-row").length;
    await forkBtns[0].trigger("click");
    await nextTick();
    expect(wrapper.findAll(".chat__rail-row").length).toBe(sessionsBefore + 1);
    expect(wrapper.text()).toMatch(/Fork of/);
  });
});
