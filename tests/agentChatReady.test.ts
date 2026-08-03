import { describe, expect, it } from "vitest";
import {
  chatModelId,
  getChatSetupChecks,
  isChatProviderReady,
} from "../src/lib/agentChat/ready";
import type { Settings } from "../src/types";

function baseSettings(over: Partial<Settings> = {}): Settings {
  return {
    workspace: { theme: "dark" },
    provider: {
      apiKeyConfigured: false,
      usageTrackingEnabled: false,
    },
    models: { localModelEnabled: false },
    server: {
      mode: "local",
      apiBaseUrl: "",
      healthStatus: "unknown",
      syncStatus: "unknown",
    },
    agentClis: {},
    sessionDirs: {
      codexEnabled: false,
      claudeCodeEnabled: false,
      opencodeEnabled: false,
      cursorEnabled: false,
      geminiEnabled: false,
      grokEnabled: false,
    },
    localPaths: {},
    appearance: { theme: "dark" },
    ...over,
  } as Settings;
}

describe("chat provider gate", () => {
  it("blocks until provider, api key, and model are set", () => {
    expect(isChatProviderReady(baseSettings())).toBe(false);
    expect(
      isChatProviderReady(
        baseSettings({
          provider: {
            name: "OpenAI",
            apiKeyConfigured: true,
            usageTrackingEnabled: false,
            defaultModel: "gpt-4",
          },
        }),
      ),
    ).toBe(true);
  });

  it("accepts coding model when default model empty", () => {
    const s = baseSettings({
      provider: {
        name: "Anthropic",
        apiKeyConfigured: true,
        usageTrackingEnabled: false,
      },
      models: { codingModel: "claude-sonnet", localModelEnabled: false },
    });
    expect(chatModelId(s)).toBe("claude-sonnet");
    expect(isChatProviderReady(s)).toBe(true);
  });

  it("lists missing checks", () => {
    const checks = getChatSetupChecks(baseSettings());
    expect(checks.every((c) => !c.done)).toBe(true);
  });
});
