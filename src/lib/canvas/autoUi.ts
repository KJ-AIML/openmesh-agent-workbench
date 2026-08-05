import { invoke } from "@tauri-apps/api/core";

export const AUTO_UI_SCHEMA = "openmesh.canvas/1" as const;

export type AutoUiBlock =
  | { type: "h1"; text: string }
  | { type: "h2"; text: string }
  | { type: "text"; text: string; tone?: string }
  | { type: "callout"; text: string; tone?: string }
  | { type: "stat"; label: string; value: string; hint?: string }
  | {
      type: "stats";
      items: Array<{ label: string; value: string; hint?: string }>;
    }
  | { type: "table"; columns: string[]; rows: string[][] }
  | { type: "pills"; items: Array<{ text: string; tone?: string }> }
  | { type: "todo"; items: Array<{ text: string; done?: boolean }> }
  | { type: "code"; code: string; language?: string }
  | { type: "divider" };

export type AutoUiDocument = {
  schema: string;
  id: string;
  title: string;
  summary?: string | null;
  blocks: AutoUiBlock[];
  updatedAt: number;
};

export function isAutoUiDocument(value: unknown): value is AutoUiDocument {
  if (!value || typeof value !== "object") return false;
  const v = value as Record<string, unknown>;
  return (
    v.schema === AUTO_UI_SCHEMA &&
    typeof v.title === "string" &&
    Array.isArray(v.blocks)
  );
}

export async function listAutoUi(projectPath: string): Promise<AutoUiDocument[]> {
  return invoke("canvas_auto_ui_list", { projectPath });
}

export async function loadAutoUi(
  projectPath: string,
  id: string,
): Promise<AutoUiDocument> {
  return invoke("canvas_auto_ui_load", { projectPath, id });
}

export async function upsertAutoUi(
  projectPath: string,
  document: unknown,
): Promise<AutoUiDocument> {
  return invoke("canvas_auto_ui_upsert", { projectPath, document });
}

export async function deleteAutoUi(
  projectPath: string,
  id: string,
): Promise<void> {
  return invoke("canvas_auto_ui_delete", { projectPath, id });
}
