import { describe, expect, it } from "vitest";
import {
  emptyNotesFallback,
  normalizeArch,
  normalizeOs,
  pickReleaseAsset,
  releaseHasInstallerAssets,
  scoreAssetForPlatform,
  type HostPlatform,
  type ReleaseAsset,
} from "@/lib/updates/releaseAssets";
import { excerptReleaseNotes } from "@/lib/updates/githubReleases";

const sampleAssets: ReleaseAsset[] = [
  {
    name: "OpenMesh_0.1.29_aarch64.dmg",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_aarch64.dmg",
    size: 12_000_000,
  },
  {
    name: "OpenMesh_0.1.29_x64.dmg",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_x64.dmg",
    size: 12_400_000,
  },
  {
    name: "OpenMesh_0.1.29_aarch64.app.tar.gz",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_aarch64.app.tar.gz",
    size: 12_000_000,
  },
  {
    name: "OpenMesh_0.1.29_x64-setup.exe",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_x64-setup.exe",
    size: 8_600_000,
  },
  {
    name: "OpenMesh_0.1.29_x64_en-US.msi",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_x64_en-US.msi",
    size: 10_900_000,
  },
  {
    name: "OpenMesh_0.1.29_amd64.AppImage",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_amd64.AppImage",
    size: 90_000_000,
  },
  {
    name: "OpenMesh_0.1.29_amd64.deb",
    browserDownloadUrl:
      "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/download/v0.1.29/OpenMesh_0.1.29_amd64.deb",
    size: 13_500_000,
  },
];

describe("normalizeArch / normalizeOs", () => {
  it("maps common arch aliases", () => {
    expect(normalizeArch("arm64")).toBe("aarch64");
    expect(normalizeArch("aarch64")).toBe("aarch64");
    expect(normalizeArch("x86_64")).toBe("x64");
    expect(normalizeArch("amd64")).toBe("x64");
  });

  it("maps OS strings", () => {
    expect(normalizeOs("darwin")).toBe("macos");
    expect(normalizeOs("macos")).toBe("macos");
    expect(normalizeOs("win32")).toBe("windows");
    expect(normalizeOs("linux")).toBe("linux");
    expect(normalizeOs("freebsd")).toBe("unsupported");
  });
});

describe("pickReleaseAsset", () => {
  it("picks aarch64 dmg on Apple Silicon", () => {
    const platform: HostPlatform = { os: "macos", arch: "aarch64" };
    const pick = pickReleaseAsset(sampleAssets, platform);
    expect(pick.status).toBe("ready");
    if (pick.status === "ready") {
      expect(pick.asset.name).toContain("aarch64.dmg");
    }
  });

  it("picks x64 dmg on Intel Mac", () => {
    const platform: HostPlatform = { os: "macos", arch: "x64" };
    const pick = pickReleaseAsset(sampleAssets, platform);
    expect(pick.status).toBe("ready");
    if (pick.status === "ready") {
      expect(pick.asset.name).toContain("_x64.dmg");
    }
  });

  it("ignores .app.tar.gz on macOS", () => {
    const platform: HostPlatform = { os: "macos", arch: "aarch64" };
    expect(
      scoreAssetForPlatform(
        {
          name: "OpenMesh_0.1.29_aarch64.app.tar.gz",
          browserDownloadUrl: "https://example.com/a",
          size: 1,
        },
        platform,
      ),
    ).toBe(0);
  });

  it("prefers setup.exe over msi on Windows", () => {
    const platform: HostPlatform = { os: "windows", arch: "x64" };
    const pick = pickReleaseAsset(sampleAssets, platform);
    expect(pick.status).toBe("ready");
    if (pick.status === "ready") {
      expect(pick.asset.name).toContain("-setup.exe");
    }
  });

  it("prefers AppImage over deb on Linux", () => {
    const platform: HostPlatform = { os: "linux", arch: "x64" };
    const pick = pickReleaseAsset(sampleAssets, platform);
    expect(pick.status).toBe("ready");
    if (pick.status === "ready") {
      expect(pick.asset.name.toLowerCase()).toContain(".appimage");
    }
  });

  it("reports missing when no platform assets", () => {
    const platform: HostPlatform = { os: "macos", arch: "aarch64" };
    const pick = pickReleaseAsset(
      [
        {
          name: "OpenMesh_0.1.29_x64-setup.exe",
          browserDownloadUrl: "https://example.com/e",
          size: 1,
        },
      ],
      platform,
    );
    expect(pick.status).toBe("missing");
  });

  it("reports unsupported platform", () => {
    const pick = pickReleaseAsset(sampleAssets, {
      os: "unsupported",
      arch: "x64",
    });
    expect(pick).toEqual({
      status: "missing",
      reason: "unsupported_platform",
    });
  });
});

describe("empty notes fallback", () => {
  it("does not block install messaging when assets exist", () => {
    expect(releaseHasInstallerAssets(sampleAssets)).toBe(true);
    expect(emptyNotesFallback(sampleAssets)).toBe(
      "Assets ready — see GitHub for full notes.",
    );
    expect(emptyNotesFallback([])).toBe("Release notes are not available yet.");
    expect(excerptReleaseNotes("")).toBe("");
  });
});
