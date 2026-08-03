// Debounced session persistence. Chat mutates often (send, tool results,
// renames); syncing JSON.stringify + localStorage on every deep-watch tick
// janks the main thread. Schedule writes, coalesce, and flush on demand
// (project switch / unmount). Debounced writes yield via setTimeout(0) so
// send/paint never share a turn with stringify.

export type PersistFn<T> = (projectPath: string, sessions: T[]) => void;

export type PersistQueue<T> = {
  /** Queue a write; replaces any pending write for the same debounce window. */
  schedule(projectPath: string, sessions: T[]): void;
  /** Persist immediately if something is pending (project switch / unmount). */
  flush(): void;
  /** Drop pending work without writing. */
  cancel(): void;
  /** True while a write is queued (debounce timer and/or deferred write). */
  get pending(): boolean;
};

export function createPersistQueue<T>(
  persist: PersistFn<T>,
  delayMs = 400,
): PersistQueue<T> {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let deferred: ReturnType<typeof setTimeout> | null = null;
  let pending: { projectPath: string; sessions: T[] } | null = null;

  function clearTimer(): void {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  }

  function clearDeferred(): void {
    if (deferred !== null) {
      clearTimeout(deferred);
      deferred = null;
    }
  }

  function flush(): void {
    clearTimer();
    clearDeferred();
    if (!pending) return;
    const job = pending;
    pending = null;
    // Project switch / unmount — must finish before state swaps.
    persist(job.projectPath, job.sessions);
  }

  function schedule(projectPath: string, sessions: T[]): void {
    // Keep the live array reference — flush/deferred always sees latest mutations.
    pending = { projectPath, sessions };
    clearTimer();
    clearDeferred();
    timer = setTimeout(() => {
      timer = null;
      if (!pending) return;
      const job = pending;
      // Keep `pending` set until the deferred write (or flush) actually persists,
      // so project-switch flush never loses a coalesced job.
      deferred = setTimeout(() => {
        deferred = null;
        if (pending !== job) return;
        pending = null;
        persist(job.projectPath, job.sessions);
      }, 0);
    }, delayMs);
  }

  function cancel(): void {
    clearTimer();
    clearDeferred();
    pending = null;
  }

  return {
    schedule,
    flush,
    cancel,
    get pending() {
      return pending !== null;
    },
  };
}
