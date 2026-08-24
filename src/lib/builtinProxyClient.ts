import { invoke } from "@tauri-apps/api/core";

export type BuiltInProxyStatus = {
  ownership: "built-in";
  mode: "managed";
  running: boolean;
  bindHost?: string | null;
  port?: number | null;
  endpoint?: string | null;
  apiKeyConfigured: boolean;
  upstreamCount: number;
  modelCount: number;
  error?: string | null;
};

export type BuiltInProxyModel = {
  id: string;
  ownedBy: string;
  capabilities: string[];
};

export type BuiltInProxyManagementUpstream = {
  id: string;
  baseUrl: string;
  protocol: "open-ai-compatible" | "anthropic" | "gemini";
  apiKeyConfigured: boolean;
  enabled: boolean;
  priority: number;
  accountId?: string | null;
  oauthProvider?: string | null;
  models: BuiltInProxyModel[];
};

export type BuiltInProxyManagementConfig = {
  bindHost: string;
  port: number;
  allowUnauthenticated: boolean;
  requestTimeoutSecs: number;
  routingStrategy: "first-compatible" | "round-robin" | "fill-first";
  maxRetries: number;
  apiKeyCount: number;
  upstreamCount: number;
  modelAliasCount: number;
  upstreams: BuiltInProxyManagementUpstream[];
  modelAliases: Record<string, string>;
  modelFallbacks: Record<string, string[]>;
};

const STATUS_ERROR = "OpenMesh built-in proxy status is invalid.";
const SAFE_RUNTIME_ERROR = "The OpenMesh built-in proxy reported an error.";

function invalidStatus(): Error {
  return new Error(STATUS_ERROR);
}

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw invalidStatus();
  }
  return value as Record<string, unknown>;
}

function optionalString(value: unknown, maxLength = 240): string | null | undefined {
  if (value === undefined || value === null) return value;
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    throw invalidStatus();
  }
  if ([...value].some((character) => (character.codePointAt(0) ?? 0) <= 0x1f)) {
    throw invalidStatus();
  }
  return value;
}

function optionalPort(value: unknown): number | null | undefined {
  if (value === undefined || value === null) return value;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1 || value > 65535) {
    throw invalidStatus();
  }
  return value;
}

function count(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > 100_000) {
    throw invalidStatus();
  }
  return value;
}

function endpoint(value: unknown): string | null | undefined {
  const parsed = optionalString(value);
  if (parsed === undefined || parsed === null) return parsed;
  let url: URL;
  try {
    url = new URL(parsed);
  } catch {
    throw invalidStatus();
  }
  if (url.protocol !== "http:" || !["127.0.0.1", "localhost", "::1"].includes(url.hostname)) {
    throw invalidStatus();
  }
  return parsed;
}

export function parseBuiltInProxyStatus(value: unknown): BuiltInProxyStatus {
  const status = record(value);
  if (status.ownership !== "built-in" || status.mode !== "managed") throw invalidStatus();
  if (typeof status.running !== "boolean" || typeof status.apiKeyConfigured !== "boolean") {
    throw invalidStatus();
  }
  const error = optionalString(status.error, 512);
  return {
    ownership: "built-in",
    mode: "managed",
    running: status.running,
    bindHost: optionalString(status.bindHost, 120),
    port: optionalPort(status.port),
    endpoint: endpoint(status.endpoint),
    apiKeyConfigured: status.apiKeyConfigured,
    upstreamCount: count(status.upstreamCount),
    modelCount: count(status.modelCount),
    error: error ? SAFE_RUNTIME_ERROR : error,
  };
}

export async function getBuiltInProxyStatus(): Promise<BuiltInProxyStatus> {
  try {
    return parseBuiltInProxyStatus(await invoke("proxy_runtime_status"));
  } catch (error) {
    if (error instanceof Error && error.message === STATUS_ERROR) throw error;
    throw new Error(SAFE_RUNTIME_ERROR);
  }
}

export async function startBuiltInProxyFromSettings(): Promise<BuiltInProxyStatus> {
  try {
    return parseBuiltInProxyStatus(await invoke("proxy_runtime_start_default"));
  } catch {
    throw new Error("Unable to start the OpenMesh built-in proxy. Check Provider settings and API key.");
  }
}

export async function stopBuiltInProxy(): Promise<BuiltInProxyStatus> {
  try {
    return parseBuiltInProxyStatus(await invoke("proxy_runtime_stop"));
  } catch {
    throw new Error("Unable to stop the OpenMesh built-in proxy.");
  }
}

function parseManagementModel(value: unknown): BuiltInProxyModel {
  const model = record(value);
  const id = optionalString(model.id, 240);
  const ownedBy = optionalString(model.ownedBy, 120);
  if (!id || !ownedBy || !Array.isArray(model.capabilities)) throw invalidStatus();
  const capabilities = model.capabilities.map((capability) => {
    if (typeof capability !== "string" || capability.length > 80) throw invalidStatus();
    return capability;
  });
  return { id, ownedBy, capabilities };
}

function parseManagementUpstream(value: unknown): BuiltInProxyManagementUpstream {
  const upstream = record(value);
  const protocol = upstream.protocol;
  if (protocol !== "open-ai-compatible" && protocol !== "anthropic" && protocol !== "gemini") {
    throw invalidStatus();
  }
  if (!Array.isArray(upstream.models)) throw invalidStatus();
  const id = optionalString(upstream.id, 160);
  const baseUrl = optionalString(upstream.baseUrl, 2_048);
  const accountId = optionalString(upstream.accountId, 160);
  const oauthProvider = optionalString(upstream.oauthProvider, 32);
  if (!id || !baseUrl || typeof upstream.apiKeyConfigured !== "boolean" || typeof upstream.enabled !== "boolean") {
    throw invalidStatus();
  }
  if (typeof upstream.priority !== "number" || !Number.isSafeInteger(upstream.priority)) throw invalidStatus();
  return {
    id,
    baseUrl,
    protocol,
    apiKeyConfigured: upstream.apiKeyConfigured,
    enabled: upstream.enabled,
    priority: upstream.priority,
    accountId,
    oauthProvider,
    models: upstream.models.map(parseManagementModel),
  };
}

function parseStringMap(value: unknown, values: boolean): Record<string, string | string[]> {
  const object = record(value);
  const output: Record<string, string | string[]> = {};
  for (const [key, raw] of Object.entries(object)) {
    if (!key || key.length > 240) throw invalidStatus();
    if (values) {
      if (!Array.isArray(raw) || raw.some((item) => typeof item !== "string" || item.length > 240)) throw invalidStatus();
      output[key] = raw as string[];
    } else {
      if (typeof raw !== "string" || raw.length > 240) throw invalidStatus();
      output[key] = raw;
    }
  }
  return output;
}

export function parseBuiltInProxyManagementConfig(value: unknown): BuiltInProxyManagementConfig {
  const config = record(value);
  const routingStrategy = config.routingStrategy;
  if (routingStrategy !== "first-compatible" && routingStrategy !== "round-robin" && routingStrategy !== "fill-first") throw invalidStatus();
  if (!Array.isArray(config.upstreams)) throw invalidStatus();
  if (typeof config.allowUnauthenticated !== "boolean") throw invalidStatus();
  const port = optionalPort(config.port);
  if (port === undefined || port === null) throw invalidStatus();
  const bindHost = optionalString(config.bindHost, 120);
  if (!bindHost || typeof config.requestTimeoutSecs !== "number" || config.requestTimeoutSecs < 1 || !Number.isSafeInteger(config.requestTimeoutSecs)) throw invalidStatus();
  if (typeof config.maxRetries !== "number" || config.maxRetries < 0 || config.maxRetries > 8 || !Number.isSafeInteger(config.maxRetries)) throw invalidStatus();
  const counts = [config.apiKeyCount, config.upstreamCount, config.modelAliasCount].map(count);
  const modelAliases = parseStringMap(config.modelAliases, false) as Record<string, string>;
  const modelFallbacks = parseStringMap(config.modelFallbacks, true) as Record<string, string[]>;
  const upstreams = config.upstreams.map(parseManagementUpstream);
  if (counts[1] !== upstreams.length || counts[2] !== Object.keys(modelAliases).length) throw invalidStatus();
  return {
    bindHost,
    port,
    allowUnauthenticated: config.allowUnauthenticated,
    requestTimeoutSecs: config.requestTimeoutSecs,
    routingStrategy,
    maxRetries: config.maxRetries,
    apiKeyCount: counts[0],
    upstreamCount: counts[1],
    modelAliasCount: counts[2],
    upstreams,
    modelAliases,
    modelFallbacks,
  };
}

export async function getBuiltInProxyManagementConfig(): Promise<BuiltInProxyManagementConfig> {
  try {
    return parseBuiltInProxyManagementConfig(await invoke("proxy_management_config"));
  } catch {
    throw new Error("Unable to load OpenMesh built-in proxy configuration.");
  }
}

export async function updateBuiltInProxyManagementConfig(
  patch: Record<string, unknown>,
): Promise<BuiltInProxyManagementConfig> {
  try {
    return parseBuiltInProxyManagementConfig(
      await invoke("proxy_management_update", { patch }),
    );
  } catch {
    throw new Error("Unable to update OpenMesh built-in proxy configuration.");
  }
}

export function builtInProxyStatusLabel(status: BuiltInProxyStatus): string {
  if (status.error) return "Runtime error";
  return status.running ? "Running" : "Stopped";
}
