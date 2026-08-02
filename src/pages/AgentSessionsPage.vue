<script setup lang="ts">
import { ref, computed } from "vue";
import { useStore } from "../lib/useStore";
import * as agentSessionAdapter from "../lib/adapters/agentSessionAdapter";
import type { ScannedSession } from "../lib/adapters/types";
import { Bot, Scan, Star, Trash2, FolderOpen } from "lucide-vue-next";

const {
  currentProject,
  projectSessions,
  projectTasks,
  deleteAgentSession,
  updateAgentSession,
  addRecentItem,
  settings,
} = useStore();

const selectedSessionId = ref<string | null>(null);
const toolFilter = ref<string>("all");
const scannedSessions = ref<ScannedSession[]>([]);
const isScanning = ref(false);
const lastScanTime = ref<string | null>(null);
const starredScanIds = ref<Set<string>>(new Set());

/** Saved project sessions. Prefer scanned disk sessions in the list order. */
const filteredSessions = computed(() => {
  let sessions = projectSessions.value;
  if (toolFilter.value !== "all")
    sessions = sessions.filter((s) => s.tool === toolFilter.value);
  return sessions;
});

const filteredScannedSessions = computed(() => {
  let sessions = scannedSessions.value;
  if (toolFilter.value !== "all")
    sessions = sessions.filter((s) => s.toolName === toolFilter.value);
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

function toolIcon(tool: string): string {
  const map: Record<string, string> = {
    codex: "⚡",
    claude: "",
    "claude-code": "",
    opencode: "🔵",
    cursor: "🖱️",
    "gemini-cli": "💎",
  };
  return map[tool] ?? "";
}

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
  if (!settings.value.sessionDirs) return;

  isScanning.value = true;
  const allScanned: ScannedSession[] = [];

  try {
    if (
      settings.value.sessionDirs.codexEnabled &&
      settings.value.sessionDirs.codexDir
    ) {
      const result = await agentSessionAdapter.scanAgentSessionDirectory(
        "codex",
        settings.value.sessionDirs.codexDir,
        100,
      );
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
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
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
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
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
    }

    scannedSessions.value = allScanned;
    lastScanTime.value = new Date().toISOString();

    if (allScanned.length > 0) {
      addRecentItem({
        type: "agent_session",
        title: `Scanned ${allScanned.length} sessions`,
        sourcePath: "scan",
      });
    }
  } catch (error) {
    console.error("Scan failed:", error);
  } finally {
    isScanning.value = false;
  }
}

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
          AI tool session index.
        </p>
      </div>
      <button
        @click="handleScanSessions"
        :disabled="isScanning"
        class="btn-primary flex items-center gap-2 disabled:opacity-50"
      >
        <Scan class="h-4 w-4" />
        {{ isScanning ? "Scanning..." : "Scan Sessions" }}
      </button>
    </div>

    <div v-if="lastScanTime" class="text-[12px] text-muted">
      Last scan: {{ timeAgo(lastScanTime) }}
    </div>

    <!-- Filters -->
    <div class="flex gap-1.5">
      <button
        v-for="f in ['all', 'codex', 'claude-code', 'opencode']"
        :key="f"
        @click="toolFilter = f"
        class="btn-ghost"
        :class="
          toolFilter === f
            ? '!bg-[var(--surface-2)] !text-[var(--foreground)]'
            : ''
        "
      >
        {{ f === "all" ? "All" : f }}
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
            ? "Scan local tool session directories, or launch an agent from Home / Dev Connector."
            : "Select a project, then scan session directories."
        }}
      </p>
      <button
        type="button"
        class="btn-primary inline-flex items-center gap-2 mx-auto mt-2"
        :disabled="isScanning || !settings.sessionDirs"
        @click="handleScanSessions"
      >
        <Scan class="h-4 w-4" />
        {{ isScanning ? "Scanning…" : "Scan sessions" }}
      </button>
      <p class="text-[11px] text-subtle">
        Configure directories in Settings → Session dirs.
      </p>
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
              <span class="text-[16px]">{{ toolIcon(session.toolName) }}</span>
              <span class="text-[13px] font-medium">{{ session.title }}</span>
            </div>
            <span class="text-[12px] text-muted">{{
              timeAgo(session.lastActiveAt)
            }}</span>
          </div>
          <div class="flex items-center gap-2 mt-2">
            <span class="text-[11px] text-muted">{{
              session.toolName
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
              <span class="text-[16px]">{{ toolIcon(session.tool) }}</span>
              <span class="text-[13px] font-medium">{{ session.title }}</span>
            </div>
            <span class="text-[12px] text-muted">{{
              timeAgo(session.lastActiveAt)
            }}</span>
          </div>
          <div class="flex items-center gap-2 mt-2">
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
          <p>Tool: {{ selectedSession.tool }}</p>
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
          <p>Tool: {{ selectedScannedSession.toolName }}</p>
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
        <div class="flex gap-2 flex-wrap pt-1">
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
