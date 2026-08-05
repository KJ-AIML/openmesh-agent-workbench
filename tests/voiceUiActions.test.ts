import { describe, expect, it } from "vitest";
import {
  normalizeVoiceRoute,
  parseUiActionsFromToolSteps,
  VOICE_UI_ROUTES,
} from "../src/lib/voice/uiActions";

describe("voice uiActions", () => {
  it("normalizes aliases", () => {
    expect(normalizeVoiceRoute("docs")).toBe("/docs");
    expect(normalizeVoiceRoute("/agent-chat")).toBe("/agent-chat");
    expect(normalizeVoiceRoute("work")).toBe("/");
    expect(normalizeVoiceRoute("/nope")).toBeNull();
  });

  it("parses ui_navigate tool steps", () => {
    const actions = parseUiActionsFromToolSteps([
      {
        toolName: "ui_navigate",
        toolCallId: "1",
        ok: true,
        summary: JSON.stringify({
          ok: true,
          action: "ui_navigate",
          route: "/sprint",
          label: "Sprint",
        }),
      },
      {
        toolName: "project_info",
        toolCallId: "2",
        ok: true,
        summary: "{}",
      },
    ]);
    expect(actions).toEqual([
      { action: "ui_navigate", route: "/sprint", label: "Sprint" },
    ]);
    expect(VOICE_UI_ROUTES["/sprint"]).toBe("Sprint");
  });
});
