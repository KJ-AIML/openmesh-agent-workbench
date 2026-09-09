import { afterEach, describe, expect, it } from "vitest";
import { getRuntimeKind, isTauriRuntime } from "@/lib/adapters/environment";

type TauriWindow = Window & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
  isTauri?: boolean;
};

function tauriWindow(): TauriWindow {
  return window as unknown as TauriWindow;
}

describe("isTauriRuntime", () => {
  afterEach(() => {
    const w = tauriWindow();
    delete w.__TAURI__;
    delete w.__TAURI_INTERNALS__;
    delete w.isTauri;
  });

  it("is false in a plain browser window", () => {
    expect(isTauriRuntime()).toBe(false);
    expect(getRuntimeKind()).toBe("web");
  });

  it("does not treat withGlobalTauri window.__TAURI__ as the runtime signal", () => {
    tauriWindow().__TAURI__ = {};
    expect(isTauriRuntime()).toBe(false);
  });

  it("detects Tauri via __TAURI_INTERNALS__", () => {
    tauriWindow().__TAURI_INTERNALS__ = {};
    expect(isTauriRuntime()).toBe(true);
    expect(getRuntimeKind()).toBe("tauri");
  });

  it("detects Tauri via window.isTauri", () => {
    tauriWindow().isTauri = true;
    expect(isTauriRuntime()).toBe(true);
  });
});
