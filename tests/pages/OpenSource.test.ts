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

// Mock vue-router.
const mockQuery = ref<Record<string, string | undefined>>({});
const mockRouterPush = vi.fn();
vi.mock("vue-router", () => ({
  useRoute: () => ({ query: mockQuery.value }),
  useRouter: () => ({ push: mockRouterPush }),
}));

// Mock useStore (mutable for test control).
const mockStore = {
  currentProject: ref({ id: "p1", name: "Project A", folderPath: "/tmp/project-a" } as any),
  currentProjectPath: ref("/tmp/project-a" as any),
};

vi.mock("@/lib/useStore", () => ({
  useStore: () => mockStore,
}));

import ContextPage from "@/pages/ContextPage.vue";
import {
  searchContext,
  inspectContext,
  getContextHealth,
} from "@/lib/contextClient";

describe("ContextPage Open Source", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQuery.value = {};
    mockRouterPush.mockClear();
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

  function makeResult(documentId: string, kind: string, canonicalRef: string, title = "Test Doc") {
    return {
      document_id: documentId,
      source_id: "s1",
      source_kind: kind,
      project_id: "p1",
      canonical_ref: canonicalRef,
      title,
      snippet: "snippet",
      sensitivity: "private",
      freshness_state: "fresh",
      observed_at: "2026-07-05T03:00:00.000Z",
    };
  }

  async function openInspectorFor(result: any) {
    (searchContext as any).mockResolvedValue([result]);
    (inspectContext as any).mockResolvedValue({
      document_id: result.document_id,
      source_id: result.source_id,
      source_kind: result.source_kind,
      project_id: result.project_id,
      canonical_ref: result.canonical_ref,
      title: result.title,
      text: "Full content",
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
    return wrapper;
  }

  it("shows Open Source button for doc kind", async () => {
    const ref_str = "openmesh://project/p1/doc/readme.md";
    const wrapper = await openInspectorFor(makeResult("d1", "doc", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    expect(openSourceBtn).toBeTruthy();
  });

  it("shows Open Source button for note kind", async () => {
    const ref_str = "openmesh://project/p1/note/meeting-notes.md";
    const wrapper = await openInspectorFor(makeResult("n1", "note", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    expect(openSourceBtn).toBeTruthy();
  });

  it("does NOT show Open Source button for recent kind", async () => {
    const ref_str = "openmesh://project/p1/recent/r1";
    const wrapper = await openInspectorFor(makeResult("r1", "recent", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    expect(openSourceBtn).toBeFalsy();
  });

  it("clicking Open Source on doc navigates to /docs with file query param", async () => {
    const ref_str = "openmesh://project/p1/doc/readme.md";
    const wrapper = await openInspectorFor(makeResult("d1", "doc", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    await openSourceBtn!.trigger("click");
    await nextTick();
    expect(mockRouterPush).toHaveBeenCalledWith({
      path: "/docs",
      query: { file: "readme.md" },
    });
  });

  it("clicking Open Source on note navigates to /notes with file query param", async () => {
    const ref_str = "openmesh://project/p1/note/meeting-notes.md";
    const wrapper = await openInspectorFor(makeResult("n1", "note", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    await openSourceBtn!.trigger("click");
    await nextTick();
    expect(mockRouterPush).toHaveBeenCalledWith({
      path: "/notes",
      query: { file: "meeting-notes.md" },
    });
  });

  it("rejects cross-project source open with error", async () => {
    const ref_str = "openmesh://project/OTHER_PROJECT/doc/readme.md";
    const wrapper = await openInspectorFor(makeResult("d1", "doc", ref_str));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    await openSourceBtn!.trigger("click");
    await nextTick();
    expect(mockRouterPush).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("another project");
  });

  it("does NOT show Open Source button for invalid canonical ref", async () => {
    const wrapper = await openInspectorFor(makeResult("d1", "doc", "not-a-canonical-ref"));
    const buttons = wrapper.findAll("button");
    const openSourceBtn = buttons.find((b) => b.text().includes("Open Source"));
    // Invalid canonical ref means canOpenSource returns false.
    expect(openSourceBtn).toBeFalsy();
  });
});
