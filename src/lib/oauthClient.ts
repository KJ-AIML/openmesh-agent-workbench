import { invokeTyped as invoke } from "./ipc";

/**
 * OpenMesh-owned OAuth client contracts. The legacy command names remain so
 * existing desktop builds can migrate without a frontend API break.
 */
export type OAuthProviderId =
  | "codex"
  | "claude"
  | "gemini"
  | "grok"
  | "qwen"
  | "iflow"
  | "antigravity"
  | "kimi";
export type OAuthAdminProviderId = OAuthProviderId | "unknown" | "other";

export type OAuthConfigStatus = {
  managementPort: number;
  endpoint: string;
  dataPlaneEndpoint: string;
  /** Legacy JSON field retained for settings compatibility; it mirrors runtime state. */
  sidecarEnabled: boolean;
  secretConfigured: boolean;
  /** Legacy JSON field retained for settings compatibility; it mirrors local client auth. */
  sidecarClientKeyConfigured: boolean;
};

export type OAuthConnectionState =
  | "notConfigured"
  | "unreachable"
  | "unauthorized"
  | "ready"
  | "error";

export type OAuthDataPlaneState = OAuthConnectionState;

export type OAuthProviderSummary = {
  provider: string;
  total: number;
  enabled: number;
  disabled: number;
  unavailable: number;
  runtimeOnly: number;
};

export type OAuthConnectionStatus = OAuthConfigStatus & {
  dataPlaneStatus: OAuthDataPlaneState;
  dataPlaneError?: string | null;
  status: OAuthConnectionState;
  error?: string | null;
  providers: OAuthProviderSummary[];
};

export type OAuthModelDefinition = {
  id: string;
  displayName?: string;
};

export type OAuthStartResult = {
  url: string;
  state?: string | null;
  flow?: "browser" | "device";
  userCode?: string | null;
  verificationUri?: string | null;
  verificationUriComplete?: string | null;
  pollIntervalSeconds?: number | null;
};

export type OAuthStatusResult = {
  status: string;
  error?: string | null;
};

export type OAuthAuthFile = {
  name: string;
  provider: OAuthAdminProviderId;
  enabled: boolean;
  runtimeOnly: boolean;
  unavailable: boolean;
  priority: number;
  sizeBytes?: number | null;
  updatedAt?: string | null;
  metadataWritable: boolean;
};

export type OAuthAuthFileCatalog = {
  revision: string;
  files: OAuthAuthFile[];
};

export type OAuthAuthFileStatusRequest = {
  expectedRevision: string;
  name: string;
  disabled: boolean;
};

export type OAuthAuthFilePriorityRequest = {
  expectedRevision: string;
  name: string;
  priority: number;
};

export type OAuthExclusions = {
  provider: OAuthProviderId;
  models: string[];
};

export type OAuthExclusionsCatalog = {
  revision: string;
  exclusions: OAuthExclusions;
};

export type OAuthExclusionsRequest = {
  expectedRevision: string;
  provider: OAuthProviderId;
  models: string[];
};

export const OAUTH_PROVIDERS: Array<{
  id: OAuthProviderId;
  label: string;
  description: string;
  callbackSupported: boolean;
  deviceSupported: boolean;
}> = (
  [
  ["codex", "Codex", "Native PKCE OAuth is available; device login is available from the CLI.", true, false],
  ["claude", "Claude", "Native PKCE OAuth is available.", true, false],
  ["gemini", "Gemini", "Configure a Google-compatible endpoint while the native adapter is pinned.", false, false],
  ["grok", "Grok", "Native device authorization is available.", false, true],
  ["qwen", "Qwen", "Configure an OpenAI-compatible endpoint while native auth is pinned.", false, false],
  ["iflow", "iFlow", "Configure an OpenAI-compatible endpoint while native auth is pinned.", false, false],
  ["antigravity", "Antigravity", "Native Google OAuth and project discovery are available; provider data-plane translation remains partial.", true, false],
  ["kimi", "Kimi", "Native device authorization is available.", false, true],
  ] as Array<[string, string, string, boolean, boolean]>
).map(([id, label, description, callbackSupported, deviceSupported]) => ({
  id: id as OAuthProviderId,
  label,
  description,
  callbackSupported,
  deviceSupported,
}));

export const OAUTH_POLL_INTERVAL_MS = 3000;
export const OAUTH_MAX_POLL_MS = 10 * 60 * 1000;

const INVALID_CONFIG = "OpenMesh returned an invalid proxy compatibility response.";
const LOCAL_ONLY_ERROR =
  "This OAuth provider is not available yet; configure its OpenMesh upstream in Provider settings.";

function record(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(INVALID_CONFIG);
  }
  return value as Record<string, unknown>;
}

function stringValue(value: unknown, maxLength: number, optional = false): string | null | undefined {
  if (optional && (value === null || value === undefined)) return value;
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    throw new Error(INVALID_CONFIG);
  }
  if ([...value].some((character) => (character.codePointAt(0) ?? 0) <= 0x1f)) {
    throw new Error(INVALID_CONFIG);
  }
  return value;
}

function requiredString(value: unknown, maxLength: number): string {
  const result = stringValue(value, maxLength);
  if (!result) throw new Error(INVALID_CONFIG);
  return result;
}

function booleanValue(value: unknown): boolean {
  if (typeof value !== "boolean") throw new Error(INVALID_CONFIG);
  return value;
}

function portValue(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1 || value > 65535) {
    throw new Error(INVALID_CONFIG);
  }
  return value;
}

function countValue(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0 || value > 100_000) {
    throw new Error(INVALID_CONFIG);
  }
  return value;
}

function loopbackEndpoint(value: unknown, port: number, path: string): string {
  const endpoint = requiredString(value, 240);
  let parsed: URL;
  try {
    parsed = new URL(endpoint);
  } catch {
    throw new Error(INVALID_CONFIG);
  }
  if (
    parsed.protocol !== "http:" ||
    !["127.0.0.1", "localhost", "[::1]", "::1"].includes(parsed.hostname) ||
    parsed.port !== String(port) ||
    parsed.pathname.replace(/\/$/u, "") !== path
  ) {
    throw new Error(INVALID_CONFIG);
  }
  return endpoint.replace(/\/$/u, "");
}

function safeError(value: unknown): string | null | undefined {
  if (value === null || value === undefined) return value;
  if (typeof value !== "string") throw new Error(INVALID_CONFIG);
  return value ? "The OpenMesh proxy reported an error." : null;
}

function parseConfigStatus(value: unknown): OAuthConfigStatus {
  const config = record(value);
  const managementPort = portValue(config.managementPort);
  return {
    managementPort,
    endpoint: loopbackEndpoint(config.endpoint, managementPort, "/v0/management"),
    dataPlaneEndpoint: loopbackEndpoint(config.dataPlaneEndpoint, managementPort, "/v1"),
    sidecarEnabled: booleanValue(config.sidecarEnabled),
    secretConfigured: booleanValue(config.secretConfigured),
    sidecarClientKeyConfigured: booleanValue(config.sidecarClientKeyConfigured),
  };
}

export function parseOAuthConfig(value: unknown): OAuthConfigStatus {
  return parseConfigStatus(value);
}

export async function getOAuthConfigStatus(): Promise<OAuthConfigStatus> {
  return parseConfigStatus(await invoke("oauth_config_status"));
}

export async function setOAuthManagementPort(port: number): Promise<OAuthConfigStatus> {
  return parseConfigStatus(await invoke("oauth_set_management_port", { port }));
}

export async function setOAuthManagementSecret(_secret: string): Promise<OAuthConfigStatus> {
  throw new Error(LOCAL_ONLY_ERROR);
}

export async function setOAuthSidecarClientKey(_apiKey: string): Promise<OAuthConfigStatus> {
  throw new Error(LOCAL_ONLY_ERROR);
}

export async function clearOAuthSidecarClientKey(): Promise<OAuthConfigStatus> {
  return parseConfigStatus(await invoke("oauth_clear_sidecar_client_key"));
}

export async function setOAuthSidecarEnabled(enabled: boolean): Promise<OAuthConfigStatus> {
  return parseConfigStatus(await invoke("oauth_set_sidecar_enabled", { enabled }));
}

function parseProviderSummary(value: unknown): OAuthProviderSummary {
  const provider = record(value);
  const total = countValue(provider.total);
  const enabled = countValue(provider.enabled);
  const disabled = countValue(provider.disabled);
  const unavailable = countValue(provider.unavailable);
  const runtimeOnly = countValue(provider.runtimeOnly);
  if (
    typeof provider.provider !== "string" ||
    enabled + disabled !== total ||
    unavailable > total ||
    runtimeOnly > total
  ) {
    throw new Error(INVALID_CONFIG);
  }
  return {
    provider: provider.provider,
    total,
    enabled,
    disabled,
    unavailable,
    runtimeOnly,
  };
}

function parseConnection(value: unknown): OAuthConnectionStatus {
  const connection = record(value);
  const status = connection.status;
  const dataPlaneStatus = connection.dataPlaneStatus;
  const states: OAuthConnectionState[] = [
    "notConfigured",
    "unreachable",
    "unauthorized",
    "ready",
    "error",
  ];
  if (!states.includes(status as OAuthConnectionState) || !states.includes(dataPlaneStatus as OAuthDataPlaneState)) {
    throw new Error(INVALID_CONFIG);
  }
  const config = parseConfigStatus(connection);
  if (!Array.isArray(connection.providers)) throw new Error(INVALID_CONFIG);
  return {
    ...config,
    dataPlaneStatus: dataPlaneStatus as OAuthDataPlaneState,
    dataPlaneError: safeError(connection.dataPlaneError),
    status: status as OAuthConnectionState,
    error: safeError(connection.error),
    providers: connection.providers.map(parseProviderSummary),
  };
}

export function parseOAuthConnection(value: unknown): OAuthConnectionStatus {
  return parseConnection(value);
}

export async function getOAuthConnectionStatus(): Promise<OAuthConnectionStatus> {
  return parseConnection(await invoke("oauth_connection_status"));
}

function parseModel(value: unknown): OAuthModelDefinition {
  const model = record(value);
  const id = requiredString(model.id, 240);
  const displayName = stringValue(model.displayName, 240, true);
  return { id, displayName: displayName || undefined };
}

export function parseOAuthModelDefinitions(value: unknown): OAuthModelDefinition[] {
  if (!Array.isArray(value) || value.length > 2048) throw new Error(INVALID_CONFIG);
  const models = value.map(parseModel);
  if (new Set(models.map((model) => model.id.toLowerCase())).size !== models.length) {
    throw new Error(INVALID_CONFIG);
  }
  return models;
}

export async function getOAuthModelDefinitions(provider: OAuthProviderId): Promise<OAuthModelDefinition[]> {
  return parseOAuthModelDefinitions(await invoke("oauth_model_definitions", { provider }));
}

function parseRevision(value: unknown): string {
  return requiredString(value, 64);
}

export function parseOAuthAuthFileCatalog(value: unknown): OAuthAuthFileCatalog {
  const catalog = record(value);
  if (!Array.isArray(catalog.files) || catalog.files.length > 2048) throw new Error(INVALID_CONFIG);
  const files = catalog.files.map((raw) => {
    const file = record(raw);
    const name = requiredString(file.name, 240);
    const provider = requiredString(file.provider, 32) as OAuthAdminProviderId;
    const priority = file.priority;
    if (
      !name.toLowerCase().endsWith(".json") ||
      !["codex", "claude", "gemini", "grok", "qwen", "iflow", "antigravity", "kimi", "unknown", "other"].includes(provider) ||
      typeof priority !== "number" ||
      !Number.isSafeInteger(priority)
    ) {
      throw new Error(INVALID_CONFIG);
    }
    return {
      name,
      provider,
      enabled: booleanValue(file.enabled),
      runtimeOnly: booleanValue(file.runtimeOnly),
      unavailable: booleanValue(file.unavailable),
      priority,
      sizeBytes: file.sizeBytes === null || file.sizeBytes === undefined ? file.sizeBytes : countValue(file.sizeBytes),
      updatedAt: stringValue(file.updatedAt, 120, true),
      metadataWritable: booleanValue(file.metadataWritable),
    };
  });
  if (new Set(files.map((file) => file.name)).size !== files.length) throw new Error(INVALID_CONFIG);
  return { revision: parseRevision(catalog.revision), files };
}

export function parseOAuthExclusionsCatalog(value: unknown): OAuthExclusionsCatalog {
  const catalog = record(value);
  const exclusions = record(catalog.exclusions);
  if (!Array.isArray(exclusions.models)) throw new Error(INVALID_CONFIG);
  const provider = requiredString(exclusions.provider, 32) as OAuthProviderId;
  const models = exclusions.models.map((model) => requiredString(model, 240).toLowerCase());
  return { revision: parseRevision(catalog.revision), exclusions: { provider, models } };
}

export function parseOAuthStartResult(value: unknown): OAuthStartResult {
  const result = record(value);
  const url = requiredString(result.url, 8192);
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    throw new Error(INVALID_CONFIG);
  }
  if (!["http:", "https:"].includes(parsed.protocol) || parsed.username || parsed.password) {
    throw new Error(INVALID_CONFIG);
  }
  const state = stringValue(result.state, 512, true);
  const rawFlow = stringValue(result.flow, 16, true);
  const flow = rawFlow === undefined || rawFlow === null ? "browser" : rawFlow;
  if (flow !== "browser" && flow !== "device") throw new Error(INVALID_CONFIG);
  const userCode = stringValue(result.userCode, 128, true);
  const verificationUri = stringValue(result.verificationUri, 8192, true);
  const verificationUriComplete = stringValue(result.verificationUriComplete, 8192, true);
  const pollIntervalSeconds = result.pollIntervalSeconds;
  if (
    pollIntervalSeconds !== undefined &&
    pollIntervalSeconds !== null &&
    (typeof pollIntervalSeconds !== "number" ||
      !Number.isSafeInteger(pollIntervalSeconds) ||
      pollIntervalSeconds < 1 ||
      pollIntervalSeconds > 3600)
  ) {
    throw new Error(INVALID_CONFIG);
  }
  return {
    url,
    state,
    flow,
    userCode,
    verificationUri,
    verificationUriComplete,
    pollIntervalSeconds,
  };
}

export function parseOAuthStatusResult(value: unknown): OAuthStatusResult {
  const result = record(value);
  const status = requiredString(result.status, 32).toLowerCase();
  if (!["wait", "pending", "ok", "error"].includes(status)) throw new Error(INVALID_CONFIG);
  return { status, error: safeError(result.error) };
}

export async function startOAuth(provider: OAuthProviderId): Promise<OAuthStartResult> {
  return parseOAuthStartResult(await invoke("oauth_start", { provider }));
}

export async function getOAuthStatus(state: string): Promise<OAuthStatusResult> {
  return parseOAuthStatusResult(await invoke("oauth_status", { state }));
}

export async function submitOAuthCallback(provider: OAuthProviderId, redirectUrl: string): Promise<void> {
  await invoke("oauth_submit_callback", { provider, redirectUrl });
}

export async function cancelOAuth(state: string): Promise<void> {
  await invoke("oauth_cancel", { state });
}

export async function openOAuthUrl(url: string): Promise<void> {
  await invoke("oauth_open_url", { url });
}

export async function clearOAuthManagementSecret(): Promise<OAuthConfigStatus> {
  return parseConfigStatus(await invoke("oauth_clear_management_secret"));
}

const emptyCatalog: OAuthAuthFileCatalog = { revision: "r1-0000000000000000", files: [] };

export async function getOAuthAuthFiles(): Promise<OAuthAuthFileCatalog> {
  return emptyCatalog;
}

export async function setOAuthAuthFileStatus(_request: OAuthAuthFileStatusRequest): Promise<OAuthAuthFileCatalog> {
  throw new Error(LOCAL_ONLY_ERROR);
}

export async function setOAuthAuthFilePriority(_request: OAuthAuthFilePriorityRequest): Promise<OAuthAuthFileCatalog> {
  throw new Error(LOCAL_ONLY_ERROR);
}

export async function getOAuthExclusions(provider: OAuthProviderId): Promise<OAuthExclusionsCatalog> {
  return {
    revision: "r1-0000000000000000",
    exclusions: { provider, models: [] },
  };
}

export async function saveOAuthExclusions(_request: OAuthExclusionsRequest): Promise<OAuthExclusionsCatalog> {
  throw new Error(LOCAL_ONLY_ERROR);
}

export function oauthErrorMessage(_error: unknown): string {
  return "The OpenMesh proxy compatibility operation could not be completed.";
}

export function oauthAdminErrorMessage(_error: unknown, operation: "load" | "update" = "load"): string {
  return operation === "update"
    ? "This OpenMesh runtime does not expose native OAuth account-file editing."
    : "This OpenMesh runtime has no native OAuth account-file catalog yet.";
}

function wildcardPattern(rule: string): RegExp {
  return new RegExp(
    `^${rule.replace(/[.+?^${}()|[\]\\]/g, "\\$&").replaceAll("*", ".*")}$`,
    "i",
  );
}

export function oauthModelMatchesRule(modelId: string, rule: string): boolean {
  const normalized = rule.trim();
  return normalized.length > 0 && wildcardPattern(normalized).test(modelId.trim());
}

export function oauthOpenModelIds(models: OAuthModelDefinition[], excludedRules: Iterable<string>): Set<string> {
  const rules = Array.from(excludedRules, (rule) => rule.trim()).filter(Boolean);
  return new Set(
    models
      .filter((model) => !rules.some((rule) => oauthModelMatchesRule(model.id, rule)))
      .map((model) => model.id.toLowerCase()),
  );
}

export function oauthExclusionsForOpenModels(
  currentRules: Iterable<string>,
  models: OAuthModelDefinition[],
  openModelIds: Iterable<string>,
): string[] {
  const open = new Set(Array.from(openModelIds, (id) => id.trim().toLowerCase()).filter(Boolean));
  const modelIds = new Set(models.map((model) => model.id.toLowerCase()));
  const openedModels = models.filter((model) => open.has(model.id.toLowerCase()));
  const preservedRules = Array.from(currentRules, (rule) => rule.trim().toLowerCase())
    .filter(Boolean)
    .filter((rule) => !modelIds.has(rule))
    .filter((rule) => !openedModels.some((model) => oauthModelMatchesRule(model.id, rule)));
  const closedModels = models
    .filter((model) => !open.has(model.id.toLowerCase()))
    .map((model) => model.id.toLowerCase());
  return Array.from(new Set([...preservedRules, ...closedModels])).sort();
}

export function isOAuthSuccessStatus(status: string): boolean {
  return status.trim().toLowerCase() === "ok";
}

export function isOAuthFailureStatus(status: string): boolean {
  return status.trim().toLowerCase() === "error";
}

export function normalizeOAuthCallback(value: string): string {
  return value.trim();
}
