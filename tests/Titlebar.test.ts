import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { nextTick, ref, type Ref } from "vue";
import { mount, flushPromises } from "@vue/test-utils";
import { createRouter, createMemoryHistory } from "vue-router";
import Titlebar from "@/components/Titlebar.vue";
import { DEFAULT_APPEARANCE, type AppearancePrefs } from "@/lib/appearance";

function cloneAppearance(
  overrides: Partial<AppearancePrefs> = {},
): AppearancePrefs {
  return {
    ...DEFAULT_APPEARANCE,
    ...overrides,
    topNavbarTabs: {
      ...DEFAULT_APPEARANCE.topNavbarTabs,
      ...(overrides.topNavbarTabs ?? {}),
    },
  };
}

const { settingsRef } = vi.hoisted(() => {
  // ref must be created inside hoisted (import order / TDZ).
  // eslint-disable-next-line @typescript-eslint/no-require-imports
  const { ref: hoistedRef } = require("vue") as typeof import("vue");
  return {
    settingsRef: hoistedRef({
      appearance: {
        theme: "dark",
        fontSize: "medium",
        density: "comfortable",
        topNavbarTabs: {
          chat: true,
          work: true,
          docs: true,
          sprint: true,
        },
      },
    }) as Ref<{ appearance: AppearancePrefs }>,
  };
});

vi.mock("@/lib/useStore", () => ({
  useStore: () => ({
    currentProject: ref(null),
    settings: settingsRef,
  }),
}));

vi.mock("@/lib/adapters/windowAdapter", () => ({
  minimizeWindow: vi.fn(),
  toggleMaximizeWindow: vi.fn(),
  closeWindow: vi.fn(),
  startWindowDrag: vi.fn(),
  isMaximized: vi.fn(async () => false),
}));

async function mountTitlebar(
  props: { clearanceForTrafficLights?: boolean } = {},
  initialPath = "/",
) {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: "/", component: { template: "<div />" } },
      { path: "/agent-chat", component: { template: "<div />" } },
      { path: "/docs", component: { template: "<div />" } },
      { path: "/sprint", component: { template: "<div />" } },
      { path: "/projects/new", component: { template: "<div />" } },
      { path: "/settings", component: { template: "<div />" } },
    ],
  });
  await router.push(initialPath);
  await router.isReady();
  const wrapper = mount(Titlebar, {
    props,
    global: { plugins: [router] },
  });
  return { wrapper, router };
}

describe("Titlebar macOS traffic-light clearance", () => {
  beforeEach(() => {
    settingsRef.value = { appearance: cloneAppearance() };
    (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ =
      true;
  });

  afterEach(() => {
    delete (window as unknown as { __OPENMESH_IS_MACOS__?: boolean })
      .__OPENMESH_IS_MACOS__;
  });

  it("adds tb--mac-traffic-clearance only when clearance prop is set", async () => {
    const without = await mountTitlebar({ clearanceForTrafficLights: false });
    expect(without.wrapper.find(".tb--mac").exists()).toBe(true);
    expect(without.wrapper.find(".tb--mac-traffic-clearance").exists()).toBe(
      false,
    );
    expect(without.wrapper.find(".tb__traffic-clearance").exists()).toBe(false);
    without.wrapper.unmount();

    const withClearance = await mountTitlebar({
      clearanceForTrafficLights: true,
    });
    expect(withClearance.wrapper.find(".tb--mac-traffic-clearance").exists()).toBe(
      true,
    );
    withClearance.wrapper.unmount();
  });

  it("places a flex spacer before the tab nav so Chat clears ooo", async () => {
    const { wrapper } = await mountTitlebar({
      clearanceForTrafficLights: true,
    });
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

    const style = getComputedStyle(spacer.element);
    const widthPx = parseFloat(style.flexBasis || style.width || "0");
    expect(spacer.classes()).toContain("tb__traffic-clearance");
    expect(widthPx === 0 || widthPx >= 90).toBe(true);

    expect(nav.text()).toContain("Chat");
    wrapper.unmount();
  });

  it("does not render traffic spacer when clearance is off", async () => {
    const { wrapper } = await mountTitlebar({
      clearanceForTrafficLights: false,
    });
    expect(wrapper.find(".tb__traffic-clearance").exists()).toBe(false);
    expect(wrapper.find(".tb__nav").exists()).toBe(true);
    wrapper.unmount();
  });
});

describe("Titlebar top navbar tab prefs", () => {
  beforeEach(() => {
    settingsRef.value = { appearance: cloneAppearance() };
  });

  it("hides unchecked hot tabs while keeping Projects available", async () => {
    settingsRef.value = {
      appearance: cloneAppearance({
        topNavbarTabs: {
          chat: true,
          work: false,
          docs: true,
          sprint: false,
        },
      }),
    };
    const { wrapper } = await mountTitlebar({}, "/agent-chat");
    await nextTick();
    const labels = wrapper
      .findAll(".tb__nav .tb__tab")
      .map((n) => n.text().replace(/\s+/g, " ").trim());
    expect(labels).toEqual(["Chat", "Docs"]);
    expect(wrapper.text()).toContain("Projects");
    wrapper.unmount();
  });

  it("navigates away when the current hot tab is hidden", async () => {
    const { wrapper, router } = await mountTitlebar({}, "/sprint");
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/sprint");

    settingsRef.value = {
      appearance: cloneAppearance({
        topNavbarTabs: {
          chat: true,
          work: false,
          docs: false,
          sprint: false,
        },
      }),
    };
    await nextTick();
    await flushPromises();
    expect(router.currentRoute.value.path).toBe("/agent-chat");
    wrapper.unmount();
  });
});
