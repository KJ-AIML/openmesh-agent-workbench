import { test, expect } from "@playwright/test";

/**
 * True browser smoke against Vite + empty (non-Tauri) store.
 * Does not require OpenRouter keys or a Tauri binary.
 *
 * Richer GUI contracts (chat optimistic send, Continuity LAN/Proxy tabs,
 * Settings Extensions toggles) are covered by vitest page tests with mocks.
 */

async function waitForShell(page: import("@playwright/test").Page) {
  await page.waitForLoadState("domcontentloaded");
  // Splash holds ~700ms after load; wait until shell is interactive.
  await expect(page.locator(".shell")).toBeVisible({ timeout: 20_000 });
  await expect(page.locator(".startup-splash")).toHaveCount(0, {
    timeout: 15_000,
  });
}

test.describe("OpenMesh workbench shell smoke", () => {
  test("sidebar nav walks major surfaces", async ({ page }) => {
    const pageErrors: string[] = [];
    page.on("pageerror", (err) => pageErrors.push(String(err)));

    await page.goto("/");
    await waitForShell(page);
    expect(pageErrors, `page errors: ${pageErrors.join(" | ")}`).toEqual([]);

    // Titlebar duplicates some routes — scope to the left rail.
    const nav = page.getByRole("complementary");
    await expect(nav.getByRole("link", { name: "Chat" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Home" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Sprint" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Docs" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Notes" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Context" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Continuity" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Sessions" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Settings" })).toBeVisible();

    // Home empty state (no Tauri project list).
    await nav.getByRole("link", { name: "Home" }).click();
    await expect(page.getByText(/No project selected/i).first()).toBeVisible();

    await nav.getByRole("link", { name: "Chat" }).click();
    await expect(page).toHaveURL(/\/agent-chat/);
    // Landmark heading may be visually hidden; thread + rail are the chrome.
    await expect(page.getByRole("heading", { name: "Chat" })).toBeAttached();
    await expect(page.getByRole("button", { name: "New chat" })).toBeVisible();

    await nav.getByRole("link", { name: "Sprint" }).click();
    await expect(page).toHaveURL(/\/sprint/);
    await expect(page.getByText(/Sprint/i).first()).toBeVisible();

    await nav.getByRole("link", { name: "Docs" }).click();
    await expect(page).toHaveURL(/\/docs/);
    await expect(page.getByText("Docs").first()).toBeVisible();

    await nav.getByRole("link", { name: "Notes" }).click();
    await expect(page).toHaveURL(/\/notes/);
    await expect(page.getByText("Notes").first()).toBeVisible();

    await nav.getByRole("link", { name: "Context" }).click();
    await expect(page).toHaveURL(/\/context/);
    await expect(page.getByText(/Context|No project/i).first()).toBeVisible();

    await nav.getByRole("link", { name: "Continuity" }).click();
    await expect(page).toHaveURL(/\/continuity/);
    await expect(page.getByText("Continuity").first()).toBeVisible();
    await expect(page.getByText(/No project selected/i).first()).toBeVisible();

    await nav.getByRole("link", { name: "Sessions" }).click();
    await expect(page).toHaveURL(/\/agent-sessions/);
    await expect(
      page.getByText(/Agent Sessions|Sessions/i).first(),
    ).toBeVisible();

    await nav.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/\/settings/);
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Provider" })).toBeVisible();

    // Provider panel
    await page.getByRole("tab", { name: "Provider" }).click();
    await expect(
      page.locator(".settings__panel-title", { hasText: "Provider & Models" }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: "Save Provider & Models" }),
    ).toBeVisible();

    // Extensions under Runtime
    await page.getByRole("tab", { name: "Runtime" }).click();
    await page.getByRole("tab", { name: "Extensions" }).click();
    await expect(
      page.locator(".settings__panel-title", {
        hasText: /Skills · Hooks · Plugins/i,
      }),
    ).toBeVisible();
    await expect(page.getByRole("tab", { name: "Skills" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Hooks" })).toBeVisible();
  });

  test("direct deep links mount Continuity and Chat", async ({ page }) => {
    await page.goto("/continuity");
    await waitForShell(page);
    await expect(page.getByText("Continuity").first()).toBeVisible();

    await page.goto("/agent-chat");
    await waitForShell(page);
    await expect(page.getByText(/Chat|provider|project/i).first()).toBeVisible();
  });

  test("sidebar toggle collapses rail and expands content", async ({ page }) => {
    await page.goto("/agent-chat");
    await waitForShell(page);

    const nav = page.getByRole("complementary");
    await expect(nav).toBeVisible();
    await expect(nav.getByRole("link", { name: "Chat" })).toBeVisible();

    const main = page.locator(".shell__main");
    const expandedWidth = await main.evaluate((el) => el.getBoundingClientRect().width);

    // Single crumb toggle when expanded (no footer/edge duplicates).
    await expect(page.getByRole("button", { name: "Hide sidebar" })).toHaveCount(1);
    await page.getByRole("button", { name: "Hide sidebar" }).click();
    // aria-hidden + inert remove the rail from the a11y tree when collapsed.
    await expect(page.getByRole("complementary")).toHaveCount(0);
    await expect(page.locator(".shell__sidebar-slot.is-collapsed")).toBeAttached();
    // Collapsed: exactly one show control (crumb), no edge reopen tab.
    await expect(page.getByRole("button", { name: "Show sidebar" })).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Show sidebar" })).toBeVisible();
    await expect(page.locator(".shell__edge-toggle")).toHaveCount(0);
    // Hover-to-peek hit zone (ephemeral; pin state stays collapsed).
    await expect(page.locator(".shell__peek-zone")).toHaveCount(1);
    await page.locator(".shell__peek-zone").hover();
    await expect(page.locator(".shell__sidebar-slot.is-peeking")).toBeAttached();
    await expect(page.getByRole("complementary")).toBeVisible();
    await expect(page.locator(".shell--sidebar-collapsed")).toHaveCount(1);
    // Leave peek: move to main content, overlay collapses; pin stays off.
    await page.locator(".shell__main").hover({ position: { x: 200, y: 200 } });
    await expect(page.locator(".shell__sidebar-slot.is-peeking")).toHaveCount(0);
    await expect(page.getByRole("complementary")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Show sidebar" })).toHaveCount(1);

    await expect
      .poll(async () => main.evaluate((el) => el.getBoundingClientRect().width))
      .toBeGreaterThan(expandedWidth);

    // Chat surface still mounts and uses the wider main column.
    await expect(page.getByRole("heading", { name: "Chat" })).toBeAttached();
    await expect(page.getByRole("button", { name: "New chat" })).toBeVisible();
    await expect(page.locator(".shell--sidebar-collapsed")).toHaveCount(1);

    // macOS overlay titlebar: tabs clear ooo when rail is gone; crumb stays at normal main inset.
    if ((await page.locator(".shell--mac").count()) > 0) {
      const clearance = page.locator(".tb--mac-traffic-clearance");
      await expect(clearance).toHaveCount(1);
      const spacer = page.locator(".tb__traffic-clearance");
      await expect(spacer).toHaveCount(1);
      const inset = await spacer.evaluate((el) => {
        const style = getComputedStyle(el);
        return parseFloat(style.flexBasis || style.width || "0");
      });
      expect(inset).toBeGreaterThanOrEqual(90);
      // Chat tab's left edge must sit at/after the spacer (not under ooo).
      const chatLeft = await page
        .locator(".tb__nav a", { hasText: "Chat" })
        .evaluate((el) => el.getBoundingClientRect().left);
      expect(chatLeft).toBeGreaterThanOrEqual(90);

      // Crumb row: no ooo spacer — toggle sits near left of main (normal ~1.25rem pad).
      await expect(page.locator(".shell__crumb--traffic-clearance")).toHaveCount(0);
      await expect(page.locator(".shell__crumb-traffic-clearance")).toHaveCount(0);
      const toggleLeft = await page
        .getByRole("button", { name: "Show sidebar" })
        .evaluate((el) => el.getBoundingClientRect().left);
      expect(toggleLeft).toBeLessThan(40);
    }

    await page.getByRole("button", { name: "Show sidebar" }).click();
    await expect(page.getByRole("complementary")).toBeVisible();
    await expect(page.locator(".shell--sidebar-collapsed")).toHaveCount(0);
    await expect(page.locator(".tb--mac-traffic-clearance")).toHaveCount(0);
    await expect(page.locator(".shell__crumb--traffic-clearance")).toHaveCount(0);
    await expect(page.locator(".shell__crumb-traffic-clearance")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "Hide sidebar" })).toHaveCount(1);
  });
});

