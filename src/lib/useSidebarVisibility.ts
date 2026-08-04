import { ref, type Ref } from "vue";

/** Persisted left-rail visibility for the desktop app shell. */
export const SIDEBAR_VISIBLE_STORAGE_KEY = "openmesh.sidebarVisible";

const sidebarVisible: Ref<boolean> = ref(true);
let hydrated = false;

function readStored(): boolean {
  try {
    if (typeof localStorage === "undefined") return true;
    const raw = localStorage.getItem(SIDEBAR_VISIBLE_STORAGE_KEY);
    if (raw === null) return true;
    return raw !== "0" && raw !== "false";
  } catch {
    return true;
  }
}

function writeStored(visible: boolean) {
  try {
    if (typeof localStorage === "undefined") return;
    localStorage.setItem(SIDEBAR_VISIBLE_STORAGE_KEY, visible ? "1" : "0");
  } catch {
    // Restricted webviews / tests without storage — in-memory still works.
  }
}

function hydrate() {
  if (hydrated) return;
  hydrated = true;
  sidebarVisible.value = readStored();
}

/**
 * Singleton preference for the main Projects/Chat/Work sidebar.
 * Survives reload via localStorage; safe when storage is unavailable.
 */
export function useSidebarVisibility() {
  hydrate();

  function setSidebarVisible(visible: boolean) {
    sidebarVisible.value = visible;
    writeStored(visible);
  }

  function toggleSidebar() {
    setSidebarVisible(!sidebarVisible.value);
  }

  function showSidebar() {
    setSidebarVisible(true);
  }

  function hideSidebar() {
    setSidebarVisible(false);
  }

  return {
    sidebarVisible,
    setSidebarVisible,
    toggleSidebar,
    showSidebar,
    hideSidebar,
  };
}

/** Test-only: reset singleton hydration. Does not touch localStorage. */
export function __resetSidebarVisibilityForTests(visible = true) {
  sidebarVisible.value = visible;
  hydrated = false;
}
