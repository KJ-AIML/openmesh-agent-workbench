import { test, expect } from "./fixtures";

test.describe("Workbench navigation", () => {
  test("organizes routes into collapsible topics and preserves project access", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/");
    const nav = page.getByRole("complementary");

    await expect(nav.locator("[data-topic-toggle='projects']")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(nav.locator("[data-topic-toggle='overview']")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(nav.locator("[data-topic-toggle='build']")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(nav.locator("[data-topic-toggle='network']")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(nav.getByRole("link", { name: "Chat" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Home" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Context" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Sessions" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Settings" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Docs" })).toBeHidden();
    await expect(nav.getByRole("link", { name: "Continuity" })).toBeHidden();

    await nav.locator("[data-topic-toggle='build']").click();
    await expect(nav.getByRole("link", { name: "Sprint" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Docs" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Notes" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Canvas" })).toBeVisible();

    await nav.locator("[data-topic-toggle='network']").click();
    await expect(nav.getByRole("link", { name: "Continuity" })).toBeVisible();
    await expect(
      nav.getByRole("link", { name: "OAuth Connections" }),
    ).toBeVisible();
    await expect(nav.getByRole("link", { name: "Proxy Runtime" })).toBeVisible();
    await expect(
      nav.getByRole("link", { name: "Provider Configuration" }),
    ).toBeVisible();

    await nav.getByRole("button", { name: "Add Project", exact: true }).click();
    await expect(page).toHaveURL(/\/projects\/new/);
  });

  test("walks all primary routes from the grouped sidebar", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/");
    const nav = page.getByRole("complementary");
    const routes = [
      { link: "Home", path: "/", text: /No project selected/i },
      { link: "Chat", path: "/agent-chat", text: "Select a project first" },
    ];

    for (const route of routes) {
      await nav.getByRole("link", { name: route.link }).click();
      await expect(page).toHaveURL(new RegExp(`${route.path.replace("/", "\\/")}$`));
      await expect(page.getByText(route.text).first()).toBeVisible();
    }

    await nav.locator("[data-topic-toggle='build']").click();
    for (const route of [
      { link: "Sprint", path: "/sprint", text: /Sprint/i },
      { link: "Docs", path: "/docs", text: "Docs" },
      { link: "Notes", path: "/notes", text: "Notes" },
      { link: "Canvas", path: "/canvas", text: "Canvas" },
    ]) {
      await nav.getByRole("link", { name: route.link }).click();
      await expect(page).toHaveURL(new RegExp(`${route.path.replace("/", "\\/")}$`));
      await expect(page.getByText(route.text).first()).toBeVisible();
    }

    await nav.locator("[data-topic-toggle='network']").click();
    for (const route of [
      { link: "Continuity", path: "/continuity", text: "Continuity" },
      { link: "OAuth Connections", path: "/oauth", text: "OAuth connections" },
      { link: "Proxy Runtime", path: "/proxy-runtime", text: "Proxy Runtime" },
      {
        link: "Provider Configuration",
        path: "/proxy-providers",
        text: "Provider configuration",
      },
    ]) {
      await nav.getByRole("link", { name: route.link }).click();
      await expect(page).toHaveURL(new RegExp(`${route.path.replace("/", "\\/")}$`));
      await expect(page.getByText(route.text).first()).toBeVisible();
    }

    await nav.getByRole("link", { name: "Sessions" }).click();
    await expect(page).toHaveURL(/\/agent-sessions$/);
    await expect(page.getByRole("heading", { name: "Agent Sessions" })).toBeVisible();

    await nav.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/\/settings(?:\?section=overview)?$/);
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  });

  test("expands the active topic for direct deep links", async ({ openMesh, page }) => {
    for (const route of [
      { path: "/continuity", topic: "network", link: "/continuity" },
      { path: "/agent-chat", topic: "agents", link: "/agent-chat" },
      { path: "/oauth", topic: "network", link: "/oauth" },
      { path: "/proxy-providers", topic: "network", link: "/proxy-providers" },
    ]) {
      await openMesh(route.path);
      const nav = page.getByRole("complementary");
      await expect(nav.locator(`[data-topic-toggle='${route.topic}']`)).toHaveAttribute(
        "aria-expanded",
        "true",
      );
      await expect(route.link === "/agent-chat" ? nav.locator(".chat-primary") : nav.locator(`[data-topic] a[href='${route.link}']`)).toHaveClass(
        /active/,
      );
    }
  });

  test("collapses, peeks, and restores the desktop sidebar", async ({
    openMesh,
    page,
  }) => {
    await openMesh("/agent-chat");
    const main = page.locator(".shell__main");
    const expandedWidth = await main.evaluate((el) => el.getBoundingClientRect().width);

    await expect(page.getByRole("button", { name: "Hide sidebar" })).toHaveCount(1);
    await page.getByRole("button", { name: "Hide sidebar" }).click();
    await expect(page.getByRole("complementary")).toHaveCount(0);
    await expect(page.locator(".shell__sidebar-slot.is-collapsed")).toBeAttached();
    await expect(page.getByRole("button", { name: "Show sidebar" })).toHaveCount(1);

    await page.locator(".shell__peek-zone").hover();
    await expect(page.locator(".shell__sidebar-slot.is-peeking")).toBeAttached();
    await expect(page.getByRole("complementary")).toBeVisible();

    await page.locator(".shell__main").hover({ position: { x: 400, y: 200 }, force: true });
    await expect(page.locator(".shell__sidebar-slot.is-peeking")).toHaveCount(0);
    await expect(page.getByRole("complementary")).toHaveCount(0);
    await expect.poll(async () => main.evaluate((el) => el.getBoundingClientRect().width)).toBeGreaterThan(expandedWidth);

    await page.getByRole("button", { name: "Show sidebar" }).click();
    await expect(page.getByRole("complementary")).toBeVisible();
    await expect(page.getByRole("button", { name: "Hide sidebar" })).toHaveCount(1);
  });
});
