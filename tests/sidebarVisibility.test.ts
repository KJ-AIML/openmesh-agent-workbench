import { describe, it, expect, beforeEach } from "vitest";
import {
  SIDEBAR_VISIBLE_STORAGE_KEY,
  useSidebarVisibility,
  __resetSidebarVisibilityForTests,
} from "@/lib/useSidebarVisibility";

function memoryStorage(): Storage {
  const map = new Map<string, string>();
  return {
    get length() {
      return map.size;
    },
    clear() {
      map.clear();
    },
    getItem(key: string) {
      return map.has(key) ? map.get(key)! : null;
    },
    key(index: number) {
      return Array.from(map.keys())[index] ?? null;
    },
    removeItem(key: string) {
      map.delete(key);
    },
    setItem(key: string, value: string) {
      map.set(key, String(value));
    },
  };
}

describe("useSidebarVisibility", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: memoryStorage(),
    });
    __resetSidebarVisibilityForTests(true);
  });

  it("defaults to visible when nothing is stored", () => {
    const { sidebarVisible } = useSidebarVisibility();
    expect(sidebarVisible.value).toBe(true);
  });

  it("persists hide/show to localStorage", () => {
    const { sidebarVisible, hideSidebar, showSidebar } = useSidebarVisibility();
    hideSidebar();
    expect(sidebarVisible.value).toBe(false);
    expect(localStorage.getItem(SIDEBAR_VISIBLE_STORAGE_KEY)).toBe("0");

    showSidebar();
    expect(sidebarVisible.value).toBe(true);
    expect(localStorage.getItem(SIDEBAR_VISIBLE_STORAGE_KEY)).toBe("1");
  });

  it("toggle flips visibility", () => {
    const { sidebarVisible, toggleSidebar } = useSidebarVisibility();
    toggleSidebar();
    expect(sidebarVisible.value).toBe(false);
    toggleSidebar();
    expect(sidebarVisible.value).toBe(true);
  });

  it("hydrates collapsed preference from localStorage", () => {
    localStorage.setItem(SIDEBAR_VISIBLE_STORAGE_KEY, "0");
    __resetSidebarVisibilityForTests(true);
    const { sidebarVisible } = useSidebarVisibility();
    expect(sidebarVisible.value).toBe(false);
  });
});
