import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick, ref } from "vue";

const mockQuery = ref<Record<string, string>>({});

vi.mock("vue-router", () => ({
  useRoute: () => ({
    get query() {
      return mockQuery.value;
    },
    path: "/",
  }),
  useRouter: () => ({ push: vi.fn(), replace: vi.fn() }),
}));

vi.mock("@/lib/scanConfiguredSessions", () => ({
  scanConfiguredSessions: vi.fn().mockResolvedValue([]),
}));

const mockStore = {
  currentProject: ref({
    id: "p1",
    name: "Test Project",
    folderPath: "/tmp/test",
  } as any),
  currentProjectPath: ref("/tmp/test" as any),
  settings: ref({
    sessionDirs: {
      codexEnabled: true,
      claudeCodeEnabled: true,
      opencodeEnabled: true,
      cursorEnabled: true,
      geminiEnabled: true,
      grokEnabled: true,
    },
  } as any),
  projectSprint: ref(null as any),
  projectTasks: ref([] as any[]),
  projectSessions: ref([] as any[]),
  projectDocs: ref([] as any[]),
  docsTree: ref([] as any[]),
  projectNotes: ref([] as any[]),
  createSprint: vi.fn(),
  updateSprint: vi.fn(),
  addTask: vi.fn(),
  updateTask: vi.fn(),
  deleteTask: vi.fn(),
  refreshDocs: vi.fn().mockResolvedValue(undefined),
  readDoc: vi.fn(),
  writeDoc: vi.fn(),
  deleteDoc: vi.fn(),
  createDoc: vi.fn(),
  createFolder: vi.fn(),
  refreshNotes: vi.fn().mockResolvedValue(undefined),
  readNote: vi.fn(),
  writeNote: vi.fn(),
  deleteNote: vi.fn(),
  createNote: vi.fn(),
  deleteSession: vi.fn(),
  toggleSessionFavorite: vi.fn(),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import SprintPage from "@/pages/SprintPage.vue";
import DocsPage from "@/pages/DocsPage.vue";
import NotesPage from "@/pages/NotesPage.vue";
import AgentSessionsPage from "@/pages/AgentSessionsPage.vue";

describe("Sprint / Docs / Notes / Sessions GUI smoke", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
    mockStore.currentProject.value = {
      id: "p1",
      name: "Test Project",
      folderPath: "/tmp/test",
    };
    mockStore.currentProjectPath.value = "/tmp/test";
    mockStore.projectSprint.value = null;
    mockStore.projectTasks.value = [];
    mockStore.projectSessions.value = [];
    mockStore.projectDocs.value = [];
    mockStore.docsTree.value = [];
    mockStore.projectNotes.value = [];
  });

  it("Sprint page mounts with create controls when empty", async () => {
    const wrapper = mount(SprintPage);
    await nextTick();
    expect(wrapper.text()).toContain("Sprint & Board");
    expect(wrapper.text()).toMatch(/Start|Create|Sprint/i);
    const input = wrapper.find('input[placeholder*="Sprint name"]');
    expect(input.exists()).toBe(true);
  });

  it("Sprint page shows board when sprint exists", async () => {
    mockStore.projectSprint.value = {
      id: "s1",
      name: "Sprint Alpha",
      status: "active",
    };
    mockStore.projectTasks.value = [
      { id: "t1", title: "Ship tests", status: "pending", priority: "medium" },
    ];
    const wrapper = mount(SprintPage);
    await nextTick();
    expect(wrapper.text()).toContain("Sprint Alpha");
    expect(wrapper.text()).toContain("Ship tests");
  });

  it("Docs page mounts with Docs heading and new-doc control", async () => {
    const wrapper = mount(DocsPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Docs");
    expect(mockStore.refreshDocs).toHaveBeenCalled();
    const newDoc = wrapper.find('button[title="New doc"]');
    expect(newDoc.exists()).toBe(true);
  });

  it("Notes page mounts with Notes heading and new-note control", async () => {
    const wrapper = mount(NotesPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Notes");
    expect(mockStore.refreshNotes).toHaveBeenCalled();
    // Empty list still shows the Notes rail header.
    expect(wrapper.find("h2").text()).toContain("Notes");
  });

  it("Sessions page mounts with scan control", async () => {
    const wrapper = mount(AgentSessionsPage);
    await flushPromises();
    await nextTick();
    expect(wrapper.text()).toContain("Agent Sessions");
    expect(wrapper.text()).toMatch(/Scan/i);
  });

  it("pages show no-project state when none selected", async () => {
    mockStore.currentProject.value = null;
    mockStore.currentProjectPath.value = null;

    const sprint = mount(SprintPage);
    await nextTick();
    expect(sprint.text()).toContain("No project selected");

    const docs = mount(DocsPage);
    await nextTick();
    expect(docs.text()).toContain("No project selected");

    const notes = mount(NotesPage);
    await nextTick();
    expect(notes.text()).toContain("No project selected");
  });
});
