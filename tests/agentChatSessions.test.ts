import { describe, expect, it } from "vitest";
import {
  DEFAULT_CHAT_TITLE,
  __test_setRawSessions,
  createChatMessage,
  createChatSession,
  deriveTitleFromMessage,
  loadSessions,
  persistSessions,
  touchSession,
} from "../src/lib/agentChat/chatSessions";

describe("createChatSession", () => {
  it("creates an empty session with a default app-like title", () => {
    const s = createChatSession();
    expect(s.title).toBe(DEFAULT_CHAT_TITLE);
    expect(s.titleIsDefault).toBe(true);
    expect(s.messages).toEqual([]);
  });

  it("generates unique ids", () => {
    const a = createChatSession();
    const b = createChatSession();
    expect(a.id).not.toBe(b.id);
  });
});

describe("deriveTitleFromMessage", () => {
  it("falls back to the default title for empty input", () => {
    expect(deriveTitleFromMessage("   ")).toBe(DEFAULT_CHAT_TITLE);
  });

  it("collapses whitespace and trims", () => {
    expect(deriveTitleFromMessage("  what's   in\n\ndocs?  ")).toBe("what's in docs?");
  });

  it("truncates long messages with an ellipsis", () => {
    const long = "a".repeat(80);
    const title = deriveTitleFromMessage(long);
    expect(title.length).toBeLessThanOrEqual(42);
    expect(title.endsWith("…")).toBe(true);
  });
});

describe("session persistence", () => {
  it("round-trips sessions through storage per project", () => {
    const session = createChatSession();
    session.messages.push(createChatMessage("user", "hello"));
    persistSessions("/tmp/project-roundtrip", [session]);

    const loaded = loadSessions("/tmp/project-roundtrip");
    expect(loaded).toHaveLength(1);
    expect(loaded[0].id).toBe(session.id);
    expect(loaded[0].messages).toHaveLength(1);
    expect(loaded[0].messages[0].text).toBe("hello");
  });

  it("scopes sessions per project path", () => {
    persistSessions("/tmp/project-scope-a", [createChatSession()]);
    expect(loadSessions("/tmp/project-scope-b")).toEqual([]);
  });

  it("sorts sessions newest-updated first", () => {
    const older = createChatSession();
    const newer = createChatSession();
    older.updatedAt = 1000;
    newer.updatedAt = 2000;
    persistSessions("/tmp/project-sort", [older, newer]);

    const loaded = loadSessions("/tmp/project-sort");
    expect(loaded.map((s) => s.id)).toEqual([newer.id, older.id]);
  });

  it("ignores corrupt stored JSON instead of throwing", () => {
    __test_setRawSessions("/tmp/project-corrupt", "{not json");
    expect(loadSessions("/tmp/project-corrupt")).toEqual([]);
  });

  it("ignores non-array stored payloads", () => {
    __test_setRawSessions("/tmp/project-nonarray", JSON.stringify({ foo: "bar" }));
    expect(loadSessions("/tmp/project-nonarray")).toEqual([]);
  });
});

describe("touchSession", () => {
  it("bumps updatedAt", () => {
    const s = createChatSession();
    const before = s.updatedAt;
    s.updatedAt = before - 10_000;
    touchSession(s);
    expect(s.updatedAt).toBeGreaterThan(before - 10_000);
  });
});
