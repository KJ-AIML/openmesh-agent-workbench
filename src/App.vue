<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import Sidebar from "./components/Sidebar.vue";
import Titlebar from "./components/Titlebar.vue";
import CommandPalette from "./components/CommandPalette.vue";
import { useStore } from "./lib/useStore";
import { getCommands, type Command } from "./lib/commands";
import * as gitAdapter from "./lib/adapters/gitAdapter";
import * as agentSessionAdapter from "./lib/adapters/agentSessionAdapter";
import * as terminalAdapter from "./lib/adapters/terminalAdapter";
import type { GitStatus } from "./lib/adapters/types";
import {
  generateSnapshotMarkdown,
  generateAgentContextPrompt,
  generateSnapshotFilename,
  type SnapshotContext,
} from "./lib/snapshot";

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

const splashMinimumElapsed = ref(false);
const showSplash = computed(() => isLoading.value || !splashMinimumElapsed.value);

onMounted(() => {
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

// Global keyboard shortcut: Ctrl+K / Cmd+K
function handleGlobalKeydown(e: KeyboardEvent) {
  const isCmd = e.metaKey || e.ctrlKey;
  if (isCmd && e.key === "k") {
    e.preventDefault();
    e.stopPropagation();
    togglePalette();
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
      if (!settings.value.sessionDirs) return;
      const allScanned: any[] = [];
      if (
        settings.value.sessionDirs.codexEnabled &&
        settings.value.sessionDirs.codexDir
      ) {
        const result = await agentSessionAdapter.scanAgentSessionDirectory(
          "codex",
          settings.value.sessionDirs.codexDir,
          100,
        );
        if (result.success && result.data) allScanned.push(...result.data);
      }
      if (
        settings.value.sessionDirs.claudeCodeEnabled &&
        settings.value.sessionDirs.claudeCodeDir
      ) {
        const result = await agentSessionAdapter.scanAgentSessionDirectory(
          "claude-code",
          settings.value.sessionDirs.claudeCodeDir,
          100,
        );
        if (result.success && result.data) allScanned.push(...result.data);
      }
      if (
        settings.value.sessionDirs.opencodeEnabled &&
        settings.value.sessionDirs.opencodeDir
      ) {
        const result = await agentSessionAdapter.scanAgentSessionDirectory(
          "opencode",
          settings.value.sessionDirs.opencodeDir,
          100,
        );
        if (result.success && result.data) allScanned.push(...result.data);
      }
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

// Expose palette toggle for sidebar
defineExpose({ openPalette });
</script>

<template>
  <div
    class="flex flex-col h-screen w-full overflow-hidden"
    style="background: var(--background); color: var(--foreground)"
  >
    <!-- Custom Titlebar -->
    <Titlebar />

    <Transition name="startup-splash">
      <div v-if="showSplash" class="startup-splash">
        <div class="startup-splash-mark">
          <img src="/logo.svg" alt="OpenMesh" />
        </div>
      </div>
    </Transition>

    <div class="flex flex-1 min-h-0">
      <!-- Sidebar -->
      <Sidebar @open-palette="openPalette" />

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

        <!-- Main content area -->
        <main class="flex-1 overflow-y-auto p-5 animate-fade-in">
          <router-view />
        </main>
      </div>
    </div>

    <!-- Command Palette -->
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
