import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { isTauriRuntime } from "../adapters/environment";
import {
  normalizeArch,
  normalizeOs,
  pickReleaseAsset,
  type HostPlatform,
  type ReleaseAsset,
} from "./releaseAssets";
import { openExternalUrl } from "./updateCheck";

export const UPDATE_DOWNLOAD_PROGRESS_EVENT = "update-download-progress";

export type InstallUpdateStatus =
  | "idle"
  | "resolving"
  | "downloading"
  | "opening"
  | "opened"
  | "failed"
  | "unsupported"
  | "assets_missing";

export type DownloadProgress = {
  receivedBytes: number;
  totalBytes: number | null;
  /** 0–100 when total known; null when indeterminate. */
  percent: number | null;
};

export type InstallUpdateResult = {
  path: string;
  opened: boolean;
  nextSteps: string;
  platformOs: string;
};

export type ResolveInstallTarget =
  | { status: "ready"; asset: ReleaseAsset; platform: HostPlatform }
  | { status: "assets_missing"; platform: HostPlatform }
  | { status: "unsupported"; platform: HostPlatform };

async function detectHostPlatform(): Promise<HostPlatform> {
  if (!isTauriRuntime()) {
    const ua = typeof navigator !== "undefined" ? navigator.userAgent : "";
    const platform =
      typeof navigator !== "undefined" ? navigator.platform || "" : "";
    let os = normalizeOs("unsupported");
    if (/Mac|iPhone|iPad|iPod/i.test(platform) || /Macintosh|Mac OS X/i.test(ua)) {
      os = "macos";
    } else if (/Win/i.test(platform) || /Windows/i.test(ua)) {
      os = "windows";
    } else if (/Linux/i.test(platform) || /Linux/i.test(ua)) {
      os = "linux";
    }
    let arch = normalizeArch("unknown");
    if (/arm64|aarch64/i.test(ua) || /arm64|aarch64/i.test(platform)) {
      arch = "aarch64";
    } else if (/x86_64|Win64|WOW64|amd64/i.test(ua)) {
      arch = "x64";
    }
    // Apple Silicon often reports MacIntel in UA — prefer aarch64 when unknown on mac.
    if (os === "macos" && arch === "unknown") arch = "aarch64";
    return { os, arch };
  }

  try {
    const [osRaw, archRaw] = await Promise.all([
      invoke<string>("get_host_os"),
      invoke<string>("get_host_arch"),
    ]);
    return { os: normalizeOs(osRaw), arch: normalizeArch(archRaw) };
  } catch {
    return { os: "unsupported", arch: "unknown" };
  }
}

export async function resolveInstallTarget(
  assets: readonly ReleaseAsset[],
): Promise<ResolveInstallTarget> {
  const platform = await detectHostPlatform();
  if (platform.os === "unsupported") {
    return { status: "unsupported", platform };
  }
  const pick = pickReleaseAsset(assets, platform);
  if (pick.status !== "ready") {
    return { status: "assets_missing", platform };
  }
  return { status: "ready", asset: pick.asset, platform };
}

export async function listenDownloadProgress(
  onProgress: (p: DownloadProgress) => void,
): Promise<UnlistenFn> {
  if (!isTauriRuntime()) {
    return () => undefined;
  }
  return listen<DownloadProgress>(UPDATE_DOWNLOAD_PROGRESS_EVENT, (event) => {
    onProgress(event.payload);
  });
}

/**
 * Download the chosen asset via Tauri and open the installer.
 * In the browser (non-Tauri), opens the download URL instead.
 */
export async function downloadAndOpenInstaller(
  asset: ReleaseAsset,
): Promise<InstallUpdateResult> {
  if (!isTauriRuntime()) {
    openExternalUrl(asset.browserDownloadUrl);
    return {
      path: "",
      opened: true,
      nextSteps:
        "Download started in your browser. Open the installer when it finishes.",
      platformOs: "browser",
    };
  }

  return invoke<InstallUpdateResult>("download_and_open_update", {
    url: asset.browserDownloadUrl,
    filename: asset.name,
  });
}

export function installButtonLabel(status: InstallUpdateStatus): string {
  switch (status) {
    case "resolving":
      return "Preparing…";
    case "downloading":
      return "Downloading…";
    case "opening":
      return "Opening installer…";
    case "opened":
      return "Installer opened";
    default:
      return "Download & install";
  }
}

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n < 0) return "";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}
