import type { Router } from "vue-router";
import type { AgentToolStep } from "../agentEngineClient";
import type { VoiceUiAction } from "./types";

/** Allowlisted in-app routes the voice agent may open. */
export const VOICE_UI_ROUTES: Record<string, string> = {
  "/": "Work",
  "/agent-chat": "Chat",
  "/docs": "Docs",
  "/sprint": "Sprint",
  "/notes": "Notes",
  "/settings": "Settings",
  "/continuity": "Continuity",
  "/agent-sessions": "Agent Sessions",
  "/context": "Context",
  "/canvas": "Canvas",
};

const ALIASES: Record<string, string> = {
  work: "/",
  home: "/",
  chat: "/agent-chat",
  "agent-chat": "/agent-chat",
  docs: "/docs",
  documentation: "/docs",
  sprint: "/sprint",
  notes: "/notes",
  settings: "/settings",
  continuity: "/continuity",
  sessions: "/agent-sessions",
  "agent-sessions": "/agent-sessions",
  context: "/context",
  canvas: "/canvas",
  board: "/canvas",
  graph: "/canvas",
};

export function normalizeVoiceRoute(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;
  if (VOICE_UI_ROUTES[trimmed]) return trimmed;
  const key = trimmed.replace(/^\//, "").toLowerCase();
  const aliased = ALIASES[key];
  if (aliased) return aliased;
  // Accept "/docs" style already checked; try with leading slash
  const withSlash = trimmed.startsWith("/") ? trimmed : `/${trimmed}`;
  if (VOICE_UI_ROUTES[withSlash]) return withSlash;
  return null;
}

export function parseUiActionsFromToolSteps(steps: AgentToolStep[]): VoiceUiAction[] {
  const out: VoiceUiAction[] = [];
  for (const step of steps) {
    if (step.toolName !== "ui_navigate" || !step.ok) continue;
    try {
      const parsed = JSON.parse(step.summary) as {
        action?: string;
        route?: string;
        label?: string;
        appAction?: { type?: string; route?: string };
      };
      const rawRoute =
        parsed.appAction?.type === "navigate"
          ? parsed.appAction.route
          : parsed.action === "ui_navigate"
            ? parsed.route
            : undefined;
      if (!rawRoute) continue;
      const route = normalizeVoiceRoute(rawRoute);
      if (!route) continue;
      out.push({
        action: "ui_navigate",
        route,
        label: parsed.label || VOICE_UI_ROUTES[route],
      });
    } catch {
      /* ignore non-JSON summaries */
    }
  }
  return out;
}

export async function applyVoiceUiActions(
  router: Router,
  actions: VoiceUiAction[],
): Promise<string[]> {
  const labels: string[] = [];
  for (const action of actions) {
    if (action.action !== "ui_navigate") continue;
    await router.push(action.route);
    labels.push(action.label || action.route);
  }
  return labels;
}
