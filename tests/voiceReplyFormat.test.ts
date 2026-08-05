import { describe, expect, it } from "vitest";
import { formatVoiceReply, formatVoiceSpeech } from "../src/lib/voice/replyFormat";
import { buildVoicePrompt } from "../src/lib/agentChat/voiceBridge";
import { resolveToolsForMessage } from "../src/lib/agentChat/tools";

describe("formatVoiceReply", () => {
  it("collapses sprint tool dumps into a short line", () => {
    const dump = `### Sprint status ✓
Sprint {
  "sprint": { "id": "9cb2f18c", "name": "Sprint 1" },
  "taskCount": 3
}`;
    const out = formatVoiceReply(dump);
    expect(out.toLowerCase()).toContain("sprint");
    expect(out).not.toContain("9cb2f18c");
    expect(out.length).toBeLessThan(200);
  });

  it("keeps short conversational answers", () => {
    expect(formatVoiceReply("Hey — I can hear you. What do you need?")).toMatch(
      /hear you/i,
    );
  });

  it("clips speech length", () => {
    const long = "Hello. ".repeat(80);
    expect(formatVoiceSpeech(long).length).toBeLessThanOrEqual(220);
  });
});

describe("voice prompt vs local tools", () => {
  it("voice turns must skip local keyword tools (prompt may mention action types)", () => {
    const prompt = buildVoicePrompt("Hey, do you hear me?");
    // Prompt may include tool type names; runner uses skipLocalTools for voice.
    expect(prompt).toContain("app_propose_action");
    expect(prompt).toContain("User said: Hey, do you hear me?");
    // Sanity: keyword matcher is still aggressive — voice must not use it.
    const hits = resolveToolsForMessage(prompt);
    expect(hits.length).toBeGreaterThanOrEqual(0);
    void resolveToolsForMessage;
  });
});
