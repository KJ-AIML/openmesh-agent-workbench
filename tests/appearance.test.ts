import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  APPEARANCE_STORAGE_KEY,
  applyAppearance,
  cacheAppearance,
  enabledTopNavbarTabs,
  firstVisibleTopNavbarPath,
  normalizeAppearance,
  normalizeTopNavbarTabs,
  readCachedAppearance,
  resolveThemeMode,
  systemPrefersDark,
  topNavbarTabForPath,
} from "@/lib/appearance";

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

function mockRoot() {
  const dataset: Record<string, string | undefined> = {};
  const style = { colorScheme: "" };
  const classes = new Set<string>();
  return {
    classList: {
      toggle(token: string, force?: boolean) {
        if (force === true) classes.add(token);
        else if (force === false) classes.delete(token);
        else if (classes.has(token)) classes.delete(token);
        else classes.add(token);
      },
      contains(token: string) {
        return classes.has(token);
      },
    },
    dataset,
    style,
  };
}

describe("appearance", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: memoryStorage(),
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("normalizes missing and invalid fields to defaults", () => {
    expect(normalizeAppearance(null)).toEqual({
      theme: "dark",
      fontSize: "medium",
      density: "comfortable",
      topNavbarTabs: {
        chat: true,
        work: true,
        docs: true,
        sprint: true,
      },
    });
    expect(
      normalizeAppearance({
        theme: "neon" as any,
        fontSize: "huge" as any,
        density: "dense" as any,
      }),
    ).toEqual({
      theme: "dark",
      fontSize: "medium",
      density: "comfortable",
      topNavbarTabs: {
        chat: true,
        work: true,
        docs: true,
        sprint: true,
      },
    });
  });

  it("resolves system theme from prefers-color-scheme", () => {
    expect(resolveThemeMode("dark", false)).toBe("dark");
    expect(resolveThemeMode("light", true)).toBe("light");
    expect(resolveThemeMode("system", true)).toBe("dark");
    expect(resolveThemeMode("system", false)).toBe("light");
  });

  it("systemPrefersDark reads matchMedia when available", () => {
    expect(systemPrefersDark({ matches: false })).toBe(false);
    expect(systemPrefersDark({ matches: true })).toBe(true);
  });

  it("applies dark/light classes, datasets, and color-scheme", () => {
    const root = mockRoot();
    applyAppearance(
      { theme: "light", fontSize: "large", density: "compact" },
      root as any,
      true,
    );
    expect(root.classList.contains("dark")).toBe(false);
    expect(root.dataset.theme).toBe("light");
    expect(root.dataset.fontSize).toBe("large");
    expect(root.dataset.density).toBe("compact");
    expect(root.style.colorScheme).toBe("light");

    applyAppearance({ theme: "system" }, root as any, true);
    expect(root.classList.contains("dark")).toBe(true);
    expect(root.dataset.theme).toBe("dark");
  });

  it("caches and reads appearance from localStorage", () => {
    cacheAppearance({
      theme: "light",
      fontSize: "small",
      density: "compact",
      topNavbarTabs: {
        chat: true,
        work: false,
        docs: true,
        sprint: false,
      },
    });
    expect(localStorage.getItem(APPEARANCE_STORAGE_KEY)).toContain("light");
    expect(readCachedAppearance()).toEqual({
      theme: "light",
      fontSize: "small",
      density: "compact",
      topNavbarTabs: {
        chat: true,
        work: false,
        docs: true,
        sprint: false,
      },
    });
  });

  it("applyAppearance writes cache", () => {
    const root = mockRoot();
    applyAppearance(
      { theme: "dark", fontSize: "medium", density: "comfortable" },
      root as any,
    );
    expect(readCachedAppearance()?.theme).toBe("dark");
    expect(readCachedAppearance()?.topNavbarTabs.chat).toBe(true);
  });

  it("normalizes top navbar tabs and enforces at least one", () => {
    expect(
      normalizeTopNavbarTabs({
        chat: false,
        work: false,
        docs: true,
        sprint: false,
      }),
    ).toEqual({
      chat: false,
      work: false,
      docs: true,
      sprint: false,
    });

    expect(
      normalizeTopNavbarTabs({
        chat: false,
        work: false,
        docs: false,
        sprint: false,
      }),
    ).toEqual({
      chat: true,
      work: false,
      docs: false,
      sprint: false,
    });

    expect(normalizeTopNavbarTabs(["work", "sprint"])).toEqual({
      chat: false,
      work: true,
      docs: false,
      sprint: true,
    });
  });

  it("lists enabled tabs in canonical order", () => {
    const prefs = {
      topNavbarTabs: {
        chat: false,
        work: true,
        docs: false,
        sprint: true,
      },
    };
    expect(enabledTopNavbarTabs(prefs).map((t) => t.id)).toEqual([
      "work",
      "sprint",
    ]);
    expect(firstVisibleTopNavbarPath(prefs)).toBe("/");
  });

  it("matches hot-tab paths (Work is exact /)", () => {
    expect(topNavbarTabForPath("/agent-chat")?.id).toBe("chat");
    expect(topNavbarTabForPath("/")?.id).toBe("work");
    expect(topNavbarTabForPath("/docs")?.id).toBe("docs");
    expect(topNavbarTabForPath("/sprint")?.id).toBe("sprint");
    expect(topNavbarTabForPath("/settings")).toBeNull();
    expect(topNavbarTabForPath("/notes")).toBeNull();
  });
});
