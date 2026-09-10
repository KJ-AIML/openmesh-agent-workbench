import { test, expect } from "./fixtures";

test.describe("Settings and Continuity", () => {
  test("navigates every Settings group and saves provider, runtime, paths, appearance, and data controls", async ({ openMesh, page }) => {
    await openMesh("/settings", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();

    await page.getByRole("tab", { name: "Provider" }).click();
    await page.getByPlaceholder("openai · deepseek · xai").fill("browser-provider");
    await page.locator('input[placeholder="e.g., gpt-4o-mini"]').first().fill("browser-model");
    await page.getByRole("button", { name: "Test connection" }).click();
    await expect(page.getByText("Connection OK")).toBeVisible();
    await page.getByRole("button", { name: "Save Provider & Models" }).click();
    await expect(page.getByText("Provider & models saved", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Voice" }).click();
    await expect(page.getByRole("heading", { name: "Voice" })).toBeVisible();
    await page.locator(".voice-settings__select").first().selectOption("openai/whisper-1");
    await page.getByPlaceholder(/Custom OpenRouter slug/).fill("custom/stt");
    await page.getByRole("button", { name: "Use" }).click();
    await page.locator(".voice-settings__select").nth(1).selectOption("th");
    await page.locator("label.voice-settings__row").filter({ hasText: "Listen mode" }).locator("select").selectOption("ptt");
    await page.locator("label.voice-settings__row").filter({ hasText: "Speak replies" }).locator("input").check();

    await page.getByRole("tab", { name: "Local tools" }).click();
    await page.getByRole("tab", { name: "Agents" }).click();
    await page.getByPlaceholder(/default: codex/).fill("codex-browser");
    await page.getByRole("button", { name: "Validate" }).first().click();
    await expect(
      page
        .locator('input[placeholder="Leave empty to use default: codex"]')
        .locator("xpath=../following-sibling::p"),
    ).toContainText("✓ Valid");
    await page.getByRole("button", { name: "Save Agent CLIs" }).click();
    await expect(page.getByText("agentClis saved", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Extensions" }).click();
    await expect(page.getByText("Skills · Hooks · Plugins")).toBeVisible();
    await page.getByRole("tab", { name: "Hooks" }).click();
    await expect(page.getByText("No hooks yet")).toBeVisible();
    await page.getByRole("tab", { name: "Plugins" }).click();
    await expect(page.getByText("No plugins installed")).toBeVisible();
    await page.getByRole("tab", { name: "Marketplace" }).click();
    await expect(page.getByText("Local OpenMesh catalog")).toBeVisible();

    await page.getByRole("tab", { name: "Sessions" }).click();
    await page.getByText("Enable Codex Session Scanning").click();
    await page.getByPlaceholder("~/.codex/sessions").fill("/tmp/codex-sessions");
    await page.getByRole("button", { name: "Validate" }).click();
    await expect(
      page
        .locator('input[placeholder="~/.codex/sessions"]')
        .locator("xpath=../following-sibling::p"),
    ).toContainText("✓ Valid");
    await page.getByRole("button", { name: "Save Session Directories" }).click();

    await page.getByRole("tab", { name: "Server" }).click();
    await page.getByRole("button", { name: "Check", exact: true }).click();
    const serverPanel = page.locator(".workbench-card").filter({ hasText: "API Base URL" });
    await expect(serverPanel.locator(".badge").first()).toHaveText(/healthy|unreachable/);
    const builtInProxyPanel = page.locator(".workbench-card").filter({ hasText: "OpenMesh Built-in Proxy" });
    await page.getByRole("spinbutton").fill("9001");
    await builtInProxyPanel.getByRole("button", { name: "Save Built-in Proxy Settings" }).click();
    await expect(page.getByText("Built-in proxy settings saved", { exact: true })).toBeVisible();
    await builtInProxyPanel.getByRole("button", { name: "Stop proxy", exact: true }).click();
    await expect(builtInProxyPanel.getByText("Stopped", { exact: true })).toBeVisible();
    await builtInProxyPanel.getByRole("button", { name: "Start proxy", exact: true }).click();
    await expect(builtInProxyPanel.getByText("Running", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Project" }).click();
    await page.getByRole("tab", { name: "Tools", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Command presets", exact: true })).toBeVisible();
    await page.getByPlaceholder("Preset name").fill("Browser preset");
    await page.getByPlaceholder("Command").fill("npm");
    await page.getByPlaceholder("Args (space-separated)").fill("run test");
    await page.getByRole("button", { name: "Add preset" }).click();
    await expect(page.getByText("Browser preset")).toBeVisible();
    await page.locator('button[title="Run"]').last().click();
    await expect(page.getByText("Ran: Browser preset", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Paths" }).click();
    await page.getByPlaceholder("C:\\KJ\\Repos").fill("/tmp/projects");
    await page.getByRole("button", { name: "Validate" }).click();
    const pathsPanel = page.locator(".workbench-card").filter({ hasText: "Default Projects Directory" });
    await expect(pathsPanel.locator("p").filter({ hasText: "✓ Valid" })).toBeVisible();
    await page.getByRole("button", { name: "Save Paths" }).click();

    await page.getByRole("tab", { name: "App" }).click();
    await page.getByRole("tab", { name: "Appearance" }).click();
    await page.getByTestId("appearance-preview").isVisible();
    await page.getByRole("radio", { name: "Light" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page.getByRole("radio", { name: "Large" }).click();
    await page.getByRole("radio", { name: "Compact" }).click();
    await page.getByTestId("appearance-top-navbar-tabs").getByRole("button", { name: "Sprint" }).click();
    await expect(page.getByRole("status")).toContainText("Saved");

    await page.getByRole("tab", { name: "Data" }).click();
    await expect(page.getByText("Local filesystem only")).toBeVisible();
    await page.getByRole("button", { name: "Export Project" }).click();
    await expect(page.getByText("Project exported", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Import Data" }).click();
    await expect(page.getByText("Import not yet implemented for file-based storage", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "About" }).click();
    await expect(page.getByText(/About & updates/)).toBeVisible();
    await page.getByRole("button", { name: "Check for updates" }).click();
  });

  test("covers Continuity pending, digest, mesh, team, trust, LAN, chat, proxy, relay, connectors, org, pilot, and RC", async ({ openMesh, page }) => {
    await openMesh("/continuity", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Pending & LAN" })).toBeVisible();
    await expect(page.getByText("Review browser continuity")).toBeVisible();

    await page.getByRole("tab", { name: "Digest" }).click();
    await expect(page.getByText("Browser continuity digest")).toBeVisible();
    await page.getByRole("spinbutton").fill("48");
    await page.getByRole("spinbutton").press("Enter");

    await page.locator('[role="tablist"][aria-label="Section groups"]').getByRole("tab", { name: "Advanced" }).click();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Peers" }).click();
    await page.getByRole("textbox", { name: "Label (e.g. Yo)", exact: true }).fill("Second peer");
    await page.getByPlaceholder(/LAN host:port/).fill("127.0.0.1:41778");
    await page.getByPlaceholder("Notes (optional)").fill("Browser peer note");
    await page.getByRole("button", { name: "Add peer" }).click();
    await expect(page.getByText("Second peer")).toBeVisible();
    await page.getByText("Second peer", { exact: true }).click();
    await expect(page.getByPlaceholder(/Peer id or label/)).toHaveValue(/peer-/);
    await page.getByPlaceholder(/Peer id or label/).fill("peer-");
    await page.getByPlaceholder(/What did Yo finish/).fill("What changed?");
    await page.getByRole("button", { name: "Query peer" }).click();
    await expect(page.getByText("Offline peer answer")).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Team" }).click();
    await page.getByPlaceholder("Team display name").fill("Browser team");
    await page.getByRole("button", { name: "Initialize team" }).click();
    await expect(page.getByText("Browser team")).toBeVisible();
    await page.getByPlaceholder("Member label").fill("Browser member");
    await page.getByRole("button", { name: "Add member" }).click();
    await expect(page.getByText("Browser member")).toBeVisible();
    const memberRow = page.locator("li").filter({ hasText: "Browser member" });
    await memberRow.getByRole("button", { name: "Remove" }).click();
    await expect(page.getByText("Browser member")).toHaveCount(0);

    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Trust" }).click();
    await expect(page.getByRole("heading", { name: "Initialize trust" })).toBeVisible();
    await page.getByRole("button", { name: "Initialize trust policy" }).click();
    await expect(page.getByText("Trust this peer on LAN")).toBeVisible();
    await page.getByRole("button", { name: "Enable remote query" }).click();
    await page.getByRole("button", { name: "allowlist-only" }).click();
    await page.getByPlaceholder("Or paste mesh peer id").fill("peer-browser");
    await page.getByRole("button", { name: "Trust this peer" }).click();
    await expect(page.getByText(/allowlist 1/)).toBeVisible();
    await page.getByRole("button", { name: "Remove" }).click();
    await expect(page.getByText(/allowlist 0/)).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Section groups"]').getByRole("tab", { name: "Share" }).click();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "LAN" }).click();
    await page.getByRole("button", { name: "Start listener" }).click();
    await expect(page.getByText("Listening")).toBeVisible();
    await page.getByRole("button", { name: "Stop" }).click();
    await expect(page.getByText("Listener is stopped")).toBeVisible();
    await page.getByRole("button", { name: "Refresh discover" }).click();
    await expect(page.getByText("Browser peer")).toBeVisible();
    await page.getByRole("button", { name: "Copy pack+approve commands" }).click();
    await page.getByRole("textbox", { name: "192.168.1.20:41778", exact: true }).fill("127.0.0.1:41778");
    await page.getByRole("button", { name: "Probe" }).click();
    await expect(page.locator("span.badge").filter({ hasText: /^live$/ }).first()).toBeVisible();
    await page.getByPlaceholder("Package id (approved)").fill("package-browser-1");
    await page.getByRole("textbox", { name: "Peer host:port", exact: true }).fill("127.0.0.1:41778");
    await page.getByRole("button", { name: "Send over LAN" }).click();
    await page.getByPlaceholder(/What is in progress/).fill("What is live?");
    await page.getByRole("button", { name: "Ask peer" }).click();
    await expect(page.getByText("Live fixture answer")).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Chat" }).click();
    await page.getByRole("textbox", { name: "Peer host:port", exact: true }).fill("127.0.0.1:41778");
    await page.getByPlaceholder("Message…").fill("Hello peer");
    await page.getByRole("button", { name: "Send" }).click();
    await expect(page.getByText("Hello peer")).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Relay" }).click();
    await expect(page.getByText("Audit trail")).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Section groups"]').getByRole("tab", { name: "Advanced" }).click();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Proxy" }).click();
    await page.getByRole("button", { name: "Initialize Continuity Proxy" }).click();
    await expect(page.getByText("proxy-browser")).toBeVisible();
    await page.getByPlaceholder(/What needs attention/).fill("What needs attention?");
    await page.getByRole("button", { name: "Live ask" }).click();
    await expect(page.getByText("Browser continuity answer")).toBeVisible();

    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Connectors" }).click();
    await expect(page.getByText("Browser fixture")).toBeVisible();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Org" }).click();
    await expect(page.getByRole("heading", { name: "Nodes", exact: true })).toBeVisible();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "Pilot" }).click();
    await expect(page.getByText(/pilot ready/)).toBeVisible();
    await page.locator('[role="tablist"][aria-label="Views"]').getByRole("tab", { name: "RC" }).click();
    await expect(page.getByText(/rc ready/)).toBeVisible();
  });

  test("keeps Settings and Continuity safe without a selected project", async ({ openMesh, page }) => {
    await openMesh("/settings?section=provider", { mockTauri: true, scenario: "empty" });
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await page.getByRole("tab", { name: "Project" }).click();
    await page.getByRole("tab", { name: "Paths" }).click();
    await expect(page.getByText("Local Paths")).toBeVisible();

    await openMesh("/continuity", { mockTauri: true, scenario: "empty" });
    await expect(page.getByText("No project selected")).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh" })).toBeDisabled();
  });

  test("covers provider secret lifecycle, invalid built-in proxy input, and every session directory toggle", async ({ openMesh, page }) => {
    await openMesh("/settings?section=provider", { mockTauri: true });
    await page.getByPlaceholder("openai · deepseek · xai").fill("browser-direct");
    await page.getByPlaceholder("sk-…").fill("browser-direct-secret");
    await page.getByRole("button", { name: "Save Key" }).click();
    const providerPanel = page.locator(".workbench-card").filter({ hasText: "Provider & Models" });
    await expect(providerPanel.getByText("Configured", { exact: true })).toBeVisible();
    await expect(page.getByText(/API key saved/)).toBeVisible();
    await page.getByRole("button", { name: "Change" }).click();
    await expect(page.getByPlaceholder("sk-…")).toBeVisible();

    await page.getByRole("tab", { name: "Local tools" }).click();
    await page.getByRole("tab", { name: "Server" }).click();
    await page.getByRole("spinbutton").fill("0");
    await page.getByRole("button", { name: "Save Built-in Proxy Settings" }).click();
    await expect(
      page.getByText("Built-in proxy port must be between 1 and 65535", { exact: true }),
    ).toBeVisible();

    await page.getByRole("tab", { name: "Sessions" }).click();
    const sessionDirectories = [
      ["Enable Codex Session Scanning", "~/.codex/sessions", "/tmp/codex-sessions"],
      ["Enable Claude Code Session Scanning", "~/.claude/projects", "/tmp/claude-projects"],
      ["Enable OpenCode Session Scanning", "~/.local/share/opencode", "/tmp/opencode-sessions"],
      ["Enable Cursor Session Scanning", "~/.cursor/projects", "/tmp/cursor-projects"],
      ["Enable Gemini CLI Session Scanning", "~/.gemini/tmp", "/tmp/gemini-sessions"],
      ["Enable Grok Session Scanning", "~/.grok/sessions", "/tmp/grok-sessions"],
    ] as const;
    for (const [label, placeholder, value] of sessionDirectories) {
      const toggle = page.locator("label").filter({ hasText: label }).locator('input[type="checkbox"]');
      await toggle.check();
      const input = page.getByPlaceholder(placeholder);
      await expect(input).toBeVisible();
      await input.fill(value);
    }
    await page.getByRole("button", { name: "Save Session Directories" }).click();
    await expect(page.getByText("sessionDirs saved", { exact: true })).toBeVisible();
  });

  test("uses the browser-safe extension install fallback", async ({ openMesh, page }) => {
    await openMesh("/settings?section=extensions", {
      mockTauri: true,
      runtime: "web",
    });
    await expect(page.getByText("Skills · Hooks · Plugins")).toBeVisible();
    await page.getByRole("button", { name: "Install from folder" }).click();
    await expect(page.getByText("Install requires the desktop app", { exact: true })).toBeVisible();
  });

  test("resets browser-mocked data only after confirmation", async ({ openMesh, page }) => {
    await openMesh("/settings?section=data", { mockTauri: true });
    page.once("dialog", (dialog) => {
      expect(dialog.type()).toBe("confirm");
      void dialog.accept();
    });
    await page.getByRole("button", { name: "Reset All Data" }).click();
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible({ timeout: 6_000 });
    await page.getByRole("link", { name: "Home" }).click();
    await expect(page.getByRole("heading", { name: "No project selected" })).toBeVisible();
  });

  test("shows explicit update availability and update-check failure states", async ({ openMesh, page }) => {
    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify({
          tag_name: "v0.2.1",
          name: "Browser release",
          html_url: "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.2.1",
          published_at: "2026-08-22T00:00:00.000Z",
          body: "Browser update notes",
          draft: false,
          prerelease: false,
          assets: [],
        }),
      }),
    );
    await openMesh("/settings?section=about", { mockTauri: true });
    await expect(page.getByText("Update available").first()).toBeVisible({ timeout: 8_000 });
    await expect(page.getByText("Latest: v0.2.1")).toBeVisible();
    await expect(page.getByText("Browser update notes")).toBeVisible();

    await openMesh("/settings?section=about", { mockTauri: true });
    await page.unroute("**/api.github.com/**");
    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({ status: 500, contentType: "application/json", body: "{}" }),
    );
    await page.getByRole("button", { name: "Check for updates" }).click();
    await expect(page.getByText("GitHub returned HTTP 500.", { exact: true })).toBeVisible();
  });

  test("keeps settings and continuity layouts usable on mobile", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/settings", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.locator(".shell__sidebar-slot").getByRole("complementary")).toHaveCount(0);
    await openMesh("/continuity", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Pending & LAN" })).toBeVisible();
    await expect(page.locator(".shell__sidebar-slot").getByRole("complementary")).toHaveCount(0);
  });
});
