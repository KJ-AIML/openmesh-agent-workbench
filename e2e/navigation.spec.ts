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
    await expect(nav.locator("[data-topic-toggle='workspace']")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(nav.locator("[data-topic-toggle='agents']")).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    await expect(nav.locator("[data-topic-toggle='runtime']")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(nav.locator("[data-topic-toggle='settings']")).toHaveAttribute(
      "aria-expanded",
      "false",
    );
    await expect(nav.getByRole("link", { name: "Chat" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Home" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Context" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Docs" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Sessions" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "HTTP proxy" })).toBeHidden();
    await expect(nav.getByRole("link", { name: "Pending & LAN" })).toBeHidden();

    await nav.locator("[data-topic-toggle='settings']").click();
    await expect(nav.getByRole("link", { name: "Settings" })).toBeVisible();

    await nav.locator("[data-topic-toggle='runtime']").click();
    await expect(nav.getByRole("link", { name: "Pending & LAN" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Connections" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "HTTP proxy" })).toBeVisible();
    await expect(nav.getByRole("link", { name: "Providers" })).toBeVisible();

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

    await nav.locator("[data-topic-toggle='runtime']").click();
    for (const route of [
      { link: "Pending & LAN", path: "/continuity", text: "Pending & LAN" },
      { link: "Connections", path: "/oauth", text: "Connections" },
      { link: "HTTP proxy", path: "/proxy-runtime", text: "HTTP proxy" },
      {
        link: "Providers",
        path: "/proxy-providers",
        text: "Providers",
      },
    ]) {
      await nav.getByRole("link", { name: route.link }).click();
      await expect(page).toHaveURL(new RegExp(`${route.path.replace("/", "\\/")}$`));
      await expect(page.getByText(route.text).first()).toBeVisible();
    }

    await nav.getByRole("link", { name: "Sessions" }).click();
    await expect(page).toHaveURL(/\/agent-sessions$/);
    await expect(page.getByRole("heading", { name: "Agent Sessions" })).toBeVisible();

    await nav.locator("[data-topic-toggle='settings']").click();
    await nav.getByRole("link", { name: "Settings" }).click();
    await expect(page).toHaveURL(/\/settings(?:\?section=overview)?$/);
    await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
  });

  test("expands the active topic for direct deep links", async ({ openMesh, page }) => {
    for (const route of [
      { path: "/continuity", topic: "runtime", link: "/continuity" },
      { path: "/oauth", topic: "runtime", link: "/oauth" },
      { path: "/proxy-providers", topic: "runtime", link: "/proxy-providers" },
    ]) {
      await openMesh(route.path);
      const nav = page.getByRole("complementary");
      await expect(nav.locator(`[data-topic-toggle='${route.topic}']`)).toHaveAttribute(
        "aria-expanded",
        "true",
      );
      await expect(nav.locator(`[data-topic] a[href='${route.link}']`)).toHaveClass(
        /active/,
      );
    }

    await openMesh("/agent-chat");
    const nav = page.getByRole("complementary");
    await expect(nav.locator(".chat-primary")).toHaveClass(/is-active|active/);
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
    const collapsedWidth = await main.evaluate((el) => el.getBoundingClientRect().width);
    expect(collapsedWidth).toBeGreaterThan(expandedWidth);

    await page.locator(".shell__peek-zone").hover();
    await expect(page.getByRole("complementary")).toHaveCount(1);
    await page.mouse.move(500, 500);
    await expect(page.getByRole("complementary")).toHaveCount(0);

    await page.getByRole("button", { name: "Show sidebar" }).click();
    await expect(page.getByRole("complementary")).toHaveCount(1);
    await expect(page.locator(".shell__sidebar-slot.is-collapsed")).toHaveCount(0);
  });
});
