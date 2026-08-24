import { test, expect } from "./fixtures";

test.describe("Project lifecycle", () => {
  test("validates, creates, switches, edits, and deletes a project", async ({ openMesh, page }) => {
    await openMesh("/projects/new", { mockTauri: true });

    await expect(page.getByRole("button", { name: "Save Project" })).toBeDisabled();

    await page.getByPlaceholder("e.g., OpenMesh").fill("Second Project");
    await page.getByPlaceholder(/C:\\KJ\\Repos/).fill("/tmp/second-project");
    await page.getByPlaceholder("main").fill("develop");
    await page.getByRole("button", { name: "Save Project" }).click();

    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("heading", { name: "Second Project" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Second Project" })).toBeVisible();

    await page.getByRole("button", { name: "Browser Project" }).click();
    await expect(page).toHaveURL(/\/agent-chat$/);
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();
    await expect(page.getByText("Second Project")).toHaveCount(0);

    await page.getByRole("button", { name: "Browser Project" }).click();
    await page.goto("/projects/browser-project/edit");
    await expect(page.getByRole("heading", { name: "Edit Project" })).toBeVisible();
    await page.locator("input").first().fill("Browser Project Renamed");
    await page.locator("select").nth(1).selectOption("archived");
    await page.getByRole("button", { name: "Save Changes" }).click();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("heading", { name: "Browser Project Renamed" })).toBeVisible();

    page.once("dialog", (dialog) => dialog.dismiss());
    await page.goto("/projects/browser-project/edit");
    await page.getByRole("button", { name: "Delete Project" }).click();
    await expect(page).toHaveURL(/\/projects\/browser-project\/edit$/);

    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Delete Project" }).click();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByText("No project selected")).toBeVisible();
  });

  test("keeps the edit route safe when no project is selected", async ({ openMesh, page }) => {
    await openMesh("/projects/missing/edit", { mockTauri: true, scenario: "empty" });
    await expect(page.getByText("Project not found")).toBeVisible();
    await page.getByRole("button", { name: "Go home" }).click();
    await expect(page).toHaveURL(/\/$/);
  });

  test("covers optional project fields, browser folder picking, validation, and cancel", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/projects/new", { mockTauri: true, runtime: "web" });

    page.once("dialog", async (dialog) => {
      expect(dialog.type()).toBe("prompt");
      await dialog.accept("/tmp/picked-browser-project");
    });
    await page.getByRole("button", { name: "Choose Folder" }).click();
    await expect(page.getByPlaceholder(/C:\\KJ\\Repos/)).toHaveValue(
      "/tmp/picked-browser-project",
    );

    await page.getByPlaceholder("e.g., OpenMesh").fill("Optional Browser Project");
    await page.getByPlaceholder("https://github.com/... or local path").fill(
      "https://github.com/example/browser-project",
    );
    await page.getByPlaceholder("main").fill("develop");
    await page.getByPlaceholder("docs/").fill("handbook/");
    await page.getByPlaceholder("Defaults to folder path").fill("/tmp/browser-terminal");
    await page.locator("select").selectOption("opencode");
    await page.getByPlaceholder("Any notes about this project...").fill(
      "Optional field coverage",
    );
    await page.getByRole("button", { name: "Save Project" }).click();
    await expect(page).toHaveURL(/\/$/);
    await expect(page.getByRole("heading", { name: "Optional Browser Project" })).toBeVisible();

    await page.getByRole("link", { name: "Optional Browser Project" }).click();
    await expect(page).toHaveURL(/\/projects\/.*\/edit$/);
    await expect(page.locator("input").nth(2)).toHaveValue(
      "https://github.com/example/browser-project",
    );
    await expect(page.locator("input").nth(3)).toHaveValue("develop");
    await expect(page.locator("input").nth(4)).toHaveValue("handbook/");
    await expect(page.locator("input").nth(5)).toHaveValue("/tmp/browser-terminal");
    await expect(page.locator("select").first()).toHaveValue("opencode");
    await expect(page.locator("textarea")).toHaveValue("Optional field coverage");

    await page.getByRole("button", { name: "Cancel" }).click();
    await expect(page).toHaveURL(/\/$/);

    await page.getByRole("link", { name: "Optional Browser Project" }).click();
    await page.locator("input").first().fill("");
    await expect(page.getByText("Project name is required")).toBeVisible();
    await expect(page.getByRole("button", { name: "Save Changes" })).toBeDisabled();
  });

});
