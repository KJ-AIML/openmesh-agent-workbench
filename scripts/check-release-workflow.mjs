#!/usr/bin/env node
/* global process */
/**
 * Fail if release.yml maps APPLE_* / WINDOWS_* secrets into env.
 *
 * Empty GitHub Actions secrets become "" at runtime. tauri's macOS bundler
 * treats a present APPLE_CERTIFICATE as "import this .p12" and fails with
 * SecKeychainItemImport — the v0.1.27 → v0.1.28 regression.
 *
 * When real certs exist, update this allowlist intentionally (do not "fix"
 * the check by re-adding empty secret mappings).
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const workflowPath = join(root, ".github/workflows/release.yml");
const text = readFileSync(workflowPath, "utf8");

/** Keys that must not be wired from secrets until they are non-empty in CI. */
const FORBIDDEN = [
  "APPLE_CERTIFICATE",
  "APPLE_CERTIFICATE_PASSWORD",
  "APPLE_SIGNING_IDENTITY",
  "APPLE_ID",
  "APPLE_PASSWORD",
  "APPLE_TEAM_ID",
  "WINDOWS_CERTIFICATE",
  "WINDOWS_CERTIFICATE_PASSWORD",
];

const offenders = [];
for (const key of FORBIDDEN) {
  // Match env mapping forms: KEY: ${{ secrets.KEY }} (any whitespace)
  const re = new RegExp(
    `^\\s*${key}\\s*:\\s*\\$\\{\\{\\s*secrets\\.[A-Z0-9_]+\\s*\\}\\}`,
    "im",
  );
  if (re.test(text)) {
    offenders.push(key);
  }
}

if (offenders.length > 0) {
  console.error(
    "check-release-workflow: refusing empty-secret regression.\n" +
      "These keys are mapped from GitHub secrets in .github/workflows/release.yml:\n" +
      offenders.map((k) => `  - ${k}`).join("\n") +
      "\n\nOnly add them after real non-empty secrets exist (see docs/RELEASE_SMOKE.md).\n" +
      "An empty APPLE_CERTIFICATE breaks macOS codesign (SecKeychainItemImport).",
  );
  process.exit(1);
}

console.log(
  "check-release-workflow: ok — no APPLE_*/WINDOWS_* secret env mappings in release.yml",
);

/**
 * REGRESSION GUARD (v0.2.0-rc.1 → v0.2.0-rc.2):
 * WiX MSI bundling fails on Windows when the app version contains an alphanumeric
 * pre-release identifier (e.g. "0.2.0-rc.2") with:
 * "optional pre-release identifier in app version must be numeric-only and cannot be greater than 65535 for msi target".
 * Therefore, whenever the project version contains a pre-release identifier,
 * the release workflow matrix for windows-latest MUST restrict the bundle target
 * to NSIS (--bundles nsis).
 */
const pkgPath = join(root, "package.json");
const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const isPrerelease = Boolean(pkg.version && pkg.version.includes("-"));

if (isPrerelease) {
  const windowsMatrixMatch = text.match(
    /-\s+platform:\s*['"]windows-latest['"][\s\S]*?args:\s*['"]([^'"]*)['"]/,
  );
  if (!windowsMatrixMatch) {
    console.error(
      "check-release-workflow: could not find 'windows-latest' matrix entry with 'args' in release.yml",
    );
    process.exit(1);
  }
  const windowsArgs = windowsMatrixMatch[1].trim();
  const hasNsisBundle = /--bundles\s+nsis\b/.test(windowsArgs);
  if (!hasNsisBundle) {
    console.error(
      `check-release-workflow: refusing Windows prerelease packaging regression.\n` +
        `Current version "${pkg.version}" is a pre-release version.\n` +
        `WiX MSI targets fail on alphanumeric pre-release identifiers.\n` +
        `.github/workflows/release.yml windows-latest matrix must specify args: '--bundles nsis'.\n` +
        `Observed args: "${windowsArgs}"`,
    );
    process.exit(1);
  }
  console.log(
    `check-release-workflow: ok — prerelease version ${pkg.version} on Windows correctly restricted to NSIS (--bundles nsis)`,
  );
} else {
  console.log(
    `check-release-workflow: ok — stable version ${pkg.version} (Windows may build both MSI and NSIS)`,
  );
}
