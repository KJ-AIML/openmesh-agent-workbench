import type { ActionAuditEntry, ActionIntent, ActionResult } from "./types";

const MAX = 80;
const entries: ActionAuditEntry[] = [];
const listeners = new Set<() => void>();

function notify() {
  for (const l of listeners) l();
}

export function subscribeAudit(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function listActionAudit(limit = 20): ActionAuditEntry[] {
  return entries.slice(0, limit);
}

export function recordActionAudit(
  intent: ActionIntent,
  result: ActionResult,
): ActionAuditEntry {
  const entry: ActionAuditEntry = {
    id: `act-${Date.now()}-${Math.random().toString(16).slice(2, 6)}`,
    at: Date.now(),
    source: intent.source,
    action: intent.action,
    ok: result.ok,
    summary: result.summary,
    error: result.error,
    turnId: intent.turnId,
  };
  entries.unshift(entry);
  if (entries.length > MAX) entries.length = MAX;
  notify();
  return entry;
}

/** Simple FE undo stack of inverse intents. */
const undoStack: ActionIntent[] = [];

export function pushUndoIntent(inverse: ActionIntent): void {
  undoStack.push(inverse);
  if (undoStack.length > 40) undoStack.shift();
}

export function popUndoIntent(): ActionIntent | undefined {
  return undoStack.pop();
}

export function peekUndoIntent(): ActionIntent | undefined {
  return undoStack[undoStack.length - 1];
}
