import type { Router } from "vue-router";
import { normalizeVoiceRoute, VOICE_UI_ROUTES } from "../voice/uiActions";
import { recordActionAudit, pushUndoIntent } from "./audit";
import { enqueuePendingAction } from "./pending";
import {
  confirmationFor,
  labelForAction,
  riskClassFor,
  type ActionIntent,
  type ActionResult,
  type AppAction,
} from "./types";

export type AppActionHandlers = {
  setComposer?: (text: string) => void;
  focusComposer?: () => void;
  setMode?: (mode: string) => void;
  selectSession?: (sessionId: string) => void;
  openPanel?: (panel: string) => void;
  closePanel?: (panel: string) => void;
  createNote?: (title?: string) => Promise<string> | string;
  openNote?: (noteId: string) => Promise<void> | void;
  createSprint?: (name?: string) => Promise<string> | string;
  runRecipe?: (recipeId: string) => Promise<string> | string;
  stopRecipe?: (runKey: string) => Promise<string> | string;
  canvasAddNode?: (label: string, kind?: string) => Promise<string> | string;
  canvasConnect?: (from: string, to: string) => Promise<string> | string;
  canvasDeleteNode?: (nodeId: string) => Promise<string> | string;
  canvasFitView?: () => void;
  openCanvas?: (canvasId?: string) => Promise<void> | void;
  openBoard?: (boardId?: string) => Promise<void> | void;
  boardAddSticky?: (text: string, boardId?: string) => Promise<string> | string;
  boardConnect?: (
    from: string,
    to: string,
    boardId?: string,
  ) => Promise<string> | string;
};

let handlers: AppActionHandlers = {};

export function registerAppActionHandlers(next: AppActionHandlers): void {
  handlers = { ...handlers, ...next };
}

export function clearAppActionHandlers(): void {
  handlers = {};
}

/**
 * Apply a typed AppAction. Write/destructive/external enqueue confirmation
 * unless `confirmWrite` is true.
 */
export async function dispatchAppAction(
  router: Router,
  intent: ActionIntent,
  opts?: { confirmWrite?: boolean; enqueueIfNeeded?: boolean },
): Promise<ActionResult> {
  const action = intent.action;
  const policy = confirmationFor(action);
  const enqueue = opts?.enqueueIfNeeded !== false;

  if (policy !== "none" && !opts?.confirmWrite) {
    if (enqueue) {
      const pendingId = enqueuePendingAction(
        intent,
        labelForAction(action),
        policy,
      );
      const result: ActionResult = {
        ok: false,
        summary: `Awaiting confirmation: ${labelForAction(action)}`,
        error: "confirmation_required",
        pendingId,
      };
      recordActionAudit(intent, result);
      return result;
    }
    const result: ActionResult = {
      ok: false,
      summary: `Confirmation required for ${labelForAction(action)}`,
      error: "confirmation_required",
    };
    recordActionAudit(intent, result);
    return result;
  }

  try {
    const summary = await applyAction(router, action);
    const result: ActionResult = { ok: true, summary, applied: action };
    recordActionAudit(intent, result);
    const inverse = inverseIntent(intent);
    if (inverse) pushUndoIntent(inverse);
    return result;
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    const result: ActionResult = {
      ok: false,
      summary: labelForAction(action),
      error: msg,
    };
    recordActionAudit(intent, result);
    return result;
  }
}

function inverseIntent(intent: ActionIntent): ActionIntent | null {
  const a = intent.action;
  if (a.type === "openPanel") {
    return { ...intent, action: { type: "closePanel", panel: a.panel } };
  }
  if (a.type === "setComposer") {
    return { ...intent, action: { type: "setComposer", text: "" } };
  }
  return null;
}

async function applyAction(router: Router, action: AppAction): Promise<string> {
  switch (action.type) {
    case "navigate": {
      const route = normalizeVoiceRoute(action.route);
      if (!route) throw new Error(`unsupported route: ${action.route}`);
      await router.push(route);
      return VOICE_UI_ROUTES[route] || route;
    }
    case "setComposer":
      if (!handlers.setComposer) throw new Error("composer handler not registered");
      handlers.setComposer(action.text);
      return "Composer updated";
    case "focusComposer":
      if (!handlers.focusComposer) throw new Error("composer handler not registered");
      handlers.focusComposer();
      return "Composer focused";
    case "setMode":
      if (!handlers.setMode) throw new Error("mode handler not registered");
      handlers.setMode(action.mode);
      return `Mode ${action.mode}`;
    case "selectSession":
      if (!handlers.selectSession) throw new Error("session handler not registered");
      handlers.selectSession(action.sessionId);
      return "Session selected";
    case "openPanel":
      handlers.openPanel?.(action.panel);
      return `Open ${action.panel}`;
    case "closePanel":
      handlers.closePanel?.(action.panel);
      return `Close ${action.panel}`;
    case "openSprint": {
      await router.push("/sprint");
      return "Sprint";
    }
    case "createNote": {
      if (!handlers.createNote) throw new Error("createNote handler not registered");
      const name = await handlers.createNote(action.title);
      return typeof name === "string" ? `Note ${name}` : "Note created";
    }
    case "openNote": {
      await router.push("/notes");
      await handlers.openNote?.(action.noteId);
      return `Note ${action.noteId}`;
    }
    case "createSprint": {
      if (!handlers.createSprint) throw new Error("createSprint handler not registered");
      const name = await handlers.createSprint(action.name);
      await router.push("/sprint");
      return typeof name === "string" ? `Sprint ${name}` : "Sprint created";
    }
    case "runRecipe": {
      if (!handlers.runRecipe) throw new Error("runRecipe handler not registered");
      return await handlers.runRecipe(action.recipeId);
    }
    case "stopRecipe": {
      if (!handlers.stopRecipe) throw new Error("stopRecipe handler not registered");
      return await handlers.stopRecipe(action.runKey);
    }
    case "openCanvas": {
      if (handlers.openCanvas) await handlers.openCanvas(action.canvasId);
      else await router.push("/canvas");
      return "Canvas";
    }
    case "openBoard": {
      if (handlers.openBoard) await handlers.openBoard(action.boardId);
      else await router.push({ path: "/canvas", query: { tab: "board" } });
      return "Board";
    }
    case "boardAddSticky": {
      if (!handlers.boardAddSticky) throw new Error("board handler not registered");
      return await handlers.boardAddSticky(action.text, action.boardId);
    }
    case "boardConnect": {
      if (!handlers.boardConnect) throw new Error("board handler not registered");
      return await handlers.boardConnect(action.from, action.to, action.boardId);
    }
    case "canvasAddNode": {
      if (!handlers.canvasAddNode) throw new Error("canvas handler not registered");
      return await handlers.canvasAddNode(action.label, action.kind);
    }
    case "canvasConnect": {
      if (!handlers.canvasConnect) throw new Error("canvas handler not registered");
      return await handlers.canvasConnect(action.from, action.to);
    }
    case "canvasDeleteNode": {
      if (!handlers.canvasDeleteNode) throw new Error("canvas handler not registered");
      return await handlers.canvasDeleteNode(action.nodeId);
    }
    case "canvasFitView":
      handlers.canvasFitView?.();
      return "Fit view";
    case "noop":
      return action.reason || "noop";
  }
}

/** Parse ActionIntents from agent tool step summaries. */
export function parseActionIntentsFromToolSteps(
  steps: { toolName: string; ok: boolean; summary: string }[],
  source: ActionIntent["source"] = "chat",
): ActionIntent[] {
  const out: ActionIntent[] = [];
  for (const step of steps) {
    if (!step.ok) continue;
    if (step.toolName !== "ui_navigate" && step.toolName !== "app_propose_action") {
      continue;
    }
    try {
      const parsed = JSON.parse(step.summary) as {
        action?: string;
        route?: string;
        appAction?: AppAction;
      };
      if (parsed.appAction?.type) {
        out.push({ action: parsed.appAction, source });
        continue;
      }
      if (parsed.action === "ui_navigate" && parsed.route) {
        const route = normalizeVoiceRoute(parsed.route);
        if (route) out.push({ action: { type: "navigate", route }, source });
      }
    } catch {
      /* ignore */
    }
  }
  return out;
}

export { riskClassFor, labelForAction };
