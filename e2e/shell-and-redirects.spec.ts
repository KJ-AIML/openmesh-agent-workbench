import { test, expect } from "./fixtures";

test.describe("Shell, command palette, and redirects", () => {
  test("opens the command palette from the sidebar and keyboard shortcuts", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/", { mockTauri: true });

    await page.getByText("Search or command…", { exact: true }).click();
    const input = page.getByPlaceholder("Type a command or search...");
    await expect(input).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(input).toHaveCount(0);

    await page.locator(".shell__main").click({ position: { x: 400, y: 220 } });
    await page.keyboard.press("Meta+k");
    await expect(input).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(input).toHaveCount(0);

    await page.locator(".shell__main").click({ position: { x: 400, y: 220 } });
    await page.keyboard.press("Control+k");
    await expect(input).toBeFocused();
    await input.fill("no command matches this query");
    await expect(page.getByText("No commands found", { exact: true })).toBeVisible();
  });

  test("navigates and executes commands with keyboard selection", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/", { mockTauri: true });
    await page.getByText("Search or command…", { exact: true }).click();

    const active = page.locator(".command-palette-item-active");
    await expect(active).toContainText("Open Project Folder");
    await page.keyboard.press("ArrowDown");
    await expect(active).toContainText("Open Terminal");
    await page.keyboard.press("Escape");

    await page.getByText("Search or command…", { exact: true }).click();
    const input = page.getByPlaceholder("Type a command or search...");
    await input.fill("Open Docs");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/docs$/);
    await expect(page.getByRole("heading", { name: "Docs" })).toBeVisible();

    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();
    await page.getByText("Search or command…", { exact: true }).click();
    await page.getByPlaceholder("Type a command or search...").fill("Search Context");
    await page.keyboard.press("Enter");
    await expect(page).toHaveURL(/\/context\?focus=search&_ft=\d+$/);
    await expect(page.getByPlaceholder("Search your context…")).toBeFocused();
  });

  test("shows disabled project commands without a selected project", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/", { mockTauri: true, scenario: "empty" });
    await page.getByText("Search or command…", { exact: true }).click();
    const input = page.getByPlaceholder("Type a command or search...");
    await input.fill("Open Project Folder");

    const disabled = page
      .locator("button.command-palette-item-disabled")
      .filter({ hasText: "Open Project Folder" });
    await expect(disabled).toHaveCount(1);
    await expect(disabled).toBeDisabled();
    await expect(disabled).toContainText("No current project");
    await expect(page).toHaveURL(/\/$/);
  });

  test("redirects legacy routes and preserves the live Usage Analytics route", async ({ openMesh, page }) => {
    const redirects = [
      { from: "/models", section: "Provider" },
      { from: "/dev-connector", section: "Tools" },
      { from: "/server", section: "Server" },
      { from: "/status", section: "Overview" },
    ];

    for (const redirect of redirects) {
      await openMesh(redirect.from, { mockTauri: true });
      await expect(page).toHaveURL(/\/settings\?section=(provider|tools|server|overview)$/);
      await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
      await expect(
        page.getByRole("tab", { name: redirect.section, exact: true }),
      ).toHaveAttribute("aria-selected", "true");
    }

    await openMesh("/usage", { mockTauri: true });
    await expect(page).toHaveURL(/\/usage$/);
    await expect(page.getByRole("heading", { name: "Usage" })).toBeVisible();
  });

  test("keeps the command palette usable on a mobile viewport", async ({
    openMesh,
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/", { mockTauri: true });
    await page.keyboard.press("Control+k");
    const palette = page.locator(".command-palette");
    const box = await palette.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeLessThanOrEqual(390);
    await expect(page.getByPlaceholder("Type a command or search...")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(palette).toHaveCount(0);
  });
});
