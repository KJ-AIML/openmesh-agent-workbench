/** Platform + arch used to pick a GitHub release installer asset. */

export type UpdateOs = "macos" | "windows" | "linux" | "unsupported";
export type UpdateArch = "aarch64" | "x64" | "unknown";

export type HostPlatform = {
  os: UpdateOs;
  arch: UpdateArch;
};

export type ReleaseAsset = {
  name: string;
  browserDownloadUrl: string;
  size: number;
};

export type AssetPickResult =
  | { status: "ready"; asset: ReleaseAsset; score: number }
  | { status: "missing"; reason: "no_matching_asset" | "unsupported_platform" };

const IGNORED_SUFFIXES = [
  ".app.tar.gz",
  ".sig",
  ".json",
  ".txt",
  ".sha256",
  ".asc",
];

function lowerName(asset: ReleaseAsset): string {
  return asset.name.toLowerCase();
}

function isIgnoredAsset(name: string): boolean {
  return IGNORED_SUFFIXES.some((suffix) => name.endsWith(suffix));
}

/** Map Rust / Node arch strings onto our UpdateArch. */
export function normalizeArch(raw: string): UpdateArch {
  const a = raw.trim().toLowerCase();
  if (
    a === "aarch64" ||
    a === "arm64" ||
    a === "arm64e" ||
    a.includes("aarch64") ||
    a.includes("arm64")
  ) {
    return "aarch64";
  }
  if (
    a === "x86_64" ||
    a === "x64" ||
    a === "amd64" ||
    a.includes("x86_64") ||
    a.includes("amd64")
  ) {
    return "x64";
  }
  return "unknown";
}

export function normalizeOs(raw: string): UpdateOs {
  const o = raw.trim().toLowerCase();
  if (o === "macos" || o === "darwin") return "macos";
  if (o === "windows" || o === "win32") return "windows";
  if (o === "linux") return "linux";
  return "unsupported";
}

/**
 * Score how well an asset matches the host. Higher is better.
 * Returns 0 when the asset should not be used for this platform.
 */
export function scoreAssetForPlatform(
  asset: ReleaseAsset,
  platform: HostPlatform,
): number {
  const name = lowerName(asset);
  if (!name || isIgnoredAsset(name)) return 0;

  if (platform.os === "macos") {
    if (!name.endsWith(".dmg")) return 0;
    if (platform.arch === "aarch64") {
      if (name.includes("aarch64") || name.includes("arm64")) return 100;
      // Prefer not to pick Intel DMG on Apple Silicon.
      if (name.includes("_x64") || name.includes("-x64") || name.includes("x86_64"))
        return 10;
      return 40;
    }
    if (platform.arch === "x64") {
      if (name.includes("_x64") || name.includes("-x64") || name.includes("x86_64"))
        return 100;
      if (name.includes("aarch64") || name.includes("arm64")) return 10;
      return 40;
    }
    // Unknown arch: any dmg is better than nothing, prefer aarch64 on modern Macs.
    if (name.includes("aarch64") || name.includes("arm64")) return 60;
    if (name.includes("_x64") || name.includes("-x64")) return 55;
    return 40;
  }

  if (platform.os === "windows") {
    if (name.endsWith("-setup.exe") || name.endsWith("_x64-setup.exe")) return 100;
    if (name.endsWith(".exe") && !name.includes("unins")) return 90;
    if (name.endsWith(".msi")) return 80;
    return 0;
  }

  if (platform.os === "linux") {
    // Prefer AppImage (portable) then deb then rpm.
    if (name.endsWith(".appimage")) return 100;
    if (name.endsWith(".deb")) return 90;
    if (name.endsWith(".rpm")) return 70;
    return 0;
  }

  return 0;
}

/** Pick the best installer asset for this host from a release asset list. */
export function pickReleaseAsset(
  assets: readonly ReleaseAsset[],
  platform: HostPlatform,
): AssetPickResult {
  if (platform.os === "unsupported") {
    return { status: "missing", reason: "unsupported_platform" };
  }

  let best: ReleaseAsset | null = null;
  let bestScore = 0;
  for (const asset of assets) {
    const score = scoreAssetForPlatform(asset, platform);
    if (score > bestScore) {
      bestScore = score;
      best = asset;
    }
  }

  if (!best || bestScore <= 0) {
    return { status: "missing", reason: "no_matching_asset" };
  }
  // Reject weak cross-arch mac picks when a better one wasn't found —
  // score 10 means wrong-arch only; still allow if that's all CI uploaded.
  return { status: "ready", asset: best, score: bestScore };
}

/** True when the release has any non-ignored binary installer. */
export function releaseHasInstallerAssets(
  assets: readonly ReleaseAsset[],
): boolean {
  return assets.some((a) => {
    const name = lowerName(a);
    if (isIgnoredAsset(name)) return false;
    return (
      name.endsWith(".dmg") ||
      name.endsWith(".exe") ||
      name.endsWith(".msi") ||
      name.endsWith(".appimage") ||
      name.endsWith(".deb") ||
      name.endsWith(".rpm")
    );
  });
}

/** Fallback copy when GitHub release body is empty. */
export function emptyNotesFallback(
  assets: readonly ReleaseAsset[],
): string {
  if (releaseHasInstallerAssets(assets)) {
    return "Assets ready — see GitHub for full notes.";
  }
  return "Release notes are not available yet.";
}
