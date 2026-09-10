import { test, expect } from "./fixtures";

test.describe("Secondary browser feature coverage", () => {
  test("covers provider reorder guards, cancelled mutations, empty sections, and failures", async ({ openMesh, page }) => {
    await openMesh("/proxy-providers", { mockTauri: true });
    const recordNames = () => page.locator(".proxy-providers-page__record-title strong").allTextContents();
    await expect.poll(recordNames).toEqual(["browser-upstream-a", "browser-upstream-b"]);
    await expect(page.getByText("Key configured", { exact: true }).first()).toBeVisible();

    page.once("dialog", (dialog) => void dialog.dismiss());
    await page.locator("#routing-strategy").selectOption("fill-first");
    await expect(page.locator("#routing-strategy")).toHaveValue("round-robin");

    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-empty" });
    await expect(page.getByText("No upstreams configured", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Add upstream" }).first()).toBeEnabled();

    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-mutation-error" });
    page.once("dialog", (dialog) => void dialog.accept());
    await page.getByRole("button", { name: "Add upstream" }).first().click();
    const dialog = page.locator(".proxy-providers-page__dialog");
    await dialog.getByLabel("Upstream id").fill("failed-upstream");
    await dialog.getByRole("textbox", { name: /API key/ }).fill("write-only-key");
    await dialog.getByLabel("Base URL").fill("https://failed.example.com/v1");
    await dialog.getByLabel("Models").fill("failed-model");
    await dialog.getByRole("button", { name: "Save upstream" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "Unable to update OpenMesh built-in proxy configuration.",
    );
    await expect(page.getByText("write-only-key", { exact: true })).toHaveCount(0);
  });

  test("covers runtime status variants, safe redaction, empty metadata, and every navigation link", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-unavailable" });
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Runtime error");
    await expect(page.getByText("The OpenMesh built-in proxy reported an error.")).toBeVisible();
    await expect(page.getByText("built-in proxy is unavailable")).toHaveCount(0);

    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-empty" });
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Running");
    await expect(page.getByText("0", { exact: true }).first()).toBeVisible();

    await openMesh("/proxy-runtime", { mockTauri: true });
    const runtimeCalls = () => page.evaluate(() =>
      (window as unknown as { __OPENMESH_RUNTIME_COMMANDS__?: string[] })
        .__OPENMESH_RUNTIME_COMMANDS__ ?? [],
    );
    const initialRuntimeCalls = await runtimeCalls();
    await page.getByRole("button", { name: "Refresh", exact: true }).click();
    await expect.poll(runtimeCalls).toHaveLength(initialRuntimeCalls.length + 1);
    await page.getByRole("link", { name: "Open Provider settings" }).click();
    await expect(page).toHaveURL(/\/settings\?section=provider$/);
  });

  test("covers Settings provider failures, server persistence, invalid paths, voice controls, and seeded extensions", async ({ openMesh, page }) => {
    await openMesh("/settings?section=provider", { mockTauri: true, scenario: "agent-provider-error" });
    await page.getByPlaceholder("openai · deepseek · xai").fill("browser-provider");
    await page.getByRole("button", { name: "Test connection" }).click();
    await expect(page.getByRole("paragraph").filter({ hasText: "Connection failed" })).toBeVisible();
    await expect(page.getByText("provider probe rejected the request", { exact: true })).toBeVisible();

    await openMesh("/settings?section=provider", { mockTauri: true, scenario: "agent-secret-error" });
    await page.getByPlaceholder("sk-…").fill("browser-secret");
    await page.getByRole("button", { name: "Save Key" }).click();
    await expect(page.getByText("secret store unavailable", { exact: true })).toBeVisible();

    await openMesh("/settings?section=server", { mockTauri: true });
    await page.getByRole("tab", { name: "Local tools", exact: true }).click();
    await page.getByRole("tab", { name: "Server", exact: true }).click();
    const serverPanel = page.locator(".workbench-card:visible").filter({ hasText: "API Base URL" });
    await serverPanel.locator("input").first().fill("http://127.0.0.1:41778/v1");
    await page.getByRole("button", { name: "Save Server" }).click();
    await expect(page.getByText("server saved", { exact: true })).toBeVisible();
    await page.getByRole("spinbutton").fill("9001");
    await page.getByRole("button", { name: "Save Built-in Proxy Settings" }).click();
    await expect(page.getByText("Built-in proxy settings saved", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Stop proxy" }).click();
    await expect(page.getByText("Built-in proxy stopped", { exact: true })).toBeVisible();

    await openMesh("/settings?section=sessions", { mockTauri: true, scenario: "path-invalid" });
    await page.getByText("Enable Codex Session Scanning").click();
    const codexPath = page.getByPlaceholder("~/.codex/sessions");
    await codexPath.fill("/tmp/missing-codex");
    await codexPath.locator("xpath=..").getByRole("button", { name: "Validate" }).click();
    await expect(codexPath.locator("xpath=../following-sibling::p")).toContainText("✗ Invalid");

    await openMesh("/settings?section=voice", { mockTauri: true });
    const bargeIn = page.locator("label.voice-settings__row").filter({ hasText: "Barge-in" }).getByRole("checkbox");
    await bargeIn.check();
    await expect(bargeIn).toBeChecked();
    await page.locator("label.voice-settings__row").filter({ hasText: "Listen mode" }).locator("select").selectOption("ptt");
    await expect(page.getByText("Tiny STT", { exact: true })).toBeVisible();
    await expect(page.getByText("Cloud STT available", { exact: true })).toBeVisible();

    await openMesh("/settings?section=extensions", { mockTauri: true, scenario: "extensions-seeded" });
    await expect(page.getByText("Browser Skill", { exact: true })).toBeVisible();
    const skillRow = page.locator(".ext-row").filter({ hasText: "Browser Skill" });
    await skillRow.getByRole("checkbox").uncheck();
    await expect(skillRow.getByText("Off", { exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "Hooks" }).click();
    await expect(page.getByText("browser-hook", { exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "Plugins" }).click();
    await expect(page.getByText("Browser Plugin", { exact: true })).toBeVisible();
    await page.getByRole("tab", { name: "Marketplace" }).click();
    await expect(page.getByText("Browser Catalog Skill", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Install from folder" }).click();
    await expect(page.getByText("Installed browser-installed-skill", { exact: true })).toBeVisible();
    await expect(page.getByText("Browser Installed Skill", { exact: true })).toBeVisible();
  });

  test("covers appearance variants, last-tab protection, and titlebar route fallback", async ({ openMesh, page }) => {
    await openMesh("/settings?section=appearance", { mockTauri: true });
    const theme = page.getByRole("radiogroup", { name: "Theme" });
    for (const name of ["Dark", "System", "Light"]) {
      await theme.getByRole("radio", { name }).click();
      await expect(theme.getByRole("radio", { name })).toHaveAttribute("aria-checked", "true");
    }

    const fontSize = page.getByRole("radiogroup", { name: "Font size" });
    for (const name of ["Small", "Medium", "Large"]) {
      await fontSize.getByRole("radio", { name }).click();
      await expect(fontSize.getByRole("radio", { name })).toHaveAttribute("aria-checked", "true");
    }

    const density = page.getByRole("radiogroup", { name: "Density" });
    await density.getByRole("radio", { name: "Compact" }).click();
    await expect(density.getByRole("radio", { name: "Compact" })).toHaveAttribute("aria-checked", "true");
    await density.getByRole("radio", { name: "Comfortable" }).click();

    const topTabs = page.getByTestId("appearance-top-navbar-tabs");
    await topTabs.getByRole("button", { name: "Chat" }).click();
    await topTabs.getByRole("button", { name: "Docs" }).click();
    await topTabs.getByRole("button", { name: "Sprint" }).click();
    const workTab = topTabs.getByRole("button", { name: "Work" });
    await expect(workTab).toHaveAttribute("aria-pressed", "true");
    await expect(workTab).toBeDisabled();

    await page.goto("/docs");
    await expect(page.locator(".shell")).toBeVisible();
    await expect(page).toHaveURL(/\/$/);
  });

  test("covers update up-to-date, installer selection, and browser install fallback", async ({ openMesh, page }) => {
    const release = (tag: string, body: string, assets: unknown[]) => ({
      tag_name: tag,
      name: `Browser ${tag}`,
      html_url: `https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/${tag}`,
      published_at: "2026-08-22T00:00:00.000Z",
      body,
      draft: false,
      prerelease: false,
      assets,
    });

    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(release("v0.2.0-rc.1", "Current browser release", [])),
      }),
    );
    await openMesh("/settings?section=about", { mockTauri: true });
    await page.getByRole("button", { name: "Check for updates" }).click();
    await expect(page.getByText(/You’re on the latest release/)).toBeVisible({ timeout: 8_000 });

    await page.unroute("**/api.github.com/**");
    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(
          release("v0.2.1", "Installer browser release", [
            { name: "OpenMesh_0.2.1_aarch64.dmg", browser_download_url: "https://example.test/openmesh.dmg", size: 10 },
            { name: "OpenMesh_0.2.1_x64-setup.exe", browser_download_url: "https://example.test/openmesh.exe", size: 10 },
            { name: "OpenMesh_0.2.1.AppImage", browser_download_url: "https://example.test/openmesh.AppImage", size: 10 },
            { name: "OpenMesh_0.2.1.deb", browser_download_url: "https://example.test/openmesh.deb", size: 10 },
          ]),
        ),
      }),
    );
    await page.evaluate(() => localStorage.removeItem("openmesh.updateCheck.v1"));
    await openMesh("/settings?section=about", { mockTauri: true });
    await page.getByRole("button", { name: "Check for updates" }).click();
    await expect(page.getByText("Update available").first()).toBeVisible({ timeout: 8_000 });
    await expect(page.getByText(/Installer for this machine:/)).toBeVisible();
    await page.getByRole("button", { name: "Download & install" }).click();
    await expect(page.getByRole("button", { name: "Installer opened" })).toBeVisible();
    await expect(page.getByText("Browser fixture installer opened.", { exact: true })).toBeVisible();
  });

  test("covers command failure and project creation failure recovery", async ({ openMesh, page }) => {
    await openMesh("/settings?section=tools", { mockTauri: true, scenario: "command-error" });
    await page.getByPlaceholder("Preset name").fill("Failing preset");
    await page.getByPlaceholder("Command").fill("npm");
    await page.getByPlaceholder("Args (space-separated)").fill("run failure");
    await page.getByRole("button", { name: "Add preset" }).click();
    await page.locator('button[title="Run"]').last().click();
    await expect(page.getByText("command launch failed", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Open Terminal" }).click();
    await expect(page.getByText("command launch failed", { exact: true })).toBeVisible();

    await openMesh("/projects/new", { mockTauri: true });
    await page.getByPlaceholder("e.g., OpenMesh").fill("Cancelled Project");
    await page.getByPlaceholder(/C:\\KJ\\Repos/).fill("/tmp/cancelled-project");
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();

    await openMesh("/projects/new", { mockTauri: true, scenario: "project-error" });
    await page.getByPlaceholder("e.g., OpenMesh").fill("Failed Project");
    await page.getByPlaceholder(/C:\\KJ\\Repos/).fill("/tmp/failed-project");
    page.once("dialog", (dialog) => {
      expect(dialog.type()).toBe("alert");
      expect(dialog.message()).toContain("Failed to create project");
      void dialog.accept();
    });
    await page.getByRole("button", { name: "Save Project" }).click();
    await expect(page).toHaveURL(/\/projects\/new$/);
  });

  test("falls back safely when a foreign transcript cannot be read", async ({ openMesh, page }) => {
    await openMesh("/agent-sessions", { mockTauri: true, scenario: "transcript-error" });
    await page.getByRole("button", { name: /Scanned browser session/ }).click();
    await page.getByRole("button", { name: "Continue in Chat" }).click();
    await page.getByRole("button", { name: "Import full & continue" }).click();
    await expect(page).toHaveURL(/\/agent-chat\?chat=/);
    await expect(page.getByText("transcript unavailable", { exact: true })).toHaveCount(0);
  });

  test("covers mixed agent session filters, query preselection, saved deletion, and no-project state", async ({ openMesh, page }) => {
    await openMesh("/agent-sessions?session=scan-grok-session", { mockTauri: true, scenario: "mixed-sessions" });
    await expect(page.getByRole("heading", { name: "Scanned grok session", exact: true })).toBeVisible();
    await expect(page.getByText(/Path: .*grok\/scanned\.jsonl/)).toBeVisible();

    for (const [label, tool] of [
      ["codex Codex", "codex"],
      ["claude Claude", "claude"],
      ["opencode OpenCode", "opencode"],
      ["cursor Cursor", "cursor"],
      ["gemini Gemini", "gemini"],
      ["grok Grok", "grok"],
    ] as const) {
      await page.getByRole("button", { name: label, exact: true }).click();
      const scannedRow = page.locator("button.w-full").filter({ hasText: `Scanned ${tool} session` });
      await expect(scannedRow).toBeVisible();
      const otherTool = tool === "codex" ? "claude" : "codex";
      await expect(page.locator("button.w-full").filter({ hasText: `Scanned ${otherTool} session` })).toHaveCount(0);
    }

    await page.getByRole("button", { name: "All", exact: true }).click();
    await page.getByRole("button", { name: /Scanned claude session/ }).click();
    await page.getByRole("button", { name: "⭐ Star" }).click();
    await expect(page.getByRole("button", { name: "⭐ Unstar" })).toBeVisible();
    await page.getByRole("button", { name: "⭐ Unstar" }).click();
    await page.getByRole("button", { name: "Resume in terminal" }).click();
    await expect(page.getByText("Resume is available for Codex, Claude, and OpenCode when a project is open.")).toHaveCount(0);
    await page.getByRole("button", { name: "All", exact: true }).click();
    await page.getByRole("button", { name: /Saved codex session/ }).click();
    const detail = page.locator(".workbench-card-compact").filter({ hasText: "Changed files:" });
    await expect(detail).toBeVisible();
    page.once("dialog", (dialog) => void dialog.dismiss());
    await detail.getByRole("button").last().click();
    await expect(page.getByRole("heading", { name: "Saved codex session", exact: true })).toBeVisible();
    page.once("dialog", (dialog) => void dialog.accept());
    await detail.getByRole("button").last().click();
    await expect(page.getByRole("heading", { name: "Saved codex session", exact: true })).toHaveCount(0);

    await openMesh("/agent-sessions", { mockTauri: true, scenario: "empty" });
    await expect(page.getByText("No agent sessions yet", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh", exact: true })).toBeDisabled();
  });
});
