#!/usr/bin/env node
/* global process */
/**
 * Canonical Release Candidate verification script for OpenMesh v0.2.0-rc.2.
 *
 * Runs all quality and consistency gates in order:
 * 1. Version consistency check (all 5 manifests)
 * 2. Frontend verify (typecheck + lint + vitest + check:ipc + check:tauri-security + check:release-workflow)
 * 3. Rust formatting (cargo fmt --all -- --check)
 * 4. Rust workspace compilation (cargo check --workspace)
 * 5. Rust workspace test suite (cargo test --workspace --no-fail-fast)
 * 6. Playwright e2e suite (npm run test:e2e)
 * 7. Git diff check (git diff --check)
 */

import { execSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const STEPS = [
  { name: "Version consistency check", cmd: "node scripts/check-version-consistency.mjs 0.2.0-rc.2" },
  { name: "Frontend verification gate", cmd: "npm run verify" },
  { name: "Rust formatting check", cmd: "cargo fmt --all -- --check" },
  { name: "Rust workspace check", cmd: "cargo check --workspace" },
  { name: "Rust workspace tests", cmd: "cargo test --workspace --no-fail-fast" },
  { name: "Playwright e2e suite", cmd: "npm run test:e2e" },
  { name: "Git diff check", cmd: "git diff --check" },
];

console.log("==================================================");
console.log("  OpenMesh v0.2.0-rc.2 Release Candidate Verification");
console.log("==================================================\n");

let stepNumber = 1;
for (const step of STEPS) {
  console.log(`[${stepNumber}/${STEPS.length}] Running: ${step.name}...`);
  console.log(`$ ${step.cmd}\n`);
  const start = Date.now();
  try {
    execSync(step.cmd, {
      cwd: root,
      stdio: "inherit",
      env: process.env,
    });
    const elapsed = ((Date.now() - start) / 1000).toFixed(1);
    console.log(`\n--> PASS: ${step.name} (${elapsed}s)\n`);
  } catch (err) {
    console.error(`\n--> FAIL: ${step.name} failed with exit code ${err.status}`);
    process.exit(err.status || 1);
  }
  stepNumber++;
}

console.log("==================================================");
console.log("  ALL RELEASE CANDIDATE GATES PASSED (v0.2.0-rc.2)");
console.log("==================================================");
