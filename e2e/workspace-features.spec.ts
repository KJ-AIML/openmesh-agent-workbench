import { test, expect } from "./fixtures";

async function waitForSavedContent(page: import("@playwright/test").Page) {
  await page.waitForTimeout(650);
}

test.describe("Workspace features", () => {
  test("handles Home actions and the empty workspace state", async ({ openMesh, page }) => {
    await openMesh("/", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();

    await page.getByRole("button", { name: "Resume Work" }).click();
    await expect(page.getByText("Terminal: Browser Project")).toBeVisible();

    await page.getByRole("button", { name: "Open Folder" }).click();
    await expect(page.getByText("Opened: Browser Project")).toBeVisible();

    await page.locator('button[title*="Runs codex"]').click();
    await expect(page.getByText("codex: Browser Project")).toBeVisible();

    await page.getByRole("button", { name: "Refresh git status" }).click();
    await expect(page.getByText("main", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Start empty sprint" }).click();
    await expect(page).toHaveURL(/\/sprint$/);
    await expect(page.getByText("Sprint — Browser Project")).toBeVisible();

    await page.goto("/", { waitUntil: "domcontentloaded" });
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();

    await openMesh("/", { mockTauri: true, scenario: "empty" });
    await expect(page.getByRole("heading", { name: "No project selected" })).toBeVisible();
    await expect(page.getByRole("main").getByRole("button", { name: "Add Project" })).toBeVisible();
  });

  test("shows Home session-scan failures with an explicit retry path", async ({ openMesh, page }) => {
    await openMesh("/", { mockTauri: true, scenario: "scan-error" });
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();
    await expect(page.getByText("permission denied")).toBeVisible();
    await expect(page.getByRole("button", { name: "Retry" })).toBeVisible();
    await page.getByRole("button", { name: "Retry" }).click();
    await expect(page.getByText("permission denied")).toBeVisible();
  });

  test("starts an empty sprint, creates tasks, edits status, and uses every sprint view", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/sprint", { mockTauri: true });
    await expect(page.getByText("No active sprint")).toBeVisible();

    await page.getByPlaceholder("Sprint name (optional)").fill("Browser Sprint");
    await page.getByRole("button", { name: "Create empty sprint" }).click();
    await expect(page.getByText("Browser Sprint")).toBeVisible();

    const taskInput = page.getByPlaceholder("Add your first real task…");
    await taskInput.fill("Cover sprint interactions");
    await page.getByRole("button", { name: "Add", exact: true }).click();
    await expect(page.locator(".kanban-card__title", { hasText: "Cover sprint interactions" })).toBeVisible();

    await expect(page.getByText("Owner", { exact: true })).toBeVisible();
    const detail = page.locator(".workbench-card").filter({ hasText: "Cover sprint interactions" }).last();
    await detail.locator("select").first().selectOption("in-progress");
    await detail.locator("select").nth(1).selectOption("P1");
    await detail.locator("input[placeholder=Optional]").fill("browser-agent");
    await detail.locator("textarea[placeholder='Notes / acceptance criteria']").fill("Verify the persisted task detail.");
    await expect(detail.locator("select").first()).toHaveValue("in-progress");

    await page.getByRole("button", { name: "List" }).click();
    await expect(page.locator(".task-table")).toBeVisible();
    await page.locator(".task-table .mini-select").first().selectOption("blocked");
    await page.locator(".task-table .mini-select").nth(1).selectOption("P0");
    await expect(page.locator(".task-table")).toContainText("Cover sprint interactions");

    await page.getByRole("button", { name: "Scrum" }).click();
    await expect(page.getByText("In progress & blocked")).toBeVisible();
    await expect(page.getByText("Completion")).toBeVisible();

    await page.getByRole("button", { name: "Board" }).click();
    await expect(page.locator(".kanban__col-title", { hasText: "Blocked" })).toBeVisible();
    await page.getByRole("button", { name: "Delete task" }).click();
    await expect(page.getByText("Cover sprint interactions")).toHaveCount(0);
  });

  test("creates, edits, previews, renames, folders, and deletes docs", async ({ openMesh, page }) => {
    await openMesh("/docs", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Docs" })).toBeVisible();

    await page.locator('[data-doc-path="README.md"]').click();
    await expect(page.locator(".prose").getByRole("heading", { name: "Browser project" })).toBeVisible();
    await page.getByRole("button", { name: "Edit" }).click();
    const editor = page.locator("textarea");
    await editor.fill("# Browser project\n\nEdited from Playwright.");
    await waitForSavedContent(page);
    await page.getByRole("button", { name: "Preview" }).click();
    await expect(page.getByText("Edited from Playwright.")).toBeVisible();

    await page.getByRole("button", { name: "New folder" }).click();
    await page.getByPlaceholder("Folder name").fill("archive");
    await page.getByPlaceholder("Folder name").press("Enter");
    await expect(page.locator('[data-doc-folder="archive"]')).toBeVisible();

    await page.getByRole("button", { name: "New doc" }).click();
    await expect(page.locator("textarea")).toBeVisible();
    await page.locator("textarea").fill("# Draft\n\nNew browser doc.");
    await waitForSavedContent(page);
    await expect(page.locator('[data-doc-path="untitled-1.md"]')).toBeVisible();

    await page.getByRole("button", { name: "Rename" }).click();
    const renameInput = page.locator('[data-doc-path="untitled-1.md"] input');
    await renameInput.fill("browser-draft.md");
    await renameInput.press("Enter");
    await expect(page.locator('[data-doc-path="browser-draft.md"]')).toBeVisible();

    await page.getByRole("button", { name: "Delete doc" }).click();
    await page.getByRole("button", { name: "Delete", exact: true }).last().click();
    await expect(page.locator('[data-doc-path="browser-draft.md"]')).toHaveCount(0);
  });

  test("imports dropped markdown and exercises document context menus", async ({ openMesh, page }) => {
    await openMesh("/docs", { mockTauri: true });
    await expect(page.locator('[data-doc-path="README.md"]')).toBeVisible();

    await page.evaluate(() => {
      const root = [...document.querySelectorAll("div.animate-fade-in")].find((el) =>
        el.querySelector('[data-doc-path="README.md"]'),
      );
      if (!root) throw new Error("Docs drop root not found");
      const transfer = new DataTransfer();
      transfer.items.add(new File(["# Dropped doc\n\nImported from a browser drop."], "dropped.md", {
        type: "text/markdown",
      }));
      root.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }));
    });
    await expect(page.locator('[data-doc-path="dropped.md"]')).toBeVisible();

    await page.locator('[data-doc-path="dropped.md"]').click({ button: "right" });
    await page.getByText("Rename", { exact: true }).last().click();
    const droppedRename = page.locator('[data-doc-path="dropped.md"] input');
    await droppedRename.fill("renamed-drop.md");
    await droppedRename.press("Enter");
    await expect(page.locator('[data-doc-path="renamed-drop.md"]')).toBeVisible();

    await page.locator('[data-doc-path="renamed-drop.md"]').click({ button: "right" });
    await page.getByText("Delete file", { exact: true }).click();
    await page.getByRole("button", { name: "Cancel", exact: true }).click();
    await expect(page.locator('[data-doc-path="renamed-drop.md"]')).toBeVisible();

    await page.locator('[data-doc-folder="guides"]').click({ button: "right" });
    await page.getByText("Delete folder", { exact: true }).click();
    await page.getByRole("button", { name: "Delete", exact: true }).last().click();
    await expect(page.locator('[data-doc-folder="guides"]')).toHaveCount(0);

    await page.getByRole("link", { name: "Notes" }).click();
    await expect(page.getByRole("heading", { name: "Notes" })).toBeVisible();
    await page.evaluate(() => {
      const root = [...document.querySelectorAll("div.animate-fade-in")].find((el) =>
        el.querySelector("h2")?.textContent?.includes("Notes"),
      );
      if (!root) throw new Error("Notes drop root not found");
      const transfer = new DataTransfer();
      transfer.items.add(new File(["# Dropped note\n\nImported from a browser drop."], "dropped-note.md", {
        type: "text/markdown",
      }));
      root.dispatchEvent(new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: transfer }));
    });
    await expect(page.getByRole("button", { name: /dropped-note/i })).toBeVisible();
  });

  test("opens doc deep links and performs Notes editing, preview, rename, and delete", async ({ openMesh, page }) => {
    await openMesh("/docs?file=guides%2Fflow.md", { mockTauri: true });
    await expect(page.getByText("The browser feature flow.")).toBeVisible();

    await page.goto("/notes?file=welcome.md", { waitUntil: "domcontentloaded" });
    await expect(page.getByRole("heading", { name: "Notes" })).toBeVisible();
    await expect(page.locator("textarea")).toHaveValue(/A note used by browser tests\./);
    await page.getByRole("button", { name: /welcome/i }).click();
    const noteEditor = page.locator("textarea");
    await noteEditor.fill("# Welcome\n\nEdited note content.");
    await waitForSavedContent(page);
    await page.getByRole("button", { name: "Preview" }).click();
    await expect(page.getByText("Edited note content.")).toBeVisible();

    await page.locator('button[title="Rename note"]').click();
    await page.getByPlaceholder("Note name").fill("browser-note");
    await page.getByRole("button", { name: "Save", exact: true }).click();
    await expect(page.getByRole("button", { name: /browser-note/i })).toBeVisible();

    await page.getByRole("button", { name: "New note" }).click();
    await expect(page.locator("textarea")).toBeVisible();
    await page.locator("textarea").fill("Temporary note");
    await waitForSavedContent(page);
    await page.getByRole("button", { name: "Delete note" }).click();
    await page.getByRole("button", { name: "Delete", exact: true }).last().click();
    await expect(page.getByText("untitled-1")).toHaveCount(0);
    await page.getByRole("button", { name: /browser-note/i }).click();
    await page.getByRole("button", { name: "Preview" }).click();
    await expect(page.getByText("Edited note content.")).toBeVisible();
  });

  test("searches Context, filters, refreshes, inspects, redacts, and opens sources", async ({ openMesh, page }) => {
    await openMesh("/context", { mockTauri: true });
    await expect(page.getByText(/Healthy — 3 docs indexed/)).toBeVisible();

    const search = page.getByPlaceholder("Search your context…");
    await page.getByRole("button", { name: "Docs" }).click();
    await page.locator("select").selectOption("10");
    await search.fill("browser");
    await search.press("Enter");
    await expect(page.getByText("README.md")).toBeVisible();
    await page.getByText("README.md").click();
    await expect(page.getByText("Preview")).toBeVisible();
    await expect(page.locator("pre")).toContainText("A document used by browser tests.");

    await page.getByRole("button", { name: "Open Source" }).click();
    await expect(page).toHaveURL(/\/docs\?file=README\.md$/);
    await expect(page.locator(".prose")).toContainText("A document used by browser tests.");

    await page.goto("/context", { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: "Refresh Context" }).click();
    await expect(page.getByText("COMPLETE")).toBeVisible();
    await expect(page.getByText("1 indexed")).toBeVisible();

    await search.fill("secret");
    await search.press("Enter");
    await page.getByText("Secret README.md").click();
    await expect(page.getByText("[secret content hidden]")).toBeVisible();
    await expect(page.getByText("Bearer do-not-return")).toHaveCount(0);

    await search.fill("nonexistent");
    await search.press("Enter");
    await expect(page.getByText(/No results for/)).toBeVisible();
  });

  test("uses all Canvas surfaces and persists their mutations", async ({ openMesh, page }) => {
    await openMesh("/canvas", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Canvas" })).toBeVisible();
    await expect(page.locator(".cv__rail-item", { hasText: "Browser overview" })).toBeVisible();

    await page.getByRole("button", { name: "Delete artifact" }).click();
    await expect(page.getByText("Select an Auto UI board or create one from Chat.")).toBeVisible();

    await page.getByRole("tab", { name: "Board" }).click();
    await page.getByRole("button", { name: "New board" }).click();
    await expect(page.getByText(/board-/)).toBeVisible();
    await expect(page.getByText("Created")).toBeVisible();
    await page.getByRole("button", { name: "Delete board" }).click();
    await expect(page.getByText("Deleted")).toBeVisible();

    await page.getByRole("tab", { name: "Network" }).click();
    await page.getByRole("button", { name: "Add machine" }).click();
    await page.getByRole("button", { name: "Add machine" }).click();
    await expect(page.locator(".cv__label")).toHaveCount(3);
    await expect(page.locator(".cv__label").filter({ hasText: /Machine/ })).toHaveCount(2);
    await page.getByRole("button", { name: "Connect last two" }).click();
    await expect(page.locator('.cv__svg line[marker-end="url(#arrow)"]')).toHaveCount(1);
    await page.getByRole("button", { name: "Fit" }).click();
  });

  test("scans, filters, selects, stars, resumes, deletes, and continues agent sessions", async ({ openMesh, page }) => {
    await openMesh("/agent-sessions", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Agent Sessions" })).toBeVisible();
    await expect(page.getByText("Scanned browser session")).toBeVisible();
    await page.getByRole("button", { name: "codex Codex", exact: true }).click();
    await expect(page.getByText("Scanned browser session")).toBeVisible();

    await page.getByRole("button", { name: /Scanned browser session/ }).click();
    await page.getByRole("button", { name: "⭐ Star" }).click();
    await expect(page.getByRole("button", { name: "⭐ Unstar" })).toBeVisible();
    await page.getByRole("button", { name: "Resume in terminal" }).click();

    await page.getByRole("button", { name: "Continue in Chat" }).click();
    await expect(page.getByRole("dialog", { name: "Continue in OpenMesh Chat" })).toBeVisible();
    await page.getByRole("button", { name: "Summarize & continue" }).click();
    await expect(page).toHaveURL(/\/agent-chat\?chat=/);

    await page.getByRole("link", { name: "Sessions" }).click();
    await expect(page).toHaveURL(/\/agent-sessions$/);
    await expect(page.getByText("Saved browser session")).toBeVisible();
    await page.getByRole("button", { name: /Saved browser session/ }).click();
    await expect(page.getByText("Changed files:")).toBeVisible();

    await page.getByRole("button", { name: "Continue in Chat" }).click();
    await page.getByRole("button", { name: "Cancel / Not now" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: "Mark Important" }).click();
    await expect(page.getByRole("button", { name: "Unmark Important" })).toBeVisible();
  });

  test("deletes a scanned session without touching its source file", async ({ openMesh, page }) => {
    await openMesh("/agent-sessions", { mockTauri: true });
    await page.getByRole("button", { name: /Scanned browser session/ }).click();
    const detail = page.locator(".workbench-card-compact").filter({ hasText: "Path:" });
    page.once("dialog", (dialog) => void dialog.accept());
    await detail.getByRole("button").last().click();
    await expect(page.getByText("Scanned browser session")).toHaveCount(0);
  });

  test("keeps workspace pages usable on mobile", async ({ openMesh, page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    for (const path of ["/sprint", "/docs", "/notes", "/context", "/canvas", "/agent-sessions"]) {
      await openMesh(path, { mockTauri: true });
      await expect(page.locator(".shell__main")).toBeVisible();
      await expect(page.locator(".shell__sidebar-slot").getByRole("complementary")).toHaveCount(0);
    }
  });

  test("reports a session scan error instead of hiding it as an empty list", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-sessions", { mockTauri: true, scenario: "scan-error" });
    await expect(page.getByRole("heading", { name: "Agent Sessions" })).toBeVisible();
    await expect(page.getByText("permission denied")).toBeVisible();
    await expect(page.getByText("Saved browser session")).toBeVisible();
    await expect(page.getByRole("button", { name: "Refresh" })).toBeEnabled();
  });

  test("attaches a saved session to a task and imports the full transcript into Chat", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/sprint", { mockTauri: true });
    await page.getByPlaceholder("Sprint name (optional)").fill("Session Sprint");
    await page.getByRole("button", { name: "Create empty sprint" }).click();
    await page.getByPlaceholder("Add your first real task…").fill("Attach session task");
    await page.getByRole("button", { name: "Add", exact: true }).click();

    await page.getByRole("link", { name: "Sessions" }).click();
    await expect(page).toHaveURL(/\/agent-sessions$/);
    await expect(page.getByText("Saved browser session")).toBeVisible();
    await page.getByRole("button", { name: /Saved browser session/ }).click();
    const taskSelect = page.getByText("Attach to task:").locator("xpath=..").locator("select");
    await taskSelect.selectOption({ label: "Attach session task" });
    await expect(page.getByText("Linked task:")).toBeVisible();

    await page.getByRole("button", { name: "Continue in Chat" }).click();
    await page.getByRole("button", { name: "Import full & continue" }).click();
    await expect(page).toHaveURL(/\/agent-chat\?chat=/);
    await expect(page.getByText("Imported browser session")).toBeVisible();
  });

});
