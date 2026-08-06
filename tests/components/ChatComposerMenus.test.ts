import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import { nextTick } from "vue";

vi.mock("@/lib/store", () => ({
  store: {
    listDocs: vi.fn(async () => [{ name: "guide.md", path: "guide.md" }]),
    listNotes: vi.fn(async () => [{ name: "todo.md", path: "todo.md" }]),
  },
}));

vi.mock("@/lib/agentEngineClient", () => ({
  runAgentWorkspaceTool: vi.fn(async () => "dir: .\n- src/\n- README.md\n"),
}));

import ChatComposer from "@/components/chat/ChatComposer.vue";

describe("ChatComposer rich menus", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows rich slash rows with description", async () => {
    const wrapper = mount(ChatComposer, {
      props: { busy: false, mode: "ask" },
    });
    await wrapper.find('[data-testid="composer-commands"]').trigger("click");
    await nextTick();
    const menu = wrapper.find('[data-testid="composer-slash-menu"]');
    expect(menu.exists()).toBe(true);
    expect(menu.text()).toContain("/pilot");
    expect(menu.text()).toMatch(/Enterprise pilot|pilot/i);
    expect(menu.text()).toContain("/read");
    expect(menu.find(".chat-composer__rich-desc").exists()).toBe(true);
  });

  it("filters slash menu while typing /", async () => {
    const wrapper = mount(ChatComposer, {
      props: { busy: false, mode: "ask" },
    });
    const ta = wrapper.find("textarea");
    await ta.setValue("/pil");
    await ta.trigger("input");
    await nextTick();
    const menu = wrapper.find('[data-testid="composer-slash-menu"]');
    expect(menu.exists()).toBe(true);
    expect(menu.text()).toContain("/pilot");
  });

  it("opens @ mention menu with supported context", async () => {
    const wrapper = mount(ChatComposer, {
      props: {
        busy: false,
        mode: "ask",
        projectPath: "/tmp/test",
        projectName: "Test Project",
        showCanvas: true,
        shellTabs: [],
        terminalRuns: [],
      },
    });
    const ta = wrapper.find("textarea");
    await ta.setValue("@");
    await ta.trigger("input");
    await flushPromises();
    await nextTick();
    const menu = wrapper.find('[data-testid="composer-mention-menu"]');
    expect(menu.exists()).toBe(true);
    expect(menu.text()).toContain("Test Project");
    expect(menu.text()).toContain("Canvas");
    expect(menu.text()).toContain("README.md");
    expect(menu.text()).not.toMatch(/Browser|Chats from Cursor/i);
  });

  it("keyboard selects slash item with Enter", async () => {
    const wrapper = mount(ChatComposer, {
      props: { busy: false, mode: "ask" },
    });
    const ta = wrapper.find("textarea");
    await ta.setValue("/");
    await ta.trigger("input");
    await nextTick();
    await ta.trigger("keydown", { key: "Enter" });
    await nextTick();
    // First starter is /pilot
    expect((ta.element as HTMLTextAreaElement).value).toMatch(/^\/pilot/);
  });
});
