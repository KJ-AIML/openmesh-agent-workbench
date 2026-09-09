import { legacyInvoke as invoke } from "./ipc";

export type UsageSummary = {
  totalRequests: number;
  totalInputTokens: number;
  totalOutputTokens: number;
  totalTokens: number;
  avgDurationMs: number;
  estimatedCostUsd: number;
  periodStart: string;
  periodEnd: string;
};

export type UsageBucket = {
  timestamp: string;
  requestCount: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  avgDurationMs: number;
};

export type RequestLogEntry = {
  id: string;
  timestamp: string;
  provider: string;
  model: string;
  inputTokens: number;
  outputTokens: number;
  latencyMs: number;
  statusCode: number | null;
  accountLabel: string | null;
  source: "agent_turn" | "proxy_request";
};

export type ProviderQuota = {
  provider: string;
  requestsRemaining: number | null;
  requestsLimit: number | null;
  tokensRemaining: number | null;
  tokensLimit: number | null;
  resetsAt: string | null;
  status: "ok" | "warning" | "critical" | "unknown";
};

export type AccountInfo = {
  name: string;
  provider: string;
  enabled: boolean;
  isActive: boolean;
  unavailable: boolean;
  priority: number;
};

const SAFE_USAGE_ERROR = "Usage data is currently unavailable.";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function parseUsageError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return SAFE_USAGE_ERROR;
}

function requiredNumber(value: unknown): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(SAFE_USAGE_ERROR);
  }
  return value;
}

function optionalNumber(value: unknown): number | null {
  if (value === undefined || value === null) return null;
  if (typeof value !== "number" || !Number.isFinite(value)) return null;
  return value;
}

function requiredString(value: unknown, maxLength = 256): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    throw new Error(SAFE_USAGE_ERROR);
  }
  return value;
}

function optionalString(value: unknown, maxLength = 256): string | null {
  if (value === undefined || value === null) return null;
  if (typeof value !== "string" || value.length > maxLength) return null;
  return value;
}

function parseUsageSummary(value: unknown): UsageSummary {
  if (!isRecord(value)) throw new Error(SAFE_USAGE_ERROR);
  return {
    totalRequests: requiredNumber(value.totalRequests),
    totalInputTokens: requiredNumber(value.totalInputTokens),
    totalOutputTokens: requiredNumber(value.totalOutputTokens),
    totalTokens: requiredNumber(value.totalTokens),
    avgDurationMs: requiredNumber(value.avgDurationMs),
    estimatedCostUsd: requiredNumber(value.estimatedCostUsd),
    periodStart: requiredString(value.periodStart),
    periodEnd: requiredString(value.periodEnd),
  };
}

function parseUsageBucket(value: unknown): UsageBucket {
  if (!isRecord(value)) throw new Error(SAFE_USAGE_ERROR);
  return {
    timestamp: requiredString(value.timestamp),
    requestCount: requiredNumber(value.requestCount),
    inputTokens: requiredNumber(value.inputTokens),
    outputTokens: requiredNumber(value.outputTokens),
    totalTokens: requiredNumber(value.totalTokens),
    avgDurationMs: requiredNumber(value.avgDurationMs),
  };
}

function parseRequestLogEntry(value: unknown): RequestLogEntry {
  if (!isRecord(value)) throw new Error(SAFE_USAGE_ERROR);
  const source = value.source;
  if (source !== "agent_turn" && source !== "proxy_request") {
    throw new Error(SAFE_USAGE_ERROR);
  }
  return {
    id: requiredString(value.id, 128),
    timestamp: requiredString(value.timestamp),
    provider: requiredString(value.provider, 64),
    model: requiredString(value.model, 160),
    inputTokens: requiredNumber(value.inputTokens),
    outputTokens: requiredNumber(value.outputTokens),
    latencyMs: requiredNumber(value.latencyMs),
    statusCode: optionalNumber(value.statusCode),
    accountLabel: optionalString(value.accountLabel, 128),
    source,
  };
}

export async function getUsageSummary(start: string, end: string): Promise<UsageSummary> {
  try {
    const value: unknown = await invoke("usage_summary", { start, end });
    return parseUsageSummary(value);
  } catch (error) {
    throw new Error(parseUsageError(error));
  }
}

export async function getUsageTimeseries(
  start: string,
  end: string,
  interval: string,
): Promise<UsageBucket[]> {
  try {
    const value: unknown = await invoke("usage_timeseries", { start, end, interval });
    if (!Array.isArray(value)) throw new Error(SAFE_USAGE_ERROR);
    return value.map(parseUsageBucket);
  } catch (error) {
    throw new Error(parseUsageError(error));
  }
}

export async function getUsageRequestLogs(
  limit: number,
  offset: number,
  providerFilter?: string,
): Promise<RequestLogEntry[]> {
  try {
    const value: unknown = await invoke("usage_request_logs", {
      limit,
      offset,
      providerFilter: providerFilter ?? null,
    });
    if (!Array.isArray(value)) throw new Error(SAFE_USAGE_ERROR);
    return value.map(parseRequestLogEntry);
  } catch (error) {
    throw new Error(parseUsageError(error));
  }
}

export async function getQuotaStatus(): Promise<ProviderQuota[]> {
  // Live quota polling is deferred (docs/builtin-proxy-parity.md). There is
  // no registered Tauri command for this; do not invoke a ghost name.
  return [];
}

export async function getListAccounts(): Promise<AccountInfo[]> {
  // Account listing for the usage chrome is not a Tauri command. Provider
  // accounts are managed through the built-in proxy management API.
  return [];
}

export async function setActiveAccount(
  _provider: string,
  _accountName: string,
): Promise<void> {
  throw new Error(SAFE_USAGE_ERROR);
}

export async function getActiveAccounts(): Promise<Record<string, string>> {
  return {};
}

export function formatTokenCount(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`;
  return String(count);
}

export function formatDuration(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)}s`;
  return `${Math.round(ms)}ms`;
}

export function formatCost(usd: number): string {
  return `$${usd.toFixed(2)}`;
}

export function getDefaultDateRange(): { start: string; end: string } {
  const end = new Date();
  const start = new Date(end.getTime() - 24 * 60 * 60 * 1000);
  return {
    start: start.toISOString(),
    end: end.toISOString(),
  };
}
