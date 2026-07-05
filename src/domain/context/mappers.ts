// ============================================================================
// Pure Source Mappers
// ============================================================================
// These functions turn current OpenMesh objects into ContextSource documents.
// They are PURE: no filesystem I/O, no Tauri invoke, no network calls.
//
// All timestamp and clock-sensitive inputs are INJECTED via options so tests
// can be deterministic (no hidden Date.now()).
// ============================================================================

import {
  DEFAULT_AGENT_CONTEXT_ENABLED,
  DEFAULT_SENSITIVITY,
} from "./types";
import { buildCanonicalIdentity } from "./canonicalRef";
import type {
  ContextSource,
  Sensitivity,
} from "./types";

// ---------------------------------------------------------------------------
// Shared mapper options + helpers
// ---------------------------------------------------------------------------

export interface MapperOptions {
  ownerPersonId?: string;
  sensitivity?: Sensitivity;
  agentContextEnabled?: boolean;
  observedAt?: string;
  now?: () => Date;
}

function utcNowIso(now?: () => Date): string {
  const d = now ? now() : new Date();
  return d.toISOString();
}

function resolveAgentContext(
  source?: boolean,
  options?: boolean
): boolean {
  return options ?? source ?? DEFAULT_AGENT_CONTEXT_ENABLED;
}

function resolveSensitivity(options?: Sensitivity): Sensitivity {
  return options ?? DEFAULT_SENSITIVITY;
}

// ---------------------------------------------------------------------------
// DOC
// ---------------------------------------------------------------------------

export interface DocSourceInput {
  projectId: string;
  /** Relative path inside docs/, e.g. "architecture/overview.md". */
  relativePath: string;
  title?: string;
  contentHash?: string;
  sourceModifiedAt?: string;
  sourceCreatedAt?: string;
  agentContextEnabled?: boolean;
}

export function mapDocSource(input: DocSourceInput, options: MapperOptions = {}): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapDocSource: projectId required");
  if (!input.relativePath.trim()) throw new Error("mapDocSource: relativePath required");

  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "doc",
    sourceKey: input.relativePath.trim(),
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "doc",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId,
    canonicalRef,
    title: (input.title ?? input.relativePath).trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.sourceCreatedAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.sourceModifiedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: input.contentHash,
  };
}

// ---------------------------------------------------------------------------
// NOTE
// ---------------------------------------------------------------------------

export interface NoteSourceInput {
  projectId: string;
  filename: string;
  title?: string;
  sourceModifiedAt?: string;
  sourceCreatedAt?: string;
  agentContextEnabled?: boolean;
}

export function mapNoteSource(input: NoteSourceInput, options: MapperOptions = {}): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapNoteSource: projectId required");
  if (!input.filename.trim()) throw new Error("mapNoteSource: filename required");

  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "note",
    sourceKey: input.filename.trim(),
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "note",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId,
    canonicalRef,
    title: (input.title ?? input.filename).trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.sourceCreatedAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.sourceModifiedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: undefined,
  };
}

// ---------------------------------------------------------------------------
// SNAPSHOT
// ---------------------------------------------------------------------------

export interface SnapshotSourceInput {
  projectId: string;
  filename: string;
  title?: string;
  sourceModifiedAt?: string;
  sourceCreatedAt?: string;
  agentContextEnabled?: boolean;
}

export function mapSnapshotSource(
  input: SnapshotSourceInput,
  options: MapperOptions = {}
): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapSnapshotSource: projectId required");
  if (!input.filename.trim()) throw new Error("mapSnapshotSource: filename required");

  const sourceKey = `notes/snapshots/${input.filename.trim()}`;
  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "snapshot",
    sourceKey,
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "snapshot",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId,
    canonicalRef,
    title: (input.title ?? input.filename).trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.sourceCreatedAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.sourceModifiedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: undefined,
  };
}

// ---------------------------------------------------------------------------
// TASK
// ---------------------------------------------------------------------------

export interface TaskSourceInput {
  projectId: string;
  taskId: string;
  title: string;
  createdAt?: string;
  updatedAt?: string;
  owner?: string;
  agentContextEnabled?: boolean;
}

export function mapTaskSource(input: TaskSourceInput, options: MapperOptions = {}): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapTaskSource: projectId required");
  if (!input.taskId.trim()) throw new Error("mapTaskSource: taskId required");
  if (!input.title.trim()) throw new Error("mapTaskSource: title required");

  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "task",
    sourceKey: input.taskId.trim(),
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "task",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId ?? input.owner,
    canonicalRef,
    title: input.title.trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.createdAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.updatedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: undefined,
  };
}

// ---------------------------------------------------------------------------
// RECENT (transitional — see recent note in index.ts)
// ---------------------------------------------------------------------------

export interface RecentSourceInput {
  projectId: string;
  recentId: string;
  title: string;
  lastOpenedAt?: string;
  createdAt?: string;
  updatedAt?: string;
  agentContextEnabled?: boolean;
}

export function mapRecentSource(
  input: RecentSourceInput,
  options: MapperOptions = {}
): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapRecentSource: projectId required");
  if (!input.recentId.trim()) throw new Error("mapRecentSource: recentId required");
  if (!input.title.trim()) throw new Error("mapRecentSource: title required");

  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "recent",
    sourceKey: input.recentId.trim(),
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "recent",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId,
    canonicalRef,
    title: input.title.trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.createdAt ?? input.lastOpenedAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.updatedAt ?? input.lastOpenedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: undefined,
  };
}

// ---------------------------------------------------------------------------
// AGENT SESSION
// ---------------------------------------------------------------------------

export interface AgentSessionSourceInput {
  projectId: string;
  sessionId: string;
  title: string;
  startedAt?: string;
  lastActiveAt?: string;
  endedAt?: string;
  createdAt?: string;
  updatedAt?: string;
  agentContextEnabled?: boolean;
}

export function mapAgentSessionSource(
  input: AgentSessionSourceInput,
  options: MapperOptions = {}
): ContextSource {
  if (!input.projectId.trim()) throw new Error("mapAgentSessionSource: projectId required");
  if (!input.sessionId.trim()) throw new Error("mapAgentSessionSource: sessionId required");
  if (!input.title.trim()) throw new Error("mapAgentSessionSource: title required");

  const { canonicalRef, id } = buildCanonicalIdentity({
    projectId: input.projectId.trim(),
    kind: "agent-session",
    sourceKey: input.sessionId.trim(),
  });

  return {
    id,
    schemaVersion: "1.0.0",
    kind: "agent-session",
    projectId: input.projectId.trim(),
    ownerPersonId: options.ownerPersonId,
    canonicalRef,
    title: input.title.trim(),
    sensitivity: resolveSensitivity(options.sensitivity),
    agentContextEnabled: resolveAgentContext(input.agentContextEnabled, options.agentContextEnabled),
    createdAt: input.createdAt ?? input.startedAt ?? options.observedAt ?? utcNowIso(options.now),
    updatedAt: input.updatedAt ?? input.lastActiveAt ?? input.endedAt ?? input.startedAt ?? options.observedAt ?? utcNowIso(options.now),
    indexedAt: undefined,
    contentHash: undefined,
  };
}
