#!/usr/bin/env node
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
