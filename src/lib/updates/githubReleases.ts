import { isPrereleaseVersion, normalizeVersion } from "./semver";

export const GITHUB_OWNER = "KJ-AIML";
export const GITHUB_REPO = "openmesh-agent-workbench";
export const RELEASES_LATEST_URL = `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases/latest`;
export const RELEASES_LIST_URL = `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/releases?per_page=20`;

export type GithubRelease = {
  tagName: string;
  version: string;
  name: string;
  htmlUrl: string;
  publishedAt: string | null;
  body: string;
  prerelease: boolean;
};

type RawRelease = {
  tag_name?: string;
  name?: string | null;
  html_url?: string;
  published_at?: string | null;
  body?: string | null;
  draft?: boolean;
  prerelease?: boolean;
};

export class GithubReleaseError extends Error {
  readonly status?: number;
  readonly code: "network" | "rate_limit" | "http" | "parse" | "not_found";

  constructor(
    message: string,
    opts: { status?: number; code: GithubReleaseError["code"] },
  ) {
    super(message);
    this.name = "GithubReleaseError";
    this.status = opts.status;
    this.code = opts.code;
  }
}

function mapRelease(raw: RawRelease): GithubRelease | null {
  const tagName = String(raw.tag_name ?? "").trim();
  if (!tagName) return null;
  return {
    tagName,
    version: normalizeVersion(tagName),
    name: String(raw.name ?? tagName).trim() || tagName,
    htmlUrl:
      String(raw.html_url ?? "").trim() ||
      `https://github.com/${GITHUB_OWNER}/${GITHUB_REPO}/releases/tag/${encodeURIComponent(tagName)}`,
    publishedAt: raw.published_at ?? null,
    body: String(raw.body ?? "").trim(),
    prerelease: Boolean(raw.prerelease),
  };
}

async function fetchJson(url: string): Promise<unknown> {
  let response: Response;
  try {
    response = await fetch(url, {
      headers: {
        Accept: "application/vnd.github+json",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    });
  } catch {
    throw new GithubReleaseError(
      "Could not reach GitHub. Check your network connection.",
      { code: "network" },
    );
  }

  if (response.status === 403 || response.status === 429) {
    throw new GithubReleaseError(
      "GitHub rate limit reached. Try again later.",
      { status: response.status, code: "rate_limit" },
    );
  }
  if (response.status === 404) {
    throw new GithubReleaseError("No published releases found.", {
      status: 404,
      code: "not_found",
    });
  }
  if (!response.ok) {
    throw new GithubReleaseError(
      `GitHub returned HTTP ${response.status}.`,
      { status: response.status, code: "http" },
    );
  }

  try {
    return await response.json();
  } catch {
    throw new GithubReleaseError("Unexpected response from GitHub.", {
      code: "parse",
    });
  }
}

/**
 * Fetch the latest relevant release.
 * Stable apps use `/releases/latest` (non-draft, non-prerelease).
 * Prerelease apps scan the releases list and accept prereleases.
 */
export async function fetchLatestRelease(
  currentAppVersion: string,
): Promise<GithubRelease> {
  if (isPrereleaseVersion(currentAppVersion)) {
    const data = await fetchJson(RELEASES_LIST_URL);
    if (!Array.isArray(data)) {
      throw new GithubReleaseError("Unexpected response from GitHub.", {
        code: "parse",
      });
    }
    for (const item of data) {
      const raw = item as RawRelease;
      if (raw.draft) continue;
      const mapped = mapRelease(raw);
      if (mapped) return mapped;
    }
    throw new GithubReleaseError("No published releases found.", {
      code: "not_found",
    });
  }

  const data = await fetchJson(RELEASES_LATEST_URL);
  const mapped = mapRelease(data as RawRelease);
  if (!mapped) {
    throw new GithubReleaseError("Unexpected response from GitHub.", {
      code: "parse",
    });
  }
  return mapped;
}

/** Truncate release notes for the Settings panel. */
export function excerptReleaseNotes(body: string, maxLen = 420): string {
  const cleaned = body
    .replace(/\r\n/g, "\n")
    .replace(/^#{1,6}\s+/gm, "")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  if (!cleaned) return "";
  if (cleaned.length <= maxLen) return cleaned;
  return `${cleaned.slice(0, maxLen).trimEnd()}…`;
}
