<script setup lang="ts">
import { ref, onMounted, computed } from "vue";
import { Minus, Square, X, LayoutGrid, MessageSquare } from "lucide-vue-next";
import {
  minimizeWindow,
  toggleMaximizeWindow,
  closeWindow,
  startWindowDrag,
  isMaximized,
} from "../lib/adapters/windowAdapter";
import { isMacOS, isTauriRuntime, resolveIsMacOS } from "../lib/adapters/environment";
import { useStore } from "../lib/useStore";

const { currentProject } = useStore();

const macOS = ref(
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ ??
    isMacOS(),
);
const maximized = ref(false);

const showCustomWindowControls = computed(
  () => isTauriRuntime() && !macOS.value,
);

const projectLabel = computed(() => currentProject.value?.name ?? null);

/** Edit page when a project is open; otherwise add/select flow. */
const projectNavTo = computed(() => {
  const p = currentProject.value;
  if (p?.id) return `/projects/${p.id}/edit`;
  return "/projects/new";
});

onMounted(async () => {
  macOS.value = await resolveIsMacOS();
  document.documentElement.dataset.platform = macOS.value ? "macos" : "other";
  document.documentElement.classList.toggle("is-macos", macOS.value);
  if (showCustomWindowControls.value) {
    maximized.value = await isMaximized();
  }
});

async function handleMinimize(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await minimizeWindow();
}

async function handleToggleMaximize(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await toggleMaximizeWindow();
  setTimeout(async () => {
    maximized.value = await isMaximized();
  }, 100);
}

async function handleClose(e: MouseEvent) {
  e.stopPropagation();
  e.preventDefault();
  await closeWindow();
}

async function handleDrag(e: MouseEvent) {
  if (e.button !== 0) return;
  const t = e.target as HTMLElement | null;
  if (t?.closest("a,button,input,textarea,select,[data-no-drag]")) return;
  e.preventDefault();
  await startWindowDrag();
}
</script>

<template>
  <!--
    macOS shell: titlebar sits ONLY on the main column (right of full-height sidebar).
    Traffic lights live over the sidebar top — same physical layer as the left rail.
  -->
  <header
    class="tb"
    :class="macOS ? 'tb--mac' : 'tb--win'"
    data-tauri-drag-region
    @mousedown="handleDrag"
  >
    <div v-if="!macOS" class="tb__brand" data-no-drag>
      <span class="tb__mark" aria-hidden="true">O</span>
      <span class="tb__name">OpenMesh</span>
    </div>

    <nav class="tb__nav" data-no-drag>
      <router-link
        to="/agent-chat"
        class="tb__tab"
        :class="{ 'is-active': $route.path === '/agent-chat' }"
      >
        <MessageSquare class="tb__tab-icon" />
        Chat
      </router-link>
      <router-link to="/" class="tb__tab" :class="{ 'is-active': $route.path === '/' }">
        <LayoutGrid class="tb__tab-icon" />
        Work
      </router-link>
      <router-link
        to="/docs"
        class="tb__tab"
        :class="{ 'is-active': $route.path === '/docs' }"
      >
        Docs
      </router-link>
      <router-link
        to="/sprint"
        class="tb__tab"
        :class="{ 'is-active': $route.path === '/sprint' }"
      >
        Sprint
      </router-link>
    </nav>

    <div class="tb__spacer" data-tauri-drag-region />

    <div class="tb__end" data-no-drag>
      <!-- Active project name on the right of the nav (replaces generic Projects tab) -->
      <router-link
        v-if="projectLabel"
        :to="projectNavTo"
        class="tb__project"
        :class="{ 'is-active': $route.path.startsWith('/projects') }"
        :title="currentProject?.folderPath || projectLabel"
      >
        <span class="tb__project-dot" aria-hidden="true" />
        <span class="tb__project-name">{{ projectLabel }}</span>
      </router-link>
      <router-link
        v-else
        to="/projects/new"
        class="tb__tab tb__tab--ghost"
        :class="{ 'is-active': $route.path.startsWith('/projects') }"
      >
        Projects
      </router-link>
      <span class="tb__badge">Desktop</span>
      <template v-if="showCustomWindowControls">
        <button class="tb__winbtn" title="Minimize" aria-label="Minimize" @click="handleMinimize">
          <Minus class="h-3.5 w-3.5" />
        </button>
        <button
          class="tb__winbtn"
          :title="maximized ? 'Restore' : 'Maximize'"
          @click="handleToggleMaximize"
        >
          <Square class="h-3 w-3" />
        </button>
        <button class="tb__winbtn tb__winbtn--close" title="Close" @click="handleClose">
          <X class="h-3.5 w-3.5" />
        </button>
      </template>
    </div>
  </header>
</template>

<style scoped>
.tb {
  z-index: 50;
  box-sizing: border-box;
  height: 40px;
  min-height: 40px;
  background: var(--sidebar);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 0 12px;
  user-select: none;
  -webkit-user-select: none;
}

.tb--mac {
  /* Match sidebar-mac-top exactly — one continuous top seam */
  height: var(--chrome-top, 44px);
  min-height: var(--chrome-top, 44px);
  padding: 0 14px 0 12px;
  background: var(--sidebar);
  border-bottom: 1px solid var(--border);
}

.tb__brand {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

.tb__mark {
  width: 18px;
  height: 18px;
  border-radius: 5px;
  display: grid;
  place-items: center;
  font-size: 10px;
  font-weight: 700;
  background: var(--foreground);
  color: var(--background);
}

.tb__name {
  font-size: 13px;
  font-weight: 600;
  letter-spacing: -0.02em;
  color: var(--foreground);
}

.tb__nav {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}

.tb__tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  height: 28px;
  padding: 0 11px;
  border-radius: 7px;
  font-size: 12px;
  font-weight: 500;
  color: var(--muted-foreground);
  text-decoration: none;
  white-space: nowrap;
  transition: background 0.12s ease, color 0.12s ease;
}

.tb__tab-icon {
  width: 12px;
  height: 12px;
  opacity: 0.85;
}

.tb__tab:hover {
  color: var(--foreground);
  background: var(--surface-highlight);
}

.tb__tab.is-active {
  color: var(--foreground);
  background: var(--surface-3);
  font-weight: 600;
}

.tb__tab.is-active .tb__tab-icon {
  opacity: 1;
}

.tb__spacer {
  flex: 1;
  min-width: 12px;
  height: 100%;
}

.tb__end {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  min-width: 0;
}

.tb__project {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 200px;
  min-width: 0;
  height: 28px;
  padding: 0 10px;
  border-radius: 7px;
  background: var(--surface-2);
  border: 1px solid var(--border);
  text-decoration: none;
  color: inherit;
  transition: background 0.12s ease, border-color 0.12s ease;
  cursor: pointer;
}

.tb__project:hover {
  background: var(--surface-3);
  border-color: var(--border-strong, var(--border));
}

.tb__project.is-active {
  background: var(--surface-3);
  border-color: var(--border-strong, var(--border));
}

.tb__project-dot {
  width: 6px;
  height: 6px;
  border-radius: 999px;
  background: var(--accent-green);
  flex-shrink: 0;
}

.tb__project-name {
  font-size: 12px;
  font-weight: 600;
  letter-spacing: -0.01em;
  color: var(--foreground);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  line-height: 1;
}

.tb__tab--ghost {
  /* same as tab; used when no project selected */
}

.tb__badge {
  font-size: 10px;
  font-weight: 600;
  line-height: 1;
  padding: 5px 9px;
  border-radius: 999px;
  color: #4ade80;
  background: rgba(34, 197, 94, 0.12);
  border: 1px solid rgba(34, 197, 94, 0.28);
  flex-shrink: 0;
}

.tb__winbtn {
  width: 36px;
  height: 28px;
  display: grid;
  place-items: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--muted-foreground);
  cursor: pointer;
}

.tb__winbtn:hover {
  background: var(--surface-hover);
  color: var(--foreground);
}

.tb__winbtn--close:hover {
  background: var(--accent-red);
  color: #fff;
}
</style>
