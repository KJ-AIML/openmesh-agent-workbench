<script setup lang="ts">
import { ref, computed } from "vue";
import { useStore } from "../lib/useStore";
import * as agentSessionAdapter from "../lib/adapters/agentSessionAdapter";
import type { ScannedSession } from "../lib/adapters/types";

const {
  currentProject, projectSessions, projectTasks,
  deleteAgentSession, updateAgentSession, addRecentItem, settings
} = useStore();
const selectedSessionId = ref<string | null>(null);
const toolFilter = ref<string>("all");
const scannedSessions = ref<ScannedSession[]>([]);
const isScanning = ref(false);
const lastScanTime = ref<string | null>(null);
// Track which scanned sessions the user has "starred" (persisted to a simple Set in-memory for v0)
const starredScanIds = ref<Set<string>>(new Set());

const filteredSessions = computed(() => {
  let sessions = projectSessions.value;
  if (toolFilter.value !== "all") sessions = sessions.filter((s) => s.tool === toolFilter.value);
  return sessions;
});

const filteredScannedSessions = computed(() => {
  let sessions = scannedSessions.value;
  if (toolFilter.value !== "all") sessions = sessions.filter((s) => s.toolName === toolFilter.value);
  return sessions;
});

const selectedSession = computed(() =>
  projectSessions.value.find((s) => s.id === selectedSessionId.value)
);

const selectedScannedSession = computed(() =>
  scannedSessions.value.find((s) => s.id === selectedSessionId.value)
);

// Tasks available for linking
const availableTasks = computed(() => projectTasks.value.filter((t) => t.status !== "completed"));

function toolIcon(tool: string): string {
  const map: Record<string, string> = { codex: "⚡", "claude": "🟠", "claude-code": "🟠", opencode: "🔵", cursor: "🖱️", "gemini-cli": "💎" };
  return map[tool] ?? "🤖";
}

function selectSession(id: string) {
  selectedSessionId.value = selectedSessionId.value === id ? null : id;
  const session = projectSessions.value.find((s) => s.id === id);
  if (session) {
    addRecentItem({ type: "session", title: session.title, projectId: session.projectId, sourceId: session.id });
  }
}

function selectScannedSession(id: string) {
  selectedSessionId.value = selectedSessionId.value === id ? null : id;
  const session = scannedSessions.value.find((s) => s.id === id);
  if (session) {
    addRecentItem({ type: "agent_session", title: session.title, sourcePath: session.sessionPath });
  }
}

async function handleScanSessions() {
  if (!settings.value.sessionDirs) return;

  isScanning.value = true;
  const allScanned: ScannedSession[] = [];

  try {
    // Scan Codex sessions
    if (settings.value.sessionDirs.codexEnabled && settings.value.sessionDirs.codexDir) {
      const result = await agentSessionAdapter.scanAgentSessionDirectory("codex", settings.value.sessionDirs.codexDir, 100);
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
    }

    // Scan Claude Code sessions
    if (settings.value.sessionDirs.claudeCodeEnabled && settings.value.sessionDirs.claudeCodeDir) {
      const result = await agentSessionAdapter.scanAgentSessionDirectory("claude-code", settings.value.sessionDirs.claudeCodeDir, 100);
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
    }

    // Scan OpenCode sessions
    if (settings.value.sessionDirs.opencodeEnabled && settings.value.sessionDirs.opencodeDir) {
      const result = await agentSessionAdapter.scanAgentSessionDirectory("opencode", settings.value.sessionDirs.opencodeDir, 100);
      if (result.success && result.data) {
        allScanned.push(...result.data);
      }
    }

    scannedSessions.value = allScanned;
    lastScanTime.value = new Date().toISOString();

    if (allScanned.length > 0) {
      addRecentItem({ type: "agent_session", title: `Scanned ${allScanned.length} sessions`, sourcePath: "scan" });
    }
  } catch (error) {
    console.error("Scan failed:", error);
  } finally {
    isScanning.value = false;
  }
}

function handleDelete(id: string) {
  if (confirm("Remove this session from Openmesh index? (Original files are not deleted)")) {
    deleteAgentSession(id);
    if (selectedSessionId.value === id) selectedSessionId.value = null;
  }
}

function handleDeleteScanned(id: string) {
  if (confirm("Remove this scanned session from the list? (Original files are not deleted)")) {
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
  <div class="space-y-6">
    <div class="flex items-center justify-between">
      <div>
        <h1 class="text-2xl font-bold">Agent Sessions</h1>
        <p class="text-sm mt-1" style="color: var(--muted-foreground)">AI tool session index.</p>
      </div>
      <button
        @click="handleScanSessions"
        :disabled="isScanning"
        class="text-xs px-3 py-1.5 rounded-md"
        style="background: var(--foreground); color: var(--background)"
        :class="{ 'opacity-50 cursor-not-allowed': isScanning }"
      >
        {{ isScanning ? 'Scanning...' : 'Scan Sessions' }}
      </button>
    </div>

    <div v-if="lastScanTime" class="text-xs" style="color: var(--muted-foreground)">
      Last scan: {{ timeAgo(lastScanTime) }}
    </div>

    <!-- Filters -->
    <div class="flex gap-2">
      <button
        v-for="f in ['all', 'codex', 'claude-code', 'opencode']"
        :key="f"
        @click="toolFilter = f"
        class="text-xs px-2 py-1 rounded-md transition-colors"
        :style="{
          background: toolFilter === f ? 'var(--foreground)' : 'transparent',
          color: toolFilter === f ? 'var(--background)' : 'var(--muted-foreground)',
          border: '1px solid var(--border)',
        }"
      >
        {{ f === 'all' ? 'All' : f }}
      </button>
    </div>

    <div v-if="filteredSessions.length === 0 && filteredScannedSessions.length === 0" class="rounded-lg border p-8 text-center" style="border-color: var(--border)">
      <p class="text-lg font-medium">No agent sessions found</p>
      <p class="text-sm mt-2" style="color: var(--muted-foreground)">
        {{ currentProject ? 'No sessions for this project yet.' : 'Select a project to see sessions.' }}
      </p>
      <p class="text-xs mt-2" style="color: var(--muted-foreground)">
        Configure session directories in Settings and click "Scan Sessions" to find real sessions.
      </p>
    </div>

    <div v-else class="flex gap-4">
      <!-- Session list -->
      <div class="flex-1 space-y-2">
        <!-- Mock sessions -->
        <button
          v-for="session in filteredSessions"
          :key="session.id"
          @click="selectSession(session.id)"
          class="w-full text-left rounded-lg border p-3 transition-colors"
          style="border-color: var(--border)"
          :class="{ 'ring-1 ring-[var(--foreground)]': selectedSessionId === session.id }"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span>{{ toolIcon(session.tool) }}</span>
              <span class="text-sm font-medium">{{ session.title }}</span>
            </div>
            <span class="text-xs" style="color: var(--muted-foreground)">{{ timeAgo(session.lastActiveAt) }}</span>
          </div>
          <div class="flex items-center gap-2 mt-1">
            <span
              class="text-[10px] px-1.5 py-0.5 rounded-full"
              :style="{ background: session.status === 'active' ? '#22c55e20' : '#6b728020', color: session.status === 'active' ? '#22c55e' : '#6b7280' }"
            >
              {{ session.status }}
            </span>
            <span class="text-xs" style="color: var(--muted-foreground)">{{ session.tool }}</span>
            <span v-if="session.isImportant" class="text-[10px]">⭐</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #f59e0b20; color: #f59e0b">Mock</span>
          </div>
        </button>

        <!-- Real scanned sessions -->
        <button
          v-for="session in filteredScannedSessions"
          :key="session.id"
          @click="selectScannedSession(session.id)"
          class="w-full text-left rounded-lg border p-3 transition-colors"
          style="border-color: var(--border)"
          :class="{ 'ring-1 ring-[var(--foreground)]': selectedSessionId === session.id }"
        >
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <span>{{ toolIcon(session.toolName) }}</span>
              <span class="text-sm font-medium">{{ session.title }}</span>
            </div>
            <span class="text-xs" style="color: var(--muted-foreground)">{{ timeAgo(session.lastActiveAt) }}</span>
          </div>
          <div class="flex items-center gap-2 mt-1">
            <span class="text-xs" style="color: var(--muted-foreground)">{{ session.toolName }}</span>
            <span class="text-[10px]" style="color: var(--muted-foreground)">{{ formatBytes(session.fileSizeBytes) }}</span>
            <span v-if="starredScanIds.has(session.id)" class="text-[10px]">⭐</span>
            <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #22c55e20; color: #22c55e">Real</span>
          </div>
        </button>
      </div>

      <!-- Session detail: mock session -->
      <div v-if="selectedSession" class="w-80 rounded-lg border p-4 space-y-3 self-start" style="border-color: var(--border)">
        <div class="flex items-center justify-between">
          <h3 class="font-semibold">{{ selectedSession.title }}</h3>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #f59e0b20; color: #f59e0b">Mock</span>
        </div>
        <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Tool: {{ selectedSession.tool }}</p>
          <p>Status: {{ selectedSession.status }}</p>
          <p>Last active: {{ timeAgo(selectedSession.lastActiveAt) }}</p>
          <p v-if="selectedSession.linkedTaskId">Linked task: {{ selectedSession.linkedTaskId }}</p>
        </div>
        <p v-if="selectedSession.summary" class="text-sm">{{ selectedSession.summary }}</p>
        <div v-if="selectedSession.changedFiles?.length" class="text-xs">
          <p class="font-medium mb-1">Changed files:</p>
          <ul class="space-y-0.5" style="color: var(--muted-foreground)">
            <li v-for="f in selectedSession.changedFiles" :key="f">{{ f }}</li>
          </ul>
        </div>
        <!-- Attach to task -->
        <div v-if="availableTasks.length > 0 && currentProject">
          <p class="text-xs font-medium mb-1">Attach to task:</p>
          <select
            @change="attachSessionToTask(selectedSession.id, ($event.target as HTMLSelectElement).value)"
            class="w-full rounded-md border px-2 py-1 text-xs"
            style="background: var(--background); border-color: var(--border); color: var(--foreground)"
          >
            <option value="">— select task —</option>
            <option v-for="task in availableTasks" :key="task.id" :value="task.id">{{ task.title }}</option>
          </select>
        </div>
        <div class="flex gap-2 flex-wrap">
          <button
            @click="updateAgentSession(selectedSession.id, { isImportant: !selectedSession.isImportant })"
            class="text-xs px-2 py-1 rounded-md"
            style="border: 1px solid var(--border); color: var(--muted-foreground)"
          >
            {{ selectedSession.isImportant ? 'Unmark Important' : 'Mark Important' }}
          </button>
          <button
            @click="handleDelete(selectedSession.id)"
            class="text-xs px-2 py-1 rounded-md"
            style="border: 1px solid #ef444440; color: #ef4444"
          >
            Delete from Index
          </button>
        </div>
      </div>

      <!-- Scanned session detail -->
      <div v-if="selectedScannedSession" class="w-80 rounded-lg border p-4 space-y-3 self-start" style="border-color: var(--border)">
        <div class="flex items-center justify-between">
          <h3 class="font-semibold">{{ selectedScannedSession.title }}</h3>
          <span class="text-[10px] px-1.5 py-0.5 rounded-full" style="background: #22c55e20; color: #22c55e">Real</span>
        </div>
        <div class="text-xs space-y-1" style="color: var(--muted-foreground)">
          <p>Tool: {{ selectedScannedSession.toolName }}</p>
          <p>File: {{ selectedScannedSession.fileName }}</p>
          <p>Size: {{ formatBytes(selectedScannedSession.fileSizeBytes) }}</p>
          <p>Last active: {{ timeAgo(selectedScannedSession.lastActiveAt) }}</p>
          <p class="break-all">Path: {{ selectedScannedSession.sessionPath }}</p>
        </div>
        <div v-if="selectedScannedSession.summaryPreview" class="text-xs">
          <p class="font-medium mb-1">Preview:</p>
          <p class="whitespace-pre-wrap" style="color: var(--muted-foreground)">{{ selectedScannedSession.summaryPreview }}</p>
        </div>
        <div class="flex gap-2 flex-wrap">
          <button
            @click="toggleStarredScan(selectedScannedSession.id)"
            class="text-xs px-2 py-1 rounded-md"
            style="border: 1px solid var(--border); color: var(--muted-foreground)"
          >
            {{ starredScanIds.has(selectedScannedSession.id) ? '⭐ Unstar' : '⭐ Star' }}
          </button>
          <button
            @click="handleDeleteScanned(selectedScannedSession.id)"
            class="text-xs px-2 py-1 rounded-md"
            style="border: 1px solid #ef444440; color: #ef4444"
          >
            Remove from List
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
