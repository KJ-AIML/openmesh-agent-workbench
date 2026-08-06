/**
 * Session-scoped agent/terminal run tracker for Chat composer status chips.
 *
 * Tracks in-memory verify recipes, /delegate launches, Agent Engine tool-loop
 * progress (shell-like / long tools), and the active agent turn — not OS PTY /
 * IDE multiplex terminals. External terminals opened by delegate are
 * fire-and-forget (not supervised).
 */

export type SessionRunKind = "working" | "terminal";
export type SessionRunStatus = "running" | "done" | "failed" | "cancelled";

export type SessionRun = {
  id: string;
  kind: SessionRunKind;
  title: string;
  /** One-line command / recipe / tool summary (may be truncated for display). */
  command: string;
  status: SessionRunStatus;
  startedAt: number;
  endedAt?: number;
  output?: string;
  messageId?: string;
  toolId?: string;
};

export type CreateSessionRunInput = {
  id: string;
  kind: SessionRunKind;
  title: string;
  command: string;
  status?: SessionRunStatus;
  startedAt?: number;
  toolId?: string;
  messageId?: string;
  output?: string;
};

/**
 * Tool ids / titles that surface as Terminal session-run rows.
 * Keep read-only peek tools (read_file, list_dir, project_info) on Working only.
 */
export function looksLikeTerminalTool(toolIdOrTitle: string): boolean {
  const s = toolIdOrTitle.trim().toLowerCase().replace(/\s+/g, "_");
  if (!s) return false;
  if (
    s === "verify" ||
    s === "delegate" ||
    s === "grep" ||
    s === "git_diff" ||
    s === "git_status" ||
    s === "git_log" ||
    s === "run_recipe" ||
    s === "shell" ||
    s === "bash" ||
    s === "terminal"
  ) {
    return true;
  }
  return (
    s.includes("verify") ||
    s.includes("delegate") ||
    s.includes("recipe") ||
    s.includes("terminal") ||
    s.includes("shell") ||
    s.startsWith("run_") ||
    s.startsWith("git_")
  );
}

export function truncateCommand(cmd: string, max = 52): string {
  const oneLine = cmd.replace(/\s+/g, " ").trim();
  if (oneLine.length <= max) return oneLine;
  return `${oneLine.slice(0, Math.max(1, max - 1))}…`;
}

/** Elapsed label like `12s` or `2m 05s`. */
export function formatElapsed(
  startedAt: number,
  now: number,
  endedAt?: number,
): string {
  const end = endedAt ?? now;
  const sec = Math.max(0, Math.floor((end - startedAt) / 1000));
  if (sec < 60) return `${sec}s`;
  const min = Math.floor(sec / 60);
  const rem = sec % 60;
  return `${min}m ${String(rem).padStart(2, "0")}s`;
}

export function createSessionRun(input: CreateSessionRunInput): SessionRun {
  return {
    id: input.id,
    kind: input.kind,
    title: input.title,
    command: input.command,
    status: input.status ?? "running",
    startedAt: input.startedAt ?? Date.now(),
    toolId: input.toolId,
    messageId: input.messageId,
    output: input.output,
  };
}

export function upsertSessionRun(
  runs: SessionRun[],
  run: SessionRun,
): SessionRun[] {
  const idx = runs.findIndex((r) => r.id === run.id);
  if (idx < 0) return [...runs, run];
  const next = runs.slice();
  next[idx] = run;
  return next;
}

export function completeSessionRun(
  runs: SessionRun[],
  id: string,
  patch: {
    status: Exclude<SessionRunStatus, "running">;
    output?: string;
    messageId?: string;
    endedAt?: number;
  },
): SessionRun[] {
  const idx = runs.findIndex((r) => r.id === id);
  if (idx < 0) return runs;
  const prev = runs[idx];
  const next = runs.slice();
  next[idx] = {
    ...prev,
    status: patch.status,
    endedAt: patch.endedAt ?? Date.now(),
    output: patch.output ?? prev.output,
    messageId: patch.messageId ?? prev.messageId,
  };
  return next;
}

/** Append streamed log lines to an existing run (e.g. verify agent-run-log). */
export function appendSessionRunOutput(
  runs: SessionRun[],
  id: string,
  chunk: string,
  maxChars = 2400,
): SessionRun[] {
  const idx = runs.findIndex((r) => r.id === id);
  if (idx < 0) return runs;
  const prev = runs[idx];
  const merged = `${prev.output ?? ""}${prev.output ? "\n" : ""}${chunk}`;
  const output =
    merged.length > maxChars
      ? `…${merged.slice(merged.length - maxChars + 1)}`
      : merged;
  const next = runs.slice();
  next[idx] = { ...prev, output };
  return next;
}

export function countRunning(
  runs: SessionRun[],
  kind?: SessionRunKind,
): number {
  return runs.filter(
    (r) => r.status === "running" && (kind == null || r.kind === kind),
  ).length;
}

/**
 * Working chip count: active agent turn + in-flight non-terminal tools
 * that we track as working rows (engine mid-turn richness).
 */
export function countWorkingChip(runs: SessionRun[]): number {
  const working = countRunning(runs, "working");
  if (working === 0) return 0;
  // While a turn is active, surface at least the turn; bump when extra
  // terminal-like tools are also running so the chip feels alive.
  const terminalRunning = countRunning(runs, "terminal");
  return Math.max(1, working + terminalRunning);
}

export function listTerminalRuns(
  runs: SessionRun[],
  limit = 12,
): SessionRun[] {
  return runs
    .filter((r) => r.kind === "terminal")
    .slice()
    .sort((a, b) => b.startedAt - a.startedAt)
    .slice(0, limit);
}

/** Detect canvas/auto-ui signals already present in the active chat. */
export function sessionHasCanvasSignal(messages: {
  text: string;
  toolCalls?: { toolId: string; summary: string }[];
}[]): boolean {
  for (const m of messages) {
    const text = m.text ?? "";
    if (
      /```(?:canvas|artifact)\b/i.test(text) ||
      /openmesh\.canvas\//i.test(text)
    ) {
      return true;
    }
    for (const t of m.toolCalls ?? []) {
      const id = t.toolId.toLowerCase();
      if (
        id.includes("canvas") ||
        /openmesh\.canvas\//i.test(t.summary) ||
        /```(?:canvas|artifact)\b/i.test(t.summary)
      ) {
        return true;
      }
    }
  }
  return false;
}

/** Finish every still-running run of a kind (e.g. working turn ended). */
export function completeRunningOfKind(
  runs: SessionRun[],
  kind: SessionRunKind,
  status: Exclude<SessionRunStatus, "running"> = "done",
): SessionRun[] {
  const endedAt = Date.now();
  let changed = false;
  const next = runs.map((r) => {
    if (r.kind !== kind || r.status !== "running") return r;
    changed = true;
    return { ...r, status, endedAt };
  });
  return changed ? next : runs;
}

/** Update the active working run's command/label (mid-turn tool detail). */
export function touchWorkingRunCommand(
  runs: SessionRun[],
  command: string,
): SessionRun[] {
  const idx = runs.findIndex(
    (r) => r.kind === "working" && r.status === "running",
  );
  if (idx < 0) return runs;
  const prev = runs[idx];
  const nextCmd = truncateCommand(command, 120);
  if (prev.command === nextCmd) return runs;
  const next = runs.slice();
  next[idx] = { ...prev, command: nextCmd };
  return next;
}
