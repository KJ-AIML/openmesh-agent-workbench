import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { defineComponent, h } from "vue";
import ChatTerminalPanel from "@/components/chat/ChatTerminalPanel.vue";
import { createShellTab } from "@/lib/agentChat/shellTabs";
import { createSessionRun } from "@/lib/agentChat/sessionRuns";

vi.mock("@/components/chat/EmbeddedTerminal.vue", () => ({
  default: defineComponent({
    name: "EmbeddedTerminalStub",
    props: {
      sessionId: String,
      cwd: String,
      active: Boolean,
    },
    setup(props) {
      return () =>
        h("div", {
          "data-testid": `embedded-term-${props.sessionId}`,
          "data-active": String(props.active),
        });
    },
  }),
}));

function ensureLocalStorage() {
  const store = new Map<string, string>();
  const api: Storage = {
    get length() {
      return store.size;
    },
    clear() {
      store.clear();
    },
    getItem(key: string) {
      return store.has(key) ? store.get(key)! : null;
    },
    key(index: number) {
      return [...store.keys()][index] ?? null;
    },
    removeItem(key: string) {
      store.delete(key);
    },
    setItem(key: string, value: string) {
      store.set(key, String(value));
    },
  };
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: api,
  });
  return api;
}

describe("ChatTerminalPanel", () => {
  beforeEach(() => {
    ensureLocalStorage().clear();
  });

  it("renders tabs, + control, embedded host, and emits new-tab", async () => {
    const tab = createShellTab({
      id: "shell-1",
      cwd: "/tmp/proj",
      label: "zsh",
      status: "open",
    });
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: true,
        tabs: [tab],
        activeTabId: tab.id,
        terminalRuns: [],
        cwdLabel: "/tmp/proj",
      },
    });
    expect(wrapper.find('[data-testid="chat-terminal-panel"]').exists()).toBe(
      true,
    );
    expect(wrapper.text()).toContain("zsh");
    expect(wrapper.text()).not.toMatch(/external shell/i);
    expect(wrapper.find('[data-testid="embedded-term-shell-1"]').exists()).toBe(
      true,
    );
    expect(wrapper.find('[data-testid="term-open-external"]').exists()).toBe(
      true,
    );
    await wrapper.find('[data-testid="term-tab-add"]').trigger("click");
    expect(wrapper.emitted("new-tab")).toBeTruthy();
  });

  it("docks to the right by default with a vertical resize handle", () => {
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: true,
        tabs: [],
        activeTabId: null,
        terminalRuns: [],
        cwdLabel: "/tmp",
      },
    });
    const panel = wrapper.find('[data-testid="chat-terminal-panel"]');
    expect(panel.attributes("data-dock")).toBe("right");
    expect(panel.classes()).toContain("term-panel--dock-right");
    const handle = wrapper.find('[data-testid="term-panel-resize"]');
    expect(handle.exists()).toBe(true);
    expect(handle.attributes("aria-orientation")).toBe("vertical");
    expect((panel.element as HTMLElement).style.width).toMatch(/px$/);
  });

  it("toggles dock between right and bottom and persists preference", async () => {
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: true,
        tabs: [],
        activeTabId: null,
        terminalRuns: [],
        cwdLabel: "/tmp",
      },
    });
    await wrapper.find('[data-testid="term-dock-toggle"]').trigger("click");
    expect(wrapper.emitted("update:dock")?.[0]).toEqual(["bottom"]);
    expect(
      wrapper.find('[data-testid="chat-terminal-panel"]').attributes("data-dock"),
    ).toBe("bottom");
    expect(localStorage.getItem("openmesh.chat.terminal.dock")).toBe("bottom");
    expect(
      wrapper.find('[data-testid="term-panel-resize"]').attributes(
        "aria-orientation",
      ),
    ).toBe("horizontal");

    await wrapper.find('[data-testid="term-dock-toggle"]').trigger("click");
    expect(wrapper.emitted("update:dock")?.[1]).toEqual(["right"]);
    expect(localStorage.getItem("openmesh.chat.terminal.dock")).toBe("right");
  });

  it("resizes width via drag handle and persists", async () => {
    const wrapper = mount(ChatTerminalPanel, {
      attachTo: document.body,
      props: {
        open: true,
        tabs: [],
        activeTabId: null,
        terminalRuns: [],
        cwdLabel: "/tmp",
      },
    });
    const parent = wrapper.element.parentElement!;
    Object.defineProperty(parent, "clientWidth", {
      configurable: true,
      value: 1000,
    });
    Object.defineProperty(parent, "clientHeight", {
      configurable: true,
      value: 800,
    });

    const handle = wrapper.find('[data-testid="term-panel-resize"]');
    await handle.trigger("pointerdown", { button: 0, clientX: 600, clientY: 100 });
    window.dispatchEvent(
      new PointerEvent("pointermove", { clientX: 500, clientY: 100 }),
    );
    window.dispatchEvent(
      new PointerEvent("pointerup", { clientX: 500, clientY: 100 }),
    );
    await wrapper.vm.$nextTick();

    const panel = wrapper.find('[data-testid="chat-terminal-panel"]');
    // Drag left by 100px from default 380 → 480
    expect((panel.element as HTMLElement).style.width).toBe("480px");
    expect(localStorage.getItem("openmesh.chat.terminal.width")).toBe("480");
    wrapper.unmount();
  });

  it("emits close-tab and open-external", async () => {
    const tab = createShellTab({
      id: "shell-1",
      cwd: "/tmp/proj",
      label: "zsh",
      status: "open",
    });
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: true,
        tabs: [tab],
        activeTabId: tab.id,
        terminalRuns: [],
        cwdLabel: "/tmp/proj",
      },
    });
    await wrapper.find('[data-testid="term-tab-close"]').trigger("click");
    expect(wrapper.emitted("close-tab")?.[0]).toEqual(["shell-1"]);
    await wrapper.find('[data-testid="term-open-external"]').trigger("click");
    expect(wrapper.emitted("open-external")).toBeTruthy();
  });

  it("lists session runs", async () => {
    const run = createSessionRun({
      id: "term:1",
      kind: "terminal",
      title: "Verify",
      command: "npm run typecheck",
    });
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: true,
        tabs: [],
        activeTabId: null,
        terminalRuns: [run],
        cwdLabel: "/tmp",
      },
    });
    expect(wrapper.text()).toContain("Verify");
    expect(wrapper.text()).toContain("npm run typecheck");
    await wrapper.find(".term-panel__run-btn").trigger("click");
    expect(wrapper.emitted("select-run")?.[0]?.[0]).toMatchObject({
      id: "term:1",
    });
  });

  it("stays mounted but hidden when closed", () => {
    const wrapper = mount(ChatTerminalPanel, {
      props: {
        open: false,
        tabs: [],
        activeTabId: null,
        terminalRuns: [],
        cwdLabel: "/tmp",
      },
    });
    const panel = wrapper.find('[data-testid="chat-terminal-panel"]');
    expect(panel.exists()).toBe(true);
    // v-show keeps the node; display:none hides it (VTU isVisible is flaky in happy-dom).
    expect((panel.element as HTMLElement).style.display).toBe("none");
  });
});
