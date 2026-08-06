import { getAppVersion } from "./appVersion";
import {
  excerptReleaseNotes,
  fetchLatestRelease,
  type GithubRelease,
  GithubReleaseError,
} from "./githubReleases";
import { isUpdateAvailable } from "./semver";

export const UPDATE_CHECK_STORAGE_KEY = "openmesh.updateCheck.v1";
/** Quiet background checks at most once per 12 hours. */
export const UPDATE_CHECK_INTERVAL_MS = 12 * 60 * 60 * 1000;

export type UpdateCheckStatus =
  | "idle"
  | "checking"
  | "up_to_date"
  | "update_available"
  | "failed";

export type PersistedUpdateCheck = {
  checkedAt: string;
  currentVersion: string;
  latestVersion: string;
  updateAvailable: boolean;
  htmlUrl: string;
  publishedAt: string | null;
  name: string;
  bodyExcerpt: string;
};

export type UpdateCheckResult =
  | {
      status: "up_to_date";
      currentVersion: string;
      latest: GithubRelease;
      persisted: PersistedUpdateCheck;
    }
  | {
      status: "update_available";
      currentVersion: string;
      latest: GithubRelease;
      persisted: PersistedUpdateCheck;
    }
  | {
      status: "failed";
      currentVersion: string;
      error: string;
      code: GithubReleaseError["code"] | "unknown";
    };

function storage(): Storage | null {
  try {
    if (typeof localStorage === "undefined") return null;
    return localStorage;
  } catch {
    return null;
  }
}

export function readPersistedUpdateCheck(): PersistedUpdateCheck | null {
  const store = storage();
  if (!store) return null;
  try {
    const raw = store.getItem(UPDATE_CHECK_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as PersistedUpdateCheck;
    if (
      typeof parsed?.checkedAt !== "string" ||
      typeof parsed?.latestVersion !== "string"
    ) {
      return null;
    }
    return parsed;
  } catch {
    return null;
  }
}

export function writePersistedUpdateCheck(value: PersistedUpdateCheck): void {
  const store = storage();
  if (!store) return;
  try {
    store.setItem(UPDATE_CHECK_STORAGE_KEY, JSON.stringify(value));
  } catch {
    /* quota / private mode — ignore */
  }
}

export function shouldRunBackgroundCheck(
  persisted: PersistedUpdateCheck | null,
  nowMs = Date.now(),
  intervalMs = UPDATE_CHECK_INTERVAL_MS,
): boolean {
  if (!persisted?.checkedAt) return true;
  const checked = Date.parse(persisted.checkedAt);
  if (Number.isNaN(checked)) return true;
  return nowMs - checked >= intervalMs;
}

/** True when a prior check found a newer release than the running app. */
export function hasKnownUpdate(
  persisted: PersistedUpdateCheck | null,
  currentVersion = getAppVersion(),
): boolean {
  if (!persisted?.updateAvailable) return false;
  return isUpdateAvailable(currentVersion, persisted.latestVersion);
}

export function openExternalUrl(url: string): void {
  if (typeof window === "undefined" || !url) return;
  window.open(url, "_blank", "noopener,noreferrer");
}

export async function checkForUpdates(
  currentVersion = getAppVersion(),
): Promise<UpdateCheckResult> {
  try {
    const latest = await fetchLatestRelease(currentVersion);
    const updateAvailable = isUpdateAvailable(currentVersion, latest.version);
    const persisted: PersistedUpdateCheck = {
      checkedAt: new Date().toISOString(),
      currentVersion,
      latestVersion: latest.version,
      updateAvailable,
      htmlUrl: latest.htmlUrl,
      publishedAt: latest.publishedAt,
      name: latest.name,
      bodyExcerpt: excerptReleaseNotes(latest.body),
    };
    writePersistedUpdateCheck(persisted);
    return updateAvailable
      ? { status: "update_available", currentVersion, latest, persisted }
      : { status: "up_to_date", currentVersion, latest, persisted };
  } catch (err) {
    if (err instanceof GithubReleaseError) {
      return {
        status: "failed",
        currentVersion,
        error: err.message,
        code: err.code,
      };
    }
    return {
      status: "failed",
      currentVersion,
      error: err instanceof Error ? err.message : String(err),
      code: "unknown",
    };
  }
}

/**
 * Rate-limited quiet check for app start / Settings mount.
 * Never throws; returns null when skipped or failed (failures stay quiet).
 */
export async function maybeBackgroundUpdateCheck(
  currentVersion = getAppVersion(),
): Promise<PersistedUpdateCheck | null> {
  const existing = readPersistedUpdateCheck();
  if (!shouldRunBackgroundCheck(existing)) {
    return existing;
  }
  const result = await checkForUpdates(currentVersion);
  if (result.status === "failed") return existing;
  return result.persisted;
}
