import { describe, expect, it } from "vitest";
import {
  CANONICAL_PAGE_LABELS,
  NAV_GROUPS,
  NAV_ITEMS,
  PRIMARY_CHAT,
  PROJECT_LANDING_ROUTE,
  ROUTE_REDIRECTS,
  defaultExpandedTopics,
  itemsForGroup,
  sidebarLinkHrefs,
  topicForRoute,
} from "@/lib/navigation";

describe("v0.2 navigation registry", () => {
  it("treats Chat as the primary project landing, not an Agents child", () => {
    expect(PROJECT_LANDING_ROUTE).toBe("/agent-chat");
    expect(PRIMARY_CHAT.route).toBe("/agent-chat");
    expect(NAV_ITEMS.some((item) => item.route === "/agent-chat")).toBe(false);
    expect(topicForRoute("/agent-chat")).toBeNull();
  });

  it("groups workspace artifacts, sessions, and runtime infrastructure", () => {
    expect(NAV_GROUPS.map((group) => group.id)).toEqual([
      "workspace",
      "agents",
      "runtime",
      "settings",
    ]);
    expect(itemsForGroup("workspace").map((item) => item.route)).toEqual([
      "/",
      "/context",
      "/docs",
      "/notes",
      "/canvas",
      "/sprint",
    ]);
    expect(itemsForGroup("agents").map((item) => item.route)).toEqual([
      "/agent-sessions",
    ]);
    expect(itemsForGroup("runtime").map((item) => item.route)).toEqual([
      "/proxy-providers",
      "/oauth",
      "/proxy-runtime",
      "/usage",
      "/continuity",
    ]);
    expect(itemsForGroup("runtime").map((item) => item.label)).toEqual([
      "Providers",
      "Connections",
      "HTTP proxy",
      "Usage",
      "Pending & LAN",
    ]);
  });

  it("keeps Runtime and Settings collapsed unless the active route needs them", () => {
    const expanded = defaultExpandedTopics();
    expect(expanded.workspace).toBe(true);
    expect(expanded.agents).toBe(true);
    expect(expanded.runtime).toBe(false);
    expect(expanded.settings).toBe(false);
    expect(topicForRoute("/proxy-runtime")).toBe("runtime");
    expect(topicForRoute("/settings")).toBe("settings");
    expect(topicForRoute("/docs")).toBe("workspace");
  });

  it("exposes a deterministic sidebar href list and canonical labels", () => {
    expect(sidebarLinkHrefs()).toEqual([
      "/agent-chat",
      "/",
      "/context",
      "/docs",
      "/notes",
      "/canvas",
      "/sprint",
      "/agent-sessions",
      "/proxy-providers",
      "/oauth",
      "/proxy-runtime",
      "/usage",
      "/continuity",
      "/settings",
    ]);
    expect(CANONICAL_PAGE_LABELS["/continuity"]).toBe("Pending & LAN");
    expect(CANONICAL_PAGE_LABELS["/proxy-runtime"]).toBe("HTTP proxy");
  });

  it("preserves known deep links as redirects rather than new canonical paths", () => {
    const byPath = Object.fromEntries(
      ROUTE_REDIRECTS.map((entry) => [entry.path, entry.redirect]),
    );
    expect(byPath["/proxy-providers"]).toBeUndefined();
    expect(byPath["/oauth"]).toBeUndefined();
    expect(byPath["/proxy-runtime"]).toBeUndefined();
    expect(byPath["/continuity"]).toBeUndefined();
    expect(byPath["/usage"]).toBeUndefined();
    expect(byPath["/chat"]).toBe("/agent-chat");
    expect(byPath["/runtime"]).toBe("/proxy-providers");
    expect(byPath["/runtime/connections"]).toBe("/oauth");
    expect(byPath["/runtime/proxy"]).toBe("/proxy-runtime");
    expect(byPath["/runtime/usage"]).toBe("/usage");
    expect(byPath["/runtime/continuity"]).toBe("/continuity");
    expect(byPath["/proxy/providers"]).toBe("/proxy-providers");
    expect(byPath["/models"]).toEqual({
      path: "/settings",
      query: { section: "provider" },
    });
  });
});
