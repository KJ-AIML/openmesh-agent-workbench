// Environment detection for Openmesh
// Determines whether running in web browser or Tauri desktop environment

import { invoke } from "@tauri-apps/api/core";
import type { RuntimeKind } from "./types";

/**
 * Check if running in Tauri runtime
 * Tauri injects window.__TAURI__ object when running in desktop mode
 */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI__" in window;
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
