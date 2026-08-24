import { test, expect } from "./fixtures";

async function sendChat(page: import("@playwright/test").Page, text: string) {
  const input = page.getByRole("textbox", { name: "Message" });
  await input.fill(text);
  await page.getByTestId("composer-send").click();
  await expect(page.getByTestId("composer-send")).toBeVisible();
}

test.describe("Residual browser feature coverage", () => {
  test("renders rich Chat content, sanitizes Markdown, and saves Auto UI to Canvas", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-rich" });
    await sendChat(page, "show a rich browser response");

    await expect(
      page.getByText("Rich browser response", { exact: true }),
    ).toBeVisible();
    await expect(
      page.getByText("Markdown **bold**", { exact: false }),
    ).toHaveCount(0);
    await expect(
      page.locator(".chat-prose strong").filter({ hasText: "bold" }),
    ).toBeVisible();
    await expect(page.locator(".chat-prose img")).toHaveCount(1);
    await expect(page.locator(".chat-prose img")).not.toHaveAttribute(
      "onerror",
      /.+/,
    );
    await expect(page.locator(".chat-mermaid")).toHaveCount(2);
    await page.locator(".chat-mermaid").first().scrollIntoViewIfNeeded();
    await expect(page.locator(".chat-mermaid__canvas svg").first()).toBeVisible(
      { timeout: 15_000 },
    );
    await expect(page.locator(".chat-mermaid__fallback")).toHaveCount(1, {
      timeout: 15_000,
    });
    await expect(page.locator(".chat-artifact")).toHaveCount(2);
    await expect(
      page.locator(".chat-artifact").first().locator(".omc__title"),
    ).toHaveText("Chat board");
    await expect(
      page.locator(".chat-artifact").first().locator(".omc-stat__value"),
    ).toHaveText("All routes");
    await expect(
      page.locator(".chat-artifact").nth(1).locator(".chat-artifact__body"),
    ).toContainText('"kind": "raw"');

    await page.getByRole("button", { name: "Save to Auto UI" }).click();
    await expect(
      page.getByRole("button", { name: "Saved", exact: true }),
    ).toBeVisible();
    await page.getByTestId("composer-status-canvas").click();
    await expect(page).toHaveURL(/\/canvas$/);
    const savedArtifact = page
      .locator(".cv__rail-item")
      .filter({ hasText: "Chat board" });
    await expect(savedArtifact).toBeVisible();
    await savedArtifact.click();
    await expect(page.locator(".cv__stage .omc__title")).toHaveText(
      "Chat board",
    );
    await expect(page.locator(".cv__stage .omc-stat__value")).toHaveText(
      "All routes",
    );
  });

  test("renders user and assistant GFM content and invalid structured fences as safe fallback text", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", {
      mockTauri: true,
      scenario: "chat-invalid-rich",
    });
    const input = page.getByRole("textbox", { name: "Message" });
    await input.fill("user **bold** with [a link](https://example.test)");
    await page.getByTestId("composer-send").click();

    await expect(page.locator(".msg--user .chat-prose strong")).toHaveText("bold");
    await expect(page.locator(".msg--user .chat-prose a")).toHaveAttribute(
      "href",
      "https://example.test",
    );
    await expect(page.locator(".msg--assistant .chat-prose table")).toBeVisible();
    await expect(page.locator(".msg--assistant .chat-prose ul")).toBeVisible();
    await expect(page.locator(".msg--assistant .chat-artifact")).toHaveCount(2);
    await expect(
      page.locator(".msg--assistant .chat-artifact").first().locator(".chat-artifact__body"),
    ).toContainText("not valid JSON");
    await expect(
      page.locator(".msg--assistant .chat-artifact").nth(1).locator(".chat-artifact__body"),
    ).toContainText("not valid JSON");
  });

  test("reports a slash-tool failure while keeping the Chat thread usable", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-tool-error" });
    await sendChat(page, "/read missing.txt");
    await expect(page.locator(".msg--assistant").last()).toContainText(
      "fixture read failed",
    );
    await expect(page.locator(".msg--assistant").last().locator(".tool--fail")).toBeVisible();
    await sendChat(page, "follow-up after tool failure");
    await expect(page.locator(".msg--assistant").last()).toContainText(
      "mock sidecar reply",
    );
  });

  test("executes the local slash-tool inventory across workspace, continuity, and release paths", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-tools" });

    const cases = [
      ["/project", "Project"],
      ["/docs", "Docs (2)"],
      ["/search browser", "README.md"],
      ["/git", '"branch": "main"'],
      ["/ls .", "package.json"],
      ["/read README.md", "Read-only fixture file."],
      ["/grep Browser", "Browser fixture match"],
      ["/diff", "No changes"],
      ["/patch show patch-deadbeef", "Patch patch-deadbeef"],
      ["/verify list", "npm-typecheck"],
      ["/verify npm-typecheck", "Recipe npm-typecheck ok=true"],
      ["/delegate codex", "Launched codex"],
      ["/continue pending", "Review browser continuity"],
      ["/continue handoff teammate", "Handoff draft created"],
      ["/continue link chat-1 codex foreign-1", "Linked foreign session"],
      ["/notes", "Notes (1)"],
      ["/sprint", '"taskCount": 0'],
      ["/continuity", '"openPendingCount": 1'],
      ["/pending", "Pending open=1"],
      ["/digest", "Digest"],
      ["/team", "Browser team"],
      ["/trust", '"queryAllowlistMode": "allowlist-only"'],
      ["/connectors", "Browser fixture"],
      ["/org", "workspace:Browser Project"],
      ["/pilot", "Pilot ready=true"],
      ["/rc", "RC ready=true"],
      ["/peers", "No mesh peers registered."],
      ["/ask What is live?", "Browser continuity answer"],
    ] as const;

    for (const [command, expected] of cases) {
      await sendChat(page, command);
      await expect(page.locator(".msg--assistant").last()).toContainText(
        expected,
        {
          timeout: 10_000,
        },
      );
    }

    await sendChat(page, "/tools");
    const fold = page.locator(".tools-fold").last();
    await expect(fold).toContainText("25 tools");
    await fold.getByRole("button").click();
    await expect(fold).toContainText("/project");
    await expect(fold).toContainText("/ask");
  });

  test("covers Agent Engine progress, terminal run output, tab lifecycle, and PTY events", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", {
      mockTauri: true,
      scenario: "chat-progress",
    });
    await sendChat(page, "show progress");
    await expect(page.locator(".msg--assistant").last()).toContainText(
      "mock sidecar reply",
    );

    await page.getByTestId("composer-status-terminal").click();
    const panel = page.getByTestId("chat-terminal-panel");
    await expect(panel).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (window as unknown as { __OPENMESH_PTY_COMMANDS__?: string[] })
              .__OPENMESH_PTY_COMMANDS__ ?? [],
        ),
      )
      .toContain("pty_create");
    await expect(panel.getByRole("tab")).toHaveCount(1);
    await expect(panel.getByRole("tab").first()).toHaveAttribute(
      "aria-selected",
      "true",
    );

    await panel.getByTestId("term-tab-add").click();
    await expect(panel.getByRole("tab")).toHaveCount(2);
    await panel.getByRole("tab").first().click();
    await expect(panel.getByRole("tab").first()).toHaveAttribute(
      "aria-selected",
      "true",
    );

    const firstTabId = await panel
      .getByRole("tab")
      .first()
      .getAttribute("data-testid");
    expect(firstTabId).toMatch(/^term-tab-/);
    const firstSessionId = firstTabId!.replace("term-tab-", "");
    await page.evaluate((id) => {
      const emit = (
        window as unknown as {
          __OPENMESH_EMIT_EVENT__?: (name: string, payload: unknown) => void;
        }
      ).__OPENMESH_EMIT_EVENT__;
      emit?.("pty-data", { id, data: "fixture output" });
      emit?.("pty-exit", { id });
    }, firstSessionId);
    await expect(
      panel.locator('[data-testid^="term-tab-"] .term-panel__tab-dot'),
    ).toHaveCount(1);

    await panel.getByTestId("term-dock-toggle").click();
    await expect(panel).toHaveAttribute("data-dock", "bottom");
    const resize = panel.getByTestId("term-panel-resize");
    await expect(resize).toHaveAttribute("aria-orientation", "horizontal");
    await panel.getByTestId("term-session-run-expand").click();
    await expect(panel.getByTestId("term-session-run-output")).toContainText(
      "No changes",
    );
    await panel.getByTestId("term-tab-close").first().click();
    await expect(panel.getByRole("tab")).toHaveCount(1);
    await panel.getByTestId("term-panel-close").click();
    await expect(panel).toBeHidden();
  });

  test("covers PTY input, resize, kill, empty-panel recovery, and cleanup", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-progress" });
    await page.getByTestId("composer-status-terminal").click();
    const panel = page.getByTestId("chat-terminal-panel");
    await expect(panel).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __OPENMESH_PTY_COMMANDS__?: string[] }).__OPENMESH_PTY_COMMANDS__ ?? [],
        ),
      )
      .toEqual(expect.arrayContaining(["pty_create", "pty_resize"]));

    const terminalSurface = panel.locator(".xterm-screen").first();
    await expect(terminalSurface).toBeVisible();
    await terminalSurface.click({ position: { x: 24, y: 24 } });
    await page.keyboard.type("fixture input");
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __OPENMESH_PTY_COMMANDS__?: string[] }).__OPENMESH_PTY_COMMANDS__ ?? [],
        ),
      )
      .toContain("pty_write");

    const firstTab = panel.getByRole("tab").first();
    const firstTabId = await firstTab.getAttribute("data-testid");
    expect(firstTabId).toMatch(/^term-tab-/);
    const firstSessionId = firstTabId!.replace("term-tab-", "");
    await page.evaluate((id) => {
      const emit = (window as unknown as { __OPENMESH_EMIT_EVENT__?: (name: string, payload: unknown) => void }).__OPENMESH_EMIT_EVENT__;
      emit?.("pty-data", { id, data: "fixture output" });
    }, firstSessionId);
    await expect(panel.locator(`[data-testid="embedded-term-${firstSessionId}"] .xterm`)).toBeVisible();

    await panel.getByTestId("term-tab-add").click();
    await expect(panel.getByRole("tab")).toHaveCount(2);
    await panel.getByTestId("term-tab-close").first().click();
    await expect(panel.getByRole("tab")).toHaveCount(1);
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __OPENMESH_PTY_COMMANDS__?: string[] }).__OPENMESH_PTY_COMMANDS__ ?? [],
        ),
      )
      .toContain("pty_kill");

    await panel.getByTestId("term-tab-close").first().click();
    await expect(panel.getByTestId("term-empty")).toBeVisible();
    await panel.getByTestId("term-empty-add").click();
    await expect(panel.getByRole("tab")).toHaveCount(1);
    await panel.getByTestId("term-panel-close").click();
    await page.getByRole("link", { name: "Home" }).click();
    await expect(page).toHaveURL(/\/$/);
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __OPENMESH_PTY_COMMANDS__?: string[] }).__OPENMESH_PTY_COMMANDS__ ?? [],
        ),
      )
      .toContain("pty_kill_all");
  });

  test("reports a native PTY creation error without breaking Chat", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "pty-error" });
    await page.getByTestId("composer-status-terminal").click();
    await expect(page.getByTestId("embedded-term-error")).toContainText(
      "pty unavailable",
    );
    await page
      .getByTestId("chat-terminal-panel")
      .getByTestId("term-panel-close")
      .click();
    await expect(page.getByRole("textbox", { name: "Message" })).toBeVisible();
  });

  test("persists a freeform Canvas scene mutation through the browser IPC boundary", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/canvas?tab=board", { mockTauri: true });
    await expect(page.getByRole("heading", { name: "Canvas" })).toBeVisible();
    await expect(page.locator(".board-editor__host")).toBeVisible();

    const excalidraw = page.locator(".excalidraw");
    await expect(excalidraw).toBeVisible();
    const rectangleButton = excalidraw
      .locator('[aria-label*="rectangle" i], [title*="rectangle" i]')
      .first();
    await expect(rectangleButton).toBeVisible();
    await rectangleButton.click();
    const canvas = excalidraw.locator("canvas").last();
    const box = await canvas.boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.move(box!.x + 120, box!.y + 120);
    await page.mouse.down();
    await page.mouse.move(box!.x + 260, box!.y + 220);
    await page.mouse.up();
    await expect(page.getByText("Saved", { exact: true })).toBeVisible({
      timeout: 8_000,
    });
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (
              window as unknown as {
                __OPENMESH_BOARD_SCENE_SAVES__?: unknown[];
              }
            ).__OPENMESH_BOARD_SCENE_SAVES__ ?? [],
        ),
      )
      .toHaveLength(1);

    await page.getByRole("link", { name: "Home" }).click();
    await page.getByRole("link", { name: "Canvas" }).click();
    await page.getByRole("tab", { name: "Board" }).click();
    await expect(page.locator(".board-editor__host")).toBeVisible();
    await expect
      .poll(() =>
        page.evaluate(
          () =>
            (
              window as unknown as {
                __OPENMESH_BOARD_SCENE_SAVES__?: unknown[];
              }
            ).__OPENMESH_BOARD_SCENE_SAVES__ ?? [],
        ),
      )
      .toHaveLength(1);
  });

  test("exercises Context empty, degraded, partial refresh, failed refresh, and clear-search states", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/context", { mockTauri: true, scenario: "context-empty" });
    await expect(page.getByText("Empty — no context indexed yet")).toBeVisible();
    await page.getByRole("button", { name: "Refresh Context" }).click();
    await expect(page.getByText("0 indexed")).toBeVisible();

    await openMesh("/context", { mockTauri: true, scenario: "context-degraded" });
    await expect(page.getByText("Degraded", { exact: true })).toBeVisible();

    await openMesh("/context", { mockTauri: true, scenario: "context-partial" });
    await page.getByRole("button", { name: "Refresh Context" }).click();
    await expect(page.getByText("PARTIAL", { exact: true })).toBeVisible();
    await expect(page.getByText("1 indexed")).toBeVisible();
    await expect(page.getByText("1 failed")).toBeVisible();

    await openMesh("/context", { mockTauri: true, scenario: "context-error" });
    await page.getByRole("button", { name: "Refresh Context" }).click();
    await expect(page.getByText("Refresh failed:", { exact: false })).toBeVisible();

    await openMesh("/context", { mockTauri: true });
    const search = page.getByPlaceholder("Search your context…");
    await search.fill("browser");
    await expect(page.getByText("README.md")).toBeVisible();
    await page.getByTitle("Clear").click();
    await expect(search).toHaveValue("");
    await expect(page.getByText("Enter a search query")).toBeVisible();
  });

  test("covers all Context filters and the empty Scrum pulse", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/context", { mockTauri: true });
    const search = page.getByPlaceholder("Search your context…");
    await search.fill("browser");
    await expect(page.getByText("README.md")).toBeVisible();
    for (const label of ["Notes", "Snapshots", "Tasks", "Recent", "Agent Sessions", "Docs"]) {
      await page.getByRole("button", { name: label, exact: true }).click();
      await expect(page.getByText("README.md")).toBeVisible();
    }

    await openMesh("/sprint", { mockTauri: true });
    await page.getByPlaceholder("Sprint name (optional)").fill("Empty Scrum Sprint");
    await page.getByRole("button", { name: "Create empty sprint" }).click();
    await page.getByRole("button", { name: "Scrum" }).click();
    await expect(page.getByText("No active or blocked tasks yet — add tasks on the Board.")).toBeVisible();
    await expect(page.getByText("No tasks yet")).toBeVisible();
  });

  test("covers no-project docs, notes, Context, and Canvas empty states", async ({
    openMesh,
    page,
  }) => {
    const noProjectStates = [
      ["/docs", "No project selected"],
      ["/notes", "No project selected"],
      ["/context", "No project selected"],
      ["/canvas", "Open a project to use Canvas."],
    ] as const;
    for (const [path, expected] of noProjectStates) {
      await openMesh(path, { mockTauri: true, scenario: "empty" });
      await expect(page.getByText(expected, { exact: true })).toBeVisible();
    }

    await openMesh("/docs", { mockTauri: true, scenario: "docs-empty" });
    await expect(page.getByText("No docs yet. Create one or drag .md files here.", { exact: true })).toBeVisible();
    await openMesh("/notes", { mockTauri: true, scenario: "notes-empty" });
    await expect(page.getByText("No notes yet. Create one or drag .md files here.", { exact: true })).toBeVisible();
  });

  test("covers Docs path normalization, folder move, and missing deep links", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/docs?file=guides%5Cflow.md", { mockTauri: true });
    await expect(page.getByText("The browser feature flow.")).toBeVisible();

    await page.getByRole("link", { name: "Home" }).click();
    await page.getByLabel("Workspace navigation").getByRole("link", { name: "Docs" }).click();
    const readme = page.locator('[data-doc-path="README.md"]');
    const guides = page.locator('[data-doc-folder="guides"]');
    await expect(readme).toBeVisible();
    await expect(guides).toBeVisible();
    await guides.click();
    const sourceBox = await readme.boundingBox();
    const targetBox = await guides.boundingBox();
    expect(sourceBox).not.toBeNull();
    expect(targetBox).not.toBeNull();
    await page.mouse.move(sourceBox!.x + sourceBox!.width / 2, sourceBox!.y + sourceBox!.height / 2);
    await page.mouse.down();
    await page.mouse.move(targetBox!.x + targetBox!.width / 2, targetBox!.y + targetBox!.height / 2, { steps: 8 });
    await page.mouse.up();
    await expect(page.getByText('Moved "README.md" to "guides"')).toBeVisible({ timeout: 8_000 });

    await openMesh("/docs?file=missing.md", { mockTauri: true });
    await expect(page.getByText("No doc selected")).toBeVisible();
  });

  test("covers empty OAuth administration, Kimi callback limitations, model-dialog cancellation, and empty models", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-admin-empty" });
    await expect(page.getByText("No auth-file metadata reported.")).toBeVisible();

    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-model-empty" });
    const kimi = page.locator("article.oauth-card").filter({ hasText: "Kimi" });
    await kimi.getByRole("button", { name: "Connect" }).click();
    await expect(kimi.getByText("Waiting for browser authorization…")).toBeVisible();
    await expect(kimi.getByPlaceholder(/localhost.*callback/)).toHaveCount(0);
    await kimi.getByRole("button", { name: "Models" }).click();
    await expect(page.getByText("No model definitions reported.")).toBeVisible();
    await page.getByRole("button", { name: "Close" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("covers non-Codex OAuth cards and invalid callback recovery", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true, scenario: "oauth-success" });
    const claude = page
      .locator("article.oauth-card")
      .filter({ hasText: "Claude" });
    await claude.getByRole("button", { name: "Connect" }).click();
    await expect(
      claude.getByText("Ready for use through the local provider endpoint."),
    ).toBeVisible();

    const xai = page.locator("article.oauth-card").filter({ hasText: "xAI" });
    await xai.getByRole("button", { name: "Connect" }).click();
    await expect(
      xai.getByText("Ready for use through the local provider endpoint."),
    ).toBeVisible();

    await claude.getByRole("button", { name: "Refresh" }).click();
    const callback = claude.getByPlaceholder(/localhost.*callback/);
    await callback.fill("");
    await expect(
      claude.getByRole("button", { name: "Submit callback" }),
    ).toBeDisabled();
  });
  test("drags Sprint tasks across Kanban columns and persists sprint status transitions", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/sprint", { mockTauri: true });
    await page.getByPlaceholder("Sprint name (optional)").fill("Drag Sprint");
    await page.getByRole("button", { name: "Create empty sprint" }).click();
    await page.getByPlaceholder("Add your first real task…").fill("Drag browser task");
    await page.getByRole("button", { name: "Add", exact: true }).click();

    const card = page.locator(".kanban-card").filter({ hasText: "Drag browser task" });
    const inProgress = page.locator(".kanban__col").filter({ hasText: "In Progress" });
    await card.dragTo(inProgress);
    await expect(inProgress.locator(".kanban-card").filter({ hasText: "Drag browser task" })).toBeVisible();

    await page.locator(".sprint-status-select").selectOption("completed");
    await expect(page.locator(".sprint-status-select")).toHaveValue("completed");
    await page.getByRole("button", { name: "Scrum" }).click();
    await expect(page.getByText("Status: completed", { exact: true })).toBeVisible();
  });

  test("reports an invalid Context canonical reference without offering a source navigation", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/context", { mockTauri: true, scenario: "context-invalid-ref" });
    const search = page.getByPlaceholder("Search your context…");
    await search.fill("browser");
    await expect(page.getByText("README.md")).toBeVisible();
    await page.getByText("README.md").click();
    await expect(page.getByRole("button", { name: "Open Source" })).toHaveCount(0);
  });

  test("covers OAuth port validation, direct-provider mode, invalid priority, and cancelled model edits", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/oauth", { mockTauri: true });
    const config = page.locator(".workbench-card.oauth-page__config");
    await config.locator("input").first().fill("0");
    await config.getByRole("button", { name: "Save port" }).click();
    await expect(page.getByRole("status")).toContainText(
      "Management port must be between 1 and 65535.",
    );

    const modeToggle = config.getByRole("checkbox");
    await modeToggle.uncheck();
    await config.getByRole("button", { name: "Save Chat mode" }).click();
    await expect(page.getByRole("status")).toContainText(
      "Direct provider mode restored for Chat.",
    );
    await expect(config).toContainText("Chat: direct provider mode");

    const admin = page.locator(".workbench-card.oauth-page__admin");
    page.once("dialog", async (dialog) => {
      expect(dialog.type()).toBe("prompt");
      await dialog.accept("not-an-integer");
    });
    await admin.getByRole("button", { name: "Edit priority for claude.json" }).click();
    await expect(admin.getByRole("alert")).toContainText(
      "Auth-file priority must be an integer",
    );

    const codex = page.locator("article.oauth-card").filter({ hasText: "Codex" });
    await codex.getByRole("button", { name: "Models" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    const before = await page.evaluate(
      () => (window as unknown as { __OPENMESH_OAUTH_COMMANDS__?: string[] }).__OPENMESH_OAUTH_COMMANDS__?.length ?? 0,
    );
    await page.getByRole("dialog").getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect
      .poll(() =>
        page.evaluate(
          () => (window as unknown as { __OPENMESH_OAUTH_COMMANDS__?: string[] }).__OPENMESH_OAUTH_COMMANDS__?.length ?? 0,
        ),
      )
      .toBe(before);
  });

  test("covers optional provider endpoints, URL validation, and individual mutation failures", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/proxy-providers", { mockTauri: true });
    page.on("dialog", (dialog) => dialog.accept());

    await page.locator(".proxy-providers-page__section-button").getByText("Claude API", { exact: true }).click();
    await page.getByRole("button", { name: "Add provider" }).last().click();
    let editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel(/API key/).fill("claude-write-only");
    await editor.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByText("claude-api-key-browser")).toBeVisible();

    await page.locator(".proxy-providers-page__section-button").getByText("Gemini API", { exact: true }).click();
    await page.getByRole("button", { name: "Add provider" }).last().click();
    editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel(/API key/).fill("gemini-write-only");
    await editor.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByText("gemini-api-key-browser")).toBeVisible();

    await page.locator(".proxy-providers-page__section-button").getByText("OpenAI compatible", { exact: true }).click();
    await page.getByRole("button", { name: "Add provider" }).last().click();
    editor = page.locator(".proxy-providers-page__dialog");
    await editor.getByLabel("Provider name").fill("unsafe-url");
    await editor.getByLabel(/API key/).fill("unsafe-write-only");
    await editor.getByLabel(/Base URL/).fill("https://user:pass@example.com/v1?token=1#fragment");
    await editor.getByRole("button", { name: "Save provider" }).click();
    await expect(page.getByRole("alert")).toContainText("Unable to update CLIProxyAPI provider configuration");
    await editor.getByRole("button", { name: "Cancel" }).click();

    await openMesh("/proxy-providers", { mockTauri: true, scenario: "provider-mutation-error" });
    await page.locator('button[title="Move down"]').first().click();
    await expect(page.getByRole("alert")).toContainText("Unable to update CLIProxyAPI provider configuration");
    await page.getByRole("button", { name: "Disable" }).first().click();
    await expect(page.getByRole("alert")).toContainText("Unable to update CLIProxyAPI provider configuration");
    await page.getByRole("button", { name: "Delete" }).first().click();
    await expect(page.getByRole("alert")).toContainText("Unable to update CLIProxyAPI provider configuration");
    await page.locator("#routing-strategy").selectOption("fill-first");
    await expect(page.getByRole("alert")).toContainText("Unable to update CLIProxyAPI provider configuration");
  });

  test("keeps Cursor, Gemini, and Grok scanned sessions without unsupported terminal actions", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-sessions", { mockTauri: true, scenario: "mixed-sessions" });
    for (const tool of ["cursor", "gemini", "grok"]) {
      await page.getByRole("button", { name: `${tool} ${tool === "cursor" ? "Cursor" : tool === "gemini" ? "Gemini" : "Grok"}`, exact: true }).click();
      await page.getByRole("button", { name: new RegExp(`Scanned ${tool} session`) }).click();
      await expect(page.getByRole("button", { name: "Resume in terminal" })).toHaveCount(0);
    }
  });

  test("covers Agent Session launch failures, unsupported resume controls, and saved-session deep links", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-sessions?session=scan-grok-session", { mockTauri: true, scenario: "mixed-sessions" });
    await expect(page.getByRole("heading", { name: "Scanned grok session", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Resume in terminal" })).toHaveCount(0);

    await openMesh("/agent-sessions?session=saved-codex-session", { mockTauri: true, scenario: "mixed-sessions" });
    await expect(page.getByRole("heading", { name: "Saved codex session", exact: true })).toBeVisible();

    await openMesh("/agent-sessions", { mockTauri: true, scenario: "command-error" });
    await page.getByRole("button", { name: /Scanned browser session/ }).click();
    await page.getByRole("button", { name: "Resume in terminal" }).click();
    await expect(page.getByText("command launch failed", { exact: true })).toBeVisible();
  });

  test("toggles extension hooks and plugins through the Settings runtime surface", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/settings?section=extensions", {
      mockTauri: true,
      scenario: "extensions-seeded",
    });

    await page.getByRole("tab", { name: "Hooks" }).click();
    const hookRow = page.locator(".ext-row").filter({ hasText: "browser-hook" });
    const hookToggle = hookRow.getByRole("checkbox");
    await hookToggle.uncheck();
    await expect(hookRow.getByText("Off", { exact: true })).toBeVisible();
    await hookToggle.check();
    await expect(hookRow.getByText("On", { exact: true })).toBeVisible();

    await page.getByRole("tab", { name: "Plugins" }).click();
    const pluginRow = page.locator(".ext-row").filter({ hasText: "Browser Plugin" });
    const pluginToggle = pluginRow.getByRole("checkbox");
    await pluginToggle.uncheck();
    await expect(pluginRow.getByText("Off", { exact: true })).toBeVisible();
    await pluginToggle.check();
    await expect(pluginRow.getByText("On", { exact: true })).toBeVisible();
  });

  test("requires and honors the dangerous command preset confirmation", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/settings?section=tools", { mockTauri: true });
    const presets = page.locator(".workbench-card:visible").filter({ hasText: "Command presets" });
    await page.getByPlaceholder("Preset name").fill("Dangerous preset");
    await page.getByPlaceholder("Command").fill("rm");
    await page.getByPlaceholder("Args (space-separated)").fill("-rf fixture");
    await presets.locator("select").selectOption("dangerous");
    await page.getByRole("button", { name: "Add preset" }).click();

    const dangerous = page
      .locator("div")
      .filter({ hasText: "Dangerous preset" })
      .filter({ has: page.locator('button[title="Run"]') })
      .last();

    page.once("dialog", (dialog) => {
      expect(dialog.type()).toBe("confirm");
      expect(dialog.message()).toContain("Dangerous command");
      void dialog.dismiss();
    });
    await dangerous.locator('button[title="Run"]').click();
    await expect(page.getByText("Ran: Dangerous preset", { exact: true })).toHaveCount(0);

    page.once("dialog", (dialog) => void dialog.accept());
    await dangerous.locator('button[title="Run"]').click();
    await expect(page.getByText("Ran: Dangerous preset", { exact: true })).toBeVisible();
  });

  test("keeps a project form unchanged when the browser folder picker is cancelled", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/projects/new", { mockTauri: true, runtime: "web" });
    const folder = page.getByPlaceholder(/C:\\KJ\\Repos/);
    page.once("dialog", async (dialog) => {
      expect(dialog.type()).toBe("prompt");
      await dialog.dismiss();
    });
    await page.getByRole("button", { name: "Choose Folder" }).click();
    await expect(folder).toHaveValue("");
    await expect(page.getByRole("button", { name: "Save Project" })).toBeDisabled();
  });

  test("opens every Canvas surface from direct deep links and falls back safely", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/canvas?tab=auto-ui", { mockTauri: true });
    await expect(page.getByRole("tab", { name: "Auto UI" })).toHaveClass(/is-active/);
    await expect(page.locator(".cv__auto")).toBeVisible();

    await openMesh("/canvas?tab=board", { mockTauri: true });
    await expect(page.getByRole("tab", { name: "Board" })).toHaveClass(/is-active/);
    await expect(page.locator(".cv__stage--board")).toBeVisible();

    await openMesh("/canvas?tab=network", { mockTauri: true });
    await expect(page.getByRole("tab", { name: "Network" })).toHaveClass(/is-active/);
    await expect(page.locator(".cv__network")).toBeVisible();

    await openMesh("/canvas?tab=unknown", { mockTauri: true });
    await expect(page.getByRole("tab", { name: "Auto UI" })).toHaveClass(/is-active/);
    await expect(page.locator(".cv__auto")).toBeVisible();
  });

  test("shows Home action failures without hiding the workspace", async ({
    openMesh,
    page,
  }) => {
    const dialogs: string[] = [];
    page.on("dialog", (dialog) => {
      dialogs.push(dialog.message());
      void dialog.accept();
    });
    await openMesh("/", { mockTauri: true, scenario: "home-action-error" });
    await expect(page.getByRole("heading", { name: "Browser Project" })).toBeVisible();

    await page.getByRole("button", { name: "Resume Work", exact: true }).click();
    await page.getByRole("button", { name: "Open Folder", exact: true }).click();
    await page.getByRole("button", { name: /Claude/ }).click();
    await page.getByRole("button", { name: /OpenCode/ }).click();
    await expect.poll(() => dialogs).toHaveLength(4);
    expect(dialogs).toEqual([
      "home action failed",
      "home action failed",
      "home action failed",
      "home action failed",
    ]);
    await expect(page.getByText("main", { exact: true })).toHaveCount(0);
  });

  test("shows Continuity load failures and keeps refresh available", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/continuity", { mockTauri: true, scenario: "continuity-error" });
    await expect(page.getByText("continuity unavailable", { exact: true })).toBeVisible();
    const refresh = page.getByTitle("Refresh");
    await expect(refresh).toBeEnabled();
    await refresh.click();
    await expect(page.getByText("continuity unavailable", { exact: true })).toBeVisible();
  });

  test("covers update installer missing, unsupported, and did-not-open outcomes", async ({
    openMesh,
    page,
  }) => {
    const release = (assets: unknown[]) => ({
      tag_name: "v0.1.31",
      name: "Browser installer edge release",
      html_url: "https://github.com/KJ-AIML/openmesh-agent-workbench/releases/tag/v0.1.31",
      published_at: "2026-08-22T00:00:00.000Z",
      body: "Installer edge coverage",
      draft: false,
      prerelease: false,
      assets,
    });

    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(
          release([
            { name: "OpenMesh_0.1.31.AppImage", browser_download_url: "https://example.test/openmesh.AppImage", size: 10 },
          ]),
        ),
      }),
    );
    await openMesh("/settings?section=about", {
      mockTauri: true,
      scenario: "update-assets-missing",
    });
    await page.getByRole("button", { name: "Check for updates" }).click();
    await expect(page.getByText(/Installers not ready yet/).first()).toBeVisible({ timeout: 8_000 });

    await page.evaluate(() => localStorage.removeItem("openmesh.updateCheck.v1"));
    await openMesh("/settings?section=about", {
      mockTauri: true,
      scenario: "update-unsupported",
    });
    await page.getByRole("button", { name: "Check for updates" }).click();
    await expect(page.getByText(/In-app install isn’t available/)).toBeVisible({ timeout: 8_000 });

    await page.evaluate(() => localStorage.removeItem("openmesh.updateCheck.v1"));
    await page.unroute("**/api.github.com/**");
    await page.route("**/api.github.com/**", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(
          release([
            { name: "OpenMesh_0.1.31_aarch64.dmg", browser_download_url: "https://example.test/openmesh.dmg", size: 10 },
          ]),
        ),
      }),
    );
    await openMesh("/settings?section=about", {
      mockTauri: true,
      scenario: "update-open-failure",
    });
    await page.getByRole("button", { name: "Check for updates" }).click();
    await page.getByRole("button", { name: "Download & install" }).click();
    await expect(page.getByText("Downloaded, but the installer did not open automatically.", { exact: true })).toBeVisible();
    await expect(page.getByText("Open the downloaded installer manually.", { exact: true })).toBeVisible();
  });

  test("opens and closes the Voice HUD in push-to-talk mode", async ({
    openMesh,
    page,
  }) => {
    await page.addInitScript(() => {
      Object.defineProperty(navigator, "mediaDevices", {
        configurable: true,
        value: {
          getUserMedia: async () => ({ getTracks: () => [{ stop() {} }] }),
        },
      });
    });
    await openMesh("/settings?section=voice", { mockTauri: true });
    await page.locator("label.voice-settings__row").filter({ hasText: "Listen mode" }).locator("select").selectOption("ptt");
    await page.locator(".tb__nav").getByRole("link", { name: "Chat" }).click();
    await expect(page).toHaveURL(/\/agent-chat$/);

    await page.getByRole("button", { name: "OpenMesh Voice" }).click();
    await expect(page.getByRole("status")).toBeVisible();
    await expect(page.getByText("Hold the title-bar mic while you talk, then release.", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "Voice reply on" }).click();
    await expect(page.getByRole("button", { name: "Voice reply off" })).toBeVisible();
    await page.getByTitle("Turn voice off").click();
    await expect(page.getByRole("status")).toHaveCount(0);
  });

  test("covers Settings no-project tools, risk confirmations, preset deletion, and browser folder choosing", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/settings?section=tools", { mockTauri: true, scenario: "empty" });
    await expect(page.getByText("Select a project to manage terminal shortcuts and command presets.", { exact: true })).toBeVisible();

    await openMesh("/settings?section=tools", { mockTauri: true });
    await page.getByPlaceholder("Preset name").fill("Caution preset");
    await page.getByPlaceholder("Command").fill("npm");
    await page.getByPlaceholder("Args (space-separated)").fill("run caution");
    await page.locator(".workbench-card:visible").filter({ hasText: "Command presets" }).locator("select").selectOption("caution");
    await page.getByRole("button", { name: "Add preset" }).click();
    const caution = page.locator("div").filter({ hasText: "Caution preset" }).filter({ has: page.locator('button[title="Run"]') }).last();
    page.once("dialog", (dialog) => void dialog.dismiss());
    await caution.locator('button[title="Run"]').click();
    await expect(page.getByText("Ran: Caution preset", { exact: true })).toHaveCount(0);
    page.once("dialog", (dialog) => void dialog.accept());
    await caution.locator('button[title="Run"]').click();
    await expect(page.getByText("Ran: Caution preset", { exact: true })).toBeVisible();
    page.once("dialog", (dialog) => void dialog.dismiss());
    await caution.locator('button[title="Delete"]').click();
    await expect(page.getByText("Caution preset", { exact: true })).toBeVisible();
    page.once("dialog", (dialog) => void dialog.accept());
    await caution.locator('button[title="Delete"]').click();
    await expect(page.getByText("Caution preset", { exact: true })).toHaveCount(0);

    await openMesh("/settings?section=paths", { mockTauri: true, runtime: "web" });
    page.once("dialog", async (dialog) => {
      expect(dialog.type()).toBe("prompt");
      await dialog.accept("/tmp/settings-picked");
    });
    await page.getByRole("button", { name: "Choose Folder" }).click();
    await expect(page.getByPlaceholder("C:\\KJ\\Repos")).toHaveValue("/tmp/settings-picked");
  });

});
