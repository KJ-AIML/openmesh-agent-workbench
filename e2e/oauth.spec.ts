import { test, expect } from "./fixtures";

test.describe("OAuth connections", () => {
  test("renders setup state safely without a Tauri runtime", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth");
    await expect(
      page.getByRole("heading", { name: "OAuth connections" }),
    ).toBeVisible();
    await expect(page.getByText("Setup required")).toBeVisible();
    await expect(page.locator("article.oauth-card")).toHaveCount(5);
    await expect(
      page.getByRole("button", { name: "Connect" }).first(),
    ).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "Models" }).first(),
    ).toBeDisabled();
  });

  test("keeps auth-file metadata redacted and protects read-only records", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true });
    await expect(
      page.getByRole("heading", { name: "Auth-file metadata" }),
    ).toBeVisible();
    await expect(page.getByText("claude.json")).toBeVisible();
    await expect(page.getByText("unavailable.json")).toBeVisible();
    await expect(page.getByText("1 marked unavailable")).toBeVisible();
    await expect(page.getByText("Runtime-only", { exact: true })).toBeVisible();
    await expect(page.getByText("access_token")).toHaveCount(0);
    await expect(page.getByText("Bearer do-not-return")).toHaveCount(0);

    let dialogs = 0;
    page.on("dialog", async (dialog) => {
      dialogs += 1;
      await dialog.accept();
    });
    await page.getByRole("button", { name: "Disable claude.json" }).click();
    await expect(page.getByRole("status")).toContainText(
      "claude.json disabled",
    );
    expect(dialogs).toBe(1);

    await expect(
      page.getByRole("button", { name: "Disable runtime.json" }),
    ).toBeDisabled();
  });

  test("loads and saves OAuth model availability", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true });
    page.on("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Models" }).first().click();
    await expect(
      page.getByRole("heading", { name: "Codex OAuth models" }),
    ).toBeVisible();
    await expect(page.getByText("Provider-global exclusions")).toBeVisible();
    await expect(
      page.locator(".oauth-model-dialog input[type=checkbox]"),
    ).toHaveCount(2);

    await page.getByRole("button", { name: "Save model availability" }).click();
    await expect(page.getByText("OAuth model exclusions saved.")).toBeVisible();
  });

  test("shows an OAuth polling failure without disabling recovery", async ({ openMesh, page }) => {
    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-error" });
    const codex = page.locator("article.oauth-card").filter({ hasText: "Codex" });
    await codex.getByRole("button", { name: "Connect" }).click();
    await expect(
      codex.getByText("The CLIProxyAPI OAuth operation reported an error."),
    ).toBeVisible();
    await expect(codex.getByRole("button", { name: "Connect" })).toBeEnabled();
    await expect(codex.getByRole("button", { name: "Models" })).toBeEnabled();
  });

  test("stays usable on a mobile viewport", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openMesh("/oauth");
    await expect(
      page.getByRole("heading", { name: "OAuth connections" }),
    ).toBeVisible();
    const cards = page.locator("article.oauth-card");
    await expect(cards).toHaveCount(5);
    const firstBox = await cards.nth(0).boundingBox();
    const secondBox = await cards.nth(1).boundingBox();
    expect(firstBox).not.toBeNull();
    expect(secondBox).not.toBeNull();
    expect(secondBox!.x).toBeCloseTo(firstBox!.x, 0);
    expect(secondBox!.y).toBeGreaterThan(firstBox!.y);
  });

  test("manages OAuth connection settings, auth-file priority, connect link, callback, refresh, and cancel", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true });
    const config = page.locator(".workbench-card.oauth-page__config");

    await config.locator("input").first().fill("9002");
    await config.getByRole("button", { name: "Save port" }).click();
    await expect(page.getByRole("status")).toContainText(
      "OAuth management port saved",
    );

    await config
      .getByPlaceholder("Saved locally, never shown")
      .fill("browser-management-secret");
    await config.getByRole("button", { name: "Save secret" }).click();
    await expect(page.getByRole("status")).toContainText(
      "management secret saved",
    );

    await config
      .getByPlaceholder("Separate /v1 key, never shown")
      .fill("browser-client-key");
    await config.getByRole("button", { name: "Save /v1 key" }).click();
    await expect(page.getByRole("status")).toContainText(
      "client API key saved",
    );
    await config.getByRole("button", { name: "Clear /v1 key" }).click();
    await expect(page.getByRole("status")).toContainText(
      "client API key cleared",
    );
    await config
      .getByPlaceholder("Separate /v1 key, never shown")
      .fill("browser-client-key-restored");
    await config.getByRole("button", { name: "Save /v1 key" }).click();
    await expect(page.getByRole("status")).toContainText(
      "client API key saved",
    );

    page.on("dialog", async (dialog) => {
      if (dialog.type() === "prompt") await dialog.accept("10");
      else await dialog.accept();
    });
    const admin = page.locator(".workbench-card.oauth-page__admin");
    await admin.getByRole("button", { name: "Disable claude.json" }).click();
    await expect(page.getByRole("status")).toContainText(
      "claude.json disabled",
    );
    await admin.getByRole("button", { name: "Enable claude.json" }).click();
    await expect(page.getByRole("status")).toContainText("claude.json enabled");
    await admin
      .getByRole("button", { name: "Edit priority for claude.json" })
      .click();
    await expect(page.getByRole("status")).toContainText("priority updated");

    await config.getByRole("button", { name: "Clear secret" }).click();
    await expect(page.getByRole("status")).toContainText(
      "management secret cleared",
    );
    await config
      .getByPlaceholder("Saved locally, never shown")
      .fill("browser-management-secret-restored");
    await config.getByRole("button", { name: "Save secret" }).click();
    await expect(page.getByRole("status")).toContainText(
      "management secret saved",
    );

    const codex = page
      .locator("article.oauth-card")
      .filter({ hasText: "Codex" });
    await codex.getByRole("button", { name: "Connect" }).click();
    await expect(
      codex.getByText("Waiting for browser authorization…"),
    ).toBeVisible();
    await expect(
      codex.getByText("https://oauth.example.test/authorize"),
    ).toBeVisible();
    await codex.getByRole("button", { name: "Copy link" }).click();
    await expect(page.getByRole("status")).toContainText(
      "Authorization link copied",
    );
    await codex.getByRole("button", { name: "Open link" }).click();
    await codex
      .getByPlaceholder(/localhost.*callback/)
      .fill("http://localhost/callback?code=browser&state=browser-oauth-state");
    await codex.getByRole("button", { name: "Submit callback" }).click();
    await expect(page.getByRole("status")).toContainText(
      /callback submitted|OAuth connected/,
    );
    await expect(
      codex.getByText("Ready for use through the local provider endpoint."),
    ).toBeVisible();

    await codex.getByRole("button", { name: "Refresh" }).click();
    await expect(
      codex.getByText("Waiting for browser authorization…"),
    ).toBeVisible();
    await codex
      .getByPlaceholder(/localhost.*callback/)
      .fill(
        "http://localhost/callback?code=browser-2&state=browser-oauth-state",
      );
    await codex.getByRole("button", { name: "Submit callback" }).click();
    await expect(
      codex.getByText("Ready for use through the local provider endpoint."),
    ).toBeVisible();
  });
});
