// ============================================================================
// Runtime Validators for ContextSource and ContextDocument
// ============================================================================
// TypeScript interfaces are compile-time only. These validators enforce shape
// invariants at runtime, especially for data that may originate from JSON
// storage, mapper output, or future index rebuilds.
//
// The rule is FAIL-CLOSED: an invalid source should produce clear errors so
// callers can quarantine rather than silently accept or silently discard.
// ============================================================================

import {
  CONTEXT_SCHEMA_VERSION,
  DEFAULT_AGENT_CONTEXT_ENABLED,
  DEFAULT_SENSITIVITY,
  type ContextDocument,
  type ContextSource,
  type ContextSourceKind,
  type FreshnessState,
  type JsonValue,
  type Sensitivity,
} from "./types";

/**
 * All currently known source kinds (current + reserved).
 */
const VALID_KINDS: ReadonlySet<string> = new Set<ContextSourceKind>([
  "doc",
  "note",
  "snapshot",
  "task",
  "recent",
  "agent-session",
  "work-event",
  "git",
  "connector",
]);

/**
 * Valid sensitivity values.
 */
const VALID_SENSITIVITIES: ReadonlySet<string> = new Set<Sensitivity>([
  "public",
  "team",
  "private",
  "secret",
]);

/**
 * Valid freshness states.
 */
const VALID_FRESHNESS_STATES: ReadonlySet<string> = new Set<FreshnessState>([
  "fresh",
  "aging",
  "stale",
  "unknown",
]);

/**
 * Result of a validation operation.
 */
export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
}

export interface ValidationError {
  path: string;
  message: string;
}

function ok(): ValidationResult {
  return { valid: true, errors: [] };
}

function fail(path: string, message: string): ValidationResult {
  return { valid: false, errors: [{ path, message }] };
}

function merge(a: ValidationResult, b: ValidationResult): ValidationResult {
  return {
    valid: a.valid && b.valid,
    errors: [...a.errors, ...b.errors],
  };
}

/**
 * Check that value is a non-empty string (after trim).
 */
function isNonEmptyTrimmedString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

/**
 * Check that value is a valid UTC ISO / RFC3339 timestamp.
 *
 * We parse using Date and confirm it produces a valid time.
 * We do NOT accept non-string values or obviously invalid dates.
 */
export function isValidIsoTimestamp(value: unknown): boolean {
  if (typeof value !== "string") return false;
  const trimmed = value.trim();
  if (!trimmed) return false;
  const parsed = Date.parse(trimmed);
  return !Number.isNaN(parsed);
}

/**
 * Recursively verify a value is JSON-safe (no functions, no undefined, no
 * circular references within practical depth).
 */
export function isJsonSafe(value: unknown, depth = 0): boolean {
  if (depth > 64) return false;
  if (value === null) return true;
  const t = typeof value;
  if (t === "string" || t === "number" || t === "boolean") return true;
  if (t === "undefined" || t === "function" || t === "symbol" || t === "bigint") {
    return false;
  }
  if (Array.isArray(value)) {
    return value.every((v) => isJsonSafe(v, depth + 1));
  }
  if (t === "object") {
    return Object.values(value as Record<string, unknown>).every((v) =>
      isJsonSafe(v, depth + 1)
    );
  }
  return false;
}

/**
 * Validate a ContextSource at runtime.
 */
export function validateContextSource(value: unknown): ValidationResult {
  if (value === null || typeof value !== "object") {
    return fail("root", "ContextSource must be a non-null object");
  }
  const source = value as Record<string, unknown>;
  let result = ok();

  // Required string fields.
  for (const field of [
    "id",
    "schemaVersion",
    "kind",
    "projectId",
    "canonicalRef",
    "title",
    "createdAt",
    "updatedAt",
  ] as const) {
    if (!isNonEmptyTrimmedString(source[field])) {
      result = merge(
        result,
        fail(field, `${field} is required and must be a non-empty string`)
      );
    }
  }

  // Schema version must match the current contract.
  if (source.schemaVersion !== CONTEXT_SCHEMA_VERSION) {
    result = merge(
      result,
      fail(
        "schemaVersion",
        `schemaVersion must be ${CONTEXT_SCHEMA_VERSION}, got: ${String(
          source.schemaVersion
        )}`
      )
    );
  }

  // Kind must be a known value.
  if (typeof source.kind === "string") {
    if (!VALID_KINDS.has(source.kind)) {
      result = merge(
        result,
        fail("kind", `kind "${source.kind}" is not a valid ContextSourceKind`)
      );
    }
  }

  // Sensitivity defaults to private if omitted, but must be valid if present.
  if (source.sensitivity !== undefined) {
    if (
      typeof source.sensitivity !== "string" ||
      !VALID_SENSITIVITIES.has(source.sensitivity)
    ) {
      result = merge(
        result,
        fail(
          "sensitivity",
          `sensitivity must be one of public|team|private|secret, got: ${String(
            source.sensitivity
          )}`
        )
      );
    }
  }

  // agentContextEnabled must be boolean.
  if (
    source.agentContextEnabled !== undefined &&
    typeof source.agentContextEnabled !== "boolean"
  ) {
    result = merge(
      result,
      fail("agentContextEnabled", "agentContextEnabled must be a boolean")
    );
  }

  // Optional timestamp fields must be valid ISO strings if present.
  for (const field of ["createdAt", "updatedAt", "indexedAt"] as const) {
    const v = source[field];
    if (v !== undefined && !isValidIsoTimestamp(v)) {
      result = merge(result, fail(field, `${field} must be a valid ISO timestamp`));
    }
  }

  // canonicalRef must not be empty.
  if (
    typeof source.canonicalRef === "string" &&
    source.canonicalRef.trim().length === 0
  ) {
    result = merge(
      result,
      fail("canonicalRef", "canonicalRef must not be empty")
    );
  }

  // contentHash optional string.
  if (source.contentHash !== undefined && typeof source.contentHash !== "string") {
    result = merge(result, fail("contentHash", "contentHash must be a string"));
  }

  // ownerPersonId optional string.
  if (
    source.ownerPersonId !== undefined &&
    typeof source.ownerPersonId !== "string"
  ) {
    result = merge(
      result,
      fail("ownerPersonId", "ownerPersonId must be a string")
    );
  }

  return result;
}

/**
 * Validate a ContextDocument at runtime.
 */
export function validateContextDocument(value: unknown): ValidationResult {
  if (value === null || typeof value !== "object") {
    return fail("root", "ContextDocument must be a non-null object");
  }
  const doc = value as Record<string, unknown>;
  let result = ok();

  // Required string fields.
  for (const field of [
    "id",
    "schemaVersion",
    "sourceId",
    "kind",
    "projectId",
    "canonicalRef",
    "title",
    "text",
    "observedAt",
  ] as const) {
    if (!isNonEmptyTrimmedString(doc[field])) {
      result = merge(
        result,
        fail(field, `${field} is required and must be a non-empty string`)
      );
    }
  }

  // Schema version.
  if (doc.schemaVersion !== CONTEXT_SCHEMA_VERSION) {
    result = merge(
      result,
      fail(
        "schemaVersion",
        `schemaVersion must be ${CONTEXT_SCHEMA_VERSION}`
      )
    );
  }

  // Kind must be a known value.
  if (typeof doc.kind === "string") {
    if (!VALID_KINDS.has(doc.kind)) {
      result = merge(result, fail("kind", `kind "${doc.kind}" is not valid`));
    }
  }

  // Sensitivity.
  if (doc.sensitivity !== undefined) {
    if (
      typeof doc.sensitivity !== "string" ||
      !VALID_SENSITIVITIES.has(doc.sensitivity)
    ) {
      result = merge(
        result,
        fail(
          "sensitivity",
          `sensitivity must be one of public|team|private|secret`
        )
      );
    }
  }

  // agentContextEnabled must be boolean.
  if (
    doc.agentContextEnabled !== undefined &&
    typeof doc.agentContextEnabled !== "boolean"
  ) {
    result = merge(
      result,
      fail("agentContextEnabled", "agentContextEnabled must be a boolean")
    );
  }

  // Timestamps.
  for (const field of ["observedAt", "sourceUpdatedAt"] as const) {
    const v = doc[field];
    if (v !== undefined && !isValidIsoTimestamp(v)) {
      result = merge(result, fail(field, `${field} must be a valid ISO timestamp`));
    }
  }

  // Freshness is required and must be a valid Freshness object.
  if (doc.freshness === undefined) {
    result = merge(result, fail("freshness", "freshness is required"));
  } else if (doc.freshness !== null && typeof doc.freshness === "object") {
    const fresh = doc.freshness as Record<string, unknown>;
    if (
      typeof fresh.state !== "string" ||
      !VALID_FRESHNESS_STATES.has(fresh.state)
    ) {
      result = merge(
        result,
        fail("freshness.state", "freshness.state must be one of fresh|aging|stale|unknown")
      );
    }
    if (!isValidIsoTimestamp(fresh.observedAt)) {
      result = merge(
        result,
        fail("freshness.observedAt", "freshness.observedAt must be a valid ISO timestamp")
      );
    }
    if (fresh.sourceUpdatedAt !== undefined && !isValidIsoTimestamp(fresh.sourceUpdatedAt)) {
      result = merge(
        result,
        fail("freshness.sourceUpdatedAt", "freshness.sourceUpdatedAt must be a valid ISO timestamp")
      );
    }
  } else {
    result = merge(result, fail("freshness", "freshness must be an object"));
  }

  // metadata optional, but if present must be JSON-safe.
  if (doc.metadata !== undefined && !isJsonSafe(doc.metadata)) {
    result = merge(result, fail("metadata", "metadata must be JSON-safe"));
  }

  return result;
}

/** Convenience: throws if a ContextSource is invalid. */
export function assertValidContextSource(value: unknown): asserts value is ContextSource {
  const result = validateContextSource(value);
  if (!result.valid) {
    throw new Error(
      `Invalid ContextSource: ${result.errors
        .map((e) => `${e.path}: ${e.message}`)
        .join("; ")}`
    );
  }
}

/** Convenience: throws if a ContextDocument is invalid. */
export function assertValidContextDocument(value: unknown): asserts value is ContextDocument {
  const result = validateContextDocument(value);
  if (!result.valid) {
    throw new Error(
      `Invalid ContextDocument: ${result.errors
        .map((e) => `${e.path}: ${e.message}`)
        .join("; ")}`
    );
  }
}
