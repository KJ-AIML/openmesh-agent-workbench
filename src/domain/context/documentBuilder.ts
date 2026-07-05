// ============================================================================
// ContextDocument Builder
// ============================================================================
// Factory for building ContextDocument from a ContextSource + supplied text.
//
// Key design point: file-backed sources (doc, note, snapshot) do NOT read the
// file here. The caller obtains content via the I/O layer and passes it in as
// a string. This keeps the domain layer pure and testable.
// ============================================================================

import { deriveSourceId } from "./canonicalRef";
import type {
  ContextDocument,
  ContextSource,
  Freshness,
  FreshnessState,
  JsonValue,
} from "./types";

export interface BuildDocumentOptions {
  ownerPersonId?: string;
  sensitivity?: ContextSource["sensitivity"];
  agentContextEnabled?: boolean;
  observedAt?: string;
  sourceUpdatedAt?: string;
  freshnessState?: FreshnessState;
  /** Injectable clock for deterministic testing. */
  now?: () => Date;
  metadata?: JsonValue;
}

function utcNowIso(now?: () => Date): string {
  return (now ? now() : new Date()).toISOString();
}

/**
 * Build a ContextDocument from a ContextSource + supplied text content.
 *
 * The document ID is deterministically derived from the source canonicalRef
 * with a `#$.doc` suffix so it differs from the source ID.
 */
export function createContextDocument(
  source: ContextSource,
  text: string,
  options: BuildDocumentOptions = {}
): ContextDocument {
  if (typeof text !== "string") {
    throw new Error("createContextDocument: text must be a string");
  }

  const observedAt = options.observedAt ?? utcNowIso(options.now);
  const freshness: Freshness = {
    state: options.freshnessState ?? "unknown",
    observedAt,
    sourceUpdatedAt: options.sourceUpdatedAt ?? source.updatedAt,
  };

  // Append -doc suffix to avoid collision with source ID in derived indexes.
  const docId = deriveSourceId(`${source.canonicalRef}#.doc`);

  return {
    id: docId,
    schemaVersion: source.schemaVersion,
    sourceId: source.id,
    kind: source.kind,
    projectId: source.projectId,
    ownerPersonId: options.ownerPersonId ?? source.ownerPersonId,
    canonicalRef: source.canonicalRef,
    title: source.title,
    text,
    sensitivity: options.sensitivity ?? source.sensitivity,
    agentContextEnabled:
      options.agentContextEnabled ?? source.agentContextEnabled,
    observedAt,
    sourceUpdatedAt: options.sourceUpdatedAt ?? source.updatedAt,
    freshness,
    metadata: options.metadata,
  };
}
