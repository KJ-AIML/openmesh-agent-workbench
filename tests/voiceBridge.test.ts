import { describe, expect, it } from "vitest";
import { buildVoicePrompt, registerVoiceChatLink } from "../src/lib/agentChat/voiceBridge";
import { sanitizeVoiceError } from "../src/lib/voice/errors";
import { setAppContext, clearAppContext } from "../src/lib/appActions/context";
import {
  parseActionIntentsFromToolSteps,
} from "../src/lib/appActions/dispatcher";

describe("voiceBridge", () => {
  it("builds a short voice prompt with optional route context", () => {
    clearAppContext();
    setAppContext({ route: "/notes" });
    const prompt = buildVoicePrompt("open sprint");
    expect(prompt).toContain("[OpenMesh Voice");
    expect(prompt).toContain("User said: open sprint");
    expect(prompt).toContain("looking at /notes");
    expect(prompt).toContain("createNote");
    clearAppContext();
  });

  it("registers and clears the shared chat link", () => {
    registerVoiceChatLink({
      getHistory: () => [{ role: "user", content: "hi" }],
      getMode: () => "act",
    });
    registerVoiceChatLink(null);
  });
});

describe("sanitizeVoiceError", () => {
  it("redacts api keys and shortens noise", () => {
    expect(sanitizeVoiceError("key sk-or-v1-abcdefghijklmnop failed")).toContain(
      "[redacted]",
    );
    expect(sanitizeVoiceError("No API key for STT")).toMatch(/Settings/);
  });
});

describe("parseActionIntentsFromToolSteps", () => {
  it("prefers typed appAction payload", () => {
    const intents = parseActionIntentsFromToolSteps(
      [
        {
          toolName: "ui_navigate",
          ok: true,
          summary: JSON.stringify({
            ok: true,
            action: "ui_navigate",
            route: "/docs",
            label: "Docs",
            appAction: { type: "navigate", route: "/docs" },
          }),
        },
      ],
      "voice",
    );
    expect(intents).toEqual([
      { action: { type: "navigate", route: "/docs" }, source: "voice" },
    ]);
  });
});
