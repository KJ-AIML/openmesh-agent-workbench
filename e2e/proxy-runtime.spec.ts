import { test, expect } from "./fixtures";

test.describe("OpenMesh proxy runtime", () => {
  test("shows the built-in server contract and safe status", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "HTTP proxy" })).toBeVisible();
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Running");
    await expect(page.getByText("OpenMesh-owned server", { exact: true })).toBeVisible();
    await expect(page.getByText("built-in", { exact: true })).toBeVisible();
    await expect(page.getByText("127.0.0.1", { exact: true })).toBeVisible();
    await expect(page.getByText("OpenAI models", { exact: true })).toBeVisible();
    await expect(page.getByText("Claude/Gemini text translation", { exact: true })).toBeVisible();
    await expect(page.getByText("CLIProxyAPI")).toHaveCount(0);
    await expect(page.getByText("do-not-return")).toHaveCount(0);
  });

  test("starts and stops the owned server through visible controls", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true });
    await page.getByRole("button", { name: "Stop built-in proxy" }).click();
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Stopped");
    await expect(page.getByText("Not listening", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Start from Provider settings" }).click();
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Running");
    await expect(page.getByRole("link", { name: "Open Provider settings" })).toBeVisible();
  });

  test("redacts unavailable runtime details and keeps recovery navigation available", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-runtime", {
      mockTauri: true,
      scenario: "runtime-unavailable",
    });
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Runtime error");
    await expect(page.locator(".proxy-runtime-card__error")).toContainText(
      "The OpenMesh built-in proxy reported an error.",
    );
    await expect(page.getByText("built-in proxy is unavailable")).toHaveCount(0);
    await page.getByRole("link", { name: "Open Provider settings" }).click();
    await expect(page).toHaveURL(/\/settings\?section=provider$/);
  });

  test("shows a safe command failure state and remains usable on mobile", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-error" });
    await expect(page.getByRole("alert")).toContainText(
      "Unable to load the OpenMesh built-in proxy status.",
    );
    await expect(page.getByText("Runtime status is unavailable. Check Provider settings and try again.")).toBeVisible();

    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/proxy-runtime", { mockTauri: true });
    await expect(page.locator(".proxy-runtime-card")).toHaveCount(2);
    await expect(page.getByRole("complementary")).toHaveCount(0);
  });

  test("explains a port conflict without exposing the native error", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-port-conflict" });
    await expect(page.getByTestId("proxy-runtime-status")).toContainText("Stopped");
    await page.getByRole("button", { name: "Start from Provider settings" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "Unable to start the OpenMesh built-in proxy. Check Provider settings and API key.",
    );
    await expect(page.getByText("Address already in use", { exact: true })).toHaveCount(0);
  });

  test("guides OAuth-only setup to the provider registry", async ({ openMesh, page }) => {
    await openMesh("/proxy-runtime", { mockTauri: true, scenario: "runtime-no-upstream" });
    await page.getByRole("button", { name: "Start from Provider settings" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "Unable to start the OpenMesh built-in proxy. Check Provider settings and API key.",
    );
    await expect(page.getByText("No provider upstream is configured", { exact: true })).toHaveCount(0);
  });
});
