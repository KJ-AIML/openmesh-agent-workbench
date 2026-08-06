import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/agent-chat", query: {} }),
  useRouter: () => ({
    push: vi.fn(),
    currentRoute: { value: { path: "/agent-chat", query: {} } },
  }),
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
  extractPatchIds: (text: string) => {
    const m = text.match(/patch-[a-f0-9]+/gi);
    return m ?? [];
  },
  cancelAgentEngineTurn: vi.fn(async () => true),
  cancelAgentRecipe: vi.fn(async () => true),
  loadDurableChats: vi.fn(async () => []),
  saveDurableChats: vi.fn(async () => {}),
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

const openTerminal = vi.fn(
  async (_opts?: unknown) => ({ success: true, isMock: true }),
);

vi.mock("@/lib/adapters/terminalAdapter", () => ({
  openTerminal: (opts: unknown) => openTerminal(opts),
  openAgentCli: vi.fn(async () => ({ success: true, isMock: true })),
}));

vi.mock("@/lib/adapters/ptyAdapter", () => ({
  createPty: vi.fn(async () => ({
    id: "shell-mock",
    shell: "zsh",
    cwd: "/tmp/test",
  })),
  writePty: vi.fn(async () => {}),
  resizePty: vi.fn(async () => {}),
  killPty: vi.fn(async () => {}),
  killAllPtys: vi.fn(async () => {}),
  listenPtyData: vi.fn(async () => () => {}),
  listenPtyExit: vi.fn(async () => () => {}),
}));

vi.mock("@/components/chat/EmbeddedTerminal.vue", async () => {
  const { defineComponent, h } = await import("vue");
  return {
    default: defineComponent({
      name: "EmbeddedTerminalStub",
      props: {
        sessionId: { type: String, required: true },
        cwd: { type: String, required: true },
        active: { type: Boolean, required: true },
      },
      setup(props) {
        return () =>
          h("div", {
            "data-testid": `embedded-term-${props.sessionId}`,
          });
      },
    }),
  };
});

vi.mock("@/lib/store", () => ({
  store: {
    listDocs: vi.fn(async () => []),
    listNotes: vi.fn(async () => []),
    getProject: vi.fn(async () => null),
  },
}));

import AgentChatPage from "@/pages/AgentChatPage.vue";

describe("AgentChatPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    openTerminal.mockResolvedValue({ success: true, isMock: true });
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

  it("renders Chat shell with slim composer when ready", async () => {
    const wrapper = mount(AgentChatPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Chat");
    expect(wrapper.text()).toContain("New chat");
    expect(wrapper.find("textarea").exists()).toBe(true);
    expect(wrapper.text()).toContain("Send");
    expect(wrapper.text()).toMatch(/OpenMesh Agent Engine|MockProvider|mock-model/);
    // One composer shell: mode + commands menu, not always-on slash chip rows.
    expect(wrapper.find('[data-testid="chat-composer"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid="composer-mode"]').text()).toMatch(/ask/i);
    expect(wrapper.find('[data-testid="composer-commands"]').exists()).toBe(true);
    expect(wrapper.text()).not.toContain("More…");
    expect(wrapper.text()).not.toMatch(/slash = local/i);
    // Slash starters live in the Commands menu (not a permanent chip row).
    expect(wrapper.text()).not.toContain("/pilot");
    await wrapper.find('[data-testid="composer-commands"]').trigger("click");
    await nextTick();
    expect(wrapper.text()).toContain("/pilot");
    expect(wrapper.find('[data-testid="composer-slash-menu"]').exists()).toBe(
      true,
    );
    expect(wrapper.text()).toContain("All tools…");
    expect(wrapper.text()).not.toContain("DashScope Coding Plan");
    // Quiet status icons inside the shell (no "0 Terminal" pill).
    expect(wrapper.find('[data-testid="composer-status-bar"]').exists()).toBe(
      true,
    );
    expect(
      wrapper.find('[data-testid="composer-status-terminal-badge"]').exists(),
    ).toBe(false);
    expect(wrapper.find('[data-testid="composer-status-canvas"]').exists()).toBe(
      true,
    );
    expect(wrapper.find('[data-testid="composer-status-working"]').exists()).toBe(
      false,
    );
    // Terminal icon opens tabbed panel with an embedded PTY tab (not OS shell).
    await wrapper.find('[data-testid="composer-status-terminal"]').trigger("click");
    await flushPromises();
    await nextTick();
    expect(wrapper.find('[data-testid="chat-terminal-panel"]').exists()).toBe(
      true,
    );
    expect(wrapper.find('[data-testid="term-tab-add"]').exists()).toBe(true);
    expect(wrapper.find('[data-testid^="embedded-term-"]').exists()).toBe(true);
    expect(wrapper.text()).not.toMatch(/external shell/i);
    expect(wrapper.text()).toMatch(/\/ commands|@ context/i);
    // Right-dock sidebar by default — chat column keeps its own scroll region.
    const panel = wrapper.find('[data-testid="chat-terminal-panel"]');
    expect(panel.attributes("data-dock")).toBe("right");
    expect(wrapper.find('[data-testid="chat-main"]').classes()).toContain(
      "chat__main--term-right",
    );
    expect(wrapper.find('[data-testid="term-panel-resize"]').exists()).toBe(
      true,
    );
    expect(wrapper.find(".chat__column").exists()).toBe(true);
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
    // Composer status chip tracks the in-flight turn.
    await vi.waitFor(() => {
      expect(wrapper.find('[data-testid="composer-status-working"]').exists()).toBe(
        true,
      );
    });

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

    // User vs assistant bubbles are visually/role-labeled distinct.
    expect(wrapper.find('.msg--user[data-role="user"]').exists()).toBe(true);
    expect(wrapper.find('.msg--assistant[data-role="assistant"]').exists()).toBe(
      true,
    );
    expect(wrapper.find(".msg--user .bubble__role").text()).toMatch(/You/i);
    expect(wrapper.find(".msg--assistant .bubble__role").text()).toMatch(
      /Assistant/i,
    );
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
