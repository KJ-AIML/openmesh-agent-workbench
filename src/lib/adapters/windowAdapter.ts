// Window controls adapter for Openmesh Tauri app.
// Centralized — no component calls Tauri window APIs directly.

import { getCurrentWindow } from "@tauri-apps/api/window";

const win = getCurrentWindow();

export interface WindowActionResult {
  success: boolean;
  error?: string;
}

export async function minimizeWindow(): Promise<WindowActionResult> {
  console.log("[window] minimize called");
  try {
    await win.minimize();
    console.log("[window] minimize succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] minimize failed", error);
    return { success: false, error: String(error) };
  }
}

export async function toggleMaximizeWindow(): Promise<WindowActionResult> {
  console.log("[window] toggleMaximize called");
  try {
    await win.toggleMaximize();
    console.log("[window] toggleMaximize succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] toggleMaximize failed", error);
    return { success: false, error: String(error) };
  }
}

export async function closeWindow(): Promise<WindowActionResult> {
  console.log("[window] close called");
  try {
    await win.close();
    console.log("[window] close succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] close failed", error);
    return { success: false, error: String(error) };
  }
}

export async function startWindowDrag(): Promise<WindowActionResult> {
  console.log("[window] startDragging called");
  try {
    await win.startDragging();
    console.log("[window] startDragging succeeded");
    return { success: true };
  } catch (error) {
    console.error("[window] startDragging failed", error);
    return { success: false, error: String(error) };
  }
}

export async function isMaximized(): Promise<boolean> {
  try {
    const result = await win.isMaximized();
    return result;
  } catch {
    return false;
  }
}
