import {
  AGENT_TOOLS,
  listToolsHelp,
  resolveToolsForMessage,
  type AgentToolResult,
} from "./tools";
import { runAgentEngineTurn } from "../agentEngineClient";
import type { Settings } from "../../types";
import { chatModelId, isChatProviderReady } from "./ready";

export type ChatToolCall = {
  toolId: string;
  title: string;
  ok: boolean;
  summary: string;
};

export type ChatTurnResult = {
  assistantText: string;
  toolCalls: ChatToolCall[];
};

/** Lightweight mid-turn status for the in-thread thinking bubble. */
export type ChatTurnProgress =
  | { kind: "phase"; label: string }
  | { kind: "tool_start"; title: string }
  | { kind: "tool_done"; title: string; ok: boolean };

export type ChatTurnOptions = {
  settings?: Settings | null;
  history?: { role: string; content: string }[];
  /** Fired for UI status only — must stay sync/cheap (no stringify/IO). */
  onProgress?: (event: ChatTurnProgress) => void;
  mode?: "ask" | "plan" | "act" | "delegate";
  turnId?: string;
  /**
   * Skip local keyword/slash tool shortcuts (used by voice so prompt words
   * like "Sprint" don't dump JSON into the HUD).
   */
  skipLocalTools?: boolean;
};

export async function runAgentChatTurn(
  projectPath: string,
  userMessage: string,
  settingsOrOpts?: Settings | null | ChatTurnOptions,
  history?: { role: string; content: string }[],
): Promise<ChatTurnResult> {
  // Back-compat: (path, msg, settings, history) or (path, msg, opts).
  const opts: ChatTurnOptions =
    settingsOrOpts !== null &&
    typeof settingsOrOpts === "object" &&
    ("settings" in settingsOrOpts ||
      "history" in settingsOrOpts ||
      "onProgress" in settingsOrOpts ||
      "mode" in settingsOrOpts ||
      "turnId" in settingsOrOpts ||
      "skipLocalTools" in settingsOrOpts)
      ? settingsOrOpts
      : { settings: settingsOrOpts as Settings | null | undefined, history };

  const settings = opts.settings;
  const hist = opts.history ?? history;
  const onProgress = opts.onProgress;

  if (settings !== undefined && !isChatProviderReady(settings)) {
    return {
      assistantText:
        "Chat is locked until provider, API key, and default model are configured in Settings.",
      toolCalls: [],
    };
  }

  const trimmed = userMessage.trim();
  if (!trimmed) {
    return { assistantText: "Say something, or type /tools.", toolCalls: [] };
  }

  if (
    trimmed === "/tools" ||
    trimmed === "/help" ||
    /^help\b/i.test(trimmed) ||
    /^what can you do/i.test(trimmed)
  ) {
    return { assistantText: listToolsHelp(), toolCalls: [] };
  }

  const tools = opts.skipLocalTools ? [] : resolveToolsForMessage(trimmed);
  const toolCalls: ChatToolCall[] = [];

  if (tools.length > 0) {
    onProgress?.({ kind: "phase", label: "Working with tools…" });
    for (const tool of tools) {
      onProgress?.({ kind: "tool_start", title: tool.title });
      try {
        const result: AgentToolResult = await tool.run(projectPath, trimmed);
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: result.ok,
          summary: result.summary,
        });
        onProgress?.({ kind: "tool_done", title: tool.title, ok: result.ok });
      } catch (e) {
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
        });
        onProgress?.({ kind: "tool_done", title: tool.title, ok: false });
      }
    }

    const body = toolCalls
      .map((c) => `### ${c.title} ${c.ok ? "✓" : "✗"}\n${c.summary}`)
      .join("\n\n");

    return {
      assistantText: body,
      toolCalls,
    };
  }

  // Freeform / LLM tool loop via OpenMesh Agent Engine
  onProgress?.({ kind: "phase", label: "Thinking…" });
  try {
    const result = await runAgentEngineTurn(projectPath, trimmed, {
      messages: hist ?? [],
      providerName: settings?.provider?.name,
      model:
        chatModelId(settings ?? null) ||
        settings?.provider?.defaultModel ||
        settings?.models?.codingModel,
      baseUrl: settings?.provider?.apiBaseUrl,
      mode: opts.mode ?? "ask",
      turnId: opts.turnId,
    });

    for (const step of result.toolSteps ?? []) {
      toolCalls.push({
        toolId: step.toolName,
        title: step.toolName,
        ok: step.ok,
        summary: step.summary,
      });
    }

    const errPrefix = result.error ? `${result.error}\n\n` : "";
    return {
      assistantText: `${errPrefix}${result.assistantText}`,
      toolCalls,
    };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    return {
      assistantText:
        `Agent Engine error: ${msg}\n\n` +
        "Check Settings → Provider (API key + model). Slash tools still work without the LLM.\n\n" +
        listToolsHelp(),
      toolCalls: [
        {
          toolId: "agent_engine",
          title: "Agent Engine",
          ok: false,
          summary: msg,
        },
      ],
    };
  }
}

/** Exposed for tests */
export function __test_resolve(message: string) {
  return resolveToolsForMessage(message).map((t) => t.id);
}

export const __test_toolIds = () => AGENT_TOOLS.map((t) => t.id);
