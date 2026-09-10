// Environment detection for Openmesh
// Determines whether running in web browser or Tauri desktop environment

import { invokeTyped as invoke } from "../ipc";
import type { RuntimeKind } from "./types";

/**
 * Check if running in Tauri runtime.
 *
 * `withGlobalTauri` is false, so `window.__TAURI__` is not injected.
 * The IPC bridge still exposes `__TAURI_INTERNALS__`; recent Tauri also
 * sets `window.isTauri`.
 */
export function isTauriRuntime(): boolean {
  if (typeof window === "undefined") return false;
  const w = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    isTauri?: boolean;
    __OPENMESH_RUNTIME__?: "tauri" | "web";
  };
  if (w.__OPENMESH_RUNTIME__ === "web") return false;
  if (w.__OPENMESH_RUNTIME__ === "tauri") return true;
  return typeof w.__TAURI_INTERNALS__ !== "undefined" || w.isTauri === true;
}

/**
 * Sync heuristic — may fail in stripped WKWebView user agents.
 * Prefer resolveIsMacOS() once at app start.
 */
export function isMacOS(): boolean {
  if (typeof navigator === "undefined") return false;
  // userAgentData (Chromium) / platform / UA
  const uaData = (navigator as Navigator & { userAgentData?: { platform?: string } })
    .userAgentData;
  if (uaData?.platform) return /mac/i.test(uaData.platform);
  const platform = navigator.platform || "";
  const ua = navigator.userAgent || "";
  return /Mac|iPhone|iPad|iPod/i.test(platform) || /Macintosh|Mac OS X/i.test(ua);
}

/**
 * Authoritative OS check via Rust (std::env::consts::OS).
 * Falls back to isMacOS() outside Tauri.
 */
export async function resolveIsMacOS(): Promise<boolean> {
  if (!isTauriRuntime()) return isMacOS();
  try {
    const os = await invoke<string>("get_host_os");
    return os === "macos";
  } catch {
    return isMacOS();
  }
}

/**
 * Get the current runtime kind
 * Returns 'tauri' if running in Tauri desktop app, 'web' otherwise
 */
export function getRuntimeKind(): RuntimeKind {
  return isTauriRuntime() ? "tauri" : "web";
}

/**
 * Check if a specific native feature is available.
 * Returns true only when running inside Tauri.
 */
export function hasNativeFeature(feature: string): boolean {
  if (!isTauriRuntime()) return false;
  const supported = new Set([
    "folder-picker",
    "path-validation",
    "open-folder",
    "git-status",
    "terminal",
    "agent-cli",
    "session-scanning",
    "command-preset",
  ]);
  return supported.has(feature);
}
