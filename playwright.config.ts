import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(fileURLToPath(import.meta.url));

/** Prefer a complete local Chromium (manual/ditto extract or playwright install). */
function resolveChromiumExecutable(): string | undefined {
  const candidates = [
    process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH,
    path.join(
      root,
      ".pw-browsers/chromium-1161/chrome-mac/Chromium.app/Contents/MacOS/Chromium",
    ),
    path.join(
      root,
      ".playwright-browsers/chromium-1161/chrome-mac/Chromium.app/Contents/MacOS/Chromium",
    ),
  ].filter((x): x is string => !!x);

  for (const candidate of candidates) {
    // Require Frameworks sibling so incomplete sandbox extracts are skipped.
    const frameworks = path.resolve(
      path.dirname(candidate),
      "../Frameworks/Chromium Framework.framework",
    );
    if (fs.existsSync(candidate) && fs.existsSync(frameworks)) {
      return candidate;
    }
  }
  return undefined;
}

const chromiumExecutable = resolveChromiumExecutable();

/**
 * Browser e2e against Vite (not full Tauri binary).
 * Tauri `invoke` fails outside the desktop shell; loadAll catches that and
 * the app still renders the project shell + empty states — enough for nav smoke.
 * Interactive contracts with mocked IPC live in vitest `tests/pages/*`.
 *
 * First-time browsers: `npx playwright install chromium`
 * (or extract Chromium into `.pw-browsers/`).
 */
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  workers: process.env.CI ? 1 : undefined,
  reporter: [["list"]],
  timeout: 60_000,
  use: {
    baseURL: "http://127.0.0.1:3000",
    trace: "on-first-retry",
    ...devices["Desktop Chrome"],
    // Full Chromium (not headless_shell) — set via env or local .pw-browsers.
    launchOptions: chromiumExecutable
      ? { executablePath: chromiumExecutable }
      : undefined,
  },
  webServer: {
    command: "npm run dev -- --host 127.0.0.1 --port 3000",
    url: "http://127.0.0.1:3000",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
