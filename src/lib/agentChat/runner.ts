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

export async function runAgentChatTurn(
  projectPath: string,
  userMessage: string,
  settings?: Settings | null,
  history?: { role: string; content: string }[],
): Promise<ChatTurnResult> {
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

  const tools = resolveToolsForMessage(trimmed);
  const toolCalls: ChatToolCall[] = [];

  if (tools.length > 0) {
    for (const tool of tools) {
      try {
        const result: AgentToolResult = await tool.run(projectPath, trimmed);
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: result.ok,
          summary: result.summary,
        });
      } catch (e) {
        toolCalls.push({
          toolId: tool.id,
          title: tool.title,
          ok: false,
          summary: e instanceof Error ? e.message : String(e),
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

  // Freeform / LLM tool loop via OpenMesh Agent Engine
  try {
    const result = await runAgentEngineTurn(projectPath, trimmed, {
      messages: history ?? [],
      providerName: settings?.provider?.name,
      model:
        chatModelId(settings ?? null) ||
        settings?.provider?.defaultModel ||
        settings?.models?.codingModel,
      baseUrl: settings?.provider?.apiBaseUrl,
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
