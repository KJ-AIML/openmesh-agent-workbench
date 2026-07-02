<script setup lang="ts">
import { ref } from "vue";
import { Minus, Square, X, LayoutGrid } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const isMaximized = ref(false);

async function toggleMaximize() {
  await appWindow.toggleMaximize();
  isMaximized.value = await appWindow.isMaximized();
}

async function minimize() {
  await appWindow.minimize();
}

async function close() {
  await appWindow.close();
}
</script>

<template>
  <div class="titlebar">
    <!-- Left: Logo + App Name -->
    <div class="titlebar-no-drag flex items-center gap-2">
      <div
        class="flex h-5 w-5 items-center justify-center rounded-md"
        style="background: var(--foreground)"
      >
        <span
          class="text-[10px] font-bold"
          style="color: var(--background); letter-spacing: -0.02em"
          >O</span
        >
      </div>
      <span class="text-[12px] font-semibold" style="color: var(--foreground)">
        OpenMesh
      </span>
    </div>

    <!-- Center: Navigation Tabs -->
    <div class="titlebar-drag flex items-center justify-center gap-1">
      <router-link
        to="/"
        class="titlebar-no-drag flex items-center gap-1.5 rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors"
        :class="
          $route.path === '/'
            ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
        "
        style="text-decoration: none"
      >
        <LayoutGrid class="h-3 w-3" />
        Work
      </router-link>
      <router-link
        to="/projects/new"
        class="titlebar-no-drag rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors"
        :class="
          $route.path.startsWith('/projects')
            ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
        "
        style="text-decoration: none"
      >
        Projects
      </router-link>
      <router-link
        to="/docs"
        class="titlebar-no-drag rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors"
        :class="
          $route.path === '/docs'
            ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
        "
        style="text-decoration: none"
      >
        Docs
      </router-link>
      <router-link
        to="/agent-sessions"
        class="titlebar-no-drag rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors"
        :class="
          $route.path === '/agent-sessions'
            ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
        "
        style="text-decoration: none"
      >
        Agents
      </router-link>
      <router-link
        to="/sprint"
        class="titlebar-no-drag rounded-md px-2.5 py-1 text-[11px] font-medium transition-colors"
        :class="
          $route.path === '/sprint'
            ? 'bg-[var(--surface-3)] text-[var(--foreground)]'
            : 'text-[var(--muted-foreground)] hover:text-[var(--foreground)]'
        "
        style="text-decoration: none"
      >
        Sprint
      </router-link>
    </div>

    <!-- Right: Status + Window Controls -->
    <div class="titlebar-no-drag flex items-center gap-1">
      <!-- Desktop badge -->
      <div
        class="chip chip-success"
        style="font-size: 0.625rem; padding: 0.15rem 0.4rem"
      >
        Desktop
      </div>

      <!-- Window controls -->
      <button
        @click="minimize"
        class="titlebar-btn"
        title="Minimize"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        @click="toggleMaximize"
        class="titlebar-btn"
        :title="isMaximized ? 'Restore' : 'Maximize'"
      >
        <Square class="h-3 w-3" />
      </button>
      <button
        @click="close"
        class="titlebar-btn titlebar-btn-close"
        title="Close"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </div>
</template>
