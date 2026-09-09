import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createMemoryHistory, createRouter } from "vue-router";
import Sidebar from "@/components/Sidebar.vue";
import { PROJECT_LANDING_ROUTE, sidebarLinkHrefs } from "@/lib/navigation";

const { projectPaths, currentProjectPath, currentProject, selectProject, deleteProject, addRecentItem, getProject } = vi.hoisted(() => {
  // Keep mocked refs inside the hoisted factory so Vitest can initialize module mocks safely.
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { ref } = require("vue") as typeof import("vue");
  return {
    projectPaths: ref<string[]>([]),
    currentProjectPath: ref<string | null>(null),
    currentProject: ref<{ id: string; name: string; folderPath: string } | null>(null),
    selectProject: vi.fn(),
    deleteProject: vi.fn(),
    addRecentItem: vi.fn(),
    getProject: vi.fn(),
  };
});

vi.mock("@/lib/useStore", () => ({
  useStore: () => ({
    projectPaths,
    currentProjectPath,
    currentProject,
    selectProject,
    deleteProject,
    addRecentItem,
    store: { getProject },
  }),
}));

vi.mock("@/lib/adapters/environment", () => ({
  isMacOS: () => false,
  resolveIsMacOS: vi.fn().mockResolvedValue(false),
}));

vi.mock("@/lib/adapters/windowAdapter", () => ({
  startWindowDrag: vi.fn(),
}));

const routes = [
  "/",
  "/agent-chat",
  "/agent-sessions",
  "/sprint",
  "/docs",
  "/notes",
  "/canvas",
  "/context",
  "/continuity",
  "/oauth",
  "/proxy-runtime",
  "/proxy-providers",
  "/usage",
  "/settings",
  "/projects/new",
].map((path) => ({
  path,
  component: { template: "<div />" },
}));

async function mountSidebar(initialPath = "/") {
  const router = createRouter({
    history: createMemoryHistory(),
    routes,
  });
  await router.push(initialPath);
  await router.isReady();
  const wrapper = mount(Sidebar, {
    global: {
      plugins: [router],
    },
  });
  await flushPromises();
  return { wrapper, router };
}

describe("Sidebar navigation information architecture", () => {
  beforeEach(() => {
    projectPaths.value = [];
    currentProjectPath.value = null;
    currentProject.value = null;
    selectProject.mockReset();
    deleteProject.mockReset();
    addRecentItem.mockReset();
    getProject.mockReset();
  });

  it("keeps every existing surface under the v0.2 workbench topics", async () => {
    const { wrapper } = await mountSidebar();

    expect(wrapper.find("nav[aria-label='Workspace navigation']").exists()).toBe(true);
    expect(wrapper.findAll("[data-topic]").map((node) => node.attributes("data-topic"))).toEqual([
      "projects",
      "workspace",
      "agents",
      "runtime",
      "settings",
    ]);

    const links = wrapper.findAll("a").map((link) => link.attributes("href"));
    expect(links).toEqual(sidebarLinkHrefs());
  });

  it("toggles collapsed Runtime without changing route links", async () => {
    const { wrapper } = await mountSidebar();
    const runtimeToggle = wrapper.find("[data-topic-toggle='runtime']");
    const runtimePanel = wrapper.find("#sidebar-topic-runtime");

    expect(runtimeToggle.attributes("aria-expanded")).toBe("false");
    expect(runtimePanel.attributes("style")).toContain("display: none");

    await runtimeToggle.trigger("click");
    expect(runtimeToggle.attributes("aria-expanded")).toBe("true");
    expect(runtimePanel.attributes("style") || "").not.toContain("display: none");
    expect(runtimePanel.find("a[href='/proxy-runtime']").exists()).toBe(true);
    expect(runtimePanel.find("a[href='/continuity']").text()).toContain("Pending & LAN");

    await runtimeToggle.trigger("click");
    expect(runtimeToggle.attributes("aria-expanded")).toBe("false");
    expect(runtimePanel.attributes("style")).toContain("display: none");
  });

  it("auto-expands Runtime for a deep-linked HTTP proxy URL", async () => {
    const { wrapper } = await mountSidebar("/proxy-runtime");
    const runtimeToggle = wrapper.find("[data-topic-toggle='runtime']");
    const runtimePanel = wrapper.find("#sidebar-topic-runtime");

    expect(runtimeToggle.attributes("aria-expanded")).toBe("true");
    expect(runtimeToggle.classes()).toContain("is-active");
    expect(runtimePanel.attributes("style") || "").not.toContain("display: none");
    expect(runtimePanel.find("a[href='/proxy-runtime']").classes()).toContain("active");
  });

  it("keeps the project switcher and Chat ahead of grouped work navigation", async () => {
    const project = {
      id: "project-1",
      name: "Workbench",
      folderPath: "/tmp/workbench",
    };
    projectPaths.value = [project.folderPath];
    currentProject.value = project;
    getProject.mockResolvedValue(project);

    const { wrapper, router } = await mountSidebar();
    const nav = wrapper.find("nav");
    const chat = nav.find("a[href='/agent-chat']");
    const workspace = nav.find("[data-topic-toggle='workspace']");

    const projectsTopic = nav.find("[data-topic='projects']");
    expect(projectsTopic.text()).toContain("Projects");
    expect(projectsTopic.text()).toContain("Add Project");
    expect(chat.exists()).toBe(true);
    expect(chat.element.compareDocumentPosition(workspace.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(wrapper.find("[data-navigation-topic='chat']").exists()).toBe(true);

    await chat.trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/agent-chat");
  });

  it("opens Chat after selecting a project", async () => {
    const project = {
      id: "project-1",
      name: "Workbench",
      folderPath: "/tmp/workbench",
    };
    projectPaths.value = [project.folderPath];
    currentProject.value = project;
    getProject.mockResolvedValue(project);
    selectProject.mockImplementation(async () => {
      currentProjectPath.value = project.folderPath;
    });

    const { wrapper, router } = await mountSidebar();
    await wrapper.find("[data-topic='projects'] button.nav-item").trigger("click");
    await flushPromises();

    expect(selectProject).toHaveBeenCalledWith(project.folderPath);
    expect(router.currentRoute.value.path).toBe(PROJECT_LANDING_ROUTE);
  });

  it("keeps Settings accessible as a collapsible footer topic", async () => {
    const { wrapper } = await mountSidebar("/settings");
    const settingsToggle = wrapper.find("[data-topic-toggle='settings']");
    const settingsLink = wrapper.find("[data-topic='settings'] a[href='/settings']");

    expect(settingsToggle.attributes("aria-expanded")).toBe("true");
    expect(settingsToggle.classes()).toContain("is-active");
    expect(settingsLink.attributes("aria-current")).toBe("page");
    expect(settingsLink.text()).toContain("Settings");
  });

  it("expands Workspace by default so project artifacts stay visible", async () => {
    const { wrapper } = await mountSidebar();
    expect(wrapper.find("[data-topic-toggle='workspace']").attributes("aria-expanded")).toBe("true");
    expect(wrapper.find("[data-topic-toggle='agents']").attributes("aria-expanded")).toBe("true");
    expect(wrapper.find("[data-topic-toggle='runtime']").attributes("aria-expanded")).toBe("false");
    expect(wrapper.find("[data-topic-toggle='settings']").attributes("aria-expanded")).toBe("false");
    expect(wrapper.find("a[href='/docs']").exists()).toBe(true);
    expect(wrapper.find("a[href='/agent-sessions']").exists()).toBe(true);
  });
});
