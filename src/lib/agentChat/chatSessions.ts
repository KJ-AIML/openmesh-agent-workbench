// Local persistence for Chat app conversations ("chats"), scoped per project.
// Deliberately lightweight: localStorage only, no Rust backend command,
// so the Agent Engine wiring in runner.ts stays untouched.

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

export type ChatSession = {
  id: string;
  title: string;
  /** True until the user (or the first exchange) gives it a real title. */
  titleIsDefault: boolean;
  messages: ChatMessage[];
  createdAt: number;
  updatedAt: number;
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

function isChatSession(v: unknown): v is ChatSession {
  if (!v || typeof v !== "object") return false;
  const s = v as ChatSession;
  return (
    typeof s.id === "string" &&
    typeof s.title === "string" &&
    Array.isArray(s.messages) &&
    s.messages.every(isChatMessage) &&
    typeof s.createdAt === "number" &&
    typeof s.updatedAt === "number"
  );
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

/** Turns a user's first message into a short, app-like chat title. */
export function deriveTitleFromMessage(text: string): string {
  const cleaned = text.replace(/\s+/g, " ").trim();
  if (!cleaned) return DEFAULT_CHAT_TITLE;
  return cleaned.length > TITLE_MAX_LEN
    ? `${cleaned.slice(0, TITLE_MAX_LEN - 1).trimEnd()}…`
    : cleaned;
}

export function touchSession(session: ChatSession): void {
  session.updatedAt = Date.now();
}
