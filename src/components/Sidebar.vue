<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import {
  Home,
  FileText,
  FileEdit,
  ListTodo,
  Terminal,
  Bot,
  Folder,
  Circle,
  BarChart3,
  Globe,
  Settings,
  ChevronRight,
  ChevronDown,
  Plus,
  Trash2,
  Search,
  GitBranch,
  Zap,
  Network,
} from "lucide-vue-next";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import { isMacOS, resolveIsMacOS } from "../lib/adapters/environment";
import { startWindowDrag } from "../lib/adapters/windowAdapter";

const route = useRoute();
const router = useRouter();
const {
  projectPaths,
  currentProjectPath,
  selectProject,
  deleteProject,
  addRecentItem,
  currentProject,
  store,
  settings,
} = useStore();

const emit = defineEmits<{
  openPalette: [];
}>();

const projectsExpanded = ref(true);
const systemExpanded = ref(false);
const projectNames = ref<Record<string, string>>({});
const macOS = ref(
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ ??
    isMacOS(),
);

async function onMacTopDrag(e: MouseEvent) {
  if (!macOS.value || e.button !== 0) return;
  const t = e.target as HTMLElement | null;
  if (t?.closest("a,button,input,textarea,select,[data-no-drag]")) return;
  e.preventDefault();
  await startWindowDrag();
}

async function loadProjectNames() {
  const names: Record<string, string> = {};
  for (const path of projectPaths.value) {
    try {
      const project = await store.getProject(path);
      names[path] =
        project?.name ||
        path.split("\\").pop() ||
        path.split("/").pop() ||
        path;
    } catch {
      names[path] = path.split("\\").pop() || path.split("/").pop() || path;
    }
  }
  projectNames.value = names;
}

onMounted(async () => {
  macOS.value = await resolveIsMacOS();
  document.documentElement.dataset.platform = macOS.value ? "macos" : "other";
  document.documentElement.classList.toggle("is-macos", macOS.value);
  loadProjectNames();
});

watch(projectPaths, () => {
  loadProjectNames();
});

// Information architecture: Work → Team/Mesh → Agents → System (collapsed)
const workNav = [
  { label: "Home", icon: Home, route: "/" },
  { label: "Sprint", icon: ListTodo, route: "/sprint" },
  { label: "Docs", icon: FileText, route: "/docs" },
  { label: "Notes", icon: FileEdit, route: "/notes" },
  { label: "Context", icon: Search, route: "/context" },
];

const teamMeshNav = [
  { label: "Continuity", icon: Network, route: "/continuity" },
];

const agentsNav = [
  { label: "Agent Sessions", icon: Bot, route: "/agent-sessions" },
  { label: "Models", icon: Circle, route: "/models" },
  { label: "Dev Connector", icon: Terminal, route: "/dev-connector" },
];

const systemNav = [
  { label: "Status", icon: Circle, route: "/status" },
  { label: "Usage", icon: BarChart3, route: "/usage" },
  { label: "Server", icon: Globe, route: "/server" },
];

function isActive(path: string) {
  return route.path === path;
}

async function handleProjectClick(projectPath: string) {
  await selectProject(projectPath);
  if (currentProject.value) {
    await addRecentItem({
      type: "project",
      title: currentProject.value.name,
      projectId: currentProject.value.id,
      sourceId: currentProject.value.id,
    });
  }
  router.push("/");
}

function goToAddProject() {
  router.push("/projects/new");
}

async function handleDeleteProject(projectPath: string) {
  const projectName = projectNames.value[projectPath] || projectPath;
  if (
    confirm(
      `Delete project "${projectName}"?\n\nThis removes all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted.`,
    )
  ) {
    await deleteProject();
    router.push("/");
  }
}
</script>

<template>
  <aside
    class="app-sidebar hidden md:flex flex-col h-full"
    :class="{ 'app-sidebar--mac': macOS }"
    style="
      width: 260px;
      background: var(--sidebar);
      border-right: 1px solid var(--border);
    "
  >
    <!--
      macOS: empty lights clearance only — project name lives in the main nav bar.
    -->
    <div
      v-if="macOS"
      class="sidebar-mac-top"
      data-tauri-drag-region
      @mousedown="onMacTopDrag"
      aria-hidden="true"
    />

    <!-- Windows: project block under caption bar -->
    <div
      v-if="!macOS && currentProject"
      class="px-3 py-2.5 sidebar-project"
      style="border-bottom: 1px solid var(--border)"
      data-no-drag
    >
      <div class="flex items-center gap-2 mb-1.5">
        <div class="h-1.5 w-1.5 rounded-full" style="background: var(--accent-green)"></div>
        <span class="text-[11px] font-semibold truncate" style="color: var(--foreground)">
          {{ currentProject.name }}
        </span>
      </div>
      <div class="text-[10px] truncate" style="color: var(--muted-foreground); opacity: 0.7">
        {{ currentProject.folderPath.split('/').pop() || currentProject.folderPath.split('\\').pop() }}
      </div>
    </div>

    <!-- Search/Command Bar -->
    <div class="px-2.5 py-2">
      <div
        class="flex items-center gap-2 rounded-lg px-2.5 py-2"
        style="
          background: var(--surface-2);
          border: 1px solid var(--border);
          cursor: pointer;
          transition: all 0.15s ease;
        "
        @click="emit('openPalette')"
        @mouseenter="($event.currentTarget as HTMLElement).style.borderColor = 'var(--border-strong)'"
        @mouseleave="($event.currentTarget as HTMLElement).style.borderColor = 'var(--border)'"
      >
        <Search class="h-3.5 w-3.5 flex-shrink-0" style="color: var(--muted-foreground); opacity: 0.6" />
        <span class="flex-1 text-[11px]" style="color: var(--muted-foreground); opacity: 0.6">
          Search or command…
        </span>
        <kbd
          class="text-[9px] font-medium px-1 py-0.5 rounded"
          style="
            background: var(--surface-3);
            color: var(--muted-foreground);
            border: 1px solid var(--border);
          "
          >⌘K</kbd
        >
      </div>
    </div>

    <!-- Navigation -->
    <nav class="flex-1 overflow-y-auto px-2 py-1 space-y-3">
      <!-- PROJECTS -->
      <div>
        <button
          type="button"
          class="flex w-full items-center justify-between px-2 py-1"
          @click="projectsExpanded = !projectsExpanded"
        >
          <span class="sidebar-section-label !mb-0">Projects</span>
          <ChevronDown
            v-if="projectsExpanded"
            class="h-3 w-3"
            style="color: var(--muted-foreground); opacity: 0.6"
          />
          <ChevronRight
            v-else
            class="h-3 w-3"
            style="color: var(--muted-foreground); opacity: 0.6"
          />
        </button>

        <template v-if="projectsExpanded">
          <div
            v-if="projectPaths.length === 0"
            class="px-2 py-2 text-[11px] text-center"
            style="color: var(--muted-foreground); opacity: 0.6"
          >
            No projects
          </div>
          <button
            v-for="projectPath in projectPaths"
            :key="projectPath"
            type="button"
            class="nav-item w-full group"
            :class="{ active: currentProjectPath === projectPath }"
            @click="handleProjectClick(projectPath)"
          >
            <Folder class="h-3.5 w-3.5 flex-shrink-0 opacity-70" />
            <span class="truncate flex-1 text-[12px]">{{
              projectNames[projectPath] || projectPath
            }}</span>
            <span class="hidden group-hover:flex items-center gap-0.5">
              <button
                type="button"
                @click.stop="handleDeleteProject(projectPath)"
                class="p-0.5 rounded transition-colors"
                style="color: var(--muted-foreground)"
                @mouseenter="($event.target as HTMLElement).style.color = 'var(--accent-red)'"
                @mouseleave="($event.target as HTMLElement).style.color = 'var(--muted-foreground)'"
                title="Delete project"
              >
                <Trash2 class="h-3 w-3" />
              </button>
            </span>
          </button>
          <button
            type="button"
            class="nav-item w-full"
            @click="goToAddProject"
          >
            <Plus class="h-3.5 w-3.5 flex-shrink-0 opacity-70" />
            <span class="truncate text-[12px]">Add Project</span>
          </button>
        </template>
      </div>

      <!-- WORK -->
      <div>
        <div class="sidebar-section-label">Work</div>
        <router-link
          v-for="item in workNav"
          :key="item.label"
          :to="item.route"
          class="nav-item no-underline"
          :class="{ active: isActive(item.route) }"
        >
          <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
          <span class="truncate text-[12px]">{{ item.label }}</span>
        </router-link>
      </div>

      <!-- TEAM / MESH -->
      <div>
        <div class="sidebar-section-label">Team / Mesh</div>
        <router-link
          v-for="item in teamMeshNav"
          :key="item.label"
          :to="item.route"
          class="nav-item no-underline"
          :class="{ active: isActive(item.route) }"
        >
          <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
          <span class="truncate text-[12px]">{{ item.label }}</span>
        </router-link>
      </div>

      <!-- AGENTS -->
      <div>
        <div class="sidebar-section-label">Agents</div>
        <router-link
          v-for="item in agentsNav"
          :key="item.label"
          :to="item.route"
          class="nav-item no-underline"
          :class="{ active: isActive(item.route) }"
        >
          <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
          <span class="truncate text-[12px]">{{ item.label }}</span>
        </router-link>
      </div>

      <!-- SYSTEM (collapsed by default) -->
      <div>
        <button
          type="button"
          class="flex w-full items-center justify-between px-2 py-1"
          @click="systemExpanded = !systemExpanded"
        >
          <span class="sidebar-section-label !mb-0">System</span>
          <ChevronDown
            v-if="systemExpanded"
            class="h-3 w-3"
            style="color: var(--muted-foreground); opacity: 0.6"
          />
          <ChevronRight
            v-else
            class="h-3 w-3"
            style="color: var(--muted-foreground); opacity: 0.6"
          />
        </button>
        <template v-if="systemExpanded">
          <router-link
            v-for="item in systemNav"
            :key="item.label"
            :to="item.route"
            class="nav-item no-underline"
            :class="{ active: isActive(item.route) }"
          >
            <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
            <span class="truncate text-[12px]">{{ item.label }}</span>
          </router-link>
        </template>
      </div>
    </nav>

    <!-- Bottom: Settings -->
    <div
      class="px-2 py-2"
      style="border-top: 1px solid var(--border)"
      data-no-drag
    >
      <router-link
        to="/settings"
        class="nav-item no-underline"
        :class="{ active: isActive('/settings') }"
      >
        <Settings class="h-3.5 w-3.5 flex-shrink-0" />
        <span class="text-[12px]">Settings</span>
      </router-link>
    </div>
  </aside>
</template>

<style scoped>
/*
  Same height as main titlebar (--chrome-top).
  Content starts after traffic-light cluster — fills the dead gap.
*/
/* Match main titlebar height; empty under lights (project is in nav bar). */
.sidebar-mac-top {
  height: var(--chrome-top, 44px);
  min-height: var(--chrome-top, 44px);
  flex-shrink: 0;
  width: 100%;
  box-sizing: border-box;
  background: var(--sidebar);
  border-bottom: 1px solid var(--border);
}

.app-sidebar--mac {
  background: var(--sidebar);
}
</style>
