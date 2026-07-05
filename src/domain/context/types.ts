// ============================================================================
// OpenMesh ContextSource / ContextDocument Domain Contracts
// ============================================================================
// These types are the canonical domain contract for work-context sources.
// They are intentionally storage-agnostic: no SQLite, no Tauri, no FS here.
//
// Later tracks (derived index, ingestion, search) consume these contracts.
// mappers.ts turns current OpenMesh objects into these normalized shapes.
// ============================================================================

/**
 * Explicit schema version for the context domain contract.
 *
 * Every ContextSource and ContextDocument carries this exact string so that
 * future schema migrations can detect the shape version without heuristics.
 */
export const CONTEXT_SCHEMA_VERSION = "1.0.0" as const;

/**
 * Source kinds for context sources.
 *
 * Three categories:
 * - current:    implemented mappers exist in mappers.ts for these kinds
 * - transitional: the `recent` kind exists only to bridge current RecentItem
 *                 data until OpenMesh 0.1.3 introduces WorkEvent. It is NOT
 *                 a permanent kind.
 * - reserved:   not yet implemented. These slots are stable placeholders so
 *               later tracks can add mappers without renumbering/restructuring.
 */
export type ContextSourceKind =
  // current
  | "doc"
  | "note"
  | "snapshot"
  | "task"
  | "recent"
  | "agent-session"
  // reserved
  | "work-event"
  | "git"
  | "connector";

/**
 * Sensitivity classification.
 *
 * Privacy rule (enforced by default, not by this type alone):
 *   New context sources default SENSITIVITY DEFAULT to `private`.
 *   Nothing in the domain layer ever defaults to public or team.
 */
export type Sensitivity = "public" | "team" | "private" | "secret";

/**
 * Conservative default sensitivity for newly created context sources.
 */
export const DEFAULT_SENSITIVITY: Sensitivity = "private";

/**
 * Default agent-context flag.
 *
 * Rule: fail closed. New sources are NOT included in agent context packs
 * unless an explicit existing value says otherwise, or a caller explicitly
 * enables it.
 */
export const DEFAULT_AGENT_CONTEXT_ENABLED = false;

/**
 * Freshness classification for ContextDocument.
 *
 * This track models freshness metadata only — it does NOT define aging
 * thresholds such as "7 days = stale". Those policy decisions belong to a
 * later track that classifies against a configurable freshness policy.
 */
export type FreshnessState = "fresh" | "aging" | "stale" | "unknown";

/**
 * Structured freshness metadata attached to a ContextDocument.
 *
 * Carries the classification plus the observation and source timestamps that
 * later policy can interpret.
 */
export interface Freshness {
  state: FreshnessState;
  observedAt: string;
  sourceUpdatedAt?: string;
}

/**
 * A reference to a canonical source outside the derived index.
 *
 * Format: openmesh://project/<projectId>/<kind>/<encoded-source-key>
 *
 * The canonicalRef is:
 *   - deterministic for the same canonical source
 *   - project-scoped
 *   - kind-scoped
 *   - safe from path traversal (no `..`, normalized separators)
 *   - independent of the absolute project folder location
 */
export type CanonicalRef = string;

/**
 * A normalized, bounded representation of a work-context source.
 *
 * ContextSource is an *abstraction over* canonical sources, not a replacement
 * for them. Markdown docs, note files, snapshot files, tasks.json,
 * recent.json, and sessions.json remain the canonical storage.
 */
export interface ContextSource {
  /** Deterministic ID derived from canonicalRef. */
  id: string;
  /** Always CONTEXT_SCHEMA_VERSION for current sources. */
  schemaVersion: string;
  kind: ContextSourceKind;
  /** Required. Project-scoped for all current sources. */
  projectId: string;
  /** Optional. Person model does not exist yet; never fabricated. */
  ownerPersonId?: string;
  canonicalRef: CanonicalRef;
  title: string;
  sensitivity: Sensitivity;
  agentContextEnabled: boolean;
  createdAt: string;
  updatedAt: string;
  /** UTC ISO timestamp of the most recent successful index observation. */
  indexedAt?: string;
  /** Optional content hash for change detection. */
  contentHash?: string;
}

/**
 * A normalized text document with source and freshness metadata.
 *
 * ContextDocument is the shape that future indexes consume directly.
 * It intentionally contains NO embeddings, chunk IDs, FTS row IDs,
 * SQLite-specific fields, or provider-specific enrichment.
 */
export interface ContextDocument {
  id: string;
  schemaVersion: string;
  sourceId: string;
  kind: ContextSourceKind;
  projectId: string;
  ownerPersonId?: string;
  canonicalRef: CanonicalRef;
  title: string;
  /** Normalized, bounded text content. */
  text: string;
  sensitivity: Sensitivity;
  agentContextEnabled: boolean;
  /** UTC ISO timestamp when this document was observed/created. */
  observedAt: string;
  /** Optional source-side update timestamp. */
  sourceUpdatedAt?: string;
  freshness: Freshness;
  /** JSON-safe metadata bag for source-specific extra data. */
  metadata?: JsonValue;
}

/**
 * JSON-safe value types for the metadata bag.
 */
export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };
