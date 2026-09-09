/**
 * Explicit Chat command grammar (v0.2 A7).
 *
 * `/command [args]` → deterministic local command.
 * Anything else → Agent Engine message.
 *
 * Keyword/substring matching is intentionally absent.
 */
import { AGENT_TOOLS, listToolsHelp, type AgentTool } from "./tools";

export type ParsedChatInput =
  | { kind: "empty" }
  | { kind: "agent"; text: string }
  | {
      kind: "command";
      name: string;
      args: string[];
      raw: string;
      tool: AgentTool | "help";
    }
  | {
      kind: "unknown-command";
      name: string;
      args: string[];
      raw: string;
    };

/** Leading slash + ASCII command name. Rest is argument text. */
const COMMAND_RE = /^\/([a-z][a-z0-9-]*)(?:\s+(.*))?$/i;

export function tokenizeCommandArgs(rest: string): string[] {
  return rest.trim().split(/\s+/).filter(Boolean);
}

function canonicalSlash(token: string): string {
  const t = token.trim().toLowerCase();
  return t.startsWith("/") ? t : `/${t}`;
}

/** Registry lookup: primary slash plus optional aliases on AgentTool. */
export function lookupCommand(name: string): AgentTool | undefined {
  const slash = canonicalSlash(name);
  for (const tool of AGENT_TOOLS) {
    if (tool.slash === slash) return tool;
    if (tool.aliases?.some((alias) => canonicalSlash(alias) === slash)) {
      return tool;
    }
  }
  return undefined;
}

export function listRegisteredSlashes(): string[] {
  const names = ["/help", "/tools"];
  for (const tool of AGENT_TOOLS) {
    names.push(tool.slash);
    if (tool.aliases) names.push(...tool.aliases);
  }
  return [...new Set(names)].sort();
}

export function parseChatInput(raw: string): ParsedChatInput {
  const text = raw.trim();
  if (!text) return { kind: "empty" };

  if (!text.startsWith("/")) {
    return { kind: "agent", text };
  }

  const match = text.match(COMMAND_RE);
  if (!match) {
    const token = text.slice(1).split(/\s+/, 1)[0] ?? "";
    return {
      kind: "unknown-command",
      name: token.toLowerCase(),
      args: tokenizeCommandArgs(text.slice(1 + token.length)),
      raw: text,
    };
  }

  const name = match[1].toLowerCase();
  const args = tokenizeCommandArgs(match[2] ?? "");

  if (name === "help" || name === "tools") {
    return { kind: "command", name, args, raw: text, tool: "help" };
  }

  const tool = lookupCommand(name);
  if (!tool) {
    return { kind: "unknown-command", name, args, raw: text };
  }

  return {
    kind: "command",
    name: tool.slash.slice(1),
    args,
    raw: text,
    tool,
  };
}

export function isLocalChatCommand(raw: string): boolean {
  const parsed = parseChatInput(raw);
  return parsed.kind === "command" || parsed.kind === "unknown-command";
}

export function unknownCommandMessage(name: string): string {
  const available = listRegisteredSlashes()
    .map((slash) => `- ${slash}`)
    .join("\n");
  const shown = name ? `\`/${name}\`` : "that slash input";
  return (
    `Unknown command ${shown}.\n\n` +
    "`/` is the OpenMesh command namespace. Ordinary language goes to the agent.\n\n" +
    `Available commands:\n${available}`
  );
}

export function helpCommandText(): string {
  return listToolsHelp();
}
