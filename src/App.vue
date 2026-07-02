<script setup lang="ts">
import { computed } from "vue";
import { useRoute } from "vue-router";
import Sidebar from "./components/Sidebar.vue";
import { useStore } from "./lib/useStore";

const route = useRoute();
const { currentProject } = useStore();

const pageLabels: Record<string, string> = {
  "/": "Home",
  "/status": "Status",
  "/usage": "Usage",
  "/docs": "Docs",
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
  // Handle edit project page
  if (route.path.startsWith("/projects/") && route.path.endsWith("/edit")) {
    return ["Edit Project"];
  }
  return [currentProject.value.name, page];
});
</script>

<template>
  <div
    class="flex min-h-screen w-full"
    style="background: var(--background); color: var(--foreground)"
  >
    <Sidebar />

    <div class="flex flex-1 flex-col min-w-0">
      <!-- Header with breadcrumb -->
      <header
        class="sticky top-0 z-10 flex h-14 items-center gap-1 border-b px-4 backdrop-blur"
        style="
          border-color: var(--border);
          background: color-mix(in srgb, var(--background) 80%, transparent);
        "
      >
        <nav class="flex items-center gap-1 text-sm">
          <template v-for="(crumb, idx) in breadcrumb" :key="idx">
            <span v-if="idx > 0" class="px-1 opacity-40" style="color: var(--muted-foreground)">/</span>
            <span
              :class="idx === breadcrumb.length - 1 ? '' : 'opacity-60'"
              style="color: var(--muted-foreground)"
            >
              {{ crumb }}
            </span>
          </template>
        </nav>
      </header>

      <main class="flex-1 p-4 md:p-6 lg:p-8">
        <router-view />
      </main>
    </div>
  </div>
</template>
