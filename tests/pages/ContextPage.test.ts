import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick, ref } from "vue";

// Mock the context client.
vi.mock("@/lib/contextClient", () => ({
  refreshContext: vi.fn(),
  searchContext: vi.fn(),
  inspectContext: vi.fn(),
  getContextHealth: vi.fn(),
}));

// Mock useStore (mutable for test control).
const mockStore = {
  currentProject: ref({ id: "p1", name: "Test Project", folderPath: "/tmp/test" } as any),
  currentProjectPath: ref("/tmp/test" as any),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import ContextPage from "@/pages/ContextPage.vue";
import {
  refreshContext,
  searchContext,
  inspectContext,
  getContextHealth,
} from "@/lib/contextClient";

describe("ContextPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    (getContextHealth as any).mockResolvedValue({
      path: "/tmp/index",
      schema_version: 1,
      sqlite_version: "3.44.0",
      journal_mode: "wal",
      document_count: 5,
      fts_row_count: 5,
      wal_mode_effective: true,
      integrity_ok: true,
    });
  });

  it("renders header and search input", async () => {
    const wrapper = mount(ContextPage);
    await nextTick();
    expect(wrapper.text()).toContain("Context Search");
    expect(wrapper.find("input").exists()).toBe(true);
  });

  it("shows no-project state when no project is selected", async () => {
    mockStore.currentProject.value = null as any;
    mockStore.currentProjectPath.value = null as any;
    const wrapper = mount(ContextPage);
    await nextTick();
    expect(wrapper.text()).toContain("No project selected");
    // Restore for other tests.
    mockStore.currentProject.value = { id: "p1", name: "Test Project", folderPath: "/tmp/test" } as any;
    mockStore.currentProjectPath.value = "/tmp/test" as any;
  });

  it("shows healthy status when index is healthy", async () => {
    const wrapper = mount(ContextPage);
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Healthy");
    expect(wrapper.text()).toContain("5 docs indexed");
  });

  it("runs search and displays results", async () => {
    (searchContext as any).mockResolvedValue([
      {
        document_id: "d1",
        source_id: "s1",
        source_kind: "doc",
        project_id: "p1",
        canonical_ref: "openmesh://project/p1/doc/readme.md",
        title: "Readme",
        snippet: "OpenMesh is a workbench",
        sensitivity: "private",
        freshness_state: "fresh",
        observed_at: "2026-07-05T03:00:00.000Z",
      },
    ]);
    const wrapper = mount(ContextPage);
    await nextTick();
    const input = wrapper.find("input");
    await input.setValue("OpenMesh");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Readme");
    expect(wrapper.text()).toContain("OpenMesh is a workbench");
  });

  it("opens inspector when result is clicked", async () => {
    (searchContext as any).mockResolvedValue([
      {
        document_id: "d1",
        source_id: "s1",
        source_kind: "doc",
        project_id: "p1",
        canonical_ref: "openmesh://project/p1/doc/readme.md",
        title: "Readme",
        snippet: "snippet",
        sensitivity: "private",
        freshness_state: "fresh",
        observed_at: "2026-07-05T03:00:00.000Z",
      },
    ]);
    (inspectContext as any).mockResolvedValue({
      document_id: "d1",
      source_id: "s1",
      source_kind: "doc",
      project_id: "p1",
      canonical_ref: "openmesh://project/p1/doc/readme.md",
      title: "Readme",
      text: "Full readme content here",
      sensitivity: "private",
      agent_context_enabled: false,
      freshness_state: "fresh",
      observed_at: "2026-07-05T03:00:00.000Z",
      source_updated_at: "2026-07-04T14:00:00.000Z",
      indexed_at: "2026-07-05T03:00:00.000Z",
      metadata_json: null,
    });
    const wrapper = mount(ContextPage);
    await nextTick();
    const input = wrapper.find("input");
    await input.setValue("test");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    // Click the result.
    const resultItem = wrapper.find(".cursor-pointer");
    await resultItem.trigger("click");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Preview");
    expect(wrapper.text()).toContain("Full readme content here");
  });

  it("does not render secret text in inspector", async () => {
    (searchContext as any).mockResolvedValue([
      {
        document_id: "s1",
        source_id: "ss1",
        source_kind: "doc",
        project_id: "p1",
        canonical_ref: "openmesh://project/p1/doc/secret.md",
        title: "Secret Doc",
        snippet: "snippet",
        sensitivity: "secret",
        freshness_state: "fresh",
        observed_at: "2026-07-05T03:00:00.000Z",
      },
    ]);
    (inspectContext as any).mockResolvedValue({
      document_id: "s1",
      source_id: "ss1",
      source_kind: "doc",
      project_id: "p1",
      canonical_ref: "openmesh://project/p1/doc/secret.md",
      title: "Secret Doc",
      text: "",
      sensitivity: "secret",
      agent_context_enabled: false,
      freshness_state: "fresh",
      observed_at: "2026-07-05T03:00:00.000Z",
      source_updated_at: null,
      indexed_at: "2026-07-05T03:00:00.000Z",
      metadata_json: null,
    });
    const wrapper = mount(ContextPage);
    await nextTick();
    const input = wrapper.find("input");
    await input.setValue("secret");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    const resultItem = wrapper.find(".cursor-pointer");
    await resultItem.trigger("click");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("[secret content hidden]");
  });

  it("refresh shows COMPLETE status", async () => {
    (refreshContext as any).mockResolvedValue({
      project_id: "p1",
      status: "COMPLETE",
      started_at: "2026-07-05T03:00:00.000Z",
      completed_at: "2026-07-05T03:00:01.000Z",
      discovered: 10,
      indexed: 4,
      updated: 2,
      unchanged: 4,
      removed: 0,
      skipped: 0,
      failed: 0,
      receipts: [],
    });
    const wrapper = mount(ContextPage);
    await nextTick();
    const buttons = wrapper.findAll("button");
    const refreshBtn = buttons.find((b) => b.text().includes("Refresh Context"));
    expect(refreshBtn).toBeTruthy();
    await refreshBtn!.trigger("click");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("COMPLETE");
    expect(wrapper.text()).toContain("4 indexed");
  });

  it("shows no-results state when search returns empty", async () => {
    (searchContext as any).mockResolvedValue([]);
    const wrapper = mount(ContextPage);
    await nextTick();
    const input = wrapper.find("input");
    await input.setValue("nonexistent query");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("No results for");
    expect(wrapper.text()).toContain("nonexistent query");
  });

  it("kind filter passes selected kind to search", async () => {
    (searchContext as any).mockResolvedValue([
      {
        document_id: "d1",
        source_id: "s1",
        source_kind: "doc",
        project_id: "p1",
        canonical_ref: "openmesh://project/p1/doc/readme.md",
        title: "Doc Result",
        snippet: "doc snippet",
        sensitivity: "private",
        freshness_state: "fresh",
        observed_at: "2026-07-05T03:00:00.000Z",
      },
    ]);
    const wrapper = mount(ContextPage);
    await nextTick();
    // Find and click the "Docs" filter button.
    const badges = wrapper.findAll(".badge");
    const docsBadge = badges.find((b) => b.text().includes("Docs"));
    expect(docsBadge).toBeTruthy();
    await docsBadge!.trigger("click");
    await nextTick();
    // Now trigger search.
    const input = wrapper.find("input");
    await input.setValue("test");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    // Verify searchContext was called with kinds: ["doc"].
    expect(searchContext).toHaveBeenCalledWith(
      "/tmp/test",
      "test",
      expect.objectContaining({ kinds: ["doc"] }),
    );
    // Verify result is displayed.
    expect(wrapper.text()).toContain("Doc Result");
  });

  it("refresh shows PARTIAL status with failure count", async () => {
    (refreshContext as any).mockResolvedValue({
      project_id: "p1",
      status: "PARTIAL",
      started_at: "2026-07-05T03:00:00.000Z",
      completed_at: "2026-07-05T03:00:02.000Z",
      discovered: 15,
      indexed: 8,
      updated: 3,
      unchanged: 2,
      removed: 0,
      skipped: 0,
      failed: 2,
      receipts: [],
    });
    const wrapper = mount(ContextPage);
    await nextTick();
    const buttons = wrapper.findAll("button");
    const refreshBtn = buttons.find((b) => b.text().includes("Refresh Context"));
    await refreshBtn!.trigger("click");
    await nextTick();
    await nextTick();
    // PARTIAL status must be visible.
    expect(wrapper.text()).toContain("PARTIAL");
    // Must NOT claim COMPLETE.
    expect(wrapper.text()).not.toContain("COMPLETE");
    // Failed count must be shown.
    expect(wrapper.text()).toContain("2 failed");
    // Successful counts must also be shown.
    expect(wrapper.text()).toContain("8 indexed");
  });

  it("project switch clears stale results and inspector", async () => {
    // Set up initial search results.
    (searchContext as any).mockResolvedValue([
      {
        document_id: "d1",
        source_id: "s1",
        source_kind: "doc",
        project_id: "p1",
        canonical_ref: "openmesh://project/p1/doc/readme.md",
        title: "Project A Doc",
        snippet: "snippet A",
        sensitivity: "private",
        freshness_state: "fresh",
        observed_at: "2026-07-05T03:00:00.000Z",
      },
    ]);
    (inspectContext as any).mockResolvedValue({
      document_id: "d1",
      source_id: "s1",
      source_kind: "doc",
      project_id: "p1",
      canonical_ref: "openmesh://project/p1/doc/readme.md",
      title: "Project A Doc",
      text: "Full content from Project A",
      sensitivity: "private",
      agent_context_enabled: false,
      freshness_state: "fresh",
      observed_at: "2026-07-05T03:00:00.000Z",
      source_updated_at: null,
      indexed_at: "2026-07-05T03:00:00.000Z",
      metadata_json: null,
    });
    const wrapper = mount(ContextPage);
    await nextTick();
    // Search and select result.
    const input = wrapper.find("input");
    await input.setValue("test");
    await input.trigger("keyup.enter");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Project A Doc");
    // Click result to open inspector.
    const resultItem = wrapper.find(".cursor-pointer");
    await resultItem.trigger("click");
    await nextTick();
    await nextTick();
    expect(wrapper.text()).toContain("Full content from Project A");
    expect(wrapper.text()).toContain("Preview");
    // Now switch project.
    mockStore.currentProject.value = { id: "p2", name: "Project B", folderPath: "/tmp/test-b" } as any;
    mockStore.currentProjectPath.value = "/tmp/test-b" as any;
    // Wait for watcher to fire and state to clear.
    await nextTick();
    await nextTick();
    await nextTick();
    await nextTick();
    await nextTick();
    // Project A results must be cleared.
    expect(wrapper.text()).not.toContain("Project A Doc");
    // Inspector must be cleared.
    expect(wrapper.text()).not.toContain("Full content from Project A");
    expect(wrapper.text()).not.toContain("Preview");
    // Restore for other tests.
    mockStore.currentProject.value = { id: "p1", name: "Test Project", folderPath: "/tmp/test" } as any;
    mockStore.currentProjectPath.value = "/tmp/test" as any;
  });
});
