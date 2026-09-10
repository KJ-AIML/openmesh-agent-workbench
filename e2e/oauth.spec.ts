import { test, expect } from "./fixtures";

test.describe("OpenMesh connections", () => {
  test("renders a safe browser-only failure without a native runtime", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth");
    await expect(page.getByRole("heading", { name: "Connections" })).toBeVisible();
    await expect(page.getByRole("alert")).toContainText(
      "The OpenMesh built-in proxy reported an error.",
    );
    await expect(page.getByText("access_token")).toHaveCount(0);
  });

  test("shows the OpenMesh-owned runtime and native adapter capability map", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Connections" })).toBeVisible();
    await expect(page.getByText("built-in", { exact: true })).toBeVisible();
    await expect(page.getByText("http://127.0.0.1:8317/v1", { exact: true })).toBeVisible();
    await expect(page.getByText("OpenMesh-owned proxy", { exact: true })).toBeVisible();
    await expect(page.getByText("Native adapters", { exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Connect" })).toHaveCount(3);
    await expect(page.getByRole("button", { name: "Device login" })).toHaveCount(2);
    await expect(page.getByText("Endpoint required", { exact: true })).toHaveCount(3);
    await expect(page.getByText("CLIProxyAPI")).toHaveCount(0);
    await expect(page.getByText("sidecar", { exact: false })).toHaveCount(0);
  });

  test("completes a mocked browser OAuth flow and recovers from provider failure", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-success" });
    const codex = page.locator("li").filter({ hasText: /^Codex/ });
    await codex.getByRole("button", { name: "Connect" }).click();
    await expect(page.getByText("Codex account connected.", { exact: true })).toBeVisible();
    await expect(codex.getByRole("button", { name: "Connect" })).toBeEnabled();

    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-error" });
    const claude = page.locator("li").filter({ hasText: /^Claude/ });
    await claude.getByRole("button", { name: "Connect" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "The OpenMesh proxy reported an error.",
    );
    await expect(claude.getByRole("button", { name: "Connect" })).toBeEnabled();
  });

  test("exercises device-login and provider-registry navigation on mobile", async ({
    openMesh,
    page,
  }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/oauth", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Connections" })).toBeVisible();
    const adapters = page.locator(".oauth-adapter-list li");
    await expect(adapters).toHaveCount(8);
    const firstBox = await adapters.nth(0).boundingBox();
    const secondBox = await adapters.nth(1).boundingBox();
    expect(firstBox).not.toBeNull();
    expect(secondBox).not.toBeNull();
    expect(secondBox!.y).toBeGreaterThan(firstBox!.y);

    await page.getByRole("link", { name: /Provider registry/ }).first().click();
    await expect(page).toHaveURL(/\/proxy-providers$/);
    await expect(page.getByRole("heading", { name: "Providers" })).toBeVisible();
  });
});
