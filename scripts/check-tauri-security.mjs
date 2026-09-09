#!/usr/bin/env node
/* global process */
/**
 * Static checks for A4.2 webview capability hardening.
 *
 * Production CSP must not be null and must not include 'unsafe-eval'.
 * OAuth runs in the system browser (`oauth_open_url` → `open::that`), so
 * provider origins are not added to the webview CSP.
 */
import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const failures = [];

function fail(message) {
  failures.push(message);
}

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

function cspText(csp) {
  if (csp == null) return "";
  if (typeof csp === "string") return csp;
  return Object.entries(csp)
    .map(([k, v]) => `${k} ${Array.isArray(v) ? v.join(" ") : v}`)
    .join("; ");
}

function directiveSources(csp, name) {
  if (csp == null || typeof csp !== "object") return "";
  const v = csp[name];
  if (Array.isArray(v)) return v.join(" ");
  return String(v ?? "");
}

const confPath = join(root, "src-tauri", "tauri.conf.json");
const conf = JSON.parse(readFileSync(confPath, "utf8"));
if (conf.app?.withGlobalTauri !== false) {
  fail("app.withGlobalTauri must be false");
}

const csp = conf.app?.security?.csp;
if (csp == null) {
  fail("app.security.csp must not be null");
} else {
  const text = cspText(csp);
  if (/\bunsafe-eval\b/i.test(text)) {
    fail("production CSP must not include 'unsafe-eval'");
  }
  if (/\bunsafe-inline\b/i.test(directiveSources(csp, "script-src"))) {
    fail("production script-src must not include 'unsafe-inline'");
  }
  const defaultSrc = directiveSources(csp, "default-src") || text;
  if (!/(?:^|\s)'self'(?:\s|$)/.test(defaultSrc) && !/\bself\b/.test(defaultSrc)) {
    fail("production CSP must include default-src 'self'");
  }
}

const devCsp = conf.app?.security?.devCsp;
if (devCsp == null) {
  fail("app.security.devCsp must be set so Vite HMR is not granted in production CSP");
}

const capPath = join(root, "src-tauri", "capabilities", "default.json");
const cap = JSON.parse(readFileSync(capPath, "utf8"));
const perms = Array.isArray(cap.permissions) ? cap.permissions : [];
for (const perm of perms) {
  const id = typeof perm === "string" ? perm : perm?.identifier;
  if (typeof id === "string" && (id === "fs" || id.startsWith("fs:"))) {
    fail(`capabilities/default.json must not grant filesystem plugin permission: ${id}`);
  }
  if (typeof id === "string" && (id === "shell" || id.startsWith("shell:"))) {
    fail(`capabilities/default.json must not grant shell plugin permission: ${id}`);
  }
}

const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const frontendDeps = {
  ...(pkg.dependencies ?? {}),
  ...(pkg.devDependencies ?? {}),
};
if (frontendDeps["@tauri-apps/plugin-fs"]) {
  fail("package.json must not depend on @tauri-apps/plugin-fs");
}
if (frontendDeps["@tauri-apps/plugin-shell"]) {
  fail("package.json must not depend on @tauri-apps/plugin-shell");
}

const cargoToml = readFileSync(join(root, "src-tauri", "Cargo.toml"), "utf8");
if (/tauri-plugin-fs/.test(cargoToml)) {
  fail("src-tauri/Cargo.toml must not depend on tauri-plugin-fs");
}

const libRs = readFileSync(join(root, "src-tauri", "src", "lib.rs"), "utf8");
if (/tauri_plugin_fs/.test(libRs)) {
  fail("src-tauri/src/lib.rs must not initialize tauri_plugin_fs");
}

const srcRoot = join(root, "src");
for (const file of walk(srcRoot)) {
  if (!/\.(ts|vue|js)$/.test(file)) continue;
  const text = readFileSync(file, "utf8");
  const rel = relative(root, file);
  if (/@tauri-apps\/plugin-fs/.test(text)) {
    fail(`${rel} imports @tauri-apps/plugin-fs`);
  }
  if (/@tauri-apps\/plugin-shell/.test(text)) {
    fail(`${rel} imports @tauri-apps/plugin-shell`);
  }
  if (/\b__TAURI__\b/.test(text) && !/\b__TAURI_INTERNALS__\b/.test(text)) {
    fail(`${rel} depends on window.__TAURI__ (withGlobalTauri) instead of __TAURI_INTERNALS__`);
  }
}

if (failures.length) {
  console.error("Tauri security check failed:");
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}

console.log("Tauri security check OK: CSP set, withGlobalTauri false, plugin-fs absent.");
