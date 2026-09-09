import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import { ROUTE_REDIRECTS } from "./lib/navigation";
import HomePage from "./pages/HomePage.vue";
import SettingsPage from "./pages/SettingsPage.vue";
import AddProjectPage from "./pages/AddProjectPage.vue";
import EditProjectPage from "./pages/EditProjectPage.vue";
import DocsPage from "./pages/DocsPage.vue";
import SprintPage from "./pages/SprintPage.vue";
import AgentSessionsPage from "./pages/AgentSessionsPage.vue";
import AgentChatPage from "./pages/AgentChatPage.vue";
import NotesPage from "./pages/NotesPage.vue";
import ContextPage from "./pages/ContextPage.vue";
import ContinuityPage from "./pages/ContinuityPage.vue";
import CanvasPage from "./pages/CanvasPage.vue";
import OAuthPage from "./pages/OAuthPage.vue";
import ProxyRuntimePage from "./pages/ProxyRuntimePage.vue";
import ProxyProvidersPage from "./pages/ProxyProvidersPage.vue";
import UsageAnalyticsPage from "./pages/UsageAnalyticsPage.vue";

export const routes: RouteRecordRaw[] = [
	{ path: "/", name: "home", component: HomePage },
	{ path: "/settings", name: "settings", component: SettingsPage },
	{ path: "/projects/new", name: "add-project", component: AddProjectPage },
	{ path: "/projects/:id/edit", name: "edit-project", component: EditProjectPage },
	{ path: "/docs", name: "docs", component: DocsPage },
	{ path: "/sprint", name: "sprint", component: SprintPage },
	{
		path: "/agent-chat",
		name: "agent-chat",
		component: AgentChatPage,
	},
	{
		path: "/agent-sessions",
		name: "agent-sessions",
		component: AgentSessionsPage,
	},
	{
		path: "/oauth",
		name: "oauth",
		component: OAuthPage,
	},
	{
		path: "/proxy-runtime",
		name: "proxy-runtime",
		component: ProxyRuntimePage,
	},
	{
		path: "/proxy-providers",
		name: "proxy-providers",
		component: ProxyProvidersPage,
	},
	{
		path: "/context",
		name: "context",
		component: ContextPage,
	},
	{
		path: "/continuity",
		name: "continuity",
		component: ContinuityPage,
	},
	{
		path: "/notes",
		name: "notes",
		component: NotesPage,
	},
	{
		path: "/canvas",
		name: "canvas",
		component: CanvasPage,
	},
	{
		path: "/usage",
		name: "usage",
		component: UsageAnalyticsPage,
	},
	...ROUTE_REDIRECTS.map((entry) => ({
		path: entry.path,
		redirect: entry.redirect,
	})),
];

const router = createRouter({
	history: createWebHistory(),
	routes,
});

export default router;
