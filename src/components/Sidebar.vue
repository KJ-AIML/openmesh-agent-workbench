<script setup lang="ts">
import { ref, computed } from "vue";
import {
  Home,
  Circle,
  BarChart3,
  Folder,
  Globe,
  Settings,
  ChevronRight,
  ChevronDown,
  MessageSquare,
  FileText,
  ListTodo,
  Bot,
  Terminal,
  Plus,
  Pencil,
  Trash2,
} from "lucide-vue-next";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import { getRuntimeKind } from "../lib/adapters/environment";

const route = useRoute();
const router = useRouter();
const { projects, currentProjectId, selectProject, deleteProject, addRecentItem } = useStore();

const projectsExpanded = ref(true);
const runtime = computed(() => getRuntimeKind());

const navItems = [
  { label: "Status", icon: Circle, route: "/status" },
  { label: "Usage", icon: BarChart3, route: "/usage" },
  { label: "Docs", icon: FileText, route: "/docs" },
  { label: "Sprint", icon: ListTodo, route: "/sprint" },
  { label: "Models", icon: Folder, route: "/models" },
  { label: "Server", icon: Globe, route: "/server" },
  { label: "Dev Connector", icon: Terminal, route: "/dev-connector" },
  { label: "Agent Sessions", icon: Bot, route: "/agent-sessions" },
];

function isActive(path: string) {
  return route.path === path;
}

function handleProjectClick(projectId: string, projectName: string) {
  selectProject(projectId);
  addRecentItem({ type: "project", title: projectName, projectId, sourceId: projectId });
  router.push("/");
}

function goToAddProject() {
  router.push("/projects/new");
}

function handleDeleteProject(projectId: string, projectName: string) {
  if (confirm(`Delete project "${projectName}"?\n\nThis removes all associated data (docs, sprints, tasks, sessions, presets). Original files on disk are NOT deleted.`)) {
    deleteProject(projectId);
    router.push("/");
  }
}
</script>

<template>
  <aside
    class="hidden md:flex w-[220px] flex-shrink-0 flex-col border-r"
    style="
      border-color: var(--border);
      background: var(--sidebar);
      color: var(--sidebar-foreground);
    "
  >
    <!-- Brand -->
    <router-link to="/" class="flex h-14 items-center gap-2 px-4 no-underline">
      <span class="text-base font-semibold tracking-tight">OpenMesh</span>
    </router-link>

    <!-- Nav -->
    <nav class="flex-1 overflow-y-auto px-2 py-2 space-y-4">
      <!-- Home -->
      <router-link
        to="/"
        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors no-underline hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)]"
        :class="
          isActive('/')
            ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
            : 'text-[var(--muted-foreground)]'
        "
      >
        <Home class="h-4 w-4 flex-shrink-0" />
        <span class="truncate">Home</span>
      </router-link>

      <!-- Workspace -->
      <div class="space-y-1">
        <div
          class="px-2 text-[11px] font-medium uppercase tracking-wider opacity-70"
          style="color: var(--muted-foreground)"
        >
          Workspace
        </div>
        <router-link
          v-for="item in navItems"
          :key="item.label"
          :to="item.route"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors no-underline hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)]"
          :class="
            isActive(item.route)
              ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
              : 'text-[var(--muted-foreground)]'
          "
        >
          <component :is="item.icon" class="h-4 w-4 flex-shrink-0" />
          <span class="truncate">{{ item.label }}</span>
        </router-link>
      </div>

      <!-- Projects -->
      <div class="space-y-1">
        <button
          type="button"
          class="flex w-full items-center justify-between px-2 text-left"
          @click="projectsExpanded = !projectsExpanded"
        >
          <span
            class="text-[11px] font-medium uppercase tracking-wider opacity-70"
            style="color: var(--muted-foreground)"
          >
            Projects
          </span>
          <ChevronDown v-if="projectsExpanded" class="h-3 w-3 opacity-60" />
          <ChevronRight v-else class="h-3 w-3 opacity-60" />
        </button>

        <template v-if="projectsExpanded">
          <div v-if="projects.length === 0" class="px-2 py-1 text-xs opacity-50">
            No projects
          </div>
          <button
            v-for="project in projects"
            :key="project.id"
            type="button"
            class="group flex w-full items-center gap-1 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)]"
            :class="
              currentProjectId === project.id
                ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
                : 'text-[var(--muted-foreground)]'
            "
            @click="handleProjectClick(project.id, project.name)"
          >
            <Folder class="h-3.5 w-3.5 opacity-60 flex-shrink-0" />
            <span class="truncate flex-1">{{ project.name }}</span>
            <span
              class="h-2 w-2 rounded-full flex-shrink-0"
              :style="{ background: project.status === 'active' ? '#22c55e' : '#6b7280' }"
            ></span>
            <span class="hidden group-hover:flex items-center gap-0.5 ml-1">
              <button
                type="button"
                @click.stop="router.push(`/projects/${project.id}/edit`)"
                class="p-0.5 rounded hover:bg-[var(--border)]"
                title="Edit project"
              >
                <Pencil class="h-3 w-3 opacity-60" />
              </button>
              <button
                type="button"
                @click.stop="handleDeleteProject(project.id, project.name)"
                class="p-0.5 rounded hover:bg-[var(--border)]"
                title="Delete project"
              >
                <Trash2 class="h-3 w-3 opacity-60" style="color: #ef4444" />
              </button>
            </span>
          </button>
          <button
            type="button"
            class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)] text-[var(--muted-foreground)]"
            @click="goToAddProject"
          >
            <Plus class="h-3.5 w-3.5 opacity-60 flex-shrink-0" />
            <span class="truncate">Add Project</span>
          </button>
        </template>
      </div>

      <!-- Chats (placeholder) -->
      <div class="space-y-1">
        <div class="flex items-center justify-between px-2">
          <span
            class="text-[11px] font-medium uppercase tracking-wider opacity-70"
            style="color: var(--muted-foreground)"
          >
            Chats
          </span>
          <ChevronRight class="h-3 w-3 opacity-60" />
        </div>
        <div
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left text-[var(--muted-foreground)]"
        >
          <MessageSquare class="h-3.5 w-3.5 opacity-60 flex-shrink-0" />
          <span class="truncate">No active chats</span>
        </div>
      </div>
    </nav>

    <!-- Settings + Runtime -->
    <div class="border-t p-2 space-y-1" style="border-color: var(--border)">
      <div class="flex items-center justify-between px-2 py-1">
        <span
          class="text-[10px] px-1.5 py-0.5 rounded-full"
          :style="{ background: runtime === 'tauri' ? '#22c55e20' : '#3b82f620', color: runtime === 'tauri' ? '#22c55e' : '#3b82f6' }"
        >
          {{ runtime === 'tauri' ? 'Desktop' : 'Web' }}
        </span>
      </div>
      <router-link
        to="/settings"
        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors no-underline hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)] text-[var(--muted-foreground)]"
        :class="
          isActive('/settings')
            ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
            : 'text-[var(--muted-foreground)]'
        "
      >
        <Settings class="h-4 w-4 flex-shrink-0" />
        <span>Settings</span>
      </router-link>
    </div>
  </aside>
</template>
