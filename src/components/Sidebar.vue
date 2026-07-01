<script setup lang="ts">
import {
  Home,
  Circle,
  BarChart3,
  Folder,
  Globe,
  Settings,
  ChevronRight,
  MessageSquare,
} from "lucide-vue-next";
import { ref } from "vue";

const props = defineProps<{
  projects?: string[];
}>();

const projects = props.projects ?? [];

const activeNav = ref<string>("Usage");

const navItems = [
  { label: "Home", icon: Home },
  { label: "Status", icon: Circle, section: "Workspace" },
  { label: "Usage", icon: BarChart3, section: "Workspace" },
  { label: "Models", icon: Folder, section: "Workspace" },
  { label: "Server", icon: Globe, section: "Workspace" },
];

function setActive(label: string) {
  activeNav.value = label;
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
    <div class="flex h-14 items-center gap-2 px-4">
      <span class="text-base font-semibold tracking-tight">OpenRouter</span>
    </div>

    <!-- Nav -->
    <nav class="flex-1 overflow-y-auto px-2 py-2 space-y-4">
      <button
        type="button"
        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)]"
        :class="
          activeNav === 'Home'
            ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
            : 'text-[var(--muted-foreground)]'
        "
        @click="setActive('Home')"
      >
        <Home class="h-4 w-4 flex-shrink-0" />
        <span class="truncate">Home</span>
      </button>

      <!-- Workspace -->
      <div class="space-y-1">
        <div
          class="px-2 text-[11px] font-medium uppercase tracking-wider opacity-70"
          style="color: var(--muted-foreground)"
        >
          Workspace
        </div>
        <button
          v-for="item in navItems.filter((i) => i.section === 'Workspace')"
          :key="item.label"
          type="button"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)]"
          :class="
            activeNav === item.label
              ? 'bg-[var(--sidebar-accent)] text-[var(--sidebar-accent-foreground)]'
              : 'text-[var(--muted-foreground)]'
          "
          @click="setActive(item.label)"
        >
          <component
            :is="item.icon"
            class="h-4 w-4 flex-shrink-0"
            :class="item.label === 'Status' ? 'h-3.5 w-3.5' : ''"
          />
          <span class="truncate">{{ item.label }}</span>
        </button>
      </div>

      <!-- Projects -->
      <div class="space-y-1">
        <div class="flex items-center justify-between px-2">
          <span
            class="text-[11px] font-medium uppercase tracking-wider opacity-70"
            style="color: var(--muted-foreground)"
          >
            Projects
          </span>
          <ChevronRight class="h-3 w-3 opacity-60" />
        </div>
        <div v-if="projects.length === 0" class="px-2 py-1 text-xs opacity-50">
          No projects
        </div>
        <button
          v-for="p in projects"
          :key="p"
          type="button"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)] text-[var(--muted-foreground)]"
          @click="setActive(p)"
        >
          <Folder class="h-3.5 w-3.5 opacity-60 flex-shrink-0" />
          <span class="truncate">{{ p }}</span>
        </button>
      </div>

      <!-- Chats -->
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
        <button
          type="button"
          class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)] text-[var(--muted-foreground)]"
        >
          <MessageSquare class="h-3.5 w-3.5 opacity-60 flex-shrink-0" />
          <span class="truncate">No active chats</span>
        </button>
      </div>
    </nav>

    <!-- Settings -->
    <div class="border-t p-2" style="border-color: var(--border)">
      <button
        type="button"
        class="flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-sm text-left transition-colors hover:bg-[var(--sidebar-accent)] hover:text-[var(--sidebar-accent-foreground)] text-[var(--muted-foreground)]"
        @click="setActive('Settings')"
      >
        <Settings class="h-4 w-4 flex-shrink-0" />
        <span>Settings</span>
      </button>
    </div>
  </aside>
</template>
