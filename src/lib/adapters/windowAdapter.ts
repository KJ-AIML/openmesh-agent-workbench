// Window controls adapter for Openmesh Tauri app.
// Centralized — no component calls Tauri window APIs directly.
// Lazy getCurrentWindow(): eager init crashes Vite/web (no Tauri metadata).

import { getCurrentWindow, type Window } from "@tauri-apps/api/window";
import { isTauriRuntime } from "./environment";

let win: Window | null = null;

function getWin(): Window | null {
  if (!isTauriRuntime()) return null;
  if (!win) {
    try {
      win = getCurrentWindow();
    } catch {
      return null;
    }
  }
  return win;
}

export interface WindowActionResult {
  success: boolean;
  error?: string;
}

export async function minimizeWindow(): Promise<WindowActionResult> {
  console.log("[window] minimize called");
  const w = getWin();
  if (!w) return { success: false, error: "Window API unavailable outside Tauri" };
  try {
    await w.minimize();
    console.log("[window] minimize succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] minimize failed", error);
    return { success: false, error: String(error) };
  }
}

export async function toggleMaximizeWindow(): Promise<WindowActionResult> {
  console.log("[window] toggleMaximize called");
  const w = getWin();
  if (!w) return { success: false, error: "Window API unavailable outside Tauri" };
  try {
    await w.toggleMaximize();
    console.log("[window] toggleMaximize succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] toggleMaximize failed", error);
    return { success: false, error: String(error) };
  }
}

export async function closeWindow(): Promise<WindowActionResult> {
  console.log("[window] close called");
  const w = getWin();
  if (!w) return { success: false, error: "Window API unavailable outside Tauri" };
  try {
    await w.close();
    console.log("[window] close succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] close failed", error);
    return { success: false, error: String(error) };
  }
}

export async function startWindowDrag(): Promise<WindowActionResult> {
  console.log("[window] startDragging called");
  const w = getWin();
  if (!w) return { success: false, error: "Window API unavailable outside Tauri" };
  try {
    await w.startDragging();
    console.log("[window] startDragging succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] startDragging failed", error);
    return { success: false, error: String(error) };
  }
}

export async function isMaximized(): Promise<boolean> {
  const w = getWin();
  if (!w) return false;
  try {
    return await w.isMaximized();
  } catch {
    return false;
  }
}
