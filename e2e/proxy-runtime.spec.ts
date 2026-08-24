import { test, expect } from "./fixtures";

test.describe("Proxy runtime", () => {
  test("shows a safe sidecar capability snapshot", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Proxy Runtime" })).toBeVisible();
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Ready");
    await expect(page.getByText("round-robin")).toBeVisible();
    await expect(
      page.getByText("2 enabled · 0 disabled · 0 marked unavailable · 2 total"),
    ).toBeVisible();
    await expect(page.getByTestId("proxy-runtime-capabilities")).toContainText("Deferred");
    await expect(page.getByTestId("proxy-runtime-capabilities")).toContainText("Quotas");
    await expect(page.getByText("do-not-return")).toHaveCount(0);
  });

  test("stacks safely on a mobile viewport", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/proxy-runtime", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Proxy Runtime" })).toBeVisible();
    await expect(page.getByTestId("proxy-runtime-status")).toBeVisible();
    await expect(page.locator(".proxy-runtime-card")).toHaveCount(4);
    await expect(page.getByRole("complementary")).toHaveCount(0);
    await expect(page.locator(".shell__main")).toBeVisible();
  });

  test("shows unavailable sidecar details and keeps navigation links working", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-runtime", {
      mockTauri: true,
      scenario: "runtime-unavailable",
    });
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Sidecar unreachable");
    await expect(page.locator(".proxy-runtime-card__error")).toContainText("The CLIProxyAPI runtime reported an error.");
    await expect(page.getByText("Configuration details are unavailable from this sidecar.")).toBeVisible();
    await page.getByRole("button", { name: "Manage OAuth connections" }).click();
    await expect(page).toHaveURL(/\/oauth$/);
  });

  test("shows a safe runtime error state", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-error" });
    await expect(page.getByRole("alert")).toContainText("Unable to load CLIProxyAPI runtime status");
    await expect(page.getByText("Runtime status is unavailable. Check the local sidecar configuration in Settings.")).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh" })).toBeEnabled();
  });

});
