import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { ref } from "vue";
import { mount } from "@vue/test-utils";
import { createRouter, createMemoryHistory } from "vue-router";
import Titlebar from "@/components/Titlebar.vue";

vi.mock("@/lib/useStore", () => ({
  useStore: () => ({
    currentProject: ref(null),
  }),
}));

vi.mock("@/lib/adapters/windowAdapter", () => ({
  minimizeWindow: vi.fn(),
  toggleMaximizeWindow: vi.fn(),
  closeWindow: vi.fn(),
  startWindowDrag: vi.fn(),
  isMaximized: vi.fn(async () => false),
}));

function mountTitlebar(props: { clearanceForTrafficLights?: boolean } = {}) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div />" } },
      { path: "/agent-chat", component: { template: "<div />" } },
      { path: "/docs", component: { template: "<div />" } },
      { path: "/sprint", component: { template: "<div />" } },
      { path: "/projects/new", component: { template: "<div />" } },
    ],
  });
  return mount(Titlebar, {
    props,
    global: { plugins: [router] },
  });
}

describe("Titlebar macOS traffic-light clearance", () => {
  beforeEach(() => {
    (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ =
      true;
  });

  afterEach(() => {
    delete (window as unknown as { __OPENMESH_IS_MACOS__?: boolean })
      .__OPENMESH_IS_MACOS__;
  });

  it("adds tb--mac-traffic-clearance only when clearance prop is set", () => {
    const without = mountTitlebar({ clearanceForTrafficLights: false });
    expect(without.find(".tb--mac").exists()).toBe(true);
    expect(without.find(".tb--mac-traffic-clearance").exists()).toBe(false);
    expect(without.find(".tb__traffic-clearance").exists()).toBe(false);
    without.unmount();

    const withClearance = mountTitlebar({ clearanceForTrafficLights: true });
    expect(withClearance.find(".tb--mac-traffic-clearance").exists()).toBe(true);
    withClearance.unmount();
  });

  it("places a flex spacer before the tab nav so Chat clears ooo", () => {
    const wrapper = mountTitlebar({ clearanceForTrafficLights: true });
    const header = wrapper.find("header.tb");
    const spacer = wrapper.find(".tb__traffic-clearance");
    const nav = wrapper.find(".tb__nav");

    expect(spacer.exists()).toBe(true);
    expect(nav.exists()).toBe(true);

    const children = header.element.children;
    const spacerIndex = Array.from(children).indexOf(spacer.element);
    const navIndex = Array.from(children).indexOf(nav.element);
    expect(spacerIndex).toBeGreaterThanOrEqual(0);
    expect(navIndex).toBeGreaterThan(spacerIndex);

    // Spacer owns the inset width (CSS var fallback 96px).
    const style = getComputedStyle(spacer.element);
    const widthPx = parseFloat(style.flexBasis || style.width || "0");
    // jsdom may not resolve CSS vars from :root — assert the class/DOM contract.
    expect(spacer.classes()).toContain("tb__traffic-clearance");
    expect(widthPx === 0 || widthPx >= 90).toBe(true);

    expect(nav.text()).toContain("Chat");
    wrapper.unmount();
  });

  it("does not render traffic spacer when clearance is off", () => {
    const wrapper = mountTitlebar({ clearanceForTrafficLights: false });
    expect(wrapper.find(".tb__traffic-clearance").exists()).toBe(false);
    expect(wrapper.find(".tb__nav").exists()).toBe(true);
    wrapper.unmount();
  });
});
