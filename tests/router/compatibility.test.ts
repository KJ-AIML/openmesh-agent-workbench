import { describe, expect, it, vi } from "vitest";
import { createMemoryHistory, createRouter } from "vue-router";

vi.mock("@/pages/CanvasPage.vue", () => ({
  default: { template: "<div>Canvas</div>" },
}));

import { routes } from "@/router";

async function memoryRouter() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes,
  });
  await router.push("/");
  await router.isReady();
  return router;
}

describe("v0.2 route compatibility", () => {
  it("keeps canonical workbench routes without redirecting them away", async () => {
    const router = await memoryRouter();
    for (const path of [
      "/",
      "/agent-chat",
      "/agent-sessions",
      "/context",
      "/docs",
      "/notes",
      "/canvas",
      "/sprint",
      "/proxy-providers",
      "/oauth",
      "/proxy-runtime",
      "/usage",
      "/continuity",
      "/settings",
    ]) {
      await router.push(path);
      expect(router.currentRoute.value.path).toBe(path);
    }
  });

  it("aliases IA and known legacy URLs onto canonical routes", async () => {
    const router = await memoryRouter();
    const cases: Array<[string, string]> = [
      ["/chat", "/agent-chat"],
      ["/workspace", "/"],
      ["/workspace/docs", "/docs"],
      ["/agents", "/agent-sessions"],
      ["/agents/sessions", "/agent-sessions"],
      ["/runtime", "/proxy-providers"],
      ["/runtime/providers", "/proxy-providers"],
      ["/runtime/connections", "/oauth"],
      ["/runtime/proxy", "/proxy-runtime"],
      ["/runtime/usage", "/usage"],
      ["/runtime/continuity", "/continuity"],
      ["/proxy/providers", "/proxy-providers"],
      ["/proxy/runtime", "/proxy-runtime"],
    ];
    for (const [from, to] of cases) {
      await router.push(from);
      expect(router.currentRoute.value.path).toBe(to);
    }
  });

  it("keeps v0.1 settings section redirects", async () => {
    const router = await memoryRouter();
    await router.push("/models");
    expect(router.currentRoute.value.path).toBe("/settings");
    expect(router.currentRoute.value.query.section).toBe("provider");
    await router.push("/server");
    expect(router.currentRoute.value.query.section).toBe("server");
  });
});
