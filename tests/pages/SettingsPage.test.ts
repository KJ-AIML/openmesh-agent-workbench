import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

const mockQuery = ref<Record<string, string>>({});
const mockReplace = vi.fn();

vi.mock("vue-router", () => ({
  useRoute: () => ({
    get query() {
      return mockQuery.value;
    },
  }),
  useRouter: () => ({ push: vi.fn(), replace: mockReplace }),
}));

vi.mock("@/lib/adapters/fileSystemAdapter", () => ({
  pickFolder: vi.fn(),
  validatePath: vi.fn().mockResolvedValue({ valid: true }),
}));

vi.mock("@/lib/agentEngineClient", () => ({
  clearAgentSecret: vi.fn(),
  setAgentSecret: vi.fn(),
  testAgentProvider: vi.fn(),
}));

vi.mock("@/lib/adapters/environment", () => ({
  getRuntimeKind: () => "web",
  isTauriRuntime: () => false,
}));

vi.mock("@/lib/extensionsClient", () => ({
  listExtensions: vi.fn().mockResolvedValue({ skills: [], hooks: [], plugins: [] }),
  listCatalog: vi.fn().mockResolvedValue([]),
  setExtensionEnabled: vi.fn(),
  installExtension: vi.fn(),
}));

vi.mock("@/lib/adapters/terminalAdapter", () => ({
  openTerminal: vi.fn(),
}));

// Tools panel pulls presets; stub so Settings smoke stays focused on Provider/Extensions.
vi.mock("@/components/settings/SettingsToolsPanel.vue", () => ({
  default: {
    name: "SettingsToolsPanel",
    template: "<div data-testid=\"settings-tools-stub\" />",
  },
}));

const mockStore = {
  settings: ref({
    workspace: { theme: "dark" },
    provider: {
      name: "",
      apiKeyConfigured: false,
      usageTrackingEnabled: false,
      defaultModel: "",
    },
    models: { localModelEnabled: false, codingModel: "", chatModel: "" },
    server: {
      mode: "local",
      apiBaseUrl: "http://localhost:3000",
      healthStatus: "unknown",
      syncStatus: "unknown",
    },
    agentClis: {},
    sessionDirs: {
      codexEnabled: true,
      claudeCodeEnabled: true,
      opencodeEnabled: true,
      cursorEnabled: true,
      geminiEnabled: true,
      grokEnabled: true,
    },
    localPaths: {},
    appearance: { theme: "dark", fontSize: "medium" },
    extensions: { skills: {}, hooks: {}, plugins: {} },
  } as any),
  saveSettings: vi.fn().mockResolvedValue(undefined),
  resetAll: vi.fn(),
  currentProject: ref({
    id: "p1",
    name: "Test",
    folderPath: "/tmp/test",
  } as any),
  currentProjectPath: ref("/tmp/test" as any),
  projectCommandPresets: ref([] as any[]),
  addCommandPreset: vi.fn(),
  deleteCommandPreset: vi.fn(),
  addRecentItem: vi.fn(),
  store: {},
  projectPaths: ref(["/tmp/test"]),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import SettingsPage from "@/pages/SettingsPage.vue";

describe("SettingsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
  });

  it("renders Settings header and Setup group", async () => {
    const wrapper = mount(SettingsPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Settings");
    expect(wrapper.text()).toContain("Setup");
    expect(wrapper.text()).toContain("Overview");
    expect(wrapper.text()).toContain("Provider");
  });

  it("Provider section shows provider name and save controls", async () => {
    mockQuery.value = { section: "provider" };
    const wrapper = mount(SettingsPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Provider & Models");
    expect(wrapper.text()).toContain("Provider Name");
    expect(wrapper.text()).toContain("Save Provider & Models");
  });

  it("Extensions section shows skills/hooks tabs", async () => {
    mockQuery.value = { section: "extensions" };
    const wrapper = mount(SettingsPage);
    await flushPromises();
    await nextTick();

    // Navigate Runtime → Extensions if query didn't land (route watch).
    const runtime = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "Runtime");
    if (runtime) {
      await runtime.trigger("click");
      await nextTick();
    }
    const extensions = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().includes("Extensions"));
    if (extensions) {
      await extensions.trigger("click");
      await flushPromises();
      await nextTick();
    }

    expect(wrapper.text()).toContain("Skills · Hooks · Plugins");
    expect(wrapper.text()).toContain("Skills");
    expect(wrapper.text()).toContain("Hooks");
    // Web runtime: empty inventory messaging (no live Tauri IPC).
    expect(wrapper.text()).toMatch(/No skills yet|Built-ins should appear/i);
  });
});
