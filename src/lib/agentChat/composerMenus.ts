/**
 * Rich `/` and `@` composer menu inventories — only OpenMesh-supported items.
 */
import { AGENT_TOOLS } from "./tools";
import type { SessionRun } from "./sessionRuns";
import type { ShellTab } from "./shellTabs";
import { shortCwdLabel } from "./shellTabs";

export type SlashMenuIcon =
  | "pilot"
  | "read"
  | "diff"
  | "verify"
  | "continue"
  | "search"
  | "git"
  | "tools"
  | "generic";

export type SlashMenuItem = {
  id: string;
  slash: string;
  label: string;
  description: string;
  icon: SlashMenuIcon;
  /** Text inserted into the composer (trailing space for arg tools). */
  insert: string;
};

export type MentionKind =
  | "project"
  | "file"
  | "doc"
  | "note"
  | "terminal"
  | "shell"
  | "canvas";

export type MentionMenuItem = {
  id: string;
  kind: MentionKind;
  label: string;
  description: string;
  /** Replace the `@query` token with this text (omit for action-only). */
  insert?: string;
  action?: "open-canvas" | "select-terminal" | "focus-shell" | "open-terminal-panel";
  actionId?: string;
};

/** Starter slash commands shown first (match slim composer redesign). */
export const STARTER_SLASHES = [
  "/pilot",
  "/read",
  "/diff",
  "/verify",
  "/continue",
] as const;

const ICON_BY_SLASH: Record<string, SlashMenuIcon> = {
  "/pilot": "pilot",
  "/read": "read",
  "/diff": "diff",
  "/verify": "verify",
  "/continue": "continue",
  "/search": "search",
  "/git": "git",
  "/grep": "search",
  "/docs": "read",
  "/ls": "read",
  "/tools": "tools",
  "/help": "tools",
};

function needsArgSpace(slash: string): boolean {
  return [
    "/read",
    "/search",
    "/ls",
    "/grep",
    "/diff",
    "/patch",
    "/verify",
    "/delegate",
    "/continue",
    "/ask",
  ].includes(slash);
}

export function iconForSlash(slash: string): SlashMenuIcon {
  return ICON_BY_SLASH[slash] ?? "generic";
}

/** Full slash inventory: starters first, then remaining AGENT_TOOLS, then help. */
export function buildSlashMenuItems(): SlashMenuItem[] {
  const bySlash = new Map(
    AGENT_TOOLS.map((t) => [
      t.slash,
      {
        id: t.id,
        slash: t.slash,
        label: t.title,
        description: t.description,
        icon: iconForSlash(t.slash),
        insert: needsArgSpace(t.slash) ? `${t.slash} ` : t.slash,
      } satisfies SlashMenuItem,
    ]),
  );

  const starters: SlashMenuItem[] = [];
  for (const s of STARTER_SLASHES) {
    const hit = bySlash.get(s);
    if (hit) {
      starters.push(hit);
      bySlash.delete(s);
    }
  }

  const rest = [...bySlash.values()].sort((a, b) =>
    a.slash.localeCompare(b.slash),
  );

  const help: SlashMenuItem[] = [
    {
      id: "tools_help",
      slash: "/tools",
      label: "All tools",
      description: "List every workspace agent slash command",
      icon: "tools",
      insert: "/tools",
    },
    {
      id: "help",
      slash: "/help",
      label: "Help",
      description: "Same as /tools — command inventory",
      icon: "tools",
      insert: "/help",
    },
  ];

  return [...starters, ...rest, ...help];
}

export function filterSlashMenuItems(
  items: SlashMenuItem[],
  query: string,
): SlashMenuItem[] {
  const q = query.replace(/^\//, "").trim().toLowerCase();
  if (!q) return items;
  return items.filter((item) => {
    const hay = `${item.slash} ${item.label} ${item.description}`.toLowerCase();
    return hay.includes(q) || item.slash.slice(1).startsWith(q);
  });
}

/** Parse trailing slash token: `/` or `/pil` at start or after whitespace. */
export function matchSlashToken(
  input: string,
): { start: number; query: string } | null {
  const m = input.match(/(?:^|\s)(\/[a-z-]*)$/i);
  if (!m || m.index == null) return null;
  const token = m[1]!;
  const start = m.index + (m[0].length - token.length);
  return { start, query: token };
}

/** Parse trailing @ token. */
export function matchMentionToken(
  input: string,
): { start: number; query: string } | null {
  const m = input.match(/(?:^|\s)(@[^\s]*)$/);
  if (!m || m.index == null) return null;
  const token = m[1]!;
  const start = m.index + (m[0].length - token.length);
  return { start, query: token };
}

export type MentionMenuContext = {
  projectPath?: string | null;
  projectName?: string | null;
  /** Relative file paths from workspace list_dir (files only). */
  files?: string[];
  docs?: { name: string; path: string }[];
  notes?: { name: string; path: string }[];
  terminalRuns?: SessionRun[];
  shellTabs?: ShellTab[];
  showCanvas?: boolean;
};

export function buildMentionMenuItems(
  ctx: MentionMenuContext,
): MentionMenuItem[] {
  const items: MentionMenuItem[] = [];

  if (ctx.projectPath) {
    items.push({
      id: "project",
      kind: "project",
      label: ctx.projectName?.trim() || "Project",
      description: shortCwdLabel(ctx.projectPath, 56),
      insert: "/project",
    });
  }

  if (ctx.showCanvas !== false) {
    items.push({
      id: "canvas",
      kind: "canvas",
      label: "Canvas",
      description: "Open the project canvas",
      action: "open-canvas",
    });
  }

  for (const tab of ctx.shellTabs ?? []) {
    items.push({
      id: `shell:${tab.id}`,
      kind: "shell",
      label: tab.label,
      description: `${shortCwdLabel(tab.cwd, 40)} · terminal`,
      action: "focus-shell",
      actionId: tab.id,
      insert: `@shell:${tab.label}`,
    });
  }

  for (const run of ctx.terminalRuns ?? []) {
    items.push({
      id: `term:${run.id}`,
      kind: "terminal",
      label: run.title,
      description: run.command,
      action: "select-terminal",
      actionId: run.id,
      insert: `@terminal:${run.title}`,
    });
  }

  for (const f of ctx.files ?? []) {
    const path = f.replace(/^\.\//, "");
    if (!path || path.endsWith("/")) continue;
    items.push({
      id: `file:${path}`,
      kind: "file",
      label: path.split("/").pop() || path,
      description: path,
      insert: `/read ${path}`,
    });
  }

  for (const d of ctx.docs ?? []) {
    const name = d.name || d.path;
    items.push({
      id: `doc:${name}`,
      kind: "doc",
      label: name,
      description: "Project doc",
      insert: `/docs`,
    });
  }

  for (const n of ctx.notes ?? []) {
    const name = n.name || n.path;
    items.push({
      id: `note:${name}`,
      kind: "note",
      label: name,
      description: "Project note",
      insert: `/notes`,
    });
  }

  // Always offer opening the terminal panel when shells exist or not.
  items.push({
    id: "open-terminal-panel",
    kind: "shell",
    label: "Terminals",
    description: "Open the Chat terminal panel",
    action: "open-terminal-panel",
  });

  return items;
}

export function filterMentionMenuItems(
  items: MentionMenuItem[],
  query: string,
): MentionMenuItem[] {
  const q = query.replace(/^@/, "").trim().toLowerCase();
  if (!q) return items;
  return items.filter((item) => {
    const hay =
      `${item.kind} ${item.label} ${item.description} ${item.insert ?? ""}`.toLowerCase();
    return hay.includes(q);
  });
}

/** Replace [start, end) in text with replacement. */
export function replaceToken(
  input: string,
  start: number,
  end: number,
  replacement: string,
): string {
  return `${input.slice(0, start)}${replacement}${input.slice(end)}`;
}
