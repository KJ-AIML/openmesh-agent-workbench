import { test, expect } from "./fixtures";

test.describe("Agent Chat", () => {
  test("shows the empty workspace state without a selected project", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat");
    await expect(page.getByRole("heading", { name: "Chat" })).toBeAttached();
    await expect(page.getByText("Select a project first")).toBeVisible();
  });

  test("sends a prompt and displays the selected direct-provider route safely", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-ready" });
    await page.locator("textarea").fill("hello from browser");
    await page.getByRole("button", { name: "Send" }).click();

    await expect(page.getByText("mock direct-provider reply")).toBeVisible();
    const route = page.locator('[data-testid="chat-route-summary"]');
    await expect(route).toContainText("Direct provider");
    await expect(route).toContainText("custom compatible");
    await expect(route).toContainText("browser-model");
    await expect(route).not.toContainText("127.0.0.1");
    await expect(route).not.toContainText("mock-secret-store");
  });

  test("starts a new chat and clears the active conversation", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-ready" });
    await page.locator("textarea").fill("first message");
    await page.getByRole("button", { name: "Send" }).click();
    await expect(page.getByText("mock direct-provider reply")).toBeVisible();
    await page.getByTestId("chat-clear").click();
    await expect(page.getByText("Start a conversation")).toBeVisible();

    await page.getByRole("button", { name: "New chat", exact: true }).click();
    await expect(page.locator(".chat__rail-row")).toHaveCount(2);
    await expect(page.getByTestId("chat-route-summary")).toHaveCount(0);
  });

  test("exercises composer modes, slash commands, context mentions, message actions, and terminal fallback", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-ready", runtime: "web" });
    const composer = page.getByTestId("chat-composer");
    const input = composer.getByRole("textbox", { name: "Message" });

    await composer.getByTestId("composer-mode").click();
    await expect(
      page.getByRole("listbox", { name: "Agent mode" }),
    ).toBeVisible();
    for (const mode of ["plan", "act", "delegate"]) {
      await page
        .getByRole("option")
        .getByRole("button", { name: mode, exact: true })
        .click();
      await expect(composer.getByTestId("composer-mode")).toContainText(mode);
      if (mode !== "delegate") {
        await composer.getByTestId("composer-mode").click();
      }
    }
    await composer.getByTestId("composer-mode").click();
    await page.keyboard.press("Escape");

    await input.fill("/");
    await expect(composer.getByTestId("composer-slash-menu")).toBeVisible();
    await input.fill("/read");
    await expect(composer.getByTestId("composer-slash-menu")).toContainText(
      "Read file",
    );
    await composer
      .getByRole("menuitem")
      .filter({ hasText: "Read file" })
      .click();
    await expect(input).toHaveValue("/read ");
    await input.fill("/tools");
    await page.keyboard.press("Enter");
    const toolsFold = page.locator(".tools-fold");
    await expect(toolsFold).toBeVisible();
    await toolsFold.getByRole("button").click();
    await expect(toolsFold).toContainText("Workspace Agent tools");

    await input.fill("@");
    await expect(composer.getByTestId("composer-mention-menu")).toBeVisible();
    await expect(composer.getByTestId("composer-mention-menu")).toContainText(
      "Browser Project",
    );
    await input.fill("@README");
    await expect(composer.getByTestId("composer-mention-menu")).toContainText(
      "README.md",
    );
    await page.keyboard.press("Enter");
    await expect(input).toHaveValue(/\/read README\.md/);

    await input.fill("message actions browser coverage");
    await composer.getByTestId("composer-send").click();
    await expect(page.getByText("mock direct-provider reply")).toBeVisible();
    const copyButtons = page.getByRole("button", { name: "Copy message" });
    await copyButtons.last().click({ force: true });
    await expect(page.locator(".chat__toast")).toContainText("Copied");
    await page
      .getByRole("button", { name: "Fork chat from here" })
      .last()
      .click();
    await expect(page.locator(".chat__rail-row")).toHaveCount(2);

    await composer.getByTestId("composer-status-terminal").click();
    await expect(page.getByTestId("chat-terminal-panel")).toBeVisible();
    if ((await page.getByTestId("term-empty-add").count()) > 0) {
      await page.getByTestId("term-empty-add").click();
    }
    await expect(page.getByTestId("embedded-term-error")).toContainText(
      "requires the desktop app",
    );
    await composer.getByTestId("composer-status-terminal").click();
    await expect(page.getByTestId("chat-terminal-panel")).toBeHidden();
  });

  test("stops an in-flight browser chat turn", async ({ openMesh, page }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "slow-chat" });
    const input = page.getByRole("textbox", { name: "Message" });
    await input.fill("slow browser turn");
    await page.getByTestId("composer-send").click();
    await expect(page.getByTestId("composer-stop")).toBeVisible();
    await page.getByTestId("composer-stop").click();
    await expect(page.getByTestId("composer-send")).toBeVisible();
  });

  test("approves, verifies, hands off, and rolls back a proposed patch", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "patch" });
    const input = page.getByRole("textbox", { name: "Message" });
    await input.fill("please propose a patch");
    await page.getByTestId("composer-send").click();
    await expect(page.getByText("Proposed patch patch-deadbeef")).toBeVisible();
    const patch = page.locator(".patch-card");
    await expect(patch).toContainText("Status: proposed");
    await patch.getByRole("button", { name: "Approve & apply" }).click();
    await expect(patch).toContainText("Status: applied");
    await patch.getByRole("button", { name: "Create handoff" }).click();
    await expect(patch).toContainText("Handoff draft created");
    await patch.getByRole("button", { name: "Verify" }).click();
    await expect(page.getByText("Verify logs")).toBeVisible();
    await patch.getByRole("button", { name: "Rollback" }).click();
    await expect(patch).toContainText("Status: rolled_back");
  });

  test("shows provider setup guidance and redacted provider failures", async ({ openMesh, page }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-not-ready" });
    await page.getByRole("textbox", { name: "Message" }).fill("try without a configured route");
    await page.getByTestId("composer-send").click();
    await expect(page.getByText(/Configure the selected provider, API key, and model/)).toBeVisible();
    await expect(page.getByTestId("chat-route-summary")).toHaveCount(0);

    await openMesh("/agent-chat", { mockTauri: true, scenario: "chat-error" });
    await page.getByRole("textbox", { name: "Message" }).fill("trigger provider failure");
    await page.getByTestId("composer-send").click();
    await expect(page.getByText(/Agent Engine error: provider request failed/)).toBeVisible();
    await expect(page.getByText("mock-secret-store")).toHaveCount(0);
  });

  test("rejects a proposed patch without applying it", async ({ openMesh, page }) => {
    await openMesh("/agent-chat", { mockTauri: true, scenario: "patch" });
    await page.getByRole("textbox", { name: "Message" }).fill("please propose a patch");
    await page.getByTestId("composer-send").click();
    const patch = page.locator(".patch-card");
    await patch.getByRole("button", { name: "Reject" }).click();
    await expect(patch).toContainText("Status: rejected");
    await expect(patch.getByRole("button", { name: "Approve & apply" })).toBeDisabled();
  });

  test("manages desktop terminal tabs, docking, and external launch fallback", async ({ openMesh, page }) => {
    await openMesh("/agent-chat", { mockTauri: true });
    const composer = page.getByTestId("chat-composer");
    await composer.getByTestId("composer-status-terminal").click();
    const panel = page.getByTestId("chat-terminal-panel");
    await expect(panel).toBeVisible();
    await panel.getByTestId("term-tab-add").click();
    await expect(panel.getByRole("tab")).toHaveCount(2);
    await panel.getByTestId("term-dock-toggle").click();
    await expect(panel).toHaveAttribute("data-dock", "bottom");
    await panel.getByTestId("term-open-external").click();
    await panel.getByTestId("term-panel-close").click();
    await expect(panel).toBeHidden();
  });

});
