/** Typed AppAction contracts (mirrors openmesh-core::app_actions). */

export type ActionSource = "voice" | "chat" | "recipe" | "system";

export type RiskClass =
  | "read"
  | "navigate"
  | "compose"
  | "write"
  | "external"
  | "destructive"
  | "privileged";

export type ConfirmationPolicy = "none" | "soft" | "hard";

export type AppAction =
  | { type: "navigate"; route: string }
  | { type: "openPanel"; panel: string }
  | { type: "closePanel"; panel: string }
  | { type: "setComposer"; text: string }
  | { type: "focusComposer" }
  | { type: "setMode"; mode: string }
  | { type: "selectSession"; sessionId: string }
  | { type: "createNote"; title?: string }
  | { type: "openNote"; noteId: string }
  | { type: "openSprint"; sprintId?: string }
  | { type: "createSprint"; name?: string }
  | { type: "runRecipe"; recipeId: string }
  | { type: "stopRecipe"; runKey: string }
  | { type: "openCanvas"; canvasId?: string }
  | { type: "openBoard"; boardId?: string }
  | { type: "boardAddSticky"; text: string; boardId?: string }
  | { type: "boardConnect"; from: string; to: string; boardId?: string }
  | { type: "canvasAddNode"; label: string; kind?: string }
  | { type: "canvasConnect"; from: string; to: string }
  | { type: "canvasDeleteNode"; nodeId: string }
  | { type: "canvasFitView" }
  | { type: "noop"; reason?: string };

export type ActionIntent = {
  action: AppAction;
  source: ActionSource;
  turnId?: string;
  rationale?: string;
};

export type ActionResult = {
  ok: boolean;
  summary: string;
  applied?: AppAction;
  error?: string;
  pendingId?: string;
};

export type AppContext = {
  route?: string;
  chatMode?: string;
  activeSessionId?: string;
  openPanels?: string[];
  projectPath?: string;
  canvasId?: string;
};

export type ActionAuditEntry = {
  id: string;
  at: number;
  source: ActionSource;
  action: AppAction;
  ok: boolean;
  summary: string;
  error?: string;
  turnId?: string;
};

export function riskClassFor(action: AppAction): RiskClass {
  switch (action.type) {
    case "setComposer":
    case "setMode":
    case "canvasAddNode":
    case "canvasConnect":
    case "boardConnect":
      return "compose";
    case "createNote":
    case "createSprint":
    case "boardAddSticky":
      return "write";
    case "runRecipe":
    case "stopRecipe":
      return "external";
    case "canvasDeleteNode":
      return "destructive";
    case "noop":
      return "read";
    default:
      return "navigate";
  }
}

export function confirmationFor(action: AppAction): ConfirmationPolicy {
  if (action.type === "boardConnect") return "soft";
  const risk = riskClassFor(action);
  if (risk === "write") return "soft";
  if (risk === "external" || risk === "destructive" || risk === "privileged") return "hard";
  return "none";
}

export function labelForAction(action: AppAction): string {
  switch (action.type) {
    case "navigate":
      return `Navigate to ${action.route}`;
    case "openPanel":
      return `Open panel ${action.panel}`;
    case "closePanel":
      return `Close panel ${action.panel}`;
    case "setComposer":
      return "Set composer text";
    case "focusComposer":
      return "Focus composer";
    case "setMode":
      return `Set mode ${action.mode}`;
    case "selectSession":
      return "Select chat session";
    case "createNote":
      return "Create note";
    case "openNote":
      return "Open note";
    case "openSprint":
      return "Open sprint";
    case "createSprint":
      return "Create sprint";
    case "runRecipe":
      return `Run recipe ${action.recipeId}`;
    case "stopRecipe":
      return "Stop recipe";
    case "openCanvas":
      return "Open canvas";
    case "openBoard":
      return "Open board";
    case "boardAddSticky":
      return `Board sticky: ${action.text}`;
    case "boardConnect":
      return `Board connect ${action.from} → ${action.to}`;
    case "canvasAddNode":
      return `Add canvas node ${action.label}`;
    case "canvasConnect":
      return `Connect ${action.from} → ${action.to}`;
    case "canvasDeleteNode":
      return `Delete node ${action.nodeId}`;
    case "canvasFitView":
      return "Fit canvas view";
    case "noop":
      return action.reason || "No operation";
  }
}
