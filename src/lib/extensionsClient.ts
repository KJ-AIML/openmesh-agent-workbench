import { invoke } from "@tauri-apps/api/core";

export type ExtensionSource = "builtin" | "user" | "project" | "plugin";

export type SkillPack = {
  id: string;
  name: string;
  description: string;
  body: string;
  enabled: boolean;
  source: ExtensionSource;
  pluginId?: string | null;
  path?: string | null;
};

export type HookDefinition = {
  id: string;
  event: "on_chat_start" | "on_before_turn" | "on_after_turn";
  appendContext?: string | null;
  command?: string | null;
  enabled: boolean;
  source: ExtensionSource;
  pluginId?: string | null;
};

export type PluginRecord = {
  id: string;
  name: string;
  version: string;
  description: string;
  enabled: boolean;
  source: ExtensionSource;
  path?: string | null;
  skillIds: string[];
  hookIds: string[];
};

export type ExtensionsInventory = {
  skills: SkillPack[];
  hooks: HookDefinition[];
  plugins: PluginRecord[];
};

export type CatalogEntry = {
  id: string;
  kind: string;
  name: string;
  description: string;
  installed: boolean;
};

export type ExtensionsSettings = {
  skills: Record<string, boolean>;
  hooks: Record<string, boolean>;
  plugins: Record<string, boolean>;
};

export async function listExtensions(
  projectPath?: string | null,
): Promise<ExtensionsInventory> {
  return invoke("extensions_list", {
    projectPath: projectPath || null,
  });
}

export async function listCatalog(
  projectPath?: string | null,
): Promise<CatalogEntry[]> {
  return invoke("extensions_catalog", {
    projectPath: projectPath || null,
  });
}

export async function setExtensionEnabled(
  kind: "skill" | "hook" | "plugin",
  id: string,
  enabled: boolean,
): Promise<ExtensionsSettings> {
  return invoke("extensions_set_enabled", {
    request: { kind, id, enabled },
  });
}

export async function installExtension(
  sourcePath: string,
): Promise<{ installed: string; path: string }> {
  return invoke("extensions_install", { sourcePath });
}
