import type { Settings } from "../../types";

export type ChatSetupCheck = {
  id: "provider" | "apiKey" | "model";
  label: string;
  done: boolean;
  hint: string;
};

export type ChatReadyOptions = {
  /**
   * Truth from the user secret store / env (`agent_secret_status`).
   * When provided, overrides the settings JSON `apiKeyConfigured` flag so the
   * chat gate matches what the Agent Engine actually checks.
   */
  secretConfigured?: boolean | null;
};

export function chatModelId(settings: Settings | null | undefined): string {
  const defaultModel = settings?.provider?.defaultModel?.trim() ?? "";
  const codingModel = settings?.models?.codingModel?.trim() ?? "";
  return defaultModel || codingModel;
}

function apiKeyDone(
  settings: Settings | null | undefined,
  opts?: ChatReadyOptions,
): boolean {
  if (typeof opts?.secretConfigured === "boolean") {
    return opts.secretConfigured;
  }
  return !!settings?.provider?.apiKeyConfigured;
}

export function getChatSetupChecks(
  settings: Settings | null | undefined,
  opts?: ChatReadyOptions,
): ChatSetupCheck[] {
  const name = settings?.provider?.name?.trim() ?? "";
  const apiKey = apiKeyDone(settings, opts);
  const model = chatModelId(settings);

  return [
    {
      id: "provider",
      label: "Provider name",
      done: name.length > 0,
      hint: "Settings → Provider name",
    },
    {
      id: "apiKey",
      label: "API key",
      done: apiKey,
      hint: "Settings → API key (save to user secret store)",
    },
    {
      id: "model",
      label: "Default model",
      done: model.length > 0,
      hint: "Settings → Default model or Coding model",
    },
  ];
}

export function isChatProviderReady(
  settings: Settings | null | undefined,
  opts?: ChatReadyOptions,
): boolean {
  return getChatSetupChecks(settings, opts).every((c) => c.done);
}
