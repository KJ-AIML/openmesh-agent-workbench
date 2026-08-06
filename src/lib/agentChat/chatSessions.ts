// Chat persistence: localStorage cache + durable `.openmesh/agent/chats/` via Tauri.

export type ChatRole = "user" | "assistant" | "system";

export type ChatToolCallRecord = {
  toolId: string;
  title: string;
  ok: boolean;
  summary: string;
};

export type ChatMessage = {
  id: string;
  role: ChatRole;
  text: string;
  toolCalls?: ChatToolCallRecord[];
  at: number;
};

/** Provenance when a chat was seeded from a scanned foreign session (copy only). */
export type ChatImportProvenance = {
  source: string;
  id: string;
  path?: string;
};

export type ChatSession = {
  id: string;
  title: string;
  /** True until the user (or the first exchange) gives it a real title. */
  titleIsDefault: boolean;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
  /** Present when this OpenMesh chat continues a scanned provider session copy. */
  importedFrom?: ChatImportProvenance;
};

const STORAGE_PREFIX = "openmesh.chat.v1";
const MAX_SESSIONS_PER_PROJECT = 50;
const TITLE_MAX_LEN = 42;
export const DEFAULT_CHAT_TITLE = "New chat";

type SimpleStorage = {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
};

/**
 * In-memory fallback so chat still works for the current run when
 * localStorage is unavailable (some locked-down webviews, test runners
 * without a real DOM). Real desktop builds have localStorage.
 */
function createMemoryStorage(): SimpleStorage {
  const map = new Map<string, string>();
  return {
    getItem: (key) => (map.has(key) ? map.get(key)! : null),
    setItem: (key, value) => {
      map.set(key, value);
    },
  };
}

const memoryStorage = createMemoryStorage();

function getStorage(): SimpleStorage {
  try {
    if (typeof localStorage !== "undefined") return localStorage;
  } catch {
    // Accessing localStorage can throw in some restricted contexts.
  }
  return memoryStorage;
}

function storageKey(projectPath: string): string {
  return `${STORAGE_PREFIX}:${projectPath}`;
}

function isChatMessage(v: unknown): v is ChatMessage {
  return (
    !!v &&
    typeof v === "object" &&
    typeof (v as ChatMessage).id === "string" &&
    typeof (v as ChatMessage).text === "string" &&
    typeof (v as ChatMessage).role === "string"
  );
}

function isImportProvenance(v: unknown): v is ChatImportProvenance {
  if (!v || typeof v !== "object") return false;
  const p = v as ChatImportProvenance;
  return typeof p.source === "string" && typeof p.id === "string";
}

function isChatSession(v: unknown): v is ChatSession {
  if (!v || typeof v !== "object") return false;
  const s = v as ChatSession;
  if (
    typeof s.id !== "string" ||
    typeof s.title !== "string" ||
    !Array.isArray(s.messages) ||
    !s.messages.every(isChatMessage) ||
    typeof s.createdAt !== "number" ||
    typeof s.updatedAt !== "number"
  ) {
    return false;
  }
  if (s.importedFrom !== undefined && !isImportProvenance(s.importedFrom)) {
    return false;
  }
  return true;
}

function parseStoredSessions(raw: string | null): ChatSession[] {
  if (!raw) return [];
  try {
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isChatSession);
  } catch {
    return [];
  }
}

/** Newest-first, matching how the chat rail should list conversations. */
export function loadSessions(projectPath: string): ChatSession[] {
  const sessions = parseStoredSessions(getStorage().getItem(storageKey(projectPath)));
  return sessions.slice().sort((a, b) => b.updatedAt - a.updatedAt);
}

export function persistSessions(projectPath: string, sessions: ChatSession[]): void {
  try {
    getStorage().setItem(
      storageKey(projectPath),
      JSON.stringify(sessions.slice(0, MAX_SESSIONS_PER_PROJECT)),
    );
  } catch {
    // Storage unavailable/full — chat keeps working in-memory for this run.
  }
}

/** Load durable disk sessions (Tauri), falling back to localStorage. */
export async function loadSessionsAsync(projectPath: string): Promise<ChatSession[]> {
  try {
    const { loadDurableChats } = await import("../agentEngineClient");
    const disk = await loadDurableChats(projectPath);
    if (Array.isArray(disk) && disk.length > 0) {
      const mapped = disk
        .map((s) => ({
          id: s.id,
          title: s.title,
          titleIsDefault: !!s.titleIsDefault,
          messages: (s.messages ?? [])
            .filter(isChatMessage)
            .map((m) => ({
              ...m,
              toolCalls: Array.isArray(m.toolCalls)
                ? (m.toolCalls as ChatToolCallRecord[])
                : undefined,
            })),
          createdAt: s.createdAt,
          updatedAt: s.updatedAt,
          importedFrom: isImportProvenance(s.importedFrom)
            ? {
                source: s.importedFrom.source,
                id: s.importedFrom.id,
                path:
                  typeof s.importedFrom.path === "string"
                    ? s.importedFrom.path
                    : undefined,
              }
            : undefined,
        }))
        .filter(isChatSession)
        .sort((a, b) => b.updatedAt - a.updatedAt);
      persistSessions(projectPath, mapped);
      return mapped;
    }
  } catch {
    // Web / IPC unavailable — localStorage only.
  }
  return loadSessions(projectPath);
}

/** Write-through: localStorage + durable disk when Tauri is available. */
export async function persistSessionsAsync(
  projectPath: string,
  sessions: ChatSession[],
): Promise<void> {
  const capped = sessions.slice(0, MAX_SESSIONS_PER_PROJECT);
  persistSessions(projectPath, capped);
  try {
    const { saveDurableChats } = await import("../agentEngineClient");
    await saveDurableChats(
      projectPath,
      capped.map((s) => ({
        id: s.id,
        title: s.title,
        titleIsDefault: s.titleIsDefault,
        messages: s.messages.map((m) => ({
          id: m.id,
          role: m.role,
          text: m.text,
          toolCalls: m.toolCalls,
          at: m.at,
        })),
        createdAt: s.createdAt,
        updatedAt: s.updatedAt,
        importedFrom: s.importedFrom,
      })),
    );
  } catch {
    // localStorage already updated
  }
}

/** Exposed for tests only — writes a raw (possibly invalid) payload. */
export function __test_setRawSessions(projectPath: string, raw: string): void {
  getStorage().setItem(storageKey(projectPath), raw);
}

function uid(prefix: string): string {
  return `${prefix}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

export function createChatSession(): ChatSession {
  const now = Date.now();
  return {
    id: uid("chat"),
    title: DEFAULT_CHAT_TITLE,
    titleIsDefault: true,
    messages: [],
    createdAt: now,
    updatedAt: now,
  };
}

export function createChatMessage(
  role: ChatRole,
  text: string,
  toolCalls?: ChatToolCallRecord[],
): ChatMessage {
  return {
    id: uid("msg"),
    role,
    text,
    toolCalls,
    at: Date.now(),
  };
}

/** Deep-enough clone for forked history (new ids; tool call records copied). */
export function cloneChatMessage(message: ChatMessage): ChatMessage {
  return {
    id: uid("msg"),
    role: message.role,
    text: message.text,
    toolCalls: message.toolCalls?.map((t) => ({ ...t })),
    at: message.at,
  };
}

function truncateTitle(title: string): string {
  const cleaned = title.replace(/\s+/g, " ").trim();
  if (!cleaned) return DEFAULT_CHAT_TITLE;
  return cleaned.length > TITLE_MAX_LEN
    ? `${cleaned.slice(0, TITLE_MAX_LEN - 1).trimEnd()}…`
    : cleaned;
}

/** Title for a forked chat — prefers "Fork of …", else first user message. */
export function deriveForkTitle(
  source: ChatSession,
  forkedMessages: ChatMessage[],
): string {
  if (!source.titleIsDefault && source.title.trim()) {
    return truncateTitle(`Fork of ${source.title.trim()}`);
  }
  const firstUser = forkedMessages.find((m) => m.role === "user");
  if (firstUser && isAutoTitleWorthy(firstUser.text)) {
    return truncateTitle(`Fork of ${deriveTitleFromMessage(firstUser.text)}`);
  }
  return truncateTitle(`Fork of ${DEFAULT_CHAT_TITLE}`);
}

/**
 * New session cloning history up to and including `messageIndex`.
 * Returns null when the index is out of range.
 */
export function forkSessionAt(
  session: ChatSession,
  messageIndex: number,
): ChatSession | null {
  if (
    !Number.isInteger(messageIndex) ||
    messageIndex < 0 ||
    messageIndex >= session.messages.length
  ) {
    return null;
  }
  const now = Date.now();
  const messages = session.messages
    .slice(0, messageIndex + 1)
    .map(cloneChatMessage);
  return {
    id: uid("chat"),
    title: deriveForkTitle(session, messages),
    titleIsDefault: false,
    messages,
    createdAt: now,
    updatedAt: now,
    importedFrom: session.importedFrom
      ? { ...session.importedFrom }
      : undefined,
  };
}

/** Slash commands that should never become a raw "/tools"-style title. */
const SLASH_TITLES: Record<string, string> = {
  tools: "Tools list",
  help: "Help",
  pilot: "Pilot status",
  rc: "RC status",
  team: "Team workspace",
  search: "Search",
  git: "Git status",
  pending: "Pending questions",
  docs: "Docs",
  project: "Project info",
  notes: "Notes",
  sprint: "Sprint",
  continuity: "Continuity",
  digest: "Return digest",
  trust: "Trust policy",
  connectors: "Connectors",
  org: "Org graph",
  peers: "Mesh peers",
  ask: "Proxy ask",
};

/** Short / low-signal messages that should not rename the chat. */
const LOW_SIGNAL =
  /^(ok|okay|k|yes|y|no|n|thanks|thank you|thx|ty|hi|hey|hello|yo|sup|cool|sure|np|hmm+|lol|yep|nope)\.?$/i;

/**
 * Turns a user's first message into a short, app-like chat title.
 * Returns {@link DEFAULT_CHAT_TITLE} for empty/low-signal input — callers
 * should keep `titleIsDefault` so a later message can still title the chat.
 */
export function deriveTitleFromMessage(text: string): string {
  const cleaned = text.replace(/\s+/g, " ").trim();
  if (!cleaned || LOW_SIGNAL.test(cleaned)) return DEFAULT_CHAT_TITLE;

  const slash = cleaned.match(/^\/([a-z-]+)\b/i);
  if (slash) {
    const cmd = slash[1].toLowerCase();
    const label = SLASH_TITLES[cmd] ?? `/${cmd}`;
    const rest = cleaned.slice(slash[0].length).trim();
    if (!rest) return label;
    const combined = `${label}: ${rest}`;
    return combined.length > TITLE_MAX_LEN
      ? `${combined.slice(0, TITLE_MAX_LEN - 1).trimEnd()}…`
      : combined;
  }

  return cleaned.length > TITLE_MAX_LEN
    ? `${cleaned.slice(0, TITLE_MAX_LEN - 1).trimEnd()}…`
    : cleaned;
}

/** True when deriveTitleFromMessage produced a real title (not the default placeholder). */
export function isAutoTitleWorthy(text: string): boolean {
  return deriveTitleFromMessage(text) !== DEFAULT_CHAT_TITLE;
}

export function touchSession(session: ChatSession): void {
  session.updatedAt = Date.now();
}
