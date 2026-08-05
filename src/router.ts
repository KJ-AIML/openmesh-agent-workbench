import { createRouter, createWebHistory } from "vue-router";
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

const router = createRouter({
	history: createWebHistory(),
	routes: [
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
		// Legacy routes → Settings sections
		{ path: "/models", redirect: { path: "/settings", query: { section: "provider" } } },
		{ path: "/dev-connector", redirect: { path: "/settings", query: { section: "tools" } } },
		{ path: "/server", redirect: { path: "/settings", query: { section: "server" } } },
		{ path: "/status", redirect: { path: "/settings", query: { section: "overview" } } },
		{ path: "/usage", redirect: { path: "/settings", query: { section: "overview" } } },
	],
});

export default router;
