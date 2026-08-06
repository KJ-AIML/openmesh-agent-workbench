/**
 * Chat-adjacent shell tabs for the embedded PTY terminal panel.
 *
 * Each tab maps to an in-app PTY session (xterm + portable-pty).
 * External OS terminal launch remains available as an optional action.
 */

export type ShellTabStatus = "launching" | "open" | "exited" | "error";

export type ShellTab = {
  id: string;
  /** Short tab label, e.g. zsh */
  label: string;
  cwd: string;
  createdAt: number;
  status: ShellTabStatus;
  error?: string;
  /** True when this tab was opened as an external OS terminal (optional). */
  external?: boolean;
};

let tabSeq = 0;

/** Default shell label for the host (display only until PTY reports). */
export function defaultShellLabel(): string {
  try {
    const platform =
      typeof navigator !== "undefined" ? navigator.platform || "" : "";
    if (/win/i.test(platform)) return "powershell";
    if (typeof process !== "undefined" && process.env?.SHELL) {
      const base = process.env.SHELL.split(/[/\\]/).pop();
      if (base) return base;
    }
  } catch {
    /* ignore */
  }
  return "zsh";
}

/**
 * Working directory for a new shell: project path, else HOME/USERPROFILE, else "".
 */
export function resolveTerminalCwd(projectPath?: string | null): string {
  const p = projectPath?.trim();
  if (p) return p;
  try {
    const env =
      typeof process !== "undefined" && process.env ? process.env : undefined;
    const home = env?.HOME?.trim() || env?.USERPROFILE?.trim();
    if (home) return home;
  } catch {
    /* ignore */
  }
  return "";
}

export function createShellTab(opts: {
  cwd: string;
  label?: string;
  id?: string;
  status?: ShellTabStatus;
  error?: string;
  external?: boolean;
}): ShellTab {
  tabSeq += 1;
  return {
    id: opts.id ?? `shell-${Date.now()}-${tabSeq}`,
    label: opts.label ?? defaultShellLabel(),
    cwd: opts.cwd,
    createdAt: Date.now(),
    status: opts.status ?? "launching",
    error: opts.error,
    external: opts.external,
  };
}

export function upsertShellTab(tabs: ShellTab[], tab: ShellTab): ShellTab[] {
  const i = tabs.findIndex((t) => t.id === tab.id);
  if (i < 0) return [...tabs, tab];
  const next = tabs.slice();
  next[i] = tab;
  return next;
}

export function removeShellTab(
  tabs: ShellTab[],
  id: string,
): { tabs: ShellTab[]; nextActiveId: string | null } {
  const i = tabs.findIndex((t) => t.id === id);
  if (i < 0) return { tabs, nextActiveId: null };
  const next = tabs.filter((t) => t.id !== id);
  if (next.length === 0) return { tabs: next, nextActiveId: null };
  const fallback = next[Math.min(i, next.length - 1)]!;
  return { tabs: next, nextActiveId: fallback.id };
}

export function shortCwdLabel(cwd: string, max = 42): string {
  const t = cwd.trim();
  if (!t) return "(no cwd)";
  if (t.length <= max) return t;
  return `…${t.slice(-(max - 1))}`;
}
