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
import {
  generateSnapshotMarkdown,
  generateAgentContextPrompt,
  generateSnapshotFilename,
  type SnapshotContext,
} from "./lib/snapshot";

const route = useRoute();
const router = useRouter();
const {
  currentProject,
  projectPaths,
  settings,
  projectCommandPresets,
  addRecentItem,
  store,
} = useStore();

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
      await gitAdapter.getGitStatus(currentProject.value.folderPath);
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
      const content = generateSnapshotMarkdown({
        project: currentProject.value,
        settings: settings.value,
        gitStatus: null, // Will be fetched if needed
        recentItems: [], // Will be populated from store
        tasks: [], // Will be populated from store
        sprint: null, // Will be populated from store
        sessions: [], // Will be populated from store
        presets: projectCommandPresets.value,
      });
      const result = await store.writeSnapshot(projectPath, filename, content);
      if (!result.success) {
        console.error("[Snapshot] Failed to create snapshot:", result.error);
      }
    },
    async copyAgentContext() {
      if (!currentProject.value) return;
      const prompt = generateAgentContextPrompt({
        project: currentProject.value,
        settings: settings.value,
        gitStatus: null,
        recentItems: [],
        tasks: [],
        sprint: null,
        sessions: [],
        presets: projectCommandPresets.value,
      });
      try {
        await navigator.clipboard.writeText(prompt);
      } catch (e) {
        console.error("[AgentContext] Failed to copy to clipboard:", e);
        // Fallback: show in alert
        alert("Failed to copy to clipboard. Please copy manually:\n\n" + prompt);
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
