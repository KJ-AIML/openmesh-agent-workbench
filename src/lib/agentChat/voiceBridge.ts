/**
 * Voice → Agent Chat turn bridge (Jarvis-style).
 * Always-listen captures an utterance → Agent Engine tools → short spoken reply.
 */

import type { Router } from "vue-router";
import {
  dispatchAppAction,
  parseActionIntentsFromToolSteps,
} from "../appActions/dispatcher";
import { confirmationFor } from "../appActions/types";
import { getAppContext } from "../appActions/context";
import { sanitizeVoiceError } from "../voice/errors";
import { formatVoiceReply } from "../voice/replyFormat";
import { runAgentChatTurn, type ChatTurnResult } from "./runner";
import type { Settings } from "../../types";

export type VoiceChatMode = "ask" | "plan" | "act" | "delegate";

export type VoiceChatLink = {
  getHistory: () => { role: string; content: string }[];
  getMode: () => VoiceChatMode;
  appendExchange?: (userText: string, result: ChatTurnResult) => void;
  getSettings?: () => Settings | null;
};

let link: VoiceChatLink | null = null;

export function registerVoiceChatLink(next: VoiceChatLink | null): void {
  link = next;
}

export function getVoiceChatLink(): VoiceChatLink | null {
  return link;
}

export function buildVoicePrompt(heard: string): string {
  const ctx = getAppContext();
  const where = ctx.route ? `They are looking at ${ctx.route}.` : "";
  return [
    "[OpenMesh Voice — Jarvis assistant]",
    "You help by talking and by using in-app tools. Keep every spoken reply to 1–3 short sentences.",
    "No JSON, no markdown dumps, no code fences in your spoken text.",
    "Greetings: answer warmly and briefly.",
    "When they want something done in the app, use tools:",
    '- ui_navigate with route aliases: chat, work, docs, notes, settings, sessions, canvas',
    '- app_propose_action with {"type":"createNote","title":"…"} to create a note',
    '- app_propose_action with {"type":"createSprint","name":"…"} for a new work cycle',
    '- app_propose_action with {"type":"openCanvas"} or {"type":"canvasAddNode","label":"…"} for Network graph',
    '- app_propose_action with {"type":"openBoard"} or {"type":"boardAddSticky","text":"…"} for freeform Board',
    '- app_propose_action with {"type":"boardConnect","from":"A","to":"B"} to link Board stickies',
    "After a tool runs, say what you did in plain speech (e.g. \"Created a note called Colleague API key.\").",
    "If you cannot do it, say so briefly and suggest the next step.",
    where,
    "",
    `User said: ${heard}`,
  ]
    .filter(Boolean)
    .join("\n");
}

export type VoiceBridgeTurnInput = {
  projectPath: string;
  heard: string;
  settings?: Settings | null;
  turnId?: string;
  mode?: VoiceChatMode;
  history?: { role: string; content: string }[];
};

export type VoiceBridgeTurnResult = {
  reply: string;
  toolCalls: ChatTurnResult["toolCalls"];
  actionLabels: string[];
  error?: string;
  usedSharedSession: boolean;
};

export async function runVoiceBridgeTurn(
  router: Router,
  input: VoiceBridgeTurnInput,
): Promise<VoiceBridgeTurnResult> {
  const active = link;
  // Voice is action-oriented (navigate, notes, canvas). Do not inherit Agent Chat's
  // Ask mode — that caps tools at 3 rounds and blocks UI writes.
  const mode = input.mode ?? "act";
  const history = input.history ?? active?.getHistory() ?? [];
  const settings = input.settings ?? active?.getSettings?.() ?? null;
  const prompt = buildVoicePrompt(input.heard);

  try {
    const result = await runAgentChatTurn(input.projectPath, prompt, {
      settings,
      history,
      mode,
      turnId: input.turnId,
      skipLocalTools: true,
    });

    const intents = parseActionIntentsFromToolSteps(
      (result.toolCalls ?? []).map((t) => ({
        toolName: t.toolId,
        ok: t.ok,
        summary: t.summary,
      })),
      "voice",
    );

    const actionLabels: string[] = [];
    const appliedBits: string[] = [];
    for (const intent of intents) {
      // Soft writes from voice auto-run (create note/sprint). Hard/external still need the card.
      const autoSoft =
        confirmationFor(intent.action) === "soft" && intent.source === "voice";
      const applied = await dispatchAppAction(router, intent, {
        confirmWrite: autoSoft,
        enqueueIfNeeded: !autoSoft,
      });
      if (applied.ok && applied.summary) {
        actionLabels.push(applied.summary);
        appliedBits.push(applied.summary);
      } else if (applied.error === "confirmation_required") {
        appliedBits.push(`needs your approval: ${applied.summary}`);
      }
    }

    let spoken = formatVoiceReply(result.assistantText || "");
    if (appliedBits.length && !/created|opened|added|done/i.test(spoken)) {
      spoken = formatVoiceReply(
        `${spoken} ${appliedBits.map((b) => `I ${b.toLowerCase()}.`).join(" ")}`,
      );
    }
    if (!spoken || spoken === "I didn't catch a reply.") {
      spoken = appliedBits.length
        ? formatVoiceReply(`Done. ${appliedBits.join(". ")}.`)
        : "Okay.";
    }

    active?.appendExchange?.(input.heard, {
      ...result,
      assistantText: spoken,
    });

    return {
      reply: spoken,
      toolCalls: result.toolCalls,
      actionLabels,
      usedSharedSession: !!active,
    };
  } catch (e) {
    return {
      reply: "",
      toolCalls: [],
      actionLabels: [],
      error: sanitizeVoiceError(e),
      usedSharedSession: !!active,
    };
  }
}
