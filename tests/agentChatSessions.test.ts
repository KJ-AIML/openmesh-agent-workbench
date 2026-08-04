import { describe, expect, it } from "vitest";
import {
  DEFAULT_CHAT_TITLE,
  __test_setRawSessions,
  cloneChatMessage,
  createChatMessage,
  createChatSession,
  deriveForkTitle,
  deriveTitleFromMessage,
  forkSessionAt,
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

  it("keeps the default title for low-signal messages", () => {
    expect(deriveTitleFromMessage("ok")).toBe(DEFAULT_CHAT_TITLE);
    expect(deriveTitleFromMessage("thanks")).toBe(DEFAULT_CHAT_TITLE);
  });

  it("uses friendly titles for slash commands", () => {
    expect(deriveTitleFromMessage("/tools")).toBe("Tools list");
    expect(deriveTitleFromMessage("/pilot")).toBe("Pilot status");
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

describe("cloneChatMessage", () => {
  it("copies fields with a new id", () => {
    const original = createChatMessage("assistant", "hello", [
      { toolId: "t1", title: "Tool", ok: true, summary: "ok" },
    ]);
    const cloned = cloneChatMessage(original);
    expect(cloned.id).not.toBe(original.id);
    expect(cloned.text).toBe("hello");
    expect(cloned.role).toBe("assistant");
    expect(cloned.toolCalls).toEqual(original.toolCalls);
    expect(cloned.toolCalls).not.toBe(original.toolCalls);
  });
});

describe("forkSessionAt", () => {
  it("returns null for out-of-range indexes", () => {
    const s = createChatSession();
    s.messages.push(createChatMessage("user", "hi"));
    expect(forkSessionAt(s, -1)).toBeNull();
    expect(forkSessionAt(s, 1)).toBeNull();
    expect(forkSessionAt(s, 0.5)).toBeNull();
  });

  it("clones history up to and including the message index", () => {
    const s = createChatSession();
    s.title = "Docs dig";
    s.titleIsDefault = false;
    s.messages.push(
      createChatMessage("system", "welcome"),
      createChatMessage("user", "what is in docs?"),
      createChatMessage("assistant", "Plenty."),
      createChatMessage("user", "more detail"),
    );

    const forked = forkSessionAt(s, 2);
    expect(forked).not.toBeNull();
    expect(forked!.id).not.toBe(s.id);
    expect(forked!.messages).toHaveLength(3);
    expect(forked!.messages.map((m) => m.text)).toEqual([
      "welcome",
      "what is in docs?",
      "Plenty.",
    ]);
    expect(forked!.messages.every((m, i) => m.id !== s.messages[i].id)).toBe(
      true,
    );
    expect(forked!.title).toBe("Fork of Docs dig");
    expect(forked!.titleIsDefault).toBe(false);
    // Source session left intact
    expect(s.messages).toHaveLength(4);
  });

  it("derives a fork title from the first user message when source is untitled", () => {
    const s = createChatSession();
    s.messages.push(
      createChatMessage("system", "welcome"),
      createChatMessage("user", "explain the sprint board"),
    );
    const forked = forkSessionAt(s, 1);
    expect(forked!.title).toBe("Fork of explain the sprint board");
  });
});

describe("deriveForkTitle", () => {
  it("prefixes the source title when it is not the default", () => {
    const s = createChatSession();
    s.title = "Pilot check";
    s.titleIsDefault = false;
    expect(deriveForkTitle(s, [])).toBe("Fork of Pilot check");
  });
});
