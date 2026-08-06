import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ComposerStatusBar from "@/components/chat/ComposerStatusBar.vue";
import { createSessionRun } from "@/lib/agentChat/sessionRuns";

describe("ComposerStatusBar", () => {
  it("shows quiet Terminal + Canvas icons when idle (no Working, no badge)", () => {
    const wrapper = mount(ComposerStatusBar, {
      props: {
        workingCount: 0,
        terminalRuns: [],
        showCanvas: true,
      },
    });
    expect(wrapper.find('[data-testid="composer-status-bar"]').exists()).toBe(
      true,
    );
    expect(wrapper.find('[data-testid="composer-status-working"]').exists()).toBe(
      false,
    );
    const term = wrapper.find('[data-testid="composer-status-terminal"]');
    expect(term.exists()).toBe(true);
    expect(term.attributes("aria-label")).toMatch(/Terminals/i);
    expect(
      wrapper.find('[data-testid="composer-status-terminal-badge"]').exists(),
    ).toBe(false);
    expect(wrapper.find('[data-testid="composer-status-canvas"]').exists()).toBe(
      true,
    );
    expect(wrapper.text()).not.toMatch(/0 Terminal/);
  });

  it("shows working + terminal badge and toggles panel", async () => {
    const run = createSessionRun({
      id: "term:1",
      kind: "terminal",
      title: "Verify",
      command: "npm run typecheck",
      toolId: "verify",
    });
    const wrapper = mount(ComposerStatusBar, {
      props: {
        workingCount: 2,
        workingLabel: "grep src",
        terminalRuns: [run],
        shellTabCount: 0,
        showCanvas: true,
      },
    });

    const working = wrapper.find('[data-testid="composer-status-working"]');
    expect(working.exists()).toBe(true);
    expect(working.attributes("title")).toMatch(/grep src/);
    expect(
      wrapper.find('[data-testid="composer-status-terminal-badge"]').text(),
    ).toBe("1");
    expect(wrapper.find('[data-testid="composer-status-canvas"]').exists()).toBe(
      true,
    );

    await wrapper.find('[data-testid="composer-status-terminal"]').trigger("click");
    expect(wrapper.emitted("toggle-terminal-panel")).toBeTruthy();
  });

  it("badges shell tab count when no runs", () => {
    const wrapper = mount(ComposerStatusBar, {
      props: {
        workingCount: 0,
        terminalRuns: [],
        shellTabCount: 2,
        showCanvas: true,
      },
    });
    expect(
      wrapper.find('[data-testid="composer-status-terminal-badge"]').text(),
    ).toBe("2");
  });

  it("emits open-canvas", async () => {
    const wrapper = mount(ComposerStatusBar, {
      props: {
        workingCount: 0,
        terminalRuns: [],
        showCanvas: true,
      },
    });
    await wrapper.find('[data-testid="composer-status-canvas"]').trigger("click");
    expect(wrapper.emitted("open-canvas")).toBeTruthy();
  });
});
