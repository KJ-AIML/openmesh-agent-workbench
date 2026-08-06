/**
 * Lightweight semver helpers for release comparison.
 * Handles `v` prefixes and optional prerelease suffixes (`-rc.1`).
 */

export type SemverParts = {
  major: number;
  minor: number;
  patch: number;
  /** Empty string when absent. Lower than a release of the same core version. */
  prerelease: string;
};

/** Strip a leading `v`/`V` and whitespace. */
export function normalizeVersion(input: string): string {
  return input.trim().replace(/^v/i, "");
}

/**
 * Parse `X.Y.Z` or `X.Y.Z-prerelease`.
 * Returns null when the core triple is not numeric.
 */
export function parseSemver(input: string): SemverParts | null {
  const normalized = normalizeVersion(input);
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/.exec(normalized);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: match[4] ?? "",
  };
}

function compareIdentifiers(a: string, b: string): number {
  const aNum = /^\d+$/.test(a);
  const bNum = /^\d+$/.test(b);
  if (aNum && bNum) {
    const diff = Number(a) - Number(b);
    return diff === 0 ? 0 : diff < 0 ? -1 : 1;
  }
  if (aNum && !bNum) return -1;
  if (!aNum && bNum) return 1;
  if (a === b) return 0;
  return a < b ? -1 : 1;
}

function comparePrerelease(a: string, b: string): number {
  // No prerelease > any prerelease (1.0.0 > 1.0.0-rc.1)
  if (!a && !b) return 0;
  if (!a) return 1;
  if (!b) return -1;
  const aParts = a.split(".");
  const bParts = b.split(".");
  const len = Math.max(aParts.length, bParts.length);
  for (let i = 0; i < len; i++) {
    const left = aParts[i];
    const right = bParts[i];
    if (left === undefined) return -1;
    if (right === undefined) return 1;
    const cmp = compareIdentifiers(left, right);
    if (cmp !== 0) return cmp;
  }
  return 0;
}

/**
 * Compare two version strings.
 * @returns -1 if a < b, 0 if equal, 1 if a > b
 * Falls back to string compare when either side fails to parse.
 */
export function compareSemver(a: string, b: string): number {
  const left = parseSemver(a);
  const right = parseSemver(b);
  if (!left || !right) {
    const an = normalizeVersion(a);
    const bn = normalizeVersion(b);
    if (an === bn) return 0;
    return an < bn ? -1 : 1;
  }
  if (left.major !== right.major) return left.major < right.major ? -1 : 1;
  if (left.minor !== right.minor) return left.minor < right.minor ? -1 : 1;
  if (left.patch !== right.patch) return left.patch < right.patch ? -1 : 1;
  return comparePrerelease(left.prerelease, right.prerelease);
}

/** True when `latest` is strictly newer than `current`. */
export function isUpdateAvailable(current: string, latest: string): boolean {
  return compareSemver(current, latest) < 0;
}

/** True when the running app version itself looks like a prerelease. */
export function isPrereleaseVersion(version: string): boolean {
  const parsed = parseSemver(version);
  return Boolean(parsed?.prerelease);
}
