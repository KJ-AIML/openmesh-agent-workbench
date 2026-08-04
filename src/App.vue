<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { PanelLeft, PanelLeftClose } from "lucide-vue-next";
import Sidebar from "./components/Sidebar.vue";
import Titlebar from "./components/Titlebar.vue";
import CommandPalette from "./components/CommandPalette.vue";
import { useStore } from "./lib/useStore";
import { useSidebarVisibility } from "./lib/useSidebarVisibility";
import { useSidebarPeek } from "./lib/useSidebarPeek";
import { getCommands, type Command } from "./lib/commands";
import * as gitAdapter from "./lib/adapters/gitAdapter";
import { scanConfiguredSessions } from "./lib/scanConfiguredSessions";
import * as terminalAdapter from "./lib/adapters/terminalAdapter";
import type { GitStatus } from "./lib/adapters/types";
import {
  generateSnapshotMarkdown,
  generateAgentContextPrompt,
  generateSnapshotFilename,
  type SnapshotContext,
} from "./lib/snapshot";
import { isMacOS, resolveIsMacOS } from "./lib/adapters/environment";

const route = useRoute();
const router = useRouter();
const {
  isLoading,
  currentProject,
  projectPaths,
  settings,
  projectCommandPresets,
  projectSprint,
  projectTasks,
  projectSessions,
  projectDocs,
  getRecentItemsForProject,
  addRecentItem,
  store,
} = useStore();
/** Persisted pin preference — hover peek must not write this. */
const { sidebarVisible: sidebarPinned, toggleSidebar } = useSidebarVisibility();
const { sidebarPeek, onPeekEnter, onPeekLeave, closePeek } = useSidebarPeek();
const sidebarShown = computed(() => sidebarPinned.value || sidebarPeek.value);

watch(sidebarPinned, (pinned) => {
  if (pinned) closePeek();
});

function onSidebarSlotEnter() {
  if (!sidebarPinned.value) onPeekEnter();
}

function onSidebarSlotLeave() {
  if (!sidebarPinned.value) onPeekLeave();
}

const splashMinimumElapsed = ref(false);
const showSplash = computed(() => isLoading.value || !splashMinimumElapsed.value);
// Prefer value set in main.ts before mount (authoritative Rust OS).
const macOS = ref(
  (window as unknown as { __OPENMESH_IS_MACOS__?: boolean }).__OPENMESH_IS_MACOS__ ??
    isMacOS(),
);

onMounted(async () => {
  macOS.value = await resolveIsMacOS();
  document.documentElement.dataset.platform = macOS.value ? "macos" : "other";
  document.documentElement.classList.toggle("is-macos", macOS.value);
  window.setTimeout(() => {
    splashMinimumElapsed.value = true;
  }, 700);
});
// ─── Cached Git Status ───────────────────────────────────────────────
const cachedGitStatus = ref<GitStatus | null>(null);

async function fetchGitStatus(): Promise<GitStatus | null> {
  if (!currentProject.value) return null;
  const result = await gitAdapter.getGitStatus(currentProject.value.folderPath);
  if (result.success && result.data) {
    cachedGitStatus.value = result.data;
  }
  return result.data || null;
}

// ─── Command Palette State ────────────────────────────────────────────
const paletteVisible = ref(false);
const paletteKey = ref(0); // Force re-render to refresh commands

function openPalette() {
  paletteVisible.value = true;
  paletteKey.value++;
}

function closePalette() {
  paletteVisible.value = false;
}

function togglePalette() {
  if (paletteVisible.value) {
    closePalette();
  } else {
    openPalette();
  }
}

// Global keyboard shortcuts: Ctrl/Cmd+K palette, Ctrl/Cmd+\ sidebar
function handleGlobalKeydown(e: KeyboardEvent) {
  const isCmd = e.metaKey || e.ctrlKey;
  if (isCmd && e.key === "k") {
    e.preventDefault();
    e.stopPropagation();
    togglePalette();
    return;
  }
  if (isCmd && e.key === "\\") {
    e.preventDefault();
    e.stopPropagation();
    toggleSidebar();
  }
}

onMounted(() => {
  document.addEventListener("keydown", handleGlobalKeydown);
});

onUnmounted(() => {
  document.removeEventListener("keydown", handleGlobalKeydown);
});

// ─── Command Context ──────────────────────────────────────────────────
const commands = computed(() =>
  getCommands({
    currentProject: currentProject.value,
    projectPaths: projectPaths.value,
    settings: settings.value,
    commandPresets: projectCommandPresets.value,
    addRecentItem,
    openFolder(path: string) {
      if (path.startsWith("/")) {
        router.push(path);
      }
    },
    async refreshGitStatus() {
      if (!currentProject.value) return;
      await fetchGitStatus();
    },
    async scanSessions() {
      const workspaceCwd = currentProject.value?.folderPath;
      if (!workspaceCwd) return;
      await scanConfiguredSessions(settings.value.sessionDirs, 100, workspaceCwd);
    },
    async createNote() {
      // Navigate to notes page — actual note creation handled there
      router.push("/notes");
    },
    async createSnapshot() {
      if (!currentProject.value) return;
      const projectPath = currentProject.value.folderPath;
      const filename = generateSnapshotFilename();

      // Fetch real git status
      const gitStatus = await fetchGitStatus();

      const context: SnapshotContext = {
        project: currentProject.value,
        settings: settings.value,
        gitStatus,
        recentItems: getRecentItemsForProject(10),
        tasks: projectTasks.value,
        sprint: projectSprint.value,
        sessions: projectSessions.value,
        presets: projectCommandPresets.value,
      };

      const content = generateSnapshotMarkdown(context);
      const result = await store.writeSnapshot(projectPath, filename, content);
      if (!result.success) {
        console.error("[Snapshot] Failed to create snapshot:", result.error);
      }

      if (import.meta.env.DEV) {
        console.log(`[Snapshot] Generated with ${content.length} chars, git: ${gitStatus ? 'yes' : 'no'}, tasks: ${context.tasks.length}, sessions: ${context.sessions.length}`);
      }
    },
    async copyAgentContext() {
      if (!currentProject.value) return;

      // Fetch real git status
      const gitStatus = await fetchGitStatus();

      const context: SnapshotContext = {
        project: currentProject.value,
        settings: settings.value,
        gitStatus,
        recentItems: getRecentItemsForProject(5),
        tasks: projectTasks.value,
        sprint: projectSprint.value,
        sessions: projectSessions.value,
        presets: projectCommandPresets.value,
      };

      const prompt = generateAgentContextPrompt(context);

      if (import.meta.env.DEV) {
        console.log(`[AgentContext] Generated prompt with ${prompt.length} chars`);
        console.log(`[AgentContext] Sections: project=${!!context.project}, git=${!!context.gitStatus}, recent=${context.recentItems.length}, tasks=${context.tasks.length}, sessions=${context.sessions.length}`);
      }

      try {
        await navigator.clipboard.writeText(prompt);
      } catch (e) {
        console.error("[AgentContext] Failed to copy to clipboard:", e);
        alert("Failed to copy to clipboard. Please copy manually:\n\n" + prompt);
      }
    },
    toggleSidebar,
    async launchAgentWithContext(tool: string, label: string) {
      if (!currentProject.value) return;
      const cwd = currentProject.value.terminalDir || currentProject.value.folderPath;
      // Get CLI path override from settings (optional - backend uses default if empty)
      const cliPath =
        tool === "codex"
          ? settings.value.agentClis?.codexPath
          : tool === "claude-code"
          ? settings.value.agentClis?.claudeCodePath
          : settings.value.agentClis?.opencodePath;

      // Fetch real git status before generating prompt
      const gitStatus = await fetchGitStatus();

      // Build context with real workspace state
      const context: SnapshotContext = {
        project: currentProject.value,
        settings: settings.value,
        gitStatus,
        recentItems: getRecentItemsForProject(5),
        tasks: projectTasks.value,
        sprint: projectSprint.value,
        sessions: projectSessions.value,
        presets: projectCommandPresets.value,
      };

      // Generate context prompt with real data
      const prompt = generateAgentContextPrompt(context);

      if (import.meta.env.DEV) {
        console.log(`[LaunchWithContext] Generated prompt for ${label} with ${prompt.length} chars`);
        console.log(`[LaunchWithContext] Context: git=${!!context.gitStatus}, recent=${context.recentItems.length}, tasks=${context.tasks.length}, sessions=${context.sessions.length}, sprint=${!!context.sprint}`);
      }

      // Try to copy to clipboard
      let clipboardSuccess = false;
      try {
        await navigator.clipboard.writeText(prompt);
        clipboardSuccess = true;
      } catch (e) {
        console.error("[LaunchWithContext] Failed to copy to clipboard:", e);
      }

      // Show feedback
      if (clipboardSuccess) {
        if (import.meta.env.DEV) {
          console.log(`[LaunchWithContext] Context copied. Launching ${label}...`);
        }
      } else {
        alert(
          `Failed to copy context to clipboard. Please copy manually if needed.\n\n` +
          `Launching ${label} in ${currentProject.value.name}...\n\n` +
          `Context prompt:\n${prompt}`,
        );
      }

      // Launch the agent CLI
      if (import.meta.env.DEV) {
        console.log(`[LaunchWithContext] Calling openAgentCli with tool=${tool}, cwd=${cwd}, cliPath=${cliPath || 'default'}`);
      }
      
      const launchResult = await terminalAdapter.openAgentCli(tool, cwd, cliPath);
      
      if (import.meta.env.DEV) {
        console.log(`[LaunchWithContext] Launch result:`, launchResult);
      }
      
      if (!launchResult.success) {
        const errorMsg = launchResult.error || `Failed to launch ${label}`;
        console.error(`[LaunchWithContext] Launch failed:`, errorMsg);
        alert(`Failed to launch ${label}:\n\n${errorMsg}\n\nMake sure ${tool} is installed and available in PATH, or set a custom command in Settings.`);
      } else {
        if (import.meta.env.DEV) {
          console.log(`[LaunchWithContext] ${label} launched successfully`);
        }
      }
    },
  }),
);

async function handleCommandExecuted(cmd: Command) {
  try {
    await cmd.run();
  } catch (e) {
    console.error(`[CommandPalette] Failed to run "${cmd.title}":`, e);
  }
}

// ─── Breadcrumb ───────────────────────────────────────────────────────
const pageLabels: Record<string, string> = {
  "/": "Home",
  "/docs": "Docs",
  "/notes": "Notes",
  "/sprint": "Sprint",
  "/agent-chat": "Chat",
  "/agent-sessions": "Agent Sessions",
  "/continuity": "Continuity",
  "/context": "Context",
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

// Expose palette toggle for sidebar
defineExpose({ openPalette });
</script>

<template>
  <!--
    macOS: full-height sidebar under traffic lights (one left layer).
    Titlebar only on the main column — no floating strip over the rail.
  -->
  <div
    class="shell"
    :class="{
      'shell--mac': macOS,
      'shell--sidebar-collapsed': !sidebarPinned,
      'shell--sidebar-peeking': !sidebarPinned && sidebarPeek,
    }"
    style="color: var(--foreground)"
  >
    <Transition name="startup-splash">
      <div v-if="showSplash" class="startup-splash">
        <div class="startup-splash-mark">
          <img src="/logo.svg" alt="OpenMesh" />
        </div>
      </div>
    </Transition>

    <!-- Windows: titlebar still full-width on top -->
    <Titlebar v-if="!macOS" />

    <div class="shell__body">
      <!--
        Collapsed only: thin left-edge hover strip.
        macOS: starts below chrome-top so traffic lights stay clickable.
      -->
      <div
        v-if="!sidebarPinned"
        class="shell__peek-zone"
        aria-hidden="true"
        @mouseenter="onPeekEnter"
        @mouseleave="onPeekLeave"
      />

      <div
        class="shell__sidebar-slot"
        :class="{
          'is-collapsed': !sidebarShown,
          'is-peeking': !sidebarPinned && sidebarPeek,
        }"
        :aria-hidden="!sidebarShown"
        :inert="!sidebarShown || undefined"
        @mouseenter="onSidebarSlotEnter"
        @mouseleave="onSidebarSlotLeave"
      >
        <Sidebar @open-palette="openPalette" />
      </div>

      <div class="shell__main">
        <Titlebar
          v-if="macOS"
          :clearance-for-traffic-lights="!sidebarPinned"
        />

        <header class="shell__crumb">
          <button
            type="button"
            class="shell__sidebar-toggle"
            :aria-pressed="sidebarPinned"
            :aria-label="sidebarPinned ? 'Hide sidebar' : 'Show sidebar'"
            :title="sidebarPinned ? 'Hide sidebar (⌘\\)' : 'Show sidebar (⌘\\)'"
            @click="toggleSidebar"
          >
            <PanelLeftClose v-if="sidebarPinned" class="h-3.5 w-3.5" />
            <PanelLeft v-else class="h-3.5 w-3.5" />
          </button>
          <nav class="flex items-center gap-1.5 text-[12px] min-w-0">
            <template v-for="(crumb, idx) in breadcrumb" :key="idx">
              <span
                v-if="idx > 0"
                class="px-0.5"
                style="color: var(--muted-foreground); opacity: 0.4"
                >/</span
              >
              <span
                :class="
                  idx === breadcrumb.length - 1
                    ? 'font-semibold'
                    : 'font-medium opacity-60'
                "
                style="color: var(--foreground)"
              >
                {{ crumb }}
              </span>
            </template>
          </nav>
        </header>

        <main class="shell__content animate-fade-in">
          <router-view />
        </main>
      </div>
    </div>

    <CommandPalette
      :key="paletteKey"
      :commands="commands"
      :visible="paletteVisible"
      @close="closePalette"
      @executed="handleCommandExecuted"
    />
  </div>
</template>

<style scoped>
.shell {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100%;
  overflow: hidden;
  background: var(--sidebar);
}

.shell__body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  background: var(--sidebar);
}

/*
  Invisible left-edge hit target when the rail is unpinned.
  macOS: inset below chrome-top so ooo traffic lights stay clickable.
*/
.shell__peek-zone {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 10px;
  /* Below peeked rail so nav clicks aren't stolen on the left strip */
  z-index: 35;
  /* Subtle affordance on hover without a permanent stripe */
  background: transparent;
}

.shell--mac .shell__peek-zone {
  top: var(--chrome-top, 44px);
}

/* Clip rail width; keep inner Sidebar at 260px so collapse doesn't reflow nav. */
.shell__sidebar-slot {
  display: flex;
  flex-direction: column;
  flex-shrink: 0;
  width: 260px;
  min-width: 0;
  height: 100%;
  overflow: hidden;
  opacity: 1;
  transition:
    width 200ms ease,
    opacity 180ms ease,
    box-shadow 200ms ease;
}

.shell__sidebar-slot.is-collapsed {
  width: 0;
  max-width: 0;
  flex-basis: 0;
  opacity: 0;
  pointer-events: none;
}

/*
  Unpinned rail is always overlay (out of flex flow) so peek open/close
  never resizes the main column.
*/
.shell--sidebar-collapsed .shell__sidebar-slot {
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  flex-basis: 0;
  z-index: 40;
}

.shell__sidebar-slot.is-peeking {
  width: 260px;
  max-width: 260px;
  opacity: 1;
  pointer-events: auto;
  box-shadow: 8px 0 28px rgba(0, 0, 0, 0.22);
}

.shell__sidebar-slot :deep(.app-sidebar) {
  width: 260px;
  min-width: 260px;
  flex-shrink: 0;
}

.shell__sidebar-slot.is-collapsed :deep(.app-sidebar) {
  min-width: 0;
}

.shell__sidebar-slot.is-peeking :deep(.app-sidebar) {
  min-width: 260px;
}

.shell__main {
  display: flex;
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  background: var(--background);
  /* Full-height seam only — top row is continuous with sidebar rail color via Titlebar */
  border-left: 1px solid var(--border);
}

.shell--sidebar-collapsed .shell__main {
  border-left: none;
}

.shell__crumb {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  height: 44px;
  padding: 0 1.25rem;
  border-bottom: 1px solid var(--border);
  background: var(--background);
  flex-shrink: 0;
}

/* Collapsed macOS: titlebar tabs keep ooo spacer; crumb stays at normal main inset. */
.shell--mac.shell--sidebar-collapsed .shell__main :deep(.tb.tb--mac) {
  padding-left: 0;
}

.shell--mac.shell--sidebar-collapsed .shell__main :deep(.tb__traffic-clearance) {
  flex-basis: var(--mac-traffic-lights-inset, 96px);
  width: var(--mac-traffic-lights-inset, 96px);
  min-width: var(--mac-traffic-lights-inset, 96px);
}

.shell__sidebar-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  flex-shrink: 0;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--surface-2);
  color: var(--muted-foreground);
  cursor: pointer;
  transition:
    background 0.12s ease,
    color 0.12s ease,
    border-color 0.12s ease;
}

.shell__sidebar-toggle:hover {
  background: var(--surface-3);
  color: var(--foreground);
  border-color: var(--border-strong, var(--border));
}

.shell__content {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 1.25rem;
  background: var(--background);
}

@media (prefers-reduced-motion: reduce) {
  .shell__sidebar-slot {
    transition: none;
  }

  .shell__sidebar-slot.is-peeking {
    box-shadow: 4px 0 12px rgba(0, 0, 0, 0.18);
  }
}

/* macOS: sidebar is full window height; lights sit in its top-left */
.shell--mac .shell__body {
  /* body is the full remaining height; sidebar fills it */
}

.startup-splash {
  position: fixed;
  inset: 0;
  z-index: 9999;
  display: grid;
  place-items: center;
  background:
    radial-gradient(circle at 50% 52%, rgba(255, 255, 255, 0.04), transparent 16rem),
    #202020;
}

.startup-splash-mark {
  display: grid;
  place-items: center;
  width: 58px;
  height: 58px;
  border-radius: 18px;
  background: rgba(0, 0, 0, 0.18);
  animation: startup-pulse 1.35s ease-in-out infinite;
}

.startup-splash-mark img {
  width: 42px;
  height: 42px;
  display: block;
}

.startup-splash-enter-active,
.startup-splash-leave-active {
  transition: opacity 180ms ease;
}

.startup-splash-enter-from,
.startup-splash-leave-to {
  opacity: 0;
}

@keyframes startup-pulse {
  0%, 100% {
    transform: scale(1);
    opacity: 0.72;
  }
  50% {
    transform: scale(1.06);
    opacity: 1;
  }
}
</style>
