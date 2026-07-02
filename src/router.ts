import { createRouter, createWebHistory } from "vue-router";
import HomePage from "./pages/HomePage.vue";
import StatusPage from "./pages/StatusPage.vue";
import UsagePage from "./pages/UsagePage.vue";
import ModelsPage from "./pages/ModelsPage.vue";
import ServerPage from "./pages/ServerPage.vue";
import SettingsPage from "./pages/SettingsPage.vue";
import AddProjectPage from "./pages/AddProjectPage.vue";
import EditProjectPage from "./pages/EditProjectPage.vue";
import DocsPage from "./pages/DocsPage.vue";
import SprintPage from "./pages/SprintPage.vue";
import AgentSessionsPage from "./pages/AgentSessionsPage.vue";
import DevConnectorPage from "./pages/DevConnectorPage.vue";

const router = createRouter({
	history: createWebHistory(),
	routes: [
		{ path: "/", name: "home", component: HomePage },
		{ path: "/status", name: "status", component: StatusPage },
		{ path: "/usage", name: "usage", component: UsagePage },
		{ path: "/models", name: "models", component: ModelsPage },
		{ path: "/server", name: "server", component: ServerPage },
		{ path: "/settings", name: "settings", component: SettingsPage },
		{ path: "/projects/new", name: "add-project", component: AddProjectPage },
		{ path: "/projects/:id/edit", name: "edit-project", component: EditProjectPage },
		{ path: "/docs", name: "docs", component: DocsPage },
		{ path: "/sprint", name: "sprint", component: SprintPage },
		{
			path: "/agent-sessions",
			name: "agent-sessions",
			component: AgentSessionsPage,
		},
		{
			path: "/dev-connector",
			name: "dev-connector",
			component: DevConnectorPage,
		},
	],
});

export default router;
