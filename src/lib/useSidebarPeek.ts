import { onUnmounted, ref, type Ref } from "vue";

export const SIDEBAR_PEEK_SHOW_DELAY_MS = 120;
export const SIDEBAR_PEEK_HIDE_DELAY_MS = 140;

function prefersReducedMotion(): boolean {
  try {
    return (
      typeof matchMedia !== "undefined" &&
      matchMedia("(prefers-reduced-motion: reduce)").matches
    );
  } catch {
    return false;
  }
}

/**
 * Ephemeral hover-to-peek for a collapsed (unpinned) sidebar.
 * Does not touch localStorage / pin preference.
 */
export function useSidebarPeek(options?: {
  showDelayMs?: number;
  hideDelayMs?: number;
}) {
  const sidebarPeek: Ref<boolean> = ref(false);
  let showTimer: ReturnType<typeof setTimeout> | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  const showDelayMs = options?.showDelayMs ?? SIDEBAR_PEEK_SHOW_DELAY_MS;
  const hideDelayMs = options?.hideDelayMs ?? SIDEBAR_PEEK_HIDE_DELAY_MS;

  function clearShowTimer() {
    if (showTimer != null) {
      clearTimeout(showTimer);
      showTimer = null;
    }
  }

  function clearHideTimer() {
    if (hideTimer != null) {
      clearTimeout(hideTimer);
      hideTimer = null;
    }
  }

  function clearTimers() {
    clearShowTimer();
    clearHideTimer();
  }

  function closePeek() {
    clearTimers();
    sidebarPeek.value = false;
  }

  function onPeekEnter() {
    clearHideTimer();
    if (sidebarPeek.value) return;
    const delay = prefersReducedMotion() ? 0 : showDelayMs;
    clearShowTimer();
    showTimer = setTimeout(() => {
      showTimer = null;
      sidebarPeek.value = true;
    }, delay);
  }

  function onPeekLeave() {
    clearShowTimer();
    if (!sidebarPeek.value) return;
    const delay = prefersReducedMotion() ? 0 : hideDelayMs;
    clearHideTimer();
    hideTimer = setTimeout(() => {
      hideTimer = null;
      sidebarPeek.value = false;
    }, delay);
  }

  onUnmounted(() => {
    clearTimers();
  });

  return {
    sidebarPeek,
    onPeekEnter,
    onPeekLeave,
    closePeek,
  };
}
