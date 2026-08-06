import { describe, expect, it } from "vitest";
import {
  compareSemver,
  isPrereleaseVersion,
  isUpdateAvailable,
  normalizeVersion,
  parseSemver,
} from "@/lib/updates/semver";
import {
  excerptReleaseNotes,
} from "@/lib/updates/githubReleases";
import {
  hasKnownUpdate,
  shouldRunBackgroundCheck,
  type PersistedUpdateCheck,
} from "@/lib/updates/updateCheck";

describe("normalizeVersion", () => {
  it("strips leading v", () => {
    expect(normalizeVersion("v0.1.26")).toBe("0.1.26");
    expect(normalizeVersion("V1.2.3")).toBe("1.2.3");
  });
});

describe("parseSemver", () => {
  it("parses core and prerelease", () => {
    expect(parseSemver("v0.1.26")).toEqual({
      major: 0,
      minor: 1,
      patch: 26,
      prerelease: "",
    });
    expect(parseSemver("1.0.0-rc.1")).toEqual({
      major: 1,
      minor: 0,
      patch: 0,
      prerelease: "rc.1",
    });
  });

  it("returns null for junk", () => {
    expect(parseSemver("latest")).toBeNull();
  });
});

describe("compareSemver / isUpdateAvailable", () => {
  it("orders patch/minor/major", () => {
    expect(compareSemver("0.1.25", "0.1.26")).toBe(-1);
    expect(compareSemver("0.1.26", "0.1.26")).toBe(0);
    expect(compareSemver("0.2.0", "0.1.99")).toBe(1);
  });

  it("treats release as newer than matching prerelease", () => {
    expect(compareSemver("1.0.0-rc.1", "1.0.0")).toBe(-1);
    expect(isUpdateAvailable("1.0.0-rc.1", "1.0.0")).toBe(true);
  });

  it("detects updates with v prefix", () => {
    expect(isUpdateAvailable("0.1.26", "v0.1.27")).toBe(true);
    expect(isUpdateAvailable("v0.1.27", "0.1.26")).toBe(false);
  });
});

describe("isPrereleaseVersion", () => {
  it("flags prerelease tags", () => {
    expect(isPrereleaseVersion("0.2.0-rc.1")).toBe(true);
    expect(isPrereleaseVersion("0.1.26")).toBe(false);
  });
});

describe("excerptReleaseNotes", () => {
  it("truncates long bodies", () => {
    const long = "a".repeat(500);
    const out = excerptReleaseNotes(long, 40);
    expect(out.endsWith("…")).toBe(true);
    expect(out.length).toBeLessThanOrEqual(41);
  });
});

describe("updateCheck helpers", () => {
  it("rate-limits background checks", () => {
    const recent: PersistedUpdateCheck = {
      checkedAt: new Date().toISOString(),
      currentVersion: "0.1.26",
      latestVersion: "0.1.26",
      updateAvailable: false,
      htmlUrl: "https://example.com",
      publishedAt: null,
      name: "x",
      bodyExcerpt: "",
    };
    expect(shouldRunBackgroundCheck(recent)).toBe(false);
    expect(shouldRunBackgroundCheck(null)).toBe(true);
    expect(
      shouldRunBackgroundCheck(recent, Date.now() + 13 * 60 * 60 * 1000),
    ).toBe(true);
  });

  it("hasKnownUpdate compares against current app version", () => {
    const persisted: PersistedUpdateCheck = {
      checkedAt: new Date().toISOString(),
      currentVersion: "0.1.25",
      latestVersion: "0.1.26",
      updateAvailable: true,
      htmlUrl: "https://example.com",
      publishedAt: null,
      name: "x",
      bodyExcerpt: "",
    };
    expect(hasKnownUpdate(persisted, "0.1.25")).toBe(true);
    expect(hasKnownUpdate(persisted, "0.1.26")).toBe(false);
  });
});
