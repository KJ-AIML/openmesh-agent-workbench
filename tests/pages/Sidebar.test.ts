import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { createMemoryHistory, createRouter } from "vue-router";
import Sidebar from "@/components/Sidebar.vue";

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

  it("keeps every existing surface under a named topic", async () => {
    const { wrapper } = await mountSidebar();

    expect(wrapper.find("nav[aria-label='Workspace navigation']").exists()).toBe(true);
    expect(wrapper.findAll("[data-topic]").map((node) => node.attributes("data-topic"))).toEqual([
      "projects",
      "overview",
      "build",
      "agents",
      "network",
      "settings",
    ]);

    const links = wrapper.findAll("a").map((link) => link.attributes("href"));
    expect(links).toEqual([
      "/agent-chat",
      "/",
      "/context",
      "/sprint",
      "/docs",
      "/notes",
      "/canvas",
      "/agent-sessions",
      "/continuity",
      "/oauth",
      "/proxy-runtime",
      "/proxy-providers",
      "/usage",
      "/settings",
    ]);
  });

  it("toggles topic sub-navigation without changing route links", async () => {
    const { wrapper } = await mountSidebar();
    const buildToggle = wrapper.find("[data-topic-toggle='build']");
    const buildPanel = wrapper.find("#sidebar-topic-build");

    expect(buildToggle.attributes("aria-expanded")).toBe("false");
    expect(buildPanel.attributes("style")).toContain("display: none");

    await buildToggle.trigger("click");
    expect(buildToggle.attributes("aria-expanded")).toBe("true");
    expect(buildPanel.attributes("style") || "").not.toContain("display: none");
    expect(buildPanel.find("a[href='/docs']").exists()).toBe(true);

    await buildToggle.trigger("click");
    expect(buildToggle.attributes("aria-expanded")).toBe("false");
    expect(buildPanel.attributes("style")).toContain("display: none");
  });

  it("auto-expands the topic containing the active deep link", async () => {
    const { wrapper } = await mountSidebar("/proxy-runtime");
    const networkToggle = wrapper.find("[data-topic-toggle='network']");
    const networkPanel = wrapper.find("#sidebar-topic-network");

    expect(networkToggle.attributes("aria-expanded")).toBe("true");
    expect(networkToggle.classes()).toContain("is-active");
    expect(networkPanel.attributes("style") || "").not.toContain("display: none");
    expect(networkPanel.find("a[href='/proxy-runtime']").classes()).toContain("active");
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
    const overview = nav.find("[data-topic-toggle='overview']");

    const projectsTopic = nav.find("[data-topic='projects']");
    expect(projectsTopic.text()).toContain("Projects");
    expect(projectsTopic.text()).toContain("Add Project");
    expect(chat.exists()).toBe(true);
    expect(chat.element.compareDocumentPosition(overview.element) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    await chat.trigger("click");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/agent-chat");
  });

  it("keeps Settings accessible as a collapsible footer topic", async () => {
    const { wrapper } = await mountSidebar("/settings");
    const settingsToggle = wrapper.find("[data-topic-toggle='settings']");
    const settingsLink = wrapper.find("[data-topic='settings'] a[href='/settings']");

    expect(settingsToggle.attributes("aria-expanded")).toBe("true");
    expect(settingsToggle.classes()).toContain("is-active");
    expect(settingsLink.attributes("aria-current")).toBe("page");
  });
});
