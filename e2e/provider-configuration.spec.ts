import { test, expect } from "./fixtures";

test.describe("Provider configuration", () => {
  test("creates, routes, disables, and deletes a typed provider", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Provider configuration" })).toBeVisible();
    await expect(page.getByText("browser-gateway-a")).toBeVisible();
    await expect(page.getByText("1 key configured").first()).toBeVisible();

    page.on("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Add provider" }).click();
    await page.getByRole("textbox", { name: "Provider name" }).fill("browser-created");
    await page.getByRole("textbox", { name: /API key/ }).fill("browser-write-only");
    await page.getByRole("textbox", { name: /Base URL/ }).fill("https://created.example.com/v1");
    await page.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByRole("status")).toContainText("Provider saved");

    await page.locator("#routing-strategy").selectOption("fill-first");
    await expect(page.getByRole("status")).toContainText("Routing strategy updated");
    await page.getByRole("button", { name: "Disable" }).first().click();
    await expect(page.getByRole("status")).toContainText("Provider disabled");
    await page.getByRole("button", { name: "Delete" }).last().click();
    await expect(page.getByRole("status")).toContainText("Provider deleted");

    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (window as unknown as { __OPENMESH_PROVIDER_COMMANDS__?: string[] })
              .__OPENMESH_PROVIDER_COMMANDS__ ?? [],
        ),
      )
      .toEqual(
        expect.arrayContaining([
          "cliproxy_provider_list",
          "cliproxy_provider_save",
          "cliproxy_routing_strategy",
          "cliproxy_provider_toggle",
          "cliproxy_provider_delete",
        ]),
      );
  });

  test("keeps provider configuration usable on mobile", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/proxy-providers", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Provider configuration" })).toBeVisible();
    await expect(page.getByText("browser-gateway-a")).toBeVisible();
    await expect(page.locator(".proxy-providers-page__layout")).toBeVisible();
  });

  test("covers provider sections, edit key preservation, reorder, and validation", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true });
    page.on("dialog", (dialog) => dialog.accept());

    for (const section of ["Codex API", "Claude API", "Gemini API"]) {
      await page
        .locator(".proxy-providers-page__section-button")
        .getByText(section, { exact: true })
        .click();
      await expect(page.getByText("No providers in this section")).toBeVisible();
    }

    await page
      .locator(".proxy-providers-page__section-button")
      .getByText("Codex API", { exact: true })
      .click();
    await page.getByRole("button", { name: "Add provider" }).last().click();
    const codexDialog = page.locator(".proxy-providers-page__dialog");
    await codexDialog.getByPlaceholder("Never displayed after save").fill("codex-write-only");
    await codexDialog.getByLabel(/Base URL/).fill("https://codex.example.com/v1");
    await codexDialog.getByPlaceholder("Optional").fill("42");
    await codexDialog.getByPlaceholder(/model-name/).fill("codex-main | fast\ncodex-main\ncodex-mini");
    await codexDialog.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByRole("status")).toContainText("Provider saved");
    await expect(page.getByText("codex-api-key-browser")).toBeVisible();
    await page.getByRole("button", { name: "Edit" }).click();
    const editDialog = page.locator(".proxy-providers-page__dialog");
    await expect(editDialog.getByPlaceholder("Never displayed after save")).toHaveValue("");
    await expect(editDialog.getByPlaceholder("Leave blank to preserve the configured endpoint")).toHaveValue("");
    await editDialog.getByPlaceholder("Optional").fill("43");
    await editDialog.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByRole("status")).toContainText("Provider updated");

    await page
      .locator(".proxy-providers-page__section-button")
      .getByText("OpenAI compatible", { exact: true })
      .click();
    await page.locator('button[title="Move down"]').first().click();
    await expect(page.getByRole("status")).toContainText("Provider order updated");

    await page.getByRole("button", { name: "Add provider" }).last().click();
    const invalidDialog = page.locator(".proxy-providers-page__dialog");
    await invalidDialog.getByLabel("Provider name").fill("invalid-priority");
    await invalidDialog.getByLabel(/Base URL/).fill("https://invalid.example.com/v1");
    await invalidDialog.getByPlaceholder("Optional").fill("1000001");
    await invalidDialog.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByRole("alert")).toContainText("One or more provider fields are invalid");
    await invalidDialog.getByRole("button", { name: "Cancel" }).click();
  });

  test("shows provider configuration failure without enabling unsafe controls", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-error" });
    await expect(page.getByRole("heading", { name: "Provider configuration" })).toBeVisible();
    await expect(page.getByRole("alert")).toContainText(
      "Unable to update CLIProxyAPI provider configuration",
    );
    await expect(page.getByRole("button", { name: "Add provider" }).first()).toBeDisabled();
    await expect(page.getByText("Provider configuration is unavailable")).toBeVisible();
  });

});
