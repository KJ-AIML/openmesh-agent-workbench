<script setup lang="ts">
import { computed, onMounted, ref, watch, type Component } from "vue";
import {
  BarChart3,
  Bot,
  ChevronDown,
  ChevronRight,
  FileEdit,
  FileText,
  Folder,
  Home,
  KeyRound,
  ListTodo,
  MessageSquare,
  Network,
  Plus,
  Search,
  Server,
  Settings,
  Trash2,
} from "lucide-vue-next";
import { useRoute, useRouter } from "vue-router";
import { useStore } from "../lib/useStore";
import { isMacOS, resolveIsMacOS } from "../lib/adapters/environment";
import { startWindowDrag } from "../lib/adapters/windowAdapter";
import AccountSwitcher from "./AccountSwitcher.vue";

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
} = useStore();

const emit = defineEmits<{
  openPalette: [];
}>();

type NavigationTopicId = "overview" | "build" | "agents" | "network" | "settings";
type NavigationItem = {
  label: string;
  icon: Component;
  route: string;
};
type NavigationTopic = {
  id: NavigationTopicId;
  label: string;
  items: NavigationItem[];
};

const navigationTopics: NavigationTopic[] = [
  {
    id: "overview",
    label: "Overview",
    items: [
      { label: "Home", icon: Home, route: "/" },
      { label: "Context", icon: Search, route: "/context" },
    ],
  },
  {
    id: "build",
    label: "Build",
    items: [
      { label: "Sprint", icon: ListTodo, route: "/sprint" },
      { label: "Docs", icon: FileText, route: "/docs" },
      { label: "Notes", icon: FileEdit, route: "/notes" },
      { label: "Canvas", icon: Network, route: "/canvas" },
    ],
  },
  {
    id: "agents",
    label: "Agents",
    items: [{ label: "Sessions", icon: Bot, route: "/agent-sessions" }],
  },
  {
    id: "network",
    label: "Network",
    items: [
      { label: "Continuity", icon: Network, route: "/continuity" },
      { label: "OAuth Connections", icon: KeyRound, route: "/oauth" },
      { label: "Proxy Runtime", icon: Server, route: "/proxy-runtime" },
      { label: "Provider Configuration", icon: KeyRound, route: "/proxy-providers" },
      { label: "Usage", icon: BarChart3, route: "/usage" },
    ],
  },
  {
    id: "settings",
    label: "Settings",
    items: [{ label: "Settings", icon: Settings, route: "/settings" }],
  },
];

const mainTopics = navigationTopics.filter((topic) => topic.id !== "settings");
const settingsTopic = navigationTopics.find((topic) => topic.id === "settings")!;
const projectsExpanded = ref(true);
const expandedTopics = ref<Record<NavigationTopicId, boolean>>({
  overview: true,
  build: false,
  agents: true,
  network: false,
  settings: true,
});
const projectNames = ref<Record<string, string>>({});
const macOS = ref(
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ ??
    isMacOS(),
);

const activeTopicId = computed<NavigationTopicId | null>(() => {
  if (route.path === "/agent-chat") return "agents";
  const topic = navigationTopics.find((candidate) =>
    candidate.items.some((item) => isActive(item.route)),
  );
  return topic?.id ?? null;
});

watch(
  activeTopicId,
  (topicId) => {
    if (topicId) expandedTopics.value[topicId] = true;
  },
  { immediate: true },
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

function isActive(path: string) {
  return route.path === path;
}

function isTopicExpanded(topicId: NavigationTopicId) {
  return expandedTopics.value[topicId];
}

function toggleTopic(topicId: NavigationTopicId) {
  expandedTopics.value[topicId] = !expandedTopics.value[topicId];
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
  router.push("/agent-chat");
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
    <nav class="sidebar-navigation flex-1 overflow-y-auto px-2 py-1" aria-label="Workspace navigation">
      <!-- Project switcher -->
      <section class="sidebar-topic sidebar-topic--projects" data-topic="projects">
        <button
          type="button"
          class="sidebar-topic-toggle"
          :aria-expanded="projectsExpanded"
          aria-controls="sidebar-projects-panel"
          data-topic-toggle="projects"
          @click="projectsExpanded = !projectsExpanded"
        >
          <span class="sidebar-topic-label">Projects</span>
          <ChevronDown v-if="projectsExpanded" class="h-3 w-3" aria-hidden="true" />
          <ChevronRight v-else class="h-3 w-3" aria-hidden="true" />
        </button>

        <div
          v-show="projectsExpanded"
          id="sidebar-projects-panel"
          class="sidebar-topic-items"
        >
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
          <button type="button" class="nav-item w-full" @click="goToAddProject">
            <Plus class="h-3.5 w-3.5 flex-shrink-0 opacity-70" />
            <span class="truncate text-[12px]">Add Project</span>
          </button>
        </div>
      </section>

      <!-- Primary Chat -->
      <div class="sidebar-chat" data-navigation-topic="agents">
        <router-link
          to="/agent-chat"
          class="chat-primary no-underline"
          :class="{ 'is-active': isActive('/agent-chat') }"
          aria-label="Chat"
        >
          <MessageSquare class="h-4 w-4 flex-shrink-0" />
          <span class="chat-primary__label">Chat</span>
          <span class="chat-primary__hint">workspace</span>
        </router-link>
      </div>

      <section
        v-for="topic in mainTopics"
        :key="topic.id"
        class="sidebar-topic"
        :data-topic="topic.id"
      >
        <button
          type="button"
          class="sidebar-topic-toggle"
          :class="{ 'is-active': activeTopicId === topic.id }"
          :aria-expanded="isTopicExpanded(topic.id)"
          :aria-controls="`sidebar-topic-${topic.id}`"
          :data-topic-toggle="topic.id"
          @click="toggleTopic(topic.id)"
        >
          <span class="sidebar-topic-label">{{ topic.label }}</span>
          <ChevronDown v-if="isTopicExpanded(topic.id)" class="h-3 w-3" aria-hidden="true" />
          <ChevronRight v-else class="h-3 w-3" aria-hidden="true" />
        </button>

        <div
          v-show="isTopicExpanded(topic.id)"
          :id="`sidebar-topic-${topic.id}`"
          class="sidebar-topic-items"
        >
          <router-link
            v-for="item in topic.items"
            :key="item.route"
            :to="item.route"
            class="nav-item no-underline"
            :class="{ active: isActive(item.route) }"
            :aria-current="isActive(item.route) ? 'page' : undefined"
          >
            <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
            <span class="truncate text-[12px]">{{ item.label }}</span>
          </router-link>
        </div>
      </section>
      <!-- Account Switcher -->
      <div class="sidebar-account-switcher">
        <AccountSwitcher />
      </div>
    </nav>

    <!-- Settings -->
    <div class="sidebar-settings px-2 py-2" data-no-drag>
      <section class="sidebar-topic" data-topic="settings">
        <button
          type="button"
          class="sidebar-topic-toggle"
          :class="{ 'is-active': activeTopicId === settingsTopic.id }"
          :aria-expanded="isTopicExpanded(settingsTopic.id)"
          :aria-controls="`sidebar-topic-${settingsTopic.id}`"
          data-topic-toggle="settings"
          @click="toggleTopic(settingsTopic.id)"
        >
          <span class="sidebar-topic-label">{{ settingsTopic.label }}</span>
          <ChevronDown v-if="isTopicExpanded(settingsTopic.id)" class="h-3 w-3" aria-hidden="true" />
          <ChevronRight v-else class="h-3 w-3" aria-hidden="true" />
        </button>
        <div
          v-show="isTopicExpanded(settingsTopic.id)"
          :id="`sidebar-topic-${settingsTopic.id}`"
          class="sidebar-topic-items"
        >
          <router-link
            v-for="item in settingsTopic.items"
            :key="item.route"
            :to="item.route"
            class="nav-item no-underline"
            :class="{ active: isActive(item.route) }"
            :aria-current="isActive(item.route) ? 'page' : undefined"
          >
            <component :is="item.icon" class="h-3.5 w-3.5 flex-shrink-0" />
            <span class="truncate text-[12px]">{{ item.label }}</span>
          </router-link>
        </div>
      </section>
    </div>
  </aside>
</template>

<style scoped>
.app-sidebar {
  width: 260px;
  background: var(--sidebar);
  border-right: 1px solid var(--border);
}

/*
  Same height as main titlebar (--chrome-top).
  Content starts after traffic-light cluster — fills the dead gap.
*/
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

.sidebar-navigation {
  display: flex;
  flex-direction: column;
  gap: 0.3rem;
}

.sidebar-topic--projects {
  margin-bottom: 0.15rem;
}

.sidebar-chat {
  padding: 0.15rem 0.25rem 0.35rem;
}

.sidebar-settings {
  border-top: 1px solid var(--border);
}

.sidebar-account-switcher {
  padding: 0.25rem 0;
  border-top: 1px solid var(--border);
  margin-top: 0.2rem;
}

.sidebar-topic-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  min-height: 28px;
  padding: 0 0.75rem;
  border-radius: 8px;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.sidebar-topic-toggle:hover,
.sidebar-topic-toggle.is-active {
  background: var(--surface-highlight);
  color: var(--foreground);
}

.sidebar-topic-toggle svg {
  opacity: 0.6;
}

.sidebar-topic-label {
  font-size: 0.625rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.sidebar-topic-items {
  display: flex;
  flex-direction: column;
  gap: 0.1rem;
  padding-top: 0.1rem;
}

/* Match .nav-item language; slightly taller as the primary entry */
.chat-primary {
  display: flex;
  align-items: center;
  gap: 0.625rem;
  width: 100%;
  height: 44px;
  padding: 0 0.75rem;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  font-size: 0.8125rem;
  font-weight: 500;
  letter-spacing: -0.01em;
  transition: all 0.15s ease;
}

.chat-primary:hover {
  background: var(--surface-highlight);
  color: var(--foreground);
  border-color: var(--border-strong);
}

.chat-primary.is-active {
  background: var(--surface-3);
  color: var(--foreground);
  font-weight: 600;
  border-color: var(--border-strong);
}

.chat-primary__label {
  font-size: inherit;
  font-weight: inherit;
}

.chat-primary__hint {
  margin-left: auto;
  font-size: 0.625rem;
  font-weight: 600;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--muted-foreground);
  opacity: 0.7;
}
</style>
