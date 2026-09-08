#!/usr/bin/env node
/* global process */
/**
 * Fail if the Vue/TS frontend invokes a Tauri command that is not registered
 * in src-tauri generate_handler!, or if generate_handler! lists the same
 * command twice.
 *
 * Registered-but-never-invoked commands are reported as warnings only
 * (desktop-only / future surfaces).
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const tauriSrc = join(root, "src-tauri", "src");
const frontendSrc = join(root, "src");

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

function extractFrontendInvokes(files) {
  const invokeRe = /\binvoke(?:<[^>]*>)?\(\s*(['"`])([A-Za-z_][A-Za-z0-9_]*)\1/g;
  /** @type {Map<string, Set<string>>} */
  const byCommand = new Map();
  for (const file of files) {
    if (!/\.(ts|vue|js)$/.test(file)) continue;
    const text = readFileSync(file, "utf8");
    invokeRe.lastIndex = 0;
    let m;
    while ((m = invokeRe.exec(text))) {
      const name = m[2];
      if (!byCommand.has(name)) byCommand.set(name, new Set());
      byCommand.get(name).add(relative(root, file));
    }
  }
  return byCommand;
}

const libRs = readFileSync(join(tauriSrc, "lib.rs"), "utf8");
const registered = extractHandlerCommands(libRs);
const registeredSet = new Set(registered);

const dupes = registered.filter((name, i) => registered.indexOf(name) !== i);
const uniqueDupes = [...new Set(dupes)];

const frontendFiles = walk(frontendSrc);
const invokes = extractFrontendInvokes(frontendFiles);

const missing = [...invokes.keys()].filter((name) => !registeredSet.has(name)).sort();
const unused = registered.filter((name) => !invokes.has(name));

let failed = false;

if (uniqueDupes.length) {
  failed = true;
  console.error("Duplicate generate_handler! command names:");
  for (const name of uniqueDupes) console.error(`  ${name}`);
}

if (missing.length) {
  failed = true;
  console.error("Frontend invoke() names with no registered Tauri command:");
  for (const name of missing) {
    const files = [...invokes.get(name)].sort().join(", ");
    console.error(`  ${name}  (${files})`);
  }
}

if (unused.length) {
  console.log(`Registered commands never invoked from src/ (${unused.length}):`);
  for (const name of unused) console.log(`  ${name}`);
}

if (failed) {
  process.exit(1);
}

console.log(
  `IPC contract OK: ${registeredSet.size} registered commands, ${invokes.size} frontend invoke names.`,
);
