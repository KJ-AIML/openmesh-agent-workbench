/** v0.2 workbench navigation — single source of truth for sidebar IA. */

import type { Component } from "vue";
import {
  BarChart3,
  Bot,
  FileEdit,
  FileText,
  Home,
  KeyRound,
  Link2,
  ListTodo,
  Network,
  Radio,
  Search,
  Server,
  Settings,
} from "lucide-vue-next";

export const PRIMARY_CHAT_ROUTE = "/agent-chat";
/** Selecting a project lands here (already true before A9). */
export const PROJECT_LANDING_ROUTE = PRIMARY_CHAT_ROUTE;

export type NavGroupId = "workspace" | "agents" | "runtime" | "settings";

export type NavGroup = {
  id: NavGroupId;
  label: string;
  defaultExpanded: boolean;
  placement: "main" | "footer";
};

export type NavItem = {
  id: string;
  label: string;
  route: string;
  group: NavGroupId;
  icon: Component;
  /** Documented ownership; A9 does not hide these links. */
  requiresProject?: boolean;
};

export const NAV_GROUPS: readonly NavGroup[] = [
  {
    id: "workspace",
    label: "Workspace",
    defaultExpanded: true,
    placement: "main",
  },
  {
    id: "agents",
    label: "Agents",
    defaultExpanded: true,
    placement: "main",
  },
  {
    id: "runtime",
    label: "Runtime",
    defaultExpanded: false,
    placement: "main",
  },
  {
    id: "settings",
    label: "Settings",
    defaultExpanded: false,
    placement: "footer",
  },
] as const;

export const PRIMARY_CHAT = {
  id: "chat",
  label: "Chat",
  route: PRIMARY_CHAT_ROUTE,
  hint: "work",
} as const;

export const NAV_ITEMS: readonly NavItem[] = [
  { id: "home", label: "Home", route: "/", group: "workspace", icon: Home },
  {
    id: "context",
    label: "Context",
    route: "/context",
    group: "workspace",
    icon: Search,
    requiresProject: true,
  },
  {
    id: "docs",
    label: "Docs",
    route: "/docs",
    group: "workspace",
    icon: FileText,
    requiresProject: true,
  },
  {
    id: "notes",
    label: "Notes",
    route: "/notes",
    group: "workspace",
    icon: FileEdit,
    requiresProject: true,
  },
  {
    id: "canvas",
    label: "Canvas",
    route: "/canvas",
    group: "workspace",
    icon: Network,
    requiresProject: true,
  },
  {
    id: "sprint",
    label: "Sprint",
    route: "/sprint",
    group: "workspace",
    icon: ListTodo,
    requiresProject: true,
  },
  {
    id: "sessions",
    label: "Sessions",
    route: "/agent-sessions",
    group: "agents",
    icon: Bot,
  },
  {
    id: "providers",
    label: "Providers",
    route: "/proxy-providers",
    group: "runtime",
    icon: KeyRound,
  },
  {
    id: "connections",
    label: "Connections",
    route: "/oauth",
    group: "runtime",
    icon: Link2,
  },
  {
    id: "http-proxy",
    label: "HTTP proxy",
    route: "/proxy-runtime",
    group: "runtime",
    icon: Server,
  },
  {
    id: "usage",
    label: "Usage",
    route: "/usage",
    group: "runtime",
    icon: BarChart3,
  },
  {
    id: "pending-lan",
    label: "Pending & LAN",
    route: "/continuity",
    group: "runtime",
    icon: Radio,
  },
  {
    id: "preferences",
    label: "Settings",
    route: "/settings",
    group: "settings",
    icon: Settings,
  },
];

export type RouteRedirect = {
  path: string;
  redirect: string | { path: string; query: Record<string, string> };
};

/** IA aliases + pre-existing v0.1 settings redirects. Canonical paths stay put. */
export const ROUTE_REDIRECTS: readonly RouteRedirect[] = [
  { path: "/models", redirect: { path: "/settings", query: { section: "provider" } } },
  { path: "/dev-connector", redirect: { path: "/settings", query: { section: "tools" } } },
  { path: "/server", redirect: { path: "/settings", query: { section: "server" } } },
  { path: "/status", redirect: { path: "/settings", query: { section: "overview" } } },
  { path: "/chat", redirect: "/agent-chat" },
  { path: "/workspace", redirect: "/" },
  { path: "/workspace/home", redirect: "/" },
  { path: "/workspace/context", redirect: "/context" },
  { path: "/workspace/docs", redirect: "/docs" },
  { path: "/workspace/notes", redirect: "/notes" },
  { path: "/workspace/canvas", redirect: "/canvas" },
  { path: "/workspace/sprint", redirect: "/sprint" },
  { path: "/agents", redirect: "/agent-sessions" },
  { path: "/agents/sessions", redirect: "/agent-sessions" },
  { path: "/runtime", redirect: "/proxy-providers" },
  { path: "/runtime/providers", redirect: "/proxy-providers" },
  { path: "/runtime/connections", redirect: "/oauth" },
  { path: "/runtime/proxy", redirect: "/proxy-runtime" },
  { path: "/runtime/usage", redirect: "/usage" },
  { path: "/runtime/continuity", redirect: "/continuity" },
  { path: "/proxy/providers", redirect: "/proxy-providers" },
  { path: "/proxy/runtime", redirect: "/proxy-runtime" },
];

export const CANONICAL_PAGE_LABELS: Readonly<Record<string, string>> = {
  "/": "Home",
  "/docs": "Docs",
  "/notes": "Notes",
  "/sprint": "Sprint",
  "/canvas": "Canvas",
  "/agent-chat": "Chat",
  "/agent-sessions": "Sessions",
  "/oauth": "Connections",
  "/proxy-runtime": "HTTP proxy",
  "/proxy-providers": "Providers",
  "/usage": "Usage",
  "/continuity": "Pending & LAN",
  "/context": "Context",
  "/settings": "Settings",
  "/projects/new": "Add Project",
};

export const CHAT_PROVIDER_SETTINGS_ROUTE = {
  path: "/settings",
  query: { section: "provider" },
} as const;

export function itemsForGroup(groupId: NavGroupId): NavItem[] {
  return NAV_ITEMS.filter((item) => item.group === groupId);
}

export function groupsWithPlacement(placement: NavGroup["placement"]): NavGroup[] {
  return NAV_GROUPS.filter((group) => group.placement === placement);
}

export function defaultExpandedTopics(): Record<NavGroupId, boolean> {
  return {
    workspace: true,
    agents: true,
    runtime: false,
    settings: false,
  };
}

export function topicForRoute(path: string): NavGroupId | null {
  if (path === PRIMARY_CHAT.route) return null;
  return NAV_ITEMS.find((item) => item.route === path)?.group ?? null;
}

export function sidebarLinkHrefs(): string[] {
  return [PRIMARY_CHAT.route, ...NAV_ITEMS.map((item) => item.route)];
}
