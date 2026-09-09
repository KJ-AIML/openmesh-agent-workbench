import {
  AGENT_TOOLS,
  listToolsHelp,
  type AgentToolResult,
} from "./tools";
import {
  helpCommandText,
  parseChatInput,
  unknownCommandMessage,
} from "./commandRouting";
import {
  runAgentEngineTurn,
  type EngineRouteMetadata,
} from "../agentEngineClient";
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
  route?: EngineRouteMetadata;
  model?: string;
  iterations?: number;
};

/** Lightweight mid-turn status for the in-thread thinking bubble + session runs. */
export type ChatTurnProgress =
  | { kind: "phase"; label: string }
  | {
      kind: "tool_start";
      title: string;
      toolId?: string;
      callId?: string;
    }
  | {
      kind: "tool_done";
      title: string;
      ok: boolean;
      toolId?: string;
      callId?: string;
      summary?: string;
    };

export type ChatTurnOptions = {
  settings?: Settings | null;
  /** Runtime readiness for the selected direct provider route. */
  runtimeReady?: boolean | null;
  history?: { role: string; content: string }[];
  /** Fired for UI status only — must stay sync/cheap (no stringify/IO). */
  onProgress?: (event: ChatTurnProgress) => void;
  mode?: "ask" | "plan" | "act" | "delegate";
  turnId?: string;
  /**
   * Skip the `/` command namespace entirely (voice). Transcribed text always
   * goes to Agent Engine; A7 does not parse spoken slash commands.
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
      "runtimeReady" in settingsOrOpts ||
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

  const trimmed = userMessage.trim();
  if (!trimmed) {
    return { assistantText: "Say something, or type /tools.", toolCalls: [] };
  }

  const parsed = opts.skipLocalTools
    ? ({ kind: "agent", text: trimmed } as const)
    : parseChatInput(trimmed);

  if (parsed.kind === "unknown-command") {
    return {
      assistantText: unknownCommandMessage(parsed.name),
      toolCalls: [],
    };
  }

  if (parsed.kind === "command" && parsed.tool === "help") {
    return { assistantText: helpCommandText(), toolCalls: [] };
  }

  const tools =
    parsed.kind === "command" && parsed.tool !== "help" ? [parsed.tool] : [];
  const toolCalls: ChatToolCall[] = [];

  if (tools.length > 0) {
    onProgress?.({ kind: "phase", label: "Working with tools…" });
    for (const tool of tools) {
      onProgress?.({
        kind: "tool_start",
        title: tool.title,
        toolId: tool.id,
      });
      try {
        const result: AgentToolResult = await tool.run(projectPath, trimmed);
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: result.ok,
          summary: result.summary,
        });
        onProgress?.({
          kind: "tool_done",
          title: tool.title,
          ok: result.ok,
          toolId: tool.id,
          summary: result.summary,
        });
      } catch (e) {
        const summary = e instanceof Error ? e.message : String(e);
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: false,
          summary,
        });
        onProgress?.({
          kind: "tool_done",
          title: tool.title,
          ok: false,
          toolId: tool.id,
          summary,
        });
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

  const routeNotReady =
    opts.runtimeReady !== undefined
      ? opts.runtimeReady !== true
      : settings !== undefined && !isChatProviderReady(settings);
  if (settings !== undefined && routeNotReady) {
    return {
      assistantText:
        "Workspace chat is available. Configure the selected provider, API key, and model in Settings to enable free-form model replies.",
      toolCalls: [],
    };
  }

  // Freeform / LLM tool loop via OpenMesh Agent Engine.
  // Mid-turn tool progress is emitted as Tauri `agent-turn-progress` and
  // forwarded by the Chat page (listenAgentTurnProgress) into onProgress.
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

    const errPrefix =
      result.error && result.error !== "provider_request_failed"
        ? `${result.error}\n\n`
        : "";
    return {
      assistantText: `${errPrefix}${result.assistantText}`,
      toolCalls,
      route: result.route ?? undefined,
      model: result.model,
      iterations: result.iterations,
    };
  } catch {
    return {
      assistantText:
        "Agent Engine error: provider request failed (details redacted).\n\n" +
        "Check Settings → Provider (provider key + model). Slash tools still work without the LLM.\n\n" +
        listToolsHelp(),
      toolCalls: [
        {
          toolId: "agent_engine",
          title: "Agent Engine",
          ok: false,
          summary: "provider request failed (details redacted)",
        },
      ],
    };
  }
}

/** Exposed for tests: local command tool ids, empty for agent messages. */
export function __test_resolve(message: string) {
  const parsed = parseChatInput(message);
  if (parsed.kind === "command" && parsed.tool !== "help") {
    return [parsed.tool.id];
  }
  return [];
}

export const __test_toolIds = () => AGENT_TOOLS.map((t) => t.id);
