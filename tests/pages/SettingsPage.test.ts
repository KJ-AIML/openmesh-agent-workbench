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

vi.mock("@/lib/updates/updateCheck", () => ({
  hasKnownUpdate: () => false,
  readPersistedUpdateCheck: () => null,
  maybeBackgroundUpdateCheck: vi.fn().mockResolvedValue(null),
  checkForUpdates: vi.fn(),
  openExternalUrl: vi.fn(),
}));

vi.mock("@/lib/updates/appVersion", () => ({
  getAppVersion: () => "0.1.26",
}));

// Tools panel pulls presets; stub so Settings smoke stays focused on Provider/Extensions.
vi.mock("@/components/settings/SettingsToolsPanel.vue", () => ({
  default: {
    name: "SettingsToolsPanel",
    template: "<div data-testid=\"settings-tools-stub\" />",
  },
}));

vi.mock("@/components/settings/SettingsUpdatesPanel.vue", () => ({
  default: {
    name: "SettingsUpdatesPanel",
    template:
      '<div data-testid="settings-updates-stub">Check for updates</div>',
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
    appearance: {
      theme: "dark",
      fontSize: "medium",
      density: "comfortable",
      topNavbarTabs: {
        chat: true,
        work: true,
        docs: true,
        sprint: true,
      },
    },
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

  it("About section shows version and check-for-updates entry", async () => {
    mockQuery.value = { section: "about" };
    const wrapper = mount(SettingsPage);
    await flushPromises();
    await nextTick();

    const appGroup = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "App");
    if (appGroup) {
      await appGroup.trigger("click");
      await nextTick();
    }
    const about = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().includes("About"));
    if (about) {
      await about.trigger("click");
      await flushPromises();
      await nextTick();
    }

    expect(wrapper.text()).toMatch(/v0\.1\.26/);
    expect(wrapper.text()).toContain("Check for updates");
  });

  it("Appearance section shows theme controls and live preview", async () => {
    mockQuery.value = { section: "appearance" };
    const wrapper = mount(SettingsPage);
    await flushPromises();
    await nextTick();

    const appGroup = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().trim() === "App");
    if (appGroup) {
      await appGroup.trigger("click");
      await nextTick();
    }
    const appearance = wrapper
      .findAll('[role="tab"]')
      .find((b) => b.text().includes("Appearance"));
    if (appearance) {
      await appearance.trigger("click");
      await flushPromises();
      await nextTick();
    }

    expect(wrapper.text()).toContain("Appearance");
    expect(wrapper.text()).toMatch(/Theme/);
    expect(wrapper.text()).toContain("Dark");
    expect(wrapper.text()).toContain("Light");
    expect(wrapper.text()).toContain("System");
    expect(wrapper.text()).toContain("Compact");
    expect(wrapper.text()).toContain("Top navbar tabs");
    expect(wrapper.find('[data-testid="appearance-preview"]').exists()).toBe(
      true,
    );
    expect(
      wrapper.find('[data-testid="appearance-top-navbar-tabs"]').exists(),
    ).toBe(true);

    const light = wrapper
      .findAll('[role="radio"]')
      .find((b) => b.text().trim() === "Light");
    expect(light).toBeTruthy();
    await light!.trigger("click");
    await flushPromises();
    expect(mockStore.saveSettings).toHaveBeenCalled();
    const payload = mockStore.saveSettings.mock.calls.at(-1)?.[0];
    expect(payload?.appearance?.theme).toBe("light");

    const sprintToggle = wrapper
      .find('[data-testid="appearance-top-navbar-tabs"]')
      .findAll("button")
      .find((b) => b.text().trim() === "Sprint");
    expect(sprintToggle).toBeTruthy();
    await sprintToggle!.trigger("click");
    await flushPromises();
    const tabPayload = mockStore.saveSettings.mock.calls.at(-1)?.[0];
    expect(tabPayload?.appearance?.topNavbarTabs?.sprint).toBe(false);
  });
});
