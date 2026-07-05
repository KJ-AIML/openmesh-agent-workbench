// OpenMesh Context Search Client — Dev Track 0.1.2.5
// Thin wrapper around Tauri IPC commands for context search.

import { invoke } from "@tauri-apps/api/core";

export interface SourceReceipt {
  source_kind: string;
  source_id: string;
  canonical_ref?: string;
  outcome:
    | "Indexed"
    | "Updated"
    | "Unchanged"
    | "Removed"
    | "SkippedPolicy"
    | "SkippedTooLarge"
    | "SkippedSymlink"
    | "FailedRead"
    | "FailedParse"
    | "FailedValidation"
    | "FailedIndex";
  fingerprint?: string;
  bytes_read?: number;
  error?: string;
}

export interface RefreshResult {
  project_id: string;
  status: "COMPLETE" | "PARTIAL" | "FAILED";
  started_at: string;
  completed_at: string;
  discovered: number;
  indexed: number;
  updated: number;
  unchanged: number;
  removed: number;
  skipped: number;
  failed: number;
  receipts: SourceReceipt[];
}

export interface ContextSearchResult {
  document_id: string;
  source_id: string;
  source_kind: string;
  project_id: string;
  canonical_ref: string;
  title: string;
  snippet: string;
  sensitivity: string;
  freshness_state: string;
  observed_at: string;
  source_updated_at?: string;
}

export interface ContextInspection {
  document_id: string;
  source_id: string;
  source_kind: string;
  project_id: string;
  canonical_ref: string;
  title: string;
  text: string;
  sensitivity: string;
  agent_context_enabled: boolean;
  freshness_state: string;
  observed_at: string;
  source_updated_at?: string;
  indexed_at: string;
  metadata_json?: string;
}

export interface ContextHealth {
  path: string;
  schema_version: number;
  sqlite_version: string;
  journal_mode: string;
  document_count: number;
  fts_row_count: number;
  wal_mode_effective: boolean;
  integrity_ok: boolean;
}

const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 100;

export async function refreshContext(
  projectPath: string,
): Promise<RefreshResult> {
  return invoke<RefreshResult>("context_refresh", { projectPath });
}

export async function searchContext(
  projectPath: string,
  query: string,
  opts?: { kinds?: string[]; limit?: number },
): Promise<ContextSearchResult[]> {
  const limit = Math.min(opts?.limit ?? DEFAULT_LIMIT, MAX_LIMIT);
  return invoke<ContextSearchResult[]>("context_search", {
    projectPath,
    query,
    kinds: opts?.kinds ?? null,
    limit,
  });
}

export async function inspectContext(
  projectPath: string,
  documentId: string,
): Promise<ContextInspection | null> {
  return invoke<ContextInspection | null>("context_inspect", {
    projectPath,
    documentId,
  });
}

export async function getContextHealth(
  projectPath: string,
): Promise<ContextHealth> {
  return invoke<ContextHealth>("context_health", { projectPath });
}
