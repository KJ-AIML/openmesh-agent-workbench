<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useStore } from "../lib/useStore";
import type { ScannedSession } from "../lib/adapters/types";
import { scanConfiguredSessions } from "../lib/scanConfiguredSessions";
import { Bot, Scan, Star, Trash2, FolderOpen, Play } from "lucide-vue-next";
import AgentToolIcon from "../components/AgentToolIcon.vue";
import {
  AGENT_TOOL_FILTERS,
  agentToolLabel,
} from "../lib/agentToolIcons";
import { openAgentCli } from "../lib/adapters/terminalAdapter";

const {
  currentProject,
  projectSessions,
  projectTasks,
  deleteAgentSession,
  updateAgentSession,
  addRecentItem,
  settings,
} = useStore();

const resumeError = ref<string | null>(null);
const resumeBusy = ref(false);

const selectedSessionId = ref<string | null>(null);
const toolFilter = ref<string>("all");
const scannedSessions = ref<ScannedSession[]>([]);
const isScanning = ref(false);
const lastScanTime = ref<string | null>(null);
const starredScanIds = ref<Set<string>>(new Set());
const scanError = ref<string | null>(null);

/** Saved project sessions. Prefer scanned disk sessions in the list order. */
function toolMatches(filter: string, tool: string): boolean {
  if (filter === "all") return true;
  if (filter === "claude") return tool === "claude" || tool === "claude-code";
  if (filter === "gemini") return tool === "gemini" || tool === "gemini-cli";
  return tool === filter;
}

const filteredSessions = computed(() => {
  let sessions = projectSessions.value;
  if (toolFilter.value !== "all")
    sessions = sessions.filter((s) => toolMatches(toolFilter.value, s.tool));
  return sessions;
});

const filteredScannedSessions = computed(() => {
  let sessions = scannedSessions.value;
  if (toolFilter.value !== "all")
    sessions = sessions.filter((s) => toolMatches(toolFilter.value, s.toolName));
  return sessions;
});

const hasAnySessions = computed(
  () => filteredSessions.value.length > 0 || filteredScannedSessions.value.length > 0,
);

const selectedSession = computed(() =>
  projectSessions.value.find((s) => s.id === selectedSessionId.value),
);

const selectedScannedSession = computed(() =>
  scannedSessions.value.find((s) => s.id === selectedSessionId.value),
);

const availableTasks = computed(() =>
  projectTasks.value.filter((t) => t.status !== "completed"),
);

function selectSession(id: string) {
  selectedSessionId.value = selectedSessionId.value === id ? null : id;
  const session = projectSessions.value.find((s) => s.id === id);
  if (session) {
    addRecentItem({
      type: "session",
      title: session.title,
      projectId: session.projectId,
      sourceId: session.id,
    });
  }
}

function selectScannedSession(id: string) {
  selectedSessionId.value = selectedSessionId.value === id ? null : id;
  const session = scannedSessions.value.find((s) => s.id === id);
  if (session) {
    addRecentItem({
      type: "agent_session",
      title: session.title,
      sourcePath: session.sessionPath,
    });
  }
}

async function handleScanSessions() {
  const workspaceCwd = currentProject.value?.folderPath;
  if (!workspaceCwd) {
    scanError.value = "Open a project first — sessions are scoped to that folder.";
    return;
  }

  isScanning.value = true;
  scanError.value = null;

  try {
    const allScanned = await scanConfiguredSessions(
      settings.value.sessionDirs,
      100,
      workspaceCwd,
    );

    scannedSessions.value = allScanned;
    lastScanTime.value = new Date().toISOString();

    if (allScanned.length > 0) {
      addRecentItem({
        type: "agent_session",
        title: `Scanned ${allScanned.length} sessions for ${currentProject.value?.name || "project"}`,
        sourcePath: workspaceCwd,
        projectId: currentProject.value?.id,
      });
    }
  } catch (error) {
    console.error("Scan failed:", error);
    scanError.value = error instanceof Error ? error.message : "Scan failed";
  } finally {
    isScanning.value = false;
  }
}

// Auto-scan when the open project changes — show that workspace's agent sessions.
watch(
  () => currentProject.value?.folderPath,
  (folderPath) => {
    if (folderPath) {
      void handleScanSessions();
    } else {
      scannedSessions.value = [];
      lastScanTime.value = null;
    }
  },
  { immediate: true },
);

function handleDelete(id: string) {
  if (
    confirm(
      "Remove this session from Openmesh index? (Original files are not deleted)",
    )
  ) {
    deleteAgentSession(id);
    if (selectedSessionId.value === id) selectedSessionId.value = null;
  }
}

function handleDeleteScanned(id: string) {
  if (
    confirm(
      "Remove this scanned session from the list? (Original files are not deleted)",
    )
  ) {
    scannedSessions.value = scannedSessions.value.filter((s) => s.id !== id);
    starredScanIds.value.delete(id);
    if (selectedSessionId.value === id) selectedSessionId.value = null;
  }
}

function toggleStarredScan(id: string) {
  if (starredScanIds.value.has(id)) {
    starredScanIds.value.delete(id);
  } else {
    starredScanIds.value.add(id);
  }
}

function attachSessionToTask(sessionId: string, taskId: string) {
  updateAgentSession(sessionId, { linkedTaskId: taskId });
}

function resumeToolName(toolName: string): string | null {
  const t = toolName.toLowerCase();
  if (t.includes("codex")) return "codex";
  if (t.includes("claude")) return "claude";
  if (t.includes("opencode")) return "opencode";
  return null;
}

/** Prefer filename stem as CLI resume id (Codex/Claude session files). */
function resumeSessionIdFor(session: ScannedSession): string {
  const stem = session.fileName.replace(/\.(jsonl?|json)$/i, "");
  return stem || session.id;
}

async function resumeScannedInTerminal(session: ScannedSession) {
  resumeError.value = null;
  const tool = resumeToolName(session.toolName);
  const cwd = currentProject.value?.folderPath;
  if (!tool || !cwd) {
    resumeError.value =
      "Resume is available for Codex, Claude, and OpenCode when a project is open.";
    return;
  }
  resumeBusy.value = true;
  try {
    const cliPath =
      tool === "codex"
        ? settings.value?.agentClis?.codexPath
        : tool === "claude"
          ? settings.value?.agentClis?.claudeCodePath
          : settings.value?.agentClis?.opencodePath;
    const result = await openAgentCli(tool, cwd, cliPath || undefined, {
      resumeSessionId: resumeSessionIdFor(session),
    });
    if (!result.success) {
      resumeError.value = result.error || "Failed to launch terminal";
    }
  } catch (e) {
    resumeError.value = e instanceof Error ? e.message : String(e);
  } finally {
    resumeBusy.value = false;
  }
}

function timeAgo(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hrs = Math.floor(mins / 60);
  if (hrs < 24) return `${hrs}h ago`;
  return `${Math.floor(hrs / 24)}d ago`;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
</script>

<template>
  <div class="space-y-8 animate-fade-in">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-title">Agent Sessions</h1>
        <p class="text-body text-muted mt-1">
          {{
            currentProject
              ? `Sessions from Codex, Claude, Cursor, OpenCode, Gemini, and Grok under ${currentProject.folderPath}`
              : "Open a project to see agent sessions for that workspace."
          }}
        </p>
      </div>
      <button
        @click="handleScanSessions"
        :disabled="isScanning || !currentProject"
        class="btn-primary flex items-center gap-2 disabled:opacity-50"
      >
        <Scan class="h-4 w-4" />
        {{ isScanning ? "Scanning..." : "Refresh" }}
      </button>
    </div>

    <div v-if="lastScanTime" class="text-[12px] text-muted">
      Last scan: {{ timeAgo(lastScanTime) }}
      <span v-if="currentProject"> · scoped to this project</span>
    </div>
    <p v-if="scanError" class="text-[12px]" style="color: #ef4444">{{ scanError }}</p>

    <!-- Filters -->
    <div class="flex gap-1.5 flex-wrap">
      <button
        v-for="f in AGENT_TOOL_FILTERS"
        :key="f"
        @click="toolFilter = f"
        class="btn-ghost inline-flex items-center gap-1.5"
        :class="
          toolFilter === f
            ? '!bg-[var(--surface-2)] !text-[var(--foreground)]'
            : ''
        "
      >
        <AgentToolIcon v-if="f !== 'all'" :tool="f" :size="14" />
        {{ f === "all" ? "All" : agentToolLabel(f) }}
      </button>
    </div>

    <div v-if="!hasAnySessions" class="workbench-card p-12 text-center space-y-3">
      <div
        class="flex h-16 w-16 mx-auto mb-2 items-center justify-center rounded-2xl"
        style="background: var(--surface-2); border: 1px solid var(--border)"
      >
        <Bot class="h-7 w-7 text-subtle" />
      </div>
      <p class="text-[15px] font-semibold">No agent sessions yet</p>
      <p class="text-sm text-muted max-w-md mx-auto">
        {{
          currentProject
            ? "No foreign agent sessions found for this project folder yet. Run Codex/Claude/Cursor/Grok here, then refresh."
            : "Select a project — OpenMesh lists agent sessions for that workspace path by default."
        }}
      </p>
      <button
        type="button"
        class="btn-primary inline-flex items-center gap-2 mx-auto mt-2"
        :disabled="isScanning || !currentProject"
        @click="handleScanSessions"
      >
        <Scan class="h-4 w-4" />
        {{ isScanning ? "Scanning…" : "Refresh sessions" }}
      </button>
    </div>

    <div v-else class="grid grid-cols-1 lg:grid-cols-3 gap-6">
      <!-- Session list: disk first, then project-saved -->
      <div class="lg:col-span-2 space-y-2">
        <button
          v-for="session in filteredScannedSessions"
          :key="session.id"
          @click="selectScannedSession(session.id)"
          class="w-full text-left workbench-card-compact p-4 transition-all"
          :class="
            selectedSessionId === session.id
              ? '!border-[rgba(255,255,255,0.12)]'
              : ''
          "
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <AgentToolIcon :tool="session.toolName" :size="16" />
              <span class="text-[13px] font-medium">{{ session.title }}</span>
            </div>
            <span class="text-[12px] text-muted">{{
              timeAgo(session.lastActiveAt)
            }}</span>
          </div>
          <div class="flex items-center gap-2 mt-2">
            <span class="text-[11px] text-muted">{{
              agentToolLabel(session.toolName)
            }}</span>
            <span class="text-[11px] text-muted">{{
              formatBytes(session.fileSizeBytes)
            }}</span>
            <span v-if="starredScanIds.has(session.id)" class="text-[12px]">⭐</span>
            <span class="badge badge-success">On disk</span>
          </div>
        </button>

        <button
          v-for="session in filteredSessions"
          :key="session.id"
          @click="selectSession(session.id)"
          class="w-full text-left workbench-card-compact p-4 transition-all"
          :class="
            selectedSessionId === session.id
              ? '!border-[rgba(255,255,255,0.12)]'
              : ''
          "
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2.5">
              <AgentToolIcon :tool="session.tool" :size="16" />
              <span class="text-[13px] font-medium">{{ session.title }}</span>
            </div>
            <span class="text-[12px] text-muted">{{
              timeAgo(session.lastActiveAt)
            }}</span>
          </div>
          <div class="flex items-center gap-2 mt-2">
            <span class="text-[11px] text-muted">{{
              agentToolLabel(session.tool)
            }}</span>
            <span
              class="badge"
              :style="{
                background:
                  session.status === 'active'
                    ? 'rgba(34, 197, 94, 0.1)'
                    : 'rgba(107, 114, 128, 0.1)',
                color: session.status === 'active' ? '#22c55e' : '#6b7280',
              }"
              >{{ session.status }}</span
            >
            <span class="text-[11px] text-muted">{{
              session.tool
            }}</span>
            <span v-if="session.isImportant" class="text-[12px]">⭐</span>
            <span class="badge chip-muted">Saved</span>
          </div>
        </button>
      </div>

      <!-- Session detail: saved -->
      <div v-if="selectedSession" class="workbench-card-compact p-5 space-y-4 self-start">
        <div class="flex items-center justify-between">
          <h3 class="text-[14px] font-semibold">{{ selectedSession.title }}</h3>
          <span class="badge chip-muted">Saved</span>
        </div>
        <div class="text-[12px] space-y-1.5 text-muted">
          <p class="flex items-center gap-1.5">
            Tool:
            <AgentToolIcon :tool="selectedSession.tool" :size="13" />
            {{ agentToolLabel(selectedSession.tool) }}
          </p>
          <p>Status: {{ selectedSession.status }}</p>
          <p>Last active: {{ timeAgo(selectedSession.lastActiveAt) }}</p>
          <p v-if="selectedSession.linkedTaskId">
            Linked task: {{ selectedSession.linkedTaskId }}
          </p>
        </div>
        <p v-if="selectedSession.summary" class="text-[13px] leading-relaxed">
          {{ selectedSession.summary }}
        </p>
        <div
          v-if="selectedSession.changedFiles?.length"
          class="text-[12px]"
        >
          <p class="font-medium mb-1.5">Changed files:</p>
          <ul class="space-y-0.5 text-muted">
            <li
              v-for="f in selectedSession.changedFiles"
              :key="f"
              class="font-mono text-[11px]"
              >{{ f }}</li
            >
          </ul>
        </div>
        <!-- Attach to task -->
        <div v-if="availableTasks.length > 0 && currentProject">
          <p class="text-[12px] font-medium mb-1.5">Attach to task:</p>
          <select
            @change="
              attachSessionToTask(
                selectedSession.id,
                ($event.target as HTMLSelectElement).value,
              )
            "
            class="input-luxury w-full"
          >
            <option value="">— select task —</option>
            <option
              v-for="task in availableTasks"
              :key="task.id"
              :value="task.id"
              >{{ task.title }}</option
            >
          </select>
        </div>
        <div class="flex gap-2 flex-wrap pt-1">
          <button
            @click="
              updateAgentSession(selectedSession.id, {
                isImportant: !selectedSession.isImportant,
              })
            "
            class="btn-secondary text-[12px]"
          >
            {{ selectedSession.isImportant ? "Unmark Important" : "Mark Important" }}
          </button>
          <button
            @click="handleDelete(selectedSession.id)"
            class="btn-ghost text-[12px]"
            style="color: #ef4444"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>

      <!-- Scanned session detail -->
      <div v-if="selectedScannedSession" class="workbench-card-compact p-5 space-y-4 self-start">
        <div class="flex items-center justify-between">
          <h3 class="text-[14px] font-semibold">{{ selectedScannedSession.title }}</h3>
          <span
            class="badge badge-success"
            >Real</span
          >
        </div>
        <div class="text-[12px] space-y-1.5 text-muted">
          <p class="flex items-center gap-1.5">
            Tool:
            <AgentToolIcon :tool="selectedScannedSession.toolName" :size="13" />
            {{ agentToolLabel(selectedScannedSession.toolName) }}
          </p>
          <p>File: {{ selectedScannedSession.fileName }}</p>
          <p>Size: {{ formatBytes(selectedScannedSession.fileSizeBytes) }}</p>
          <p>Last active: {{ timeAgo(selectedScannedSession.lastActiveAt) }}</p>
          <p class="break-all font-mono text-[11px]">
            Path: {{ selectedScannedSession.sessionPath }}
          </p>
        </div>
        <div
          v-if="selectedScannedSession.summaryPreview"
          class="text-[12px]"
        >
          <p class="font-medium mb-1.5">Preview:</p>
          <p
            class="whitespace-pre-wrap leading-relaxed text-muted"
            >{{ selectedScannedSession.summaryPreview }}</p
          >
        </div>
        <p v-if="resumeError" class="text-[12px]" style="color: #ef4444">
          {{ resumeError }}
        </p>
        <div class="flex gap-2 flex-wrap pt-1">
          <button
            v-if="resumeToolName(selectedScannedSession.toolName)"
            type="button"
            class="btn-primary text-[12px] inline-flex items-center gap-1.5"
            :disabled="resumeBusy"
            @click="resumeScannedInTerminal(selectedScannedSession)"
          >
            <Play class="h-3.5 w-3.5" />
            {{ resumeBusy ? "Launching…" : "Resume in terminal" }}
          </button>
          <button
            @click="toggleStarredScan(selectedScannedSession.id)"
            class="btn-secondary text-[12px]"
          >
            {{ starredScanIds.has(selectedScannedSession.id) ? "⭐ Unstar" : "⭐ Star" }}
          </button>
          <button
            @click="handleDeleteScanned(selectedScannedSession.id)"
            class="btn-ghost text-[12px]"
            style="color: #ef4444"
          >
            <Trash2 class="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
