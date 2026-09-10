import { test, expect } from "./fixtures";

test.describe("Built-in provider configuration", () => {
  test("adds, routes, disables, edits, and removes an upstream without returning its key", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Providers" })).toBeVisible();
    await expect(page.getByText("browser-upstream-a", { exact: true })).toBeVisible();
    await expect(page.getByText("Key configured", { exact: true }).first()).toBeVisible();

    page.on("dialog", (dialog) => void dialog.accept());
    await page.getByRole("button", { name: "Add upstream" }).first().click();
    const editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel("Upstream id").fill("browser-created");
    await editor.getByLabel("Base URL").fill("https://created.example.com/v1");
    await editor.getByRole("textbox", { name: /API key/ }).fill("browser-write-only");
    await editor.getByLabel("Models").fill("created-model");
    await editor.getByRole("button", { name: "Save upstream" }).click();
    await expect(page.getByRole("status")).toContainText("Upstream added.");
    await expect(page.getByText("browser-created", { exact: true })).toBeVisible();
    await expect(page.getByText("browser-write-only", { exact: true })).toHaveCount(0);

    await page.locator("#routing-strategy").selectOption("fill-first");
    await expect(page.getByRole("status")).toContainText("Routing strategy updated.");

    const created = page.locator(".proxy-providers-page__record").filter({ hasText: "browser-created" });
    await created.getByRole("button", { name: "Disable" }).click();
    await expect(page.getByRole("status")).toContainText("Upstream disabled.");
    await created.getByRole("button", { name: "Edit" }).click();
    await expect(
      page.locator(".proxy-providers-page__dialog").getByRole("textbox", { name: /API key/ }),
    ).toHaveValue("");
    await page.locator(".proxy-providers-page__dialog").getByRole("button", { name: "Cancel" }).click();
    await created.getByRole("button", { name: "Remove" }).click();
    await expect(page.getByRole("status")).toContainText("Upstream removed.");
    await expect(page.getByText("browser-created", { exact: true })).toHaveCount(0);

    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (window as unknown as { __OPENMESH_RUNTIME_COMMANDS__?: string[] })
              .__OPENMESH_RUNTIME_COMMANDS__ ?? [],
        ),
      )
      .toEqual(expect.arrayContaining(["proxy_management_config", "proxy_management_update"]));
  });

  test("keeps provider configuration usable on mobile", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/proxy-providers", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Providers" })).toBeVisible();
    await expect(page.locator(".proxy-providers-page__layout")).toBeVisible();
    await expect(page.getByRole("button", { name: "Add upstream" }).first()).toBeVisible();
  });

  test("validates upstream URLs and supports an empty registry", async ({ openMesh, page }) => {
    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-empty" });
    await expect(page.getByText("No upstreams configured", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Add upstream" }).last().click();
    const editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel("Upstream id").fill("unsafe-url");
    await editor.getByLabel("Base URL").fill("https://user:pass@example.com/v1?token=1#fragment");
    await editor.getByLabel("Models").fill("unsafe-model");
    await editor.getByRole("button", { name: "Save upstream" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "The upstream URL cannot contain credentials, query parameters, or fragments.",
    );
    await expect(page.getByText("unsafe-url", { exact: true })).toHaveCount(0);
    await editor.getByRole("button", { name: "Cancel" }).click();
  });

  test("keeps mutation failures redacted and disables unsafe controls when loading fails", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-mutation-error" });
    page.on("dialog", (dialog) => void dialog.accept());
    await page.getByRole("button", { name: "Add upstream" }).first().click();
    const editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel("Upstream id").fill("failed-upstream");
    await editor.getByLabel("Base URL").fill("https://failed.example.com/v1");
    await editor.getByLabel("Models").fill("failed-model");
    await editor.getByRole("button", { name: "Save upstream" }).click();
    await expect(page.getByRole("alert")).toContainText(
      "Unable to update OpenMesh built-in proxy configuration.",
    );
    await expect(page.getByText("failed-upstream", { exact: true })).toHaveCount(0);

    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-error" });
    await expect(page.getByRole("alert")).toContainText(
      "Unable to load OpenMesh built-in proxy configuration.",
    );
    await expect(page.getByRole("button", { name: "Add upstream" }).first()).toBeDisabled();
  });
});
