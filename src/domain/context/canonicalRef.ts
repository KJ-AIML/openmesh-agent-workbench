// ============================================================================
// Canonical Reference & Identity Helpers
// ============================================================================
// These helpers are PURE and DETERMINISTIC.
// The same canonical source always produces the same canonicalRef and sourceId
// across repeated rebuilds, regardless of project folder location.
// ============================================================================

import type { CanonicalRef, ContextSourceKind } from "./types";

/**
 * Base scheme for OpenMesh canonical references.
 */
const CANONICAL_SCHEME = "openmesh";

/**
 * Percent-encode a source key for safe embedding inside a canonicalRef.
 *
 * Encodes characters that would be unsafe inside a URI-like path segment:
 *   - `%` itself (to avoid double-encoding ambiguity)
 *   - `#` (fragment separator)
 *   - `?` (query separator)
 *   - control chars
 *
 * Preserves `/` so that nested relative paths (e.g. `architecture/design.md`)
 * remain readable and do not collapse hierarchy.
 */
export function encodeSourceKey(sourceKey: string): string {
  // Avoid double-encoding: decode first, then re-encode.
  let decoded: string;
  try {
    decoded = decodeURIComponent(sourceKey);
  } catch {
    decoded = sourceKey;
  }

  // Encode after decoding to get a stable, fully-encoded form.
  return decoded
    .split("")
    .map((ch) => {
      // Preserve unreserved characters and '/' for path hierarchy.
      if (/[A-Za-z0-9_.!~-]/.test(ch)) return ch;
      if (ch === "/") return "/";
      // Percent-encode everything else.
      return encodeURIComponent(ch);
    })
    .join("");
}

/**
 * Normalize path separators to forward slashes.
 *
 * Windows paths (`\`) become POSIX (`/`) so that the same document under
 * either OS generates the same canonical reference.
 */
export function normalizeSeparators(raw: string): string {
  return raw.replace(/\\/g, "/");
}

/**
 * Build a canonical reference for a context source.
 *
 * Format: openmesh://project/<projectId>/<kind>/<encoded-source-key>
 *
 * Rules:
 *   - projectId, kind, and sourceKey are required and non-empty.
 *   - All segments are trimmed of surrounding whitespace.
 *   - sourceKey separators are normalized.
 *   - The result is deterministic for identical inputs.
 */
export function buildCanonicalRef(params: {
  projectId: string;
  kind: ContextSourceKind;
  sourceKey: string;
}): CanonicalRef {
  const { projectId, kind, sourceKey } = params;

  const cleanProject = projectId.trim();
  const cleanKind = kind.trim();
  const cleanKey = encodeSourceKey(normalizeSeparators(sourceKey.trim()));

  if (!cleanProject) {
    throw new Error("buildCanonicalRef: projectId must be non-empty");
  }
  if (!cleanKind) {
    throw new Error("buildCanonicalRef: kind must be non-empty");
  }
  if (!cleanKey) {
    throw new Error("buildCanonicalRef: sourceKey must be non-empty");
  }

  return (
    `${CANONICAL_SCHEME}://project/` +
    `${encodeURIComponent(cleanProject)}/` +
    `${encodeURIComponent(cleanKind)}/` +
    `${cleanKey}`
  );
}

/**
 * Derive a deterministic source ID from a canonical reference.
 *
 * The ID is a stable hash of the canonicalRef string. This ensures:
 *   - Same canonicalRef → same ID
 *   - The ID survives project folder moves
 *   - The ID can be used as a primary key in derived indexes
 *
 * We use a simple deterministic hash (FNV-1a variant) rather than a random
 * UUID so that rebuilds remain stable and deduplication works.
 *
 * Returns a 16-character hex string.
 */
export function deriveSourceId(canonicalRef: CanonicalRef): string {
  let hash = 0x811c9dc5;
  for (let i = 0; i < canonicalRef.length; i++) {
    hash ^= canonicalRef.charCodeAt(i);
    // 32-bit FNV-1a prime multiply.
    hash = Math.imul(hash, 0x01000193);
  }
  // Convert to unsigned 32-bit, then pad to hex.
  const hex = (hash >>> 0).toString(16).padStart(8, "0");
  // Extend to 16 chars by hashing a second pass with a seed.
  let hash2 = 0x811c9dc5 ^ 0xdeadbeef;
  for (let i = 0; i < canonicalRef.length; i++) {
    hash2 ^= canonicalRef.charCodeAt(canonicalRef.length - 1 - i);
    hash2 = Math.imul(hash2, 0x01000193);
  }
  const hex2 = (hash2 >>> 0).toString(16).padStart(8, "0");
  return hex + hex2;
}

/**
 * Convenience: build both canonicalRef and derived ID in one call.
 */
export function buildCanonicalIdentity(params: {
  projectId: string;
  kind: ContextSourceKind;
  sourceKey: string;
}): { canonicalRef: CanonicalRef; id: string } {
  const canonicalRef = buildCanonicalRef(params);
  return {
    canonicalRef,
    id: deriveSourceId(canonicalRef),
  };
}
