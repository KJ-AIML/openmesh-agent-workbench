import type { Settings } from "../../types";

export type ChatSetupCheck = {
  id: "provider" | "apiKey" | "model";
  label: string;
  done: boolean;
  hint: string;
};

export function chatModelId(settings: Settings | null | undefined): string {
  const defaultModel = settings?.provider?.defaultModel?.trim() ?? "";
  const codingModel = settings?.models?.codingModel?.trim() ?? "";
  return defaultModel || codingModel;
}

export function getChatSetupChecks(
  settings: Settings | null | undefined,
): ChatSetupCheck[] {
  const name = settings?.provider?.name?.trim() ?? "";
  const apiKey = !!settings?.provider?.apiKeyConfigured;
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
      hint: "Settings → API key (mark configured)",
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
): boolean {
  return getChatSetupChecks(settings).every((c) => c.done);
}
