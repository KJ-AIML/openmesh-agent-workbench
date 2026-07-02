<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import Sidebar from "./components/Sidebar.vue";
import Titlebar from "./components/Titlebar.vue";
import { useStore } from "./lib/useStore";

const route = useRoute();
const { currentProject } = useStore();

const pageLabels: Record<string, string> = {
  "/": "Home",
  "/status": "Status",
  "/usage": "Usage",
  "/docs": "Docs",
  "/notes": "Notes",
  "/sprint": "Sprint",
  "/models": "Models",
  "/server": "Server",
  "/dev-connector": "Dev Connector",
  "/agent-sessions": "Agent Sessions",
  "/settings": "Settings",
  "/projects/new": "Add Project",
};

const breadcrumb = computed(() => {
  const page = pageLabels[route.path] ?? "Page";
  if (route.path === "/" || !currentProject.value) return [page];
  if (page === "Home") return [currentProject.value.name];
  if (route.path.startsWith("/projects/") && route.path.endsWith("/edit")) {
    return ["Edit Project"];
  }
  return [currentProject.value.name, page];
});
</script>

<template>
  <div
    class="flex flex-col h-screen w-full overflow-hidden"
    style="background: var(--background); color: var(--foreground)"
  >
    <!-- Custom Titlebar -->
    <Titlebar />

    <div class="flex flex-1 min-h-0">
      <!-- Sidebar -->
      <Sidebar />

      <!-- Main Content -->
      <div class="flex flex-1 flex-col min-w-0">
        <!-- Breadcrumb header -->
        <header
          class="flex h-11 items-center px-5"
          style="border-bottom: 1px solid var(--border)"
        >
          <nav class="flex items-center gap-1.5 text-[12px]">
            <template v-for="(crumb, idx) in breadcrumb" :key="idx">
              <span
                v-if="idx > 0"
                class="px-0.5"
                style="color: var(--muted-foreground); opacity: 0.4"
                >/</span
              >
              <span
                :class="idx === breadcrumb.length - 1 ? 'font-semibold' : 'font-medium opacity-60'"
                style="color: var(--foreground)"
              >
                {{ crumb }}
              </span>
            </template>
          </nav>
        </header>

        <!-- Main content area -->
        <main class="flex-1 overflow-y-auto p-5 animate-fade-in">
          <router-view />
        </main>
      </div>
    </div>
  </div>
</template>
