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
    mode?: "ask" | "plan" | "act" | "delegate";
    turnId?: string;
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
      mode: opts?.mode ?? "ask",
      turnId: opts?.turnId,
    },
  });
}

export async function cancelAgentEngineTurn(turnId: string): Promise<boolean> {
  return invoke("agent_engine_cancel", { turnId });
}

export type StoredChatSession = {
  id: string;
  title: string;
  titleIsDefault: boolean;
  messages: Array<{
    id: string;
    role: string;
    text: string;
    toolCalls?: unknown;
    at: number;
  }>;
  createdAt: number;
  updatedAt: number;
};

export async function loadDurableChats(
  projectPath: string,
): Promise<StoredChatSession[]> {
  return invoke("agent_chat_load", { projectPath });
}

export async function saveDurableChats(
  projectPath: string,
  sessions: StoredChatSession[],
): Promise<void> {
  return invoke("agent_chat_save", { projectPath, sessions });
}

/** Read-mostly workspace tool for slash/keyword fast paths (no LLM). */
export async function runAgentWorkspaceTool(
  projectPath: string,
  toolName: string,
  argumentsJson: Record<string, unknown> | string = {},
): Promise<string> {
  const payload =
    typeof argumentsJson === "string"
      ? argumentsJson
      : JSON.stringify(argumentsJson ?? {});
  return invoke("agent_workspace_tool", {
    projectPath,
    request: {
      toolName,
      argumentsJson: payload,
    },
  });
}

export type PatchRecord = {
  id: string;
  status: string;
  summary: string;
  files: Array<{ path: string; baseSha256: string; newContent: string }>;
  createdAt: string;
  appliedAt?: string | null;
  rejectedAt?: string | null;
  rolledBackAt?: string | null;
  runId: string;
};

export async function getAgentPatch(
  projectPath: string,
  patchId: string,
): Promise<PatchRecord> {
  return invoke("agent_patch_get", { projectPath, patchId });
}

export async function applyAgentPatch(
  projectPath: string,
  patchId: string,
): Promise<PatchRecord> {
  return invoke("agent_patch_apply", { projectPath, patchId });
}

export async function rejectAgentPatch(
  projectPath: string,
  patchId: string,
): Promise<PatchRecord> {
  return invoke("agent_patch_reject", { projectPath, patchId });
}

export async function rollbackAgentPatch(
  projectPath: string,
  patchId: string,
): Promise<PatchRecord> {
  return invoke("agent_patch_rollback", { projectPath, patchId });
}

export async function summarizeAgentPatch(
  projectPath: string,
  patchId: string,
): Promise<string> {
  return invoke("agent_patch_summary", { projectPath, patchId });
}

export type AgentRecipe = {
  id: string;
  title: string;
  argv: string[];
  cwdRel: string;
  timeoutMs: number;
};

export type RecipeRunResult = {
  recipeId: string;
  ok: boolean;
  exitCode?: number | null;
  timedOut: boolean;
  cancelled: boolean;
  stdout: string;
  stderr: string;
  durationMs: number;
  runId: string;
};

export async function listAgentRecipes(projectPath: string): Promise<AgentRecipe[]> {
  return invoke("agent_recipe_list", { projectPath });
}

export async function runAgentRecipe(
  projectPath: string,
  recipeId: string,
  runKey?: string,
  patchId?: string,
): Promise<RecipeRunResult> {
  return invoke("agent_recipe_run", {
    projectPath,
    request: { recipeId, runKey, patchId: patchId ?? null },
  });
}

export async function suggestAgentRecipe(
  projectPath: string,
  changedPaths?: string[],
): Promise<string> {
  return invoke("agent_recipe_suggest", {
    projectPath,
    changedPaths: changedPaths ?? null,
  });
}

export async function cancelAgentRecipe(runKey: string): Promise<boolean> {
  return invoke("agent_recipe_cancel", { runKey });
}

export async function writeDelegateBrief(
  projectPath: string,
  tool: string,
  summary: string,
): Promise<string> {
  return invoke("agent_delegate_brief", {
    projectPath,
    request: { tool, summary },
  });
}

export async function recordDelegateLaunch(
  projectPath: string,
  tool: string,
  opts?: { briefPath?: string; resumeSessionId?: string },
): Promise<string> {
  return invoke("agent_delegate_record_launch", {
    projectPath,
    request: {
      tool,
      briefPath: opts?.briefPath ?? null,
      resumeSessionId: opts?.resumeSessionId ?? null,
    },
  });
}

export async function approveAgentHandoff(
  projectPath: string,
  handoffId: string,
): Promise<string> {
  return invoke("agent_handoff_approve", { projectPath, handoffId });
}

/** Extract proposed patch id from tool summary / assistant text. */
export function extractPatchIds(text: string): string[] {
  const ids = new Set<string>();
  const re = /"patchId"\s*:\s*"(patch-[a-f0-9]+)"/gi;
  let m: RegExpExecArray | null;
  while ((m = re.exec(text))) {
    ids.add(m[1]);
  }
  const bare = text.match(/\bpatch-[a-f0-9]+\b/gi) ?? [];
  for (const id of bare) ids.add(id);
  return [...ids];
}
