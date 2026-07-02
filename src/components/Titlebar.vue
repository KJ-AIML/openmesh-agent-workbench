<script setup lang="ts">
import { ref, onMounted } from "vue";
import { Minus, Square, X, LayoutGrid } from "lucide-vue-next";
import {
  minimizeWindow,
  toggleMaximizeWindow,
  closeWindow,
  startWindowDrag,
  isMaximized,
} from "../lib/adapters/windowAdapter";

const maximized = ref(false);

onMounted(async () => {
  console.log("[Titlebar] mounted");
  maximized.value = await isMaximized();
  console.log("[Titlebar] isMaximized:", maximized.value);
});

async function handleMinimize(e: MouseEvent) {
  console.log("[Titlebar] minimize button clicked");
  e.stopPropagation();
  e.preventDefault();
  const result = await minimizeWindow();
  console.log("[Titlebar] minimize result:", result);
}

async function handleToggleMaximize(e: MouseEvent) {
  console.log("[Titlebar] maximize button clicked");
  e.stopPropagation();
  e.preventDefault();
  const result = await toggleMaximizeWindow();
  console.log("[Titlebar] toggleMaximize result:", result);
  setTimeout(async () => {
    maximized.value = await isMaximized();
  }, 100);
}

async function handleClose(e: MouseEvent) {
  console.log("[Titlebar] close button clicked");
  e.stopPropagation();
  e.preventDefault();
  const result = await closeWindow();
  console.log("[Titlebar] close result:", result);
}

async function handleDragAreaMouseDown(e: MouseEvent) {
  console.log("[Titlebar] drag area mousedown");
  e.preventDefault();
  const result = await startWindowDrag();
  console.log("[Titlebar] drag result:", result);
}
</script>

<template>
  <!--
    Titlebar structure:
    - Dedicated drag area in the center only
    - Buttons/nav are OUTSIDE the drag area
    - No -webkit-app-region CSS anywhere
  -->
  <header class="titlebar">
    <!-- Left: Logo + App Name (no drag) -->
    <div class="titlebar-left" data-no-drag>
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

    <!-- Center: Navigation Tabs (no drag) -->
    <nav class="titlebar-nav" data-no-drag>
      <router-link
        to="/"
        class="titlebar-tab"
        :class="{ 'titlebar-tab-active': $route.path === '/' }"
      >
        <LayoutGrid class="h-3 w-3" />
        Work
      </router-link>
      <router-link
        to="/projects/new"
        class="titlebar-tab"
        :class="{ 'titlebar-tab-active': $route.path.startsWith('/projects') }"
      >
        Projects
      </router-link>
      <router-link
        to="/docs"
        class="titlebar-tab"
        :class="{ 'titlebar-tab-active': $route.path === '/docs' }"
      >
        Docs
      </router-link>
      <router-link
        to="/agent-sessions"
        class="titlebar-tab"
        :class="{ 'titlebar-tab-active': $route.path === '/agent-sessions' }"
      >
        Agents
      </router-link>
      <router-link
        to="/sprint"
        class="titlebar-tab"
        :class="{ 'titlebar-tab-active': $route.path === '/sprint' }"
      >
        Sprint
      </router-link>
    </nav>

    <!-- Dedicated drag area — ONLY this div triggers window dragging -->
    <div class="titlebar-drag-area" @mousedown="handleDragAreaMouseDown"></div>

    <!-- Right: Status + Window Controls (no drag) -->
    <div class="titlebar-right" data-no-drag>
      <!-- Desktop badge -->
      <div
        class="chip chip-success"
        style="font-size: 0.625rem; padding: 0.15rem 0.4rem"
        data-no-drag
      >
        Desktop
      </div>

      <!-- Window controls -->
      <button
        class="titlebar-btn"
        title="Minimize"
        aria-label="Minimize window"
        data-no-drag
        @click="handleMinimize"
      >
        <Minus class="h-3.5 w-3.5" />
      </button>
      <button
        class="titlebar-btn"
        :title="maximized ? 'Restore' : 'Maximize'"
        :aria-label="maximized ? 'Restore window' : 'Maximize window'"
        data-no-drag
        @click="handleToggleMaximize"
      >
        <Square class="h-3 w-3" />
      </button>
      <button
        class="titlebar-btn titlebar-btn-close"
        title="Close"
        aria-label="Close window"
        data-no-drag
        @click="handleClose"
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  z-index: 100;
  height: 40px;
  background: var(--surface-1);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  padding: 0 0.75rem;
  user-select: none;
  cursor: default;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  flex-shrink: 0;
  position: relative;
  z-index: 2;
  pointer-events: auto;
}

.titlebar-nav {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  flex-shrink: 0;
  position: relative;
  z-index: 2;
  pointer-events: auto;
}

.titlebar-tab {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  padding: 0.25rem 0.625rem;
  border-radius: 6px;
  font-size: 11px;
  font-weight: 500;
  color: var(--muted-foreground);
  text-decoration: none;
  transition: all 0.15s ease;
}

.titlebar-tab:hover {
  color: var(--foreground);
  background: var(--surface-highlight);
}

.titlebar-tab-active {
  color: var(--foreground);
  background: var(--surface-3);
  font-weight: 600;
}

/* Dedicated drag area — takes up remaining space between nav and controls */
.titlebar-drag-area {
  flex: 1;
  height: 100%;
  cursor: default;
  min-width: 20px;
}

.titlebar-right {
  display: flex;
  align-items: center;
  gap: 0.375rem;
  flex-shrink: 0;
  position: relative;
  z-index: 2;
  pointer-events: auto;
}

.titlebar-btn {
  width: 32px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
  transition: all 0.15s ease;
  padding: 0;
  pointer-events: auto;
}

.titlebar-btn:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.titlebar-btn-close:hover {
  background: var(--accent-red);
  color: white;
}

.titlebar-btn:focus-visible {
  outline: 2px solid var(--accent-blue);
  outline-offset: -2px;
}
</style>
