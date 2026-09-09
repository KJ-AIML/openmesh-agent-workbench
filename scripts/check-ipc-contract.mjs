#!/usr/bin/env node
/* global process */
/**
 * IPC identity + typed-catalog contract (A8).
 *
 * Fails when:
 * - generate_handler! has duplicate names
 * - frontend invoke/legacyInvoke/invokeTyped string names are unregistered
 * - typed catalog name is not registered
 * - a name is both typed and legacy
 * - @tauri-apps/api/core `invoke` is imported outside src/lib/ipc/client.ts
 *
 * Does not fail merely because legacy or unused registered commands remain.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const tauriSrc = join(root, "src-tauri", "src");
const frontendSrc = join(root, "src");
const catalogPath = join(root, "src/lib/ipc/catalog.ts");
const allowedCoreInvokeImport = join(root, "src/lib/ipc/client.ts");

function walk(dir, acc = []) {
  for (const entry of readdirSync(dir)) {
    if (entry === "node_modules" || entry === "target" || entry === "gen") continue;
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) walk(full, acc);
    else acc.push(full);
  }
  return acc;
}

function extractHandlerCommands(libRs) {
  const start = libRs.indexOf("tauri::generate_handler![");
  if (start < 0) {
    throw new Error("src-tauri/src/lib.rs does not contain tauri::generate_handler![");
  }
  const open = libRs.indexOf("[", start);
  let depth = 0;
  let end = -1;
  for (let i = open; i < libRs.length; i++) {
    const ch = libRs[i];
    if (ch === "[") depth++;
    else if (ch === "]") {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  if (end < 0) throw new Error("unterminated generate_handler! in lib.rs");
  const body = libRs.slice(open + 1, end);
  const names = [];
  for (const raw of body.split(",")) {
    const token = raw.replace(/\/\/.*$/gm, "").replace(/\s+/g, "").trim();
    if (!token) continue;
    const name = token.split("::").pop();
    if (!name || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(name)) {
      throw new Error(`unparseable generate_handler entry: ${raw.trim()}`);
    }
    names.push(name);
  }
  return names;
}

function extractCallNames(files, fnNames) {
  const fn = fnNames.join("|");
  const re = new RegExp(
    "\\b(?:" + fn + ")(?:<[^>]*>)?\\(\\s*(['\"`])([A-Za-z_][A-Za-z0-9_]*)\\1",
    "g",
  );
  /** @type {Map<string, Set<string>>} */
  const byCommand = new Map();
  for (const file of files) {
    if (!/\.(ts|vue|js)$/.test(file)) continue;
    const text = readFileSync(file, "utf8");
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(text))) {
      const name = m[2];
      if (!byCommand.has(name)) byCommand.set(name, new Set());
      byCommand.get(name).add(relative(root, file));
    }
  }
  return byCommand;
}

function extractTypedCatalog(source) {
  const start = source.indexOf("export const TYPED_COMMAND_NAMES = [");
  if (start < 0) throw new Error("TYPED_COMMAND_NAMES not found in catalog.ts");
  const open = source.indexOf("[", start);
  const end = source.indexOf("] as const", open);
  if (end < 0) throw new Error("unterminated TYPED_COMMAND_NAMES");
  const body = source.slice(open + 1, end);
  const names = [];
  for (const raw of body.split(",")) {
    const m = raw.match(/"([A-Za-z_][A-Za-z0-9_]*)"/);
    if (m) names.push(m[1]);
  }
  return names;
}

function extractCoreInvokeImports(files) {
  const re =
    /import\s*\{[^}]*\binvoke\b[^}]*\}\s*from\s*["']@tauri-apps\/api\/core["']/;
  const hits = [];
  for (const file of files) {
    if (!/\.(ts|vue|js)$/.test(file)) continue;
    const text = readFileSync(file, "utf8");
    if (re.test(text) && file !== allowedCoreInvokeImport) {
      hits.push(relative(root, file));
    }
  }
  return hits;
}

const libRs = readFileSync(join(tauriSrc, "lib.rs"), "utf8");
const registered = extractHandlerCommands(libRs);
const registeredSet = new Set(registered);

const dupes = registered.filter((name, i) => registered.indexOf(name) !== i);
const uniqueDupes = [...new Set(dupes)];

const frontendFiles = walk(frontendSrc);
const invokeCalls = extractCallNames(frontendFiles, ["invoke", "invokeTyped"]);
const legacyCalls = extractCallNames(frontendFiles, ["legacyInvoke"]);
const typedCatalog = extractTypedCatalog(readFileSync(catalogPath, "utf8"));
const typedSet = new Set(typedCatalog);
const forbiddenImports = extractCoreInvokeImports(frontendFiles);

const catalogDupes = typedCatalog.filter(
  (name, i) => typedCatalog.indexOf(name) !== i,
);
const uniqueCatalogDupes = [...new Set(catalogDupes)];

const typedUnregistered = typedCatalog.filter((name) => !registeredSet.has(name));
const overlap = [...typedSet].filter((name) => legacyCalls.has(name));

const frontendNames = new Set([...invokeCalls.keys(), ...legacyCalls.keys()]);
const missing = [...frontendNames].filter((name) => !registeredSet.has(name)).sort();

const unused = registered.filter((name) => !frontendNames.has(name) && !typedSet.has(name));

let failed = false;

if (uniqueDupes.length) {
  failed = true;
  console.error("Duplicate generate_handler! command names:");
  for (const name of uniqueDupes) console.error(`  ${name}`);
}

if (uniqueCatalogDupes.length) {
  failed = true;
  console.error("Duplicate typed IPC catalog identities:");
  for (const name of uniqueCatalogDupes) console.error(`  ${name}`);
}

if (typedUnregistered.length) {
  failed = true;
  console.error("Typed catalog commands with no registered Tauri handler:");
  for (const name of typedUnregistered) console.error(`  ${name}`);
}

if (overlap.length) {
  failed = true;
  console.error("Commands present in both typed catalog and legacyInvoke:");
  for (const name of overlap) console.error(`  ${name}`);
}

if (forbiddenImports.length) {
  failed = true;
  console.error(
    "Raw @tauri-apps/api/core invoke import outside src/lib/ipc/client.ts:",
  );
  for (const file of forbiddenImports.sort()) console.error(`  ${file}`);
}

if (missing.length) {
  failed = true;
  console.error("Frontend invoke names with no registered Tauri command:");
  for (const name of missing) {
    const files = [
      ...(invokeCalls.get(name) ?? []),
      ...(legacyCalls.get(name) ?? []),
    ]
      .sort()
      .join(", ");
    console.error(`  ${name}  (${files})`);
  }
}

if (unused.length) {
  console.log(`Registered commands never invoked from src/ (${unused.length}):`);
  for (const name of unused) console.log(`  ${name}`);
}

const legacyCount = [...legacyCalls.keys()].filter((n) => !typedSet.has(n)).length;
console.log(
  `IPC contract: registered=${registeredSet.size} typed=${typedSet.size} legacy=${legacyCount} unused=${unused.length} unknown=${missing.length} duplicates=${uniqueDupes.length + uniqueCatalogDupes.length}`,
);

if (failed) {
  process.exit(1);
}

console.log(
  `IPC contract OK: ${registeredSet.size} registered commands, ${frontendNames.size} frontend invoke names, ${typedSet.size} typed.`,
);
