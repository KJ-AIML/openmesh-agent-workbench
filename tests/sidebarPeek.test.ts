import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { defineComponent, nextTick } from "vue";
import { mount } from "@vue/test-utils";
import {
  SIDEBAR_PEEK_HIDE_DELAY_MS,
  SIDEBAR_PEEK_SHOW_DELAY_MS,
  useSidebarPeek,
} from "@/lib/useSidebarPeek";

function mountPeek(options?: { showDelayMs?: number; hideDelayMs?: number }) {
  const Comp = defineComponent({
    setup() {
      return useSidebarPeek(options);
    },
    template: "<div />",
  });
  return mount(Comp);
}

describe("useSidebarPeek", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("opens after show delay on enter", async () => {
    const wrapper = mountPeek();
    const vm = wrapper.vm as unknown as ReturnType<typeof useSidebarPeek>;

    expect(vm.sidebarPeek).toBe(false);
    vm.onPeekEnter();
    expect(vm.sidebarPeek).toBe(false);

    vi.advanceTimersByTime(SIDEBAR_PEEK_SHOW_DELAY_MS - 1);
    expect(vm.sidebarPeek).toBe(false);

    vi.advanceTimersByTime(1);
    await nextTick();
    expect(vm.sidebarPeek).toBe(true);
    wrapper.unmount();
  });

  it("closes after hide delay on leave", async () => {
    const wrapper = mountPeek();
    const vm = wrapper.vm as unknown as ReturnType<typeof useSidebarPeek>;

    vm.onPeekEnter();
    vi.advanceTimersByTime(SIDEBAR_PEEK_SHOW_DELAY_MS);
    expect(vm.sidebarPeek).toBe(true);

    vm.onPeekLeave();
    vi.advanceTimersByTime(SIDEBAR_PEEK_HIDE_DELAY_MS - 1);
    expect(vm.sidebarPeek).toBe(true);

    vi.advanceTimersByTime(1);
    expect(vm.sidebarPeek).toBe(false);
    wrapper.unmount();
  });

  it("cancels pending hide when re-entering (zone ↔ sidebar bridge)", async () => {
    const wrapper = mountPeek();
    const vm = wrapper.vm as unknown as ReturnType<typeof useSidebarPeek>;

    vm.onPeekEnter();
    vi.advanceTimersByTime(SIDEBAR_PEEK_SHOW_DELAY_MS);
    expect(vm.sidebarPeek).toBe(true);

    vm.onPeekLeave();
    vi.advanceTimersByTime(SIDEBAR_PEEK_HIDE_DELAY_MS / 2);
    vm.onPeekEnter();
    vi.advanceTimersByTime(SIDEBAR_PEEK_HIDE_DELAY_MS);
    expect(vm.sidebarPeek).toBe(true);
    wrapper.unmount();
  });

  it("closePeek clears timers and collapses immediately", async () => {
    const wrapper = mountPeek();
    const vm = wrapper.vm as unknown as ReturnType<typeof useSidebarPeek>;

    vm.onPeekEnter();
    vi.advanceTimersByTime(SIDEBAR_PEEK_SHOW_DELAY_MS);
    expect(vm.sidebarPeek).toBe(true);

    vm.closePeek();
    expect(vm.sidebarPeek).toBe(false);
    wrapper.unmount();
  });

  it("does not persist anything to localStorage", async () => {
    const setItem = vi.fn();
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: () => null,
        setItem,
        removeItem: () => {},
        clear: () => {},
        key: () => null,
        length: 0,
      },
    });

    const wrapper = mountPeek({ showDelayMs: 0, hideDelayMs: 0 });
    const vm = wrapper.vm as unknown as ReturnType<typeof useSidebarPeek>;
    vm.onPeekEnter();
    vi.advanceTimersByTime(0);
    expect(vm.sidebarPeek).toBe(true);
    expect(setItem).not.toHaveBeenCalled();
    wrapper.unmount();
  });
});
