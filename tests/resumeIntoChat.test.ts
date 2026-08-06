import { describe, expect, it } from "vitest";
import {
  buildLocalSummary,
  buildResumedChatSession,
  type ForeignTranscript,
  type ResumeSourceMeta,
} from "../src/lib/agentChat/resumeIntoChat";

const meta: ResumeSourceMeta = {
  source: "cursor",
  id: "sess-1",
  path: "/tmp/agent-transcripts/sess-1/sess-1.jsonl",
  title: "Fix login",
  summaryPreview: "User asked about auth",
};

function sampleTranscript(
  overrides: Partial<ForeignTranscript> = {},
): ForeignTranscript {
  return {
    tool: "cursor",
    path: meta.path!,
    title: "Fix login",
    messages: [
      { role: "user", text: "Fix the login bug" },
      { role: "assistant", text: "Looking at auth.ts" },
      { role: "user", text: "Also add tests" },
      { role: "assistant", text: "Added unit tests" },
    ],
    truncated: false,
    previewOnly: false,
    ...overrides,
  };
}

describe("buildLocalSummary", () => {
  it("includes title and first/last turns", () => {
    const summary = buildLocalSummary(sampleTranscript(), meta);
    expect(summary).toContain("Fix login");
    expect(summary).toContain("Fix the login bug");
    expect(summary).toContain("Added unit tests");
    expect(summary).toContain("Turns available: 4");
  });

  it("falls back to preview when no messages", () => {
    const summary = buildLocalSummary(
      sampleTranscript({ messages: [], previewOnly: true }),
      meta,
    );
    expect(summary).toContain("Preview-only");
    expect(summary).toContain("User asked about auth");
  });
});

describe("buildResumedChatSession", () => {
  it("summarize path seeds system provenance + summary and sets importedFrom", () => {
    const session = buildResumedChatSession(
      "summarize",
      meta,
      sampleTranscript(),
    );
    expect(session.importedFrom).toEqual({
      source: "cursor",
      id: "sess-1",
      path: meta.path,
    });
    expect(session.title).toBe("Fix login");
    expect(session.titleIsDefault).toBe(false);
    const systemTexts = session.messages
      .filter((m) => m.role === "system")
      .map((m) => m.text);
    expect(systemTexts.some((t) => t.includes("was not modified"))).toBe(true);
    expect(systemTexts.some((t) => t.includes("Agent Engine"))).toBe(true);
    expect(systemTexts.some((t) => t.includes("Fix the login bug"))).toBe(true);
    expect(session.messages.some((m) => m.role === "assistant")).toBe(true);
  });

  it("import path copies user/assistant turns and labels as a copy", () => {
    const session = buildResumedChatSession("import", meta, sampleTranscript());
    const userAssistant = session.messages.filter(
      (m) => m.role === "user" || m.role === "assistant",
    );
    expect(userAssistant.some((m) => m.text === "Fix the login bug")).toBe(true);
    expect(userAssistant.some((m) => m.text === "Looking at auth.ts")).toBe(true);
    expect(
      session.messages.some((m) =>
        m.text.includes("not the live provider thread"),
      ),
    ).toBe(true);
  });

  it("import preserves per-turn roles (human/model aliases → user/assistant)", () => {
    const session = buildResumedChatSession(
      "import",
      { ...meta, source: "grok", id: "grok_019fc668-demo" },
      sampleTranscript({
        tool: "grok",
        messages: [
          { role: "human", text: "Fix the login bug" },
          { role: "model", text: "Hi! How can I help you today?" },
          { role: "user", text: "Also add tests" },
          { role: "assistant", text: "Added unit tests" },
          { role: "system", text: "Provider context note" },
          { role: "tool", text: "should be skipped" },
        ],
      }),
    );
    const roles = session.messages.map((m) => m.role);
    expect(roles).toContain("user");
    expect(roles).toContain("assistant");
    expect(roles).toContain("system");

    const turns = session.messages.filter(
      (m) =>
        m.text === "Fix the login bug" ||
        m.text === "Hi! How can I help you today?" ||
        m.text === "Also add tests" ||
        m.text === "Added unit tests" ||
        m.text === "Provider context note",
    );
    expect(turns.map((m) => m.role)).toEqual([
      "user",
      "assistant",
      "user",
      "assistant",
      "system",
    ]);
    expect(session.messages.some((m) => m.text === "should be skipped")).toBe(
      false,
    );
  });

  it("summarize path labels roles for both user and assistant turns", () => {
    const summary = buildLocalSummary(
      sampleTranscript({
        messages: [
          { role: "human", text: "Hello from human" },
          { role: "grok", text: "Hello from grok" },
        ],
      }),
      { ...meta, source: "grok" },
    );
    expect(summary).toContain("[user] Hello from human");
    expect(summary).toContain("[assistant] Hello from grok");
  });

  it("import falls back to summary when transcript empty", () => {
    const session = buildResumedChatSession(
      "import",
      meta,
      sampleTranscript({ messages: [], previewOnly: true }),
    );
    expect(
      session.messages.some((m) =>
        m.text.includes("Full message history was not available"),
      ),
    ).toBe(true);
    expect(session.importedFrom?.source).toBe("cursor");
  });

  it("works with null transcript using preview meta only", () => {
    const session = buildResumedChatSession("summarize", meta, null);
    expect(session.messages.some((m) => m.text.includes("User asked about auth"))).toBe(
      true,
    );
  });
});
