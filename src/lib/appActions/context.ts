import type { AppContext } from "./types";

/** Live app context for `app.get_context` / voice bridge (updated by UI shells). */
let current: AppContext = {};

export function setAppContext(patch: Partial<AppContext>): void {
  current = { ...current, ...patch };
}

export function getAppContext(): AppContext {
  return { ...current, openPanels: [...(current.openPanels ?? [])] };
}

export function clearAppContext(): void {
  current = {};
}
