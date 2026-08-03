import { invoke } from "@tauri-apps/api/core";

export type AgentSecretStatus = {
  configured: boolean;
  store: string;
};

export type AgentToolStep = {
  toolName: string;
  toolCallId: string;
  ok: boolean;
  summary: string;
};

export type EngineTurnResult = {
  assistantText: string;
  toolSteps: AgentToolStep[];
  iterations: number;
  model: string;
  provider: string;
  refused: boolean;
  error?: string | null;
};

export type AgentUiMessage = {
  role: string;
  content: string;
};

export async function getAgentSecretStatus(): Promise<AgentSecretStatus> {
  return invoke("agent_secret_status");
}

export async function setAgentSecret(apiKey: string): Promise<AgentSecretStatus> {
  return invoke("agent_secret_set", { apiKey });
}

export async function clearAgentSecret(): Promise<AgentSecretStatus> {
  return invoke("agent_secret_clear");
}

export type ProviderProbeResult = {
  ok: boolean;
  model: string;
  baseUrl: string;
  latencyMs: number;
  replyPreview?: string | null;
  error?: string | null;
};

export async function testAgentProvider(opts: {
  providerName?: string;
  model?: string;
  baseUrl?: string;
  /** Unsaved key from the input — not persisted by this call */
  apiKey?: string;
}): Promise<ProviderProbeResult> {
  return invoke("agent_provider_test", {
    request: {
      providerName: opts.providerName,
      model: opts.model,
      baseUrl: opts.baseUrl,
      apiKey: opts.apiKey,
    },
  });
}

export async function runAgentEngineTurn(
  projectPath: string,
  question: string,
  opts?: {
    messages?: AgentUiMessage[];
    providerName?: string;
    model?: string;
    baseUrl?: string;
  },
): Promise<EngineTurnResult> {
  return invoke("agent_engine_turn", {
    projectPath,
    request: {
      question,
      messages: opts?.messages ?? [],
      providerName: opts?.providerName,
      model: opts?.model,
      baseUrl: opts?.baseUrl,
    },
  });
}
