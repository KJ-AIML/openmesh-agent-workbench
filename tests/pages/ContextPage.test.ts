import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";

// Mock the context client.
vi.mock("@/lib/contextClient", () => ({
  refreshContext: vi.fn(),
  searchContext: vi.fn(),
  inspectContext: vi.fn(),
  getContextHealth: vi.fn(),
}));

// Mock useStore (mutable for test control).
const mockStore = {
  currentProject: { value: { id: "p1", name: "Test Project", folderPath: "/tmp/test" } as any },
  currentProjectPath: { value: "/tmp/test" as any },
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
});
