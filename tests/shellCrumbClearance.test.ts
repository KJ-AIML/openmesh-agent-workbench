import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";
import {
  SIDEBAR_VISIBLE_STORAGE_KEY,
  __resetSidebarVisibilityForTests,
} from "@/lib/useSidebarVisibility";

vi.mock("vue-router", () => ({
  useRoute: () => ({ path: "/sprint", query: {} }),
  useRouter: () => ({ push: vi.fn() }),
}));

vi.mock("@/lib/adapters/gitAdapter", () => ({
  getGitStatus: vi.fn().mockResolvedValue({ success: true, data: null, isMock: true }),
}));

vi.mock("@/lib/adapters/terminalAdapter", () => ({
  openAgentCli: vi.fn(),
}));

vi.mock("@/lib/scanConfiguredSessions", () => ({
  scanConfiguredSessions: vi.fn(),
}));

vi.mock("@/lib/useStore", () => ({
  useStore: () => ({
    isLoading: ref(false),
    currentProject: ref({ id: "p1", name: "openmesh-ws", folderPath: "/tmp/ws" }),
    projectPaths: ref(["/tmp/ws"]),
    settings: ref({ sessionDirs: [], agentClis: {} }),
    projectCommandPresets: ref([]),
    projectSprint: ref(null),
    projectTasks: ref([]),
    projectSessions: ref([]),
    projectDocs: ref([]),
    getRecentItemsForProject: vi.fn(() => []),
    addRecentItem: vi.fn(),
    store: { writeSnapshot: vi.fn() },
  }),
}));

vi.mock("@/lib/adapters/environment", () => ({
  isMacOS: () => true,
  resolveIsMacOS: async () => true,
  isTauriRuntime: () => false,
  getRuntimeKind: () => "web",
  hasNativeFeature: () => false,
}));

import App from "@/App.vue";

function memoryStorage(): Storage {
  const map = new Map<string, string>();
  return {
    get length() {
      return map.size;
    },
    clear() {
      map.clear();
    },
    getItem(key: string) {
      return map.has(key) ? map.get(key)! : null;
    },
    key(index: number) {
      return Array.from(map.keys())[index] ?? null;
    },
    removeItem(key: string) {
      map.delete(key);
    },
    setItem(key: string, value: string) {
      map.set(key, String(value));
    },
  };
}

function mountApp() {
  return mount(App, {
    global: {
      stubs: {
        Sidebar: true,
        Titlebar: true,
        CommandPalette: true,
        Transition: false,
        "router-view": true,
        PanelLeft: true,
        PanelLeftClose: true,
      },
    },
  });
}

describe("shell crumb macOS traffic-light clearance", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: memoryStorage(),
    });
    (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ =
      true;
    __resetSidebarVisibilityForTests(true);
  });

  afterEach(() => {
    delete (window as unknown as { __OPENMESH_IS_MACOS__?: boolean })
      .__OPENMESH_IS_MACOS__;
    __resetSidebarVisibilityForTests(true);
  });

  it("inserts crumb flex spacer only when sidebar is collapsed on macOS", async () => {
    const wrapper = mountApp();
    await flushPromises();
    await nextTick();

    expect(wrapper.find(".shell--mac").exists()).toBe(true);
    expect(wrapper.find(".shell__crumb--traffic-clearance").exists()).toBe(false);
    expect(wrapper.find(".shell__crumb-traffic-clearance").exists()).toBe(false);
    expect(wrapper.findAll(".shell__sidebar-toggle")).toHaveLength(1);

    await wrapper.get(".shell__sidebar-toggle").trigger("click");
    await nextTick();

    expect(wrapper.find(".shell--sidebar-collapsed").exists()).toBe(true);
    expect(wrapper.find(".shell__crumb--traffic-clearance").exists()).toBe(true);
    const spacer = wrapper.find(".shell__crumb-traffic-clearance");
    expect(spacer.exists()).toBe(true);

    const crumb = wrapper.find("header.shell__crumb");
    const toggle = wrapper.find(".shell__sidebar-toggle");
    const children = Array.from(crumb.element.children);
    expect(children.indexOf(spacer.element)).toBeLessThan(
      children.indexOf(toggle.element),
    );

    // Single toggle still — spacer is not a button.
    expect(wrapper.findAll(".shell__sidebar-toggle")).toHaveLength(1);
    expect(wrapper.get(".shell__sidebar-toggle").attributes("aria-label")).toBe(
      "Show sidebar",
    );

    await wrapper.get(".shell__sidebar-toggle").trigger("click");
    await nextTick();
    expect(wrapper.find(".shell__crumb--traffic-clearance").exists()).toBe(false);
    expect(wrapper.find(".shell__crumb-traffic-clearance").exists()).toBe(false);
    wrapper.unmount();
  });

  it("hydrates collapsed preference with crumb clearance already on", async () => {
    localStorage.setItem(SIDEBAR_VISIBLE_STORAGE_KEY, "0");
    __resetSidebarVisibilityForTests(true);

    const wrapper = mountApp();
    await flushPromises();
    await nextTick();

    expect(wrapper.find(".shell--sidebar-collapsed").exists()).toBe(true);
    expect(wrapper.find(".shell__crumb--traffic-clearance").exists()).toBe(true);
    expect(wrapper.find(".shell__crumb-traffic-clearance").exists()).toBe(true);
    wrapper.unmount();
  });
});
