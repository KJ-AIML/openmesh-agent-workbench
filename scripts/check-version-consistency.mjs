#!/usr/bin/env node
/* global process */
/**
 * Verifies version consistency across all workspace manifests:
 * - package.json
 * - src-tauri/tauri.conf.json
 * - src-tauri/Cargo.toml
 * - crates/openmesh-core/Cargo.toml
 * - crates/openmesh-cli/Cargo.toml
 *
 * Usage:
 *   node scripts/check-version-consistency.mjs [expected_version]
 */

import { readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const MANIFESTS = [
  {
    path: join(root, "package.json"),
    extract: (content) => JSON.parse(content).version,
  },
  {
    path: join(root, "src-tauri", "tauri.conf.json"),
    extract: (content) => JSON.parse(content).version,
  },
  {
    path: join(root, "src-tauri", "Cargo.toml"),
    extract: (content) => extractCargoPackageVersion(content),
  },
  {
    path: join(root, "crates", "openmesh-core", "Cargo.toml"),
    extract: (content) => extractCargoPackageVersion(content),
  },
  {
    path: join(root, "crates", "openmesh-cli", "Cargo.toml"),
    extract: (content) => extractCargoPackageVersion(content),
  },
];

function extractCargoPackageVersion(content) {
  // Extract version from [package] section
  const packageMatch = content.match(/\[package\][\s\S]*?(?:(?:\r?\n\[)|$)/);
  if (!packageMatch) {
    throw new Error("No [package] section found");
  }
  const versionMatch = packageMatch[0].match(/version\s*=\s*"([^"]+)"/);
  if (!versionMatch) {
    throw new Error("No version field found in [package] section");
  }
  return versionMatch[1];
}

function main() {
  const expectedVersion = process.argv[2];
  const results = [];
  let hasError = false;

  for (const manifest of MANIFESTS) {
    const rel = relative(root, manifest.path);
    try {
      const content = readFileSync(manifest.path, "utf-8");
      const version = manifest.extract(content);
      results.push({ path: rel, version, ok: true });
    } catch (err) {
      results.push({ path: rel, version: `ERROR: ${err.message}`, ok: false });
      hasError = true;
    }
  }

  console.log("=== Version Consistency Check ===");
  for (const res of results) {
    console.log(`  ${res.path.padEnd(36)} : ${res.version}`);
  }

  if (hasError) {
    console.error("\nFAIL: One or more manifests could not be read.");
    process.exit(1);
  }

  const versions = results.map((r) => r.version);
  const firstVersion = versions[0];
  const allMatch = versions.every((v) => v === firstVersion);

  if (!allMatch) {
    console.error("\nFAIL: Manifest versions do not match each other!");
    process.exit(1);
  }

  if (expectedVersion && firstVersion !== expectedVersion) {
    console.error(
      `\nFAIL: Manifest versions (${firstVersion}) do not match expected version (${expectedVersion})!`,
    );
    process.exit(1);
  }

  console.log(`\nOK: All ${results.length} manifests consistent at version ${firstVersion}.`);
}

main();
